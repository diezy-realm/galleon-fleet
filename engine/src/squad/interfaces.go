package squad

import (
	"context"
	"time"
)

// Squad represents a specialized operational unit inside a Ship
type Squad struct {
	ID        string    `json:"id"`
	ShipID    string    `json:"ship_id"`
	FleetID   string    `json:"fleet_id"`
	Name      string    `json:"name"`
	CrewIDs   []string  `json:"crew_ids"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

// Service manages squad lifecycle and crew assignment
type Service interface {
	CreateSquad(ctx context.Context, shipID, fleetID, name string) (*Squad, error)
	GetSquad(ctx context.Context, id string) (*Squad, error)
	ListSquadsByShip(ctx context.Context, shipID string) ([]*Squad, error)
	AddCrewToSquad(ctx context.Context, squadID, crewID string) error
	RemoveCrewFromSquad(ctx context.Context, squadID, crewID string) error
}
