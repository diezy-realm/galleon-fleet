package ship

import (
	"context"
	"errors"
	"sync"

	"github.com/diezy-realm/galleon-fleet/engine/core/id"
)

type Service interface {
	CreateShip(ctx context.Context, fleetID string, charterID string) (*Ship, error)
	GetShip(ctx context.Context, id string) (*Ship, error)
}

type shipService struct {
	mu    sync.RWMutex
	ships map[string]*Ship
}

func NewService() Service {
	return &shipService{
		ships: make(map[string]*Ship),
	}
}

func (s *shipService) CreateShip(ctx context.Context, fleetID string, charterID string) (*Ship, error) {
	if fleetID == "" {
		return nil, errors.New("invalid ship request")
	}

	s.mu.Lock()
	defer s.mu.Unlock()

	// BL-81 AC #1: ID Ship unik per instans untuk mencegah kolisi pada Fleet yang sama
	shipID := id.Generate("shp_")
	ship := &Ship{
		ID:        shipID,
		FleetID:   fleetID,
		CharterID: charterID,
		SquadIDs:  make([]string, 0),
		CrewIDs:   make([]string, 0),
	}

	s.ships[shipID] = ship
	return ship, nil
}

func (s *shipService) GetShip(ctx context.Context, id string) (*Ship, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	ship, exists := s.ships[id]
	if !exists {
		return nil, errors.New("ship not found")
	}

	return ship, nil
}
