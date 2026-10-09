package schedule

import (
	"context"
	"fmt"
	"sync"
	"sync/atomic"
	"time"
)

var idCounter int64

type inMemoryService struct {
	mu        sync.RWMutex
	schedules map[string]*Schedule
}

// NewService creates an in-memory schedule service
func NewService() Service {
	return &inMemoryService{
		schedules: make(map[string]*Schedule),
	}
}

func (s *inMemoryService) CreateSchedule(ctx context.Context, sched *Schedule) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if sched.SquadID == "" && sched.CrewID == "" {
		return fmt.Errorf("schedule must be bound to either a squad or crew")
	}
	if sched.ID == "" {
		seq := atomic.AddInt64(&idCounter, 1)
		sched.ID = fmt.Sprintf("sch_%d_%d", time.Now().UnixNano(), seq)
	}
	sched.CreatedAt = time.Now()
	s.schedules[sched.ID] = sched
	return nil
}

func (s *inMemoryService) GetSchedule(ctx context.Context, id string) (*Schedule, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	sched, exists := s.schedules[id]
	if !exists {
		return nil, fmt.Errorf("schedule %s not found", id)
	}
	return sched, nil
}

func (s *inMemoryService) ListSchedulesBySquad(ctx context.Context, squadID string) ([]*Schedule, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	var result []*Schedule
	for _, sc := range s.schedules {
		if sc.SquadID == squadID {
			result = append(result, sc)
		}
	}
	return result, nil
}

func (s *inMemoryService) ListSchedulesByCrew(ctx context.Context, crewID string) ([]*Schedule, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	var result []*Schedule
	for _, sc := range s.schedules {
		if sc.CrewID == crewID {
			result = append(result, sc)
		}
	}
	return result, nil
}

func (s *inMemoryService) DeleteSchedule(ctx context.Context, id string) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if _, exists := s.schedules[id]; !exists {
		return fmt.Errorf("schedule %s not found", id)
	}
	delete(s.schedules, id)
	return nil
}

func (s *inMemoryService) ToggleSchedule(ctx context.Context, id string, enabled bool) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	sched, exists := s.schedules[id]
	if !exists {
		return fmt.Errorf("schedule %s not found", id)
	}
	sched.Enabled = enabled
	return nil
}
