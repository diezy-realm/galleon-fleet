package sqlite

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/src/run"
	"github.com/diezy-realm/galleon-fleet/engine/src/task"
)

func setupTestDB(t *testing.T) (*DB, string) {
	t.Helper()
	tempDir, err := os.MkdirTemp("", "galleon-sqlite-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}

	dbPath := filepath.Join(tempDir, "test.db")
	db, err := Open(dbPath)
	if err != nil {
		_ = os.RemoveAll(tempDir)
		t.Fatalf("Open failed: %v", err)
	}

	return db, tempDir
}

func TestOpen_WALAndPragmas(t *testing.T) {
	db, tempDir := setupTestDB(t)
	defer db.Close()
	defer os.RemoveAll(tempDir)

	var journalMode string
	err := db.QueryRow("PRAGMA journal_mode;").Scan(&journalMode)
	if err != nil {
		t.Fatalf("failed to query journal_mode: %v", err)
	}
	if journalMode != "wal" {
		t.Errorf("expected journal_mode 'wal', got '%s'", journalMode)
	}
}

func TestRunStore_CRUDAndTransitions(t *testing.T) {
	db, tempDir := setupTestDB(t)
	defer db.Close()
	defer os.RemoveAll(tempDir)

	runStore := NewRunStore(db)
	ctx := context.Background()

	now := time.Now().UTC().Truncate(time.Second)
	testRun := &run.Run{
		ID:        "run-101",
		CrewID:    "crew-dev",
		Status:    run.StatusQueued,
		CreatedAt: now,
		Input: run.RunInput{
			Prompt: "Test sqlite migration",
		},
	}

	// 1. Save Run
	if err := runStore.Save(ctx, testRun); err != nil {
		t.Fatalf("Save failed: %v", err)
	}

	// 2. Get Run
	fetched, err := runStore.Get(ctx, "run-101")
	if err != nil {
		t.Fatalf("Get failed: %v", err)
	}
	if fetched.ID != "run-101" || fetched.Status != run.StatusQueued {
		t.Errorf("unexpected fetched run: %+v", fetched)
	}

	// 3. Update Status (valid transition: queued -> planning)
	if err := runStore.UpdateStatus(ctx, "run-101", run.StatusPlanning, ""); err != nil {
		t.Fatalf("UpdateStatus (queued -> planning) failed: %v", err)
	}

	// 4. Update Status (valid transition: planning -> running)
	if err := runStore.UpdateStatus(ctx, "run-101", run.StatusRunning, ""); err != nil {
		t.Fatalf("UpdateStatus (planning -> running) failed: %v", err)
	}

	// 5. Update Status (invalid transition: running -> queued) should error
	if err := runStore.UpdateStatus(ctx, "run-101", run.StatusQueued, ""); err == nil {
		t.Fatalf("expected error for invalid transition running -> queued, got nil")
	}

	// 6. List runs
	list, err := runStore.List(ctx)
	if err != nil {
		t.Fatalf("List failed: %v", err)
	}
	if len(list) != 1 || list[0].ID != "run-101" {
		t.Errorf("expected 1 run in list, got %d", len(list))
	}
}

func TestRunStore_Events(t *testing.T) {
	db, tempDir := setupTestDB(t)
	defer db.Close()
	defer os.RemoveAll(tempDir)

	runStore := NewRunStore(db)
	ctx := context.Background()

	testRun := &run.Run{
		ID:     "run-events-test",
		Status: run.StatusRunning,
	}
	if err := runStore.Save(ctx, testRun); err != nil {
		t.Fatalf("Save run failed: %v", err)
	}

	t1 := time.Now().UTC().Add(-2 * time.Second)
	t2 := time.Now().UTC().Add(-1 * time.Second)

	ev1 := &run.RunEvent{
		EventID:   "ev-1",
		RunID:     "run-events-test",
		Sequence:  1,
		Type:      "thought",
		Timestamp: t1,
		Payload:   map[string]any{"thought": "evaluating"},
	}
	ev2 := &run.RunEvent{
		EventID:   "ev-2",
		RunID:     "run-events-test",
		Sequence:  2,
		Type:      "tool_call",
		Timestamp: t2,
		Payload:   map[string]any{"tool": "bash"},
	}

	if err := runStore.AppendEvent(ctx, ev1); err != nil {
		t.Fatalf("AppendEvent ev1 failed: %v", err)
	}
	if err := runStore.AppendEvent(ctx, ev2); err != nil {
		t.Fatalf("AppendEvent ev2 failed: %v", err)
	}

	replayed, err := runStore.ReplayEvents(ctx, "run-events-test")
	if err != nil {
		t.Fatalf("ReplayEvents failed: %v", err)
	}
	if len(replayed) != 2 {
		t.Fatalf("expected 2 replayed events, got %d", len(replayed))
	}
	if replayed[0].Type != "thought" || replayed[1].Type != "tool_call" {
		t.Errorf("unexpected replayed events order: %+v", replayed)
	}
}

func TestRunStore_ResumeInterruptedRuns(t *testing.T) {
	db, tempDir := setupTestDB(t)
	defer db.Close()
	defer os.RemoveAll(tempDir)

	runStore := NewRunStore(db)
	ctx := context.Background()

	_ = runStore.Save(ctx, &run.Run{ID: "run-active", Status: run.StatusRunning})
	_ = runStore.Save(ctx, &run.Run{ID: "run-done", Status: run.StatusCompleted})

	interrupted, err := runStore.ResumeInterruptedRuns(ctx)
	if err != nil {
		t.Fatalf("ResumeInterruptedRuns failed: %v", err)
	}
	if len(interrupted) != 1 || interrupted[0].ID != "run-active" {
		t.Errorf("expected 1 interrupted run 'run-active', got %d", len(interrupted))
	}
}

func TestTaskStore_CRUD(t *testing.T) {
	db, tempDir := setupTestDB(t)
	defer db.Close()
	defer os.RemoveAll(tempDir)

	taskStore := NewTaskStore(db)
	ctx := context.Background()

	testTask := &task.Task{
		ID:          "task-1",
		RunID:       "run-101",
		Title:       "Setup database",
		Description: "Configure sqlite",
		Status:      task.StatusPending,
	}

	if err := taskStore.Save(ctx, testTask); err != nil {
		t.Fatalf("Save task failed: %v", err)
	}

	fetched, err := taskStore.Get(ctx, "task-1")
	if err != nil {
		t.Fatalf("Get task failed: %v", err)
	}
	if fetched.ID != "task-1" || fetched.Status != task.StatusPending {
		t.Errorf("unexpected fetched task: %+v", fetched)
	}

	if err := taskStore.UpdateStatus(ctx, "task-1", task.StatusReady, ""); err != nil {
		t.Fatalf("UpdateStatus (pending -> ready) failed: %v", err)
	}

	tasks, err := taskStore.ListByRun(ctx, "run-101")
	if err != nil {
		t.Fatalf("ListByRun failed: %v", err)
	}
	if len(tasks) != 1 || tasks[0].ID != "task-1" {
		t.Errorf("expected 1 task in list, got %d", len(tasks))
	}
}

func TestMigrateFromDiskStore(t *testing.T) {
	db, tempDir := setupTestDB(t)
	defer db.Close()
	defer os.RemoveAll(tempDir)

	// Create legacy mock DiskStore layout
	legacyDir := filepath.Join(tempDir, "legacy")
	runDir := filepath.Join(legacyDir, "runs", "legacy-run-1")
	if err := os.MkdirAll(runDir, 0755); err != nil {
		t.Fatalf("MkdirAll failed: %v", err)
	}

	legacyRun := run.Run{
		ID:     "legacy-run-1",
		CrewID: "crew-legacy",
		Status: run.StatusCompleted,
		Input: run.RunInput{
			Prompt: "Legacy run prompt",
		},
	}
	data, _ := json.Marshal(legacyRun)
	_ = os.WriteFile(filepath.Join(runDir, "run.json"), data, 0644)

	legacyEvent := run.RunEvent{
		EventID:  "legacy-ev-1",
		RunID:    "legacy-run-1",
		Sequence: 1,
		Type:     "start",
	}
	evData, _ := json.Marshal(legacyEvent)
	_ = os.WriteFile(filepath.Join(runDir, "events.jsonl"), append(evData, '\n'), 0644)

	// Independent task
	tasksDir := filepath.Join(legacyDir, "tasks")
	_ = os.MkdirAll(tasksDir, 0755)
	legacyTask := task.Task{
		ID:     "legacy-task-1",
		RunID:  "legacy-run-1",
		Title:  "Legacy Task",
		Status: task.StatusCompleted,
	}
	tData, _ := json.Marshal(legacyTask)
	_ = os.WriteFile(filepath.Join(tasksDir, "legacy-task-1.json"), tData, 0644)

	ctx := context.Background()
	res, err := db.MigrateFromDiskStore(ctx, legacyDir)
	if err != nil {
		t.Fatalf("MigrateFromDiskStore failed: %v", err)
	}

	if res.RunsImported != 1 {
		t.Errorf("expected 1 run imported, got %d", res.RunsImported)
	}
	if res.EventsImported != 1 {
		t.Errorf("expected 1 event imported, got %d", res.EventsImported)
	}
	if res.TasksImported != 1 {
		t.Errorf("expected 1 task imported, got %d", res.TasksImported)
	}

	// Verify imported run is queryable
	runStore := NewRunStore(db)
	fetched, err := runStore.Get(ctx, "legacy-run-1")
	if err != nil {
		t.Fatalf("failed to query migrated run: %v", err)
	}
	if fetched.ID != "legacy-run-1" || fetched.Status != run.StatusCompleted {
		t.Errorf("unexpected migrated run content: %+v", fetched)
	}
}
