package vector

import (
	"time"
)

// DocumentWithTime wraps a document with its creation timestamp
type TimedDocument struct {
	Document  *Document
	CreatedAt time.Time
}

// RetentionManager handles retention pruning per scope tier (BG-B5)
type RetentionManager struct {
	config RetentionConfig
}

func NewRetentionManager(config RetentionConfig) *RetentionManager {
	return &RetentionManager{config: config}
}

// IsExpired checks if a document created at createdAt is expired under its scope tier
func (r *RetentionManager) IsExpired(scope map[string]string, createdAt time.Time, now time.Time) bool {
	days := 0
	if scope != nil {
		if _, ok := scope[ScopeRunID]; ok && r.config.RunRetentionDays > 0 {
			days = r.config.RunRetentionDays
		} else if _, ok := scope[ScopeCrew]; ok && r.config.CrewRetentionDays > 0 {
			days = r.config.CrewRetentionDays
		} else if _, ok := scope[ScopeSquad]; ok && r.config.SquadRetentionDays > 0 {
			days = r.config.SquadRetentionDays
		} else if _, ok := scope[ScopeShip]; ok && r.config.ShipRetentionDays > 0 {
			days = r.config.ShipRetentionDays
		} else if _, ok := scope[ScopeFleet]; ok && r.config.FleetRetentionDays > 0 {
			days = r.config.FleetRetentionDays
		}
	}

	if days == 0 {
		return false
	}

	cutoff := now.AddDate(0, 0, -days)
	return createdAt.Before(cutoff)
}
