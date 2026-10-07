package mission

import (
	"context"
)

// QuestStep represents one sequential map step in a Quest execution.
type QuestStep struct {
	StepNumber int    `json:"stepNumber"`
	Title      string `json:"title"`
	Status     string `json:"status"` // "pending", "in_progress", "completed", "failed"
}

// Quest represents a high-level goal or mission undertaken by an assigned Ship / Squad.
type Quest struct {
	ID                   string      `json:"id"`
	Title                string      `json:"title"`
	Objective            string      `json:"objective"`
	WorkspaceID          string      `json:"workspaceId,omitempty"`
	ProjectID            string      `json:"projectId,omitempty"`
	Priority             string      `json:"priority,omitempty"`
	Status               string      `json:"status"` // "ready", "underway", "completed", "failed", "cancelled"
	TargetShipID         string      `json:"target_ship_id,omitempty"`
	SuggestedShipID      string      `json:"suggestedShipId,omitempty"`
	AssignedShipID       string      `json:"assignedShipId,omitempty"`
	WorkflowID           string      `json:"workflow_id,omitempty"`
	RequiredArtifacts    []string    `json:"requiredArtifacts,omitempty"`
	BudgetLimitUSD       float64     `json:"budgetLimitUSD,omitempty"`
	EstimatedCostUSD     float64     `json:"estimatedCostUSD,omitempty"`
	MapSteps             []QuestStep `json:"mapSteps,omitempty"`
	ActiveVoyageProgress int         `json:"activeVoyageProgress,omitempty"`
	CreatedAt            string      `json:"createdAt,omitempty"`
	UpdatedAt            string      `json:"updatedAt,omitempty"`
	DiscoveriesCount     int         `json:"discoveriesCount,omitempty"`
}

// Service provides high-level Quest lifecycle operations.
type Service interface {
	List(ctx context.Context) ([]*Quest, error)
	Get(ctx context.Context, id string) (*Quest, error)
	Create(ctx context.Context, q *Quest) (*Quest, error)
	UpdateStatus(ctx context.Context, id string, status string) (*Quest, error)
	AssignShip(ctx context.Context, id string, shipID string) (*Quest, error)
}
