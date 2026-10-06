package postgres

import (
	"context"
	"os"
)

// GetTestPostgresDSN returns the PostgreSQL connection string from environment or empty
func GetTestPostgresDSN() string {
	dsn := os.Getenv("POSTGRES_URL")
	if dsn == "" {
		dsn = os.Getenv("DATABASE_URL")
	}
	return dsn
}

// PostgresTestContainer wraps test container connection parameters
type PostgresTestContainer struct {
	DSN string
}

// SetupTestContainer retrieves the connection string if configured
func SetupTestContainer(ctx context.Context) (*PostgresTestContainer, error) {
	dsn := GetTestPostgresDSN()
	if dsn == "" {
		return nil, nil // Skips when no container or DB available
	}
	return &PostgresTestContainer{DSN: dsn}, nil
}
