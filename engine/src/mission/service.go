package mission

import (
	"context"
	"fmt"
	"sync"
	"time"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
)

type memoryService struct {
	mu     sync.RWMutex
	quests map[string]*Quest
}

// NewService creates a new in-memory mission.Service.
func NewService() Service {
	return &memoryService{
		quests: make(map[string]*Quest),
	}
}

// NewServiceWithQuests creates a new in-memory mission.Service pre-populated with initial quests.
func NewServiceWithQuests(initial []*Quest) Service {
	s := &memoryService{
		quests: make(map[string]*Quest),
	}
	for _, q := range initial {
		if q != nil && q.ID != "" {
			s.quests[q.ID] = q
		}
	}
	return s
}

func (s *memoryService) List(ctx context.Context) ([]*Quest, error) {
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	s.mu.RLock()
	defer s.mu.RUnlock()

	result := make([]*Quest, 0, len(s.quests))
	for _, q := range s.quests {
		result = append(result, q)
	}
	return result, nil
}

func (s *memoryService) Get(ctx context.Context, id string) (*Quest, error) {
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	if id == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "quest id cannot be empty", appErrors.LayerService)
	}

	s.mu.RLock()
	defer s.mu.RUnlock()

	q, ok := s.quests[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("quest not found: %s", id), appErrors.LayerService)
	}
	return q, nil
}

func (s *memoryService) Create(ctx context.Context, q *Quest) (*Quest, error) {
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	if q == nil {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "quest payload cannot be nil", appErrors.LayerService)
	}
	if q.ID == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "quest id is required", appErrors.LayerService)
	}
	if q.Title == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "quest title is required", appErrors.LayerService)
	}
	if q.Objective == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "quest objective is required", appErrors.LayerService)
	}

	s.mu.Lock()
	defer s.mu.Unlock()

	if _, exists := s.quests[q.ID]; exists {
		return nil, appErrors.New(appErrors.CodeFailedPrecondition, fmt.Sprintf("quest already exists: %s", q.ID), appErrors.LayerService)
	}

	now := time.Now().UTC().Format(time.RFC3339)
	if q.CreatedAt == "" {
		q.CreatedAt = now
	}
	q.UpdatedAt = now
	if q.Status == "" {
		q.Status = "ready"
	}

	s.quests[q.ID] = q
	return q, nil
}

func (s *memoryService) UpdateStatus(ctx context.Context, id string, status string) (*Quest, error) {
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	if id == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "quest id cannot be empty", appErrors.LayerService)
	}
	if status == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "quest status cannot be empty", appErrors.LayerService)
	}

	s.mu.Lock()
	defer s.mu.Unlock()

	q, ok := s.quests[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("quest not found: %s", id), appErrors.LayerService)
	}

	q.Status = status
	q.UpdatedAt = time.Now().UTC().Format(time.RFC3339)
	return q, nil
}

func (s *memoryService) AssignShip(ctx context.Context, id string, shipID string) (*Quest, error) {
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	if id == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "quest id cannot be empty", appErrors.LayerService)
	}
	if shipID == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "ship_id cannot be empty", appErrors.LayerService)
	}

	s.mu.Lock()
	defer s.mu.Unlock()

	q, ok := s.quests[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("quest not found: %s", id), appErrors.LayerService)
	}

	q.AssignedShipID = shipID
	q.TargetShipID = shipID
	q.UpdatedAt = time.Now().UTC().Format(time.RFC3339)
	return q, nil
}
