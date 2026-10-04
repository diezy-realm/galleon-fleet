package sqlite

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"time"

	appErrors "github.com/diezy-labs/claw-crew/engine/core/errors"
	"github.com/diezy-labs/claw-crew/engine/src/run"
)

// RunStore provides SQLite-backed storage for runs and run events.
type RunStore struct {
	db *DB
}

// NewRunStore creates a new RunStore.
func NewRunStore(db *DB) *RunStore {
	return &RunStore{db: db}
}

// Save persists or updates a Run entity in SQLite.
func (s *RunStore) Save(ctx context.Context, r *run.Run) error {
	if r == nil || r.ID == "" {
		return appErrors.New(appErrors.CodeInvalidArgument, "run or run ID cannot be empty", appErrors.LayerRepository)
	}

	data, err := json.Marshal(r)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, "failed to marshal run", appErrors.LayerRepository)
	}

	query := `
	INSERT INTO runs (id, status, request_json, error_msg, created_at, updated_at)
	VALUES (?, ?, ?, ?, ?, ?)
	ON CONFLICT(id) DO UPDATE SET
		status = excluded.status,
		request_json = excluded.request_json,
		error_msg = excluded.error_msg,
		updated_at = excluded.updated_at;
	`

	now := time.Now().UTC()
	createdAt := r.CreatedAt
	if createdAt.IsZero() {
		createdAt = now
	}

	_, err = s.db.ExecContext(ctx, query, r.ID, string(r.Status), string(data), r.ErrorMessage, createdAt, now)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to save run %s", r.ID), appErrors.LayerRepository)
	}
	return nil
}

// Get fetches a Run entity by ID from SQLite.
func (s *RunStore) Get(ctx context.Context, id string) (*run.Run, error) {
	if id == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "run ID cannot be empty", appErrors.LayerRepository)
	}

	query := `SELECT request_json FROM runs WHERE id = ? LIMIT 1;`
	row := s.db.QueryRowContext(ctx, query, id)

	var requestJSON string
	if err := row.Scan(&requestJSON); err != nil {
		if err == sql.ErrNoRows {
			return nil, appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("run not found: %s", id), appErrors.LayerRepository)
		}
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to query run %s", id), appErrors.LayerRepository)
	}

	var r run.Run
	if err := json.Unmarshal([]byte(requestJSON), &r); err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to unmarshal run %s", id), appErrors.LayerRepository)
	}
	return &r, nil
}

// UpdateStatus updates the status and error message of a Run in SQLite.
func (s *RunStore) UpdateStatus(ctx context.Context, id string, status run.RunStatus, errMsg string) error {
	r, err := s.Get(ctx, id)
	if err != nil {
		return err
	}

	if !r.CanTransitionTo(status) {
		return appErrors.New(appErrors.CodeFailedPrecondition, fmt.Sprintf("invalid run state transition from %s to %s", r.Status, status), appErrors.LayerRepository)
	}

	r.Status = status
	r.ErrorMessage = errMsg
	now := time.Now().UTC()
	if status == run.StatusRunning && r.StartedAt == nil {
		r.StartedAt = &now
	}
	if status == run.StatusCompleted || status == run.StatusFailed || status == run.StatusCancelled {
		r.CompletedAt = &now
	}

	return s.Save(ctx, r)
}

// List returns all Run entities in SQLite ordered by creation time descending.
func (s *RunStore) List(ctx context.Context) ([]*run.Run, error) {
	query := `SELECT request_json FROM runs ORDER BY created_at DESC;`
	rows, err := s.db.QueryContext(ctx, query)
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to query runs list", appErrors.LayerRepository)
	}
	defer rows.Close()

	var runs []*run.Run
	for rows.Next() {
		var requestJSON string
		if err := rows.Scan(&requestJSON); err != nil {
			return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to scan run row", appErrors.LayerRepository)
		}
		var r run.Run
		if err := json.Unmarshal([]byte(requestJSON), &r); err != nil {
			continue // graceful skip corrupted entries
		}
		runs = append(runs, &r)
	}
	return runs, rows.Err()
}

// ResumeInterruptedRuns implements run.ResumableStore by finding runs left running/planning/cancelling.
func (s *RunStore) ResumeInterruptedRuns(ctx context.Context) ([]*run.Run, error) {
	query := `
	SELECT request_json FROM runs 
	WHERE status IN ('queued', 'planning', 'running', 'waiting_for_input', 'cancelling')
	ORDER BY created_at ASC;
	`
	rows, err := s.db.QueryContext(ctx, query)
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to query interrupted runs", appErrors.LayerRepository)
	}
	defer rows.Close()

	var runs []*run.Run
	for rows.Next() {
		var requestJSON string
		if err := rows.Scan(&requestJSON); err != nil {
			return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to scan interrupted run row", appErrors.LayerRepository)
		}
		var r run.Run
		if err := json.Unmarshal([]byte(requestJSON), &r); err != nil {
			continue
		}
		runs = append(runs, &r)
	}
	return runs, rows.Err()
}

// AppendEvent stores a run event in SQLite.
func (s *RunStore) AppendEvent(ctx context.Context, event *run.RunEvent) error {
	if event == nil || event.RunID == "" {
		return appErrors.New(appErrors.CodeInvalidArgument, "event or run ID cannot be empty", appErrors.LayerRepository)
	}

	payloadJSON, err := json.Marshal(event.Payload)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, "failed to marshal event payload", appErrors.LayerRepository)
	}

	query := `
	INSERT INTO run_events (id, run_id, event_type, payload_json, error_str, timestamp)
	VALUES (?, ?, ?, ?, ?, ?);
	`
	eventID := event.EventID
	if eventID == "" {
		eventID = fmt.Sprintf("%s-%d", event.RunID, event.Sequence)
	}

	ts := event.Timestamp
	if ts.IsZero() {
		ts = time.Now().UTC()
	}

	_, err = s.db.ExecContext(ctx, query, eventID, event.RunID, event.Type, string(payloadJSON), event.Error, ts)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to append event for run %s", event.RunID), appErrors.LayerRepository)
	}
	return nil
}

// ReplayEvents loads all events for a run ordered by timestamp ascending.
func (s *RunStore) ReplayEvents(ctx context.Context, runID string) ([]*run.RunEvent, error) {
	query := `
	SELECT id, run_id, event_type, payload_json, error_str, timestamp 
	FROM run_events 
	WHERE run_id = ? 
	ORDER BY timestamp ASC;
	`
	rows, err := s.db.QueryContext(ctx, query, runID)
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to query events for run %s", runID), appErrors.LayerRepository)
	}
	defer rows.Close()

	var events []*run.RunEvent
	var seq int64 = 1
	for rows.Next() {
		var id, runID, eventType, payloadJSON, errorStr string
		var ts time.Time

		if err := rows.Scan(&id, &runID, &eventType, &payloadJSON, &errorStr, &ts); err != nil {
			return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to scan event row", appErrors.LayerRepository)
		}

		var payload any
		if len(payloadJSON) > 0 {
			_ = json.Unmarshal([]byte(payloadJSON), &payload)
		}

		events = append(events, &run.RunEvent{
			EventID:   id,
			RunID:     runID,
			Sequence:  seq,
			Type:      eventType,
			Timestamp: ts,
			Payload:   payload,
			Error:     errorStr,
		})
		seq++
	}
	return events, rows.Err()
}
