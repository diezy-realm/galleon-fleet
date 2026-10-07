package vector

import (
	"context"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/src/memory"
)

// Re-export common types from memory to preserve SSOT
type Document = memory.Document
type SearchResult = memory.SearchResult

// VectorStore defines the common vector storage contract with deletion support (BG-B3)
type VectorStore interface {
	Store(ctx context.Context, doc *Document) error
	Search(ctx context.Context, queryEmbedding []float32, topK int) ([]*SearchResult, error)
	SearchWithScope(ctx context.Context, queryEmbedding []float32, topK int, scope map[string]string) ([]*SearchResult, error)
	Delete(ctx context.Context, id string) error
}

// ScopedVectorStore extends VectorStore with explicit scope guarantees (BG-B4)
// Supporting hierarchy: Fleet -> Ship -> Squad -> Crew -> Run
type ScopedVectorStore interface {
	VectorStore
	StoreScoped(ctx context.Context, doc *Document, scope map[string]string) error
	SearchScoped(ctx context.Context, queryEmbedding []float32, topK int, scope map[string]string) ([]*SearchResult, error)
	DeleteScoped(ctx context.Context, id string, scope map[string]string) error
}

// RetentionConfig configures retention policies per scope tier (BG-B5)
type RetentionConfig struct {
	FleetRetentionDays int `json:"fleet_retention_days"`
	ShipRetentionDays  int `json:"ship_retention_days"`
	SquadRetentionDays int `json:"squad_retention_days"`
	CrewRetentionDays  int `json:"crew_retention_days"`
	RunRetentionDays   int `json:"run_retention_days"`
}

// ExportRequest specifies export filters and formats (BG-B5)
type ExportRequest struct {
	Scope    map[string]string `json:"scope"`
	Format   string            `json:"format"` // "json", "csv"
	FromTime time.Time         `json:"from_time"`
	ToTime   time.Time         `json:"to_time"`
}

// Scope keys canonical definition
const (
	ScopeFleet = "fleet"
	ScopeShip  = "ship"
	ScopeSquad = "squad"
	ScopeCrew  = "crew"
	ScopeRunID = "run_id"
)
