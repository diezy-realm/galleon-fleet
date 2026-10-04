package sqlite

import (
	"bufio"
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"

	appErrors "github.com/diezy-labs/claw-crew/engine/core/errors"
	"github.com/diezy-labs/claw-crew/engine/src/run"
	"github.com/diezy-labs/claw-crew/engine/src/task"
)

// MigrationResult summarizes the records imported during DiskStore migration.
type MigrationResult struct {
	RunsImported   int `json:"runs_imported"`
	TasksImported  int `json:"tasks_imported"`
	EventsImported int `json:"events_imported"`
}

// MigrateFromDiskStore performs an idempotent, one-way import of legacy DiskStore JSON/JSONL
// files into SQLite.
func (d *DB) MigrateFromDiskStore(ctx context.Context, legacyBaseDir string) (*MigrationResult, error) {
	if legacyBaseDir == "" {
		return &MigrationResult{}, nil
	}

	result := &MigrationResult{}

	// 1. Migrate runs from legacyBaseDir/runs/<runID>/
	runsDir := filepath.Join(legacyBaseDir, "runs")
	if entries, err := os.ReadDir(runsDir); err == nil {
		runStore := NewRunStore(d)
		for _, entry := range entries {
			if !entry.IsDir() {
				continue
			}
			runID := entry.Name()
			runFile := filepath.Join(runsDir, runID, "run.json")
			if data, err := os.ReadFile(runFile); err == nil {
				var r run.Run
				if err := json.Unmarshal(data, &r); err == nil && r.ID != "" {
					if err := runStore.Save(ctx, &r); err == nil {
						result.RunsImported++
					}
				}
			}

			// Migrate events from events.jsonl
			eventsFile := filepath.Join(runsDir, runID, "events.jsonl")
			if f, err := os.Open(eventsFile); err == nil {
				scanner := bufio.NewScanner(f)
				for scanner.Scan() {
					line := strings.TrimSpace(scanner.Text())
					if line == "" {
						continue
					}
					var ev run.RunEvent
					if err := json.Unmarshal([]byte(line), &ev); err == nil && ev.RunID != "" {
						if err := runStore.AppendEvent(ctx, &ev); err == nil {
							result.EventsImported++
						}
					}
				}
				_ = f.Close()
			}

			// Migrate tasks from tasks.json
			tasksFile := filepath.Join(runsDir, runID, "tasks.json")
			if data, err := os.ReadFile(tasksFile); err == nil {
				var tasks []*task.Task
				if err := json.Unmarshal(data, &tasks); err == nil {
					taskStore := NewTaskStore(d)
					for _, t := range tasks {
						if t != nil && t.ID != "" {
							if err := taskStore.Save(ctx, t); err == nil {
								result.TasksImported++
							}
						}
					}
				}
			}
		}
	}

	// 2. Migrate independent tasks from legacyBaseDir/tasks/<taskID>.json
	tasksDir := filepath.Join(legacyBaseDir, "tasks")
	if entries, err := os.ReadDir(tasksDir); err == nil {
		taskStore := NewTaskStore(d)
		for _, entry := range entries {
			if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".json") {
				continue
			}
			filePath := filepath.Join(tasksDir, entry.Name())
			if data, err := os.ReadFile(filePath); err == nil {
				var t task.Task
				if err := json.Unmarshal(data, &t); err == nil && t.ID != "" {
					if err := taskStore.Save(ctx, &t); err == nil {
						result.TasksImported++
					}
				}
			}
		}
	}

	// 3. Record migration entry in schema_migrations
	migrationVersion := 100 // Version 100 designates legacy DiskStore import
	_, err := d.ExecContext(ctx, "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?, CURRENT_TIMESTAMP)", migrationVersion)
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to record diskstore migration", appErrors.LayerRepository)
	}

	return result, nil
}
