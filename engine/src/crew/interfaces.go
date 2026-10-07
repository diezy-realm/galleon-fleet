package crew

import (
	"context"
)

// HandoffRequest represents a subagent handoff or A2A delegation carrying an artifact reference
type HandoffRequest struct {
	SessionID       string `json:"session_id"`
	FromAgentID     string `json:"from_agent_id"`
	ToAgentID       string `json:"to_agent_id"`
	TaskDescription string `json:"task_description"`
	ArtifactRef     string `json:"artifact_ref"` // reference to artifact ID
}

// Orchestrator defines the contract for managing multi-agent lifecycles and turns
type Orchestrator interface {
	StartTurn(ctx context.Context, req *TurnRequest, eventCh chan<- *TurnEvent) error
	CancelTurn(ctx context.Context, sessionID string) error
	SetMaxSubagents(max int)
	HandoffSubagent(ctx context.Context, req *HandoffRequest, eventCh chan<- *TurnEvent) error
}

// Registry defines storage and retrieval for crew and agent definitions
type Registry interface {
	ListSquads(ctx context.Context) ([]*Squad, error)
	GetSquad(ctx context.Context, id string) (*Squad, error)
	RegisterSquad(ctx context.Context, crew *Squad) error
	UpdateCrewMemberStatus(ctx context.Context, crewID, agentID string, status CrewMemberStatus) error
}
