package schedule

import (
	"context"
	"time"
)

// Schedule represents a recurring or cron task bound to a Squad or Crew
type Schedule struct {
	ID             string     `json:"id"`
	SquadID        string     `json:"squad_id,omitempty"`
	CrewID         string     `json:"crew_id,omitempty"`
	CronExpression string     `json:"cron_expression"`
	Prompt         string     `json:"prompt"`
	Enabled        bool       `json:"enabled"`
	LastRunAt      *time.Time `json:"last_run_at,omitempty"`
	NextRunAt      *time.Time `json:"next_run_at,omitempty"`
	CreatedAt      time.Time  `json:"created_at"`
}

// Service provides schedule lifecycle and query bound to Squad/Crew
type Service interface {
	CreateSchedule(ctx context.Context, sched *Schedule) error
	GetSchedule(ctx context.Context, id string) (*Schedule, error)
	ListSchedulesBySquad(ctx context.Context, squadID string) ([]*Schedule, error)
	ListSchedulesByCrew(ctx context.Context, crewID string) ([]*Schedule, error)
	DeleteSchedule(ctx context.Context, id string) error
	ToggleSchedule(ctx context.Context, id string, enabled bool) error
}
