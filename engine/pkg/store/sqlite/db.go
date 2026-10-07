package sqlite

import (
	"context"
	"database/sql"
	"fmt"
	"os"
	"path/filepath"
	"time"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
	_ "modernc.org/sqlite"
)

// DB wraps the *sql.DB instance with configuration optimized for embedded SQLite.
type DB struct {
	*sql.DB
	path string
}

// Open initializes and connects to the SQLite database with WAL mode and timeout pragma.
func Open(dbPath string) (*DB, error) {
	if dbPath == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "dbPath cannot be empty", appErrors.LayerRepository)
	}

	dir := filepath.Dir(dbPath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to create db directory %s", dir), appErrors.LayerRepository)
	}

	// modernc.org/sqlite connection string
	dsn := fmt.Sprintf("%s?_pragma=busy_timeout(5000)&_pragma=journal_mode(WAL)&_pragma=foreign_keys(ON)&_pragma=synchronous(NORMAL)", dbPath)
	db, err := sql.Open("sqlite", dsn)
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to open sqlite connection", appErrors.LayerRepository)
	}

	// SQLite embedded pool configuration: single writer for WAL to avoid lock contention
	db.SetMaxOpenConns(10)
	db.SetMaxIdleConns(5)
	db.SetConnMaxLifetime(time.Hour)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := db.PingContext(ctx); err != nil {
		_ = db.Close()
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to ping sqlite database", appErrors.LayerRepository)
	}

	wrapped := &DB{
		DB:   db,
		path: dbPath,
	}

	if err := wrapped.initSchema(ctx); err != nil {
		_ = db.Close()
		return nil, err
	}

	return wrapped, nil
}

// Close closes the underlying database handle.
func (d *DB) Close() error {
	return d.DB.Close()
}

// Path returns the physical filesystem path of the database.
func (d *DB) Path() string {
	return d.path
}
