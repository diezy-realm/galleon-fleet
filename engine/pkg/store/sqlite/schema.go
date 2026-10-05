package sqlite

import (
	"context"

	appErrors "github.com/diezy-labs/claw-crew/engine/core/errors"
)

const ddl = `
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    request_json TEXT NOT NULL,
    error_msg TEXT DEFAULT '',
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status);
CREATE INDEX IF NOT EXISTS idx_runs_created_at ON runs(created_at);

CREATE TABLE IF NOT EXISTS run_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    error_str TEXT DEFAULT '',
    timestamp DATETIME NOT NULL,
    FOREIGN KEY(run_id) REFERENCES runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_run_events_run_id ON run_events(run_id);
CREATE INDEX IF NOT EXISTS idx_run_events_timestamp ON run_events(timestamp);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL,
    error_msg TEXT DEFAULT '',
    raw_json TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_run_id ON tasks(run_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
`

func (d *DB) initSchema(ctx context.Context) error {
	_, err := d.ExecContext(ctx, ddl)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, "failed to initialize sqlite schema", appErrors.LayerRepository)
	}

	// Record initial migration
	_, _ = d.ExecContext(ctx, `INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (1, CURRENT_TIMESTAMP)`)
	return nil
}
