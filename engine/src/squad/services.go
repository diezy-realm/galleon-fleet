package squad

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/core/id"
)

type squadService struct {
	mu     sync.RWMutex
	squads map[string]*Squad
}

// NewService creates a thread-safe Squad service
func NewService() Service {
	return &squadService{
		squads: make(map[string]*Squad),
	}
}

func (s *squadService) CreateSquad(ctx context.Context, shipID, fleetID, name string) (*Squad, error) {
	if shipID == "" || fleetID == "" || name == "" {
		return nil, errors.New("ship_id, fleet_id, and name are required")
	}

	s.mu.Lock()
	defer s.mu.Unlock()

	sqdID := id.Generate("sqd_")
	now := time.Now().UTC()
	sq := &Squad{
		ID:        sqdID,
		ShipID:    shipID,
		FleetID:   fleetID,
		Name:      name,
		CrewIDs:   make([]string, 0),
		CreatedAt: now,
		UpdatedAt: now,
	}

	s.squads[sqdID] = sq
	return sq, nil
}

func (s *squadService) GetSquad(ctx context.Context, sqdID string) (*Squad, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	sq, exists := s.squads[sqdID]
	if !exists {
		return nil, errors.New("squad not found")
	}
	return sq, nil
}

func (s *squadService) ListSquadsByShip(ctx context.Context, shipID string) ([]*Squad, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	var result []*Squad
	for _, sq := range s.squads {
		if sq.ShipID == shipID {
			result = append(result, sq)
		}
	}
	return result, nil
}

func (s *squadService) AddCrewToSquad(ctx context.Context, squadID, crewID string) error {
	if squadID == "" || crewID == "" {
		return errors.New("squad_id and crew_id are required")
	}

	s.mu.Lock()
	defer s.mu.Unlock()

	sq, exists := s.squads[squadID]
	if !exists {
		return errors.New("squad not found")
	}

	for _, id := range sq.CrewIDs {
		if id == crewID {
			return nil // already present
		}
	}

	sq.CrewIDs = append(sq.CrewIDs, crewID)
	sq.UpdatedAt = time.Now().UTC()
	return nil
}

func (s *squadService) RemoveCrewFromSquad(ctx context.Context, squadID, crewID string) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	sq, exists := s.squads[squadID]
	if !exists {
		return errors.New("squad not found")
	}

	filtered := make([]string, 0, len(sq.CrewIDs))
	for _, id := range sq.CrewIDs {
		if id != crewID {
			filtered = append(filtered, id)
		}
	}
	sq.CrewIDs = filtered
	sq.UpdatedAt = time.Now().UTC()
	return nil
}
