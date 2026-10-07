package sqlite

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"time"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
	"github.com/diezy-realm/galleon-fleet/engine/src/task"
)

// TaskStore provides SQLite-backed storage for tasks.
type TaskStore struct {
	db *DB
}

// NewTaskStore creates a new TaskStore.
func NewTaskStore(db *DB) *TaskStore {
	return &TaskStore{db: db}
}

// Save persists or updates a Task in SQLite.
func (s *TaskStore) Save(ctx context.Context, t *task.Task) error {
	if t == nil || t.ID == "" {
		return appErrors.New(appErrors.CodeInvalidArgument, "task or task ID cannot be empty", appErrors.LayerRepository)
	}

	data, err := json.Marshal(t)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, "failed to marshal task", appErrors.LayerRepository)
	}

	query := `
	INSERT INTO tasks (id, run_id, description, status, error_msg, raw_json, created_at, updated_at)
	VALUES (?, ?, ?, ?, ?, ?, ?, ?)
	ON CONFLICT(id) DO UPDATE SET
		run_id = excluded.run_id,
		description = excluded.description,
		status = excluded.status,
		error_msg = excluded.error_msg,
		raw_json = excluded.raw_json,
		updated_at = excluded.updated_at;
	`

	now := time.Now().UTC()
	createdAt := t.CreatedAt
	if createdAt.IsZero() {
		createdAt = now
	}

	_, err = s.db.ExecContext(ctx, query, t.ID, t.RunID, t.Description, string(t.Status), t.ErrorMessage, string(data), createdAt, now)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to save task %s", t.ID), appErrors.LayerRepository)
	}
	return nil
}

// Get fetches a Task by ID from SQLite.
func (s *TaskStore) Get(ctx context.Context, taskID string) (*task.Task, error) {
	if taskID == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "task ID cannot be empty", appErrors.LayerRepository)
	}

	query := `SELECT raw_json FROM tasks WHERE id = ? LIMIT 1;`
	row := s.db.QueryRowContext(ctx, query, taskID)

	var rawJSON string
	if err := row.Scan(&rawJSON); err != nil {
		if err == sql.ErrNoRows {
			return nil, appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("task not found: %s", taskID), appErrors.LayerRepository)
		}
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to query task %s", taskID), appErrors.LayerRepository)
	}

	var t task.Task
	if err := json.Unmarshal([]byte(rawJSON), &t); err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to unmarshal task %s", taskID), appErrors.LayerRepository)
	}
	return &t, nil
}

// ListByRun returns all tasks associated with a given run ID.
func (s *TaskStore) ListByRun(ctx context.Context, runID string) ([]*task.Task, error) {
	if runID == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "run ID cannot be empty", appErrors.LayerRepository)
	}

	query := `SELECT raw_json FROM tasks WHERE run_id = ? ORDER BY created_at ASC;`
	rows, err := s.db.QueryContext(ctx, query, runID)
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to query tasks for run %s", runID), appErrors.LayerRepository)
	}
	defer rows.Close()

	var tasks []*task.Task
	for rows.Next() {
		var rawJSON string
		if err := rows.Scan(&rawJSON); err != nil {
			return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to scan task row", appErrors.LayerRepository)
		}
		var t task.Task
		if err := json.Unmarshal([]byte(rawJSON), &t); err != nil {
			continue
		}
		tasks = append(tasks, &t)
	}
	return tasks, rows.Err()
}

// UpdateStatus updates the status and error message of a task with state machine enforcement.
func (s *TaskStore) UpdateStatus(ctx context.Context, taskID string, status task.TaskStatus, errMsg string) error {
	t, err := s.Get(ctx, taskID)
	if err != nil {
		return err
	}

	if !t.CanTransitionTo(status) {
		return appErrors.New(appErrors.CodeFailedPrecondition, fmt.Sprintf("invalid task state transition from %s to %s", t.Status, status), appErrors.LayerRepository)
	}

	t.Status = status
	t.ErrorMessage = errMsg
	now := time.Now().UTC()
	if status == task.StatusRunning && t.StartedAt == nil {
		t.StartedAt = &now
	}
	if status == task.StatusCompleted || status == task.StatusFailed || status == task.StatusCancelled {
		t.CompletedAt = &now
	}

	return s.Save(ctx, t)
}
