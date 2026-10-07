package entitlement

import (
	"context"
	"fmt"
	"sync"
)

// Counts summarizes current count metrics needed for UsageReport
type Counts struct {
	FleetsCount       int
	ShipsCount        int
	SquadsCount       int
	CrewsCount        int
	ConcurrentVoyages int
}

// Service provides entitlement inspection and quota guard enforcement
type Service interface {
	Policy() QuotaPolicy
	SetPolicy(p QuotaPolicy)
	CheckQuota(collection string, currentCount, newCount int) error
	GetUsage(ctx context.Context, counts Counts) *UsageReport
}

type service struct {
	mu     sync.RWMutex
	policy QuotaPolicy
}

// NewService creates an entitlement service initialized with CommunityPolicy
func NewService() Service {
	return &service{
		policy: NewCommunityPolicy(),
	}
}

func (s *service) Policy() QuotaPolicy {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.policy
}

func (s *service) SetPolicy(p QuotaPolicy) {
	if p == nil {
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	s.policy = p
}

// CheckQuota enforces tier ceilings with grandfathering:
// If newCount <= currentCount (edit/delete/same), it always passes even if over ceiling.
// If newCount > currentCount (adding new items), it fails if newCount > ceiling.
func (s *service) CheckQuota(collection string, currentCount, newCount int) error {
	if newCount <= currentCount {
		return nil
	}

	p := s.Policy()
	var limit int

	switch collection {
	case "ships":
		limit = p.MaxShips()
	case "squads":
		limit = p.MaxSquadsTotal()
	case "crew":
		limit = p.MaxCrewsPerShip()
	default:
		return nil
	}

	if newCount > limit {
		return fmt.Errorf("quota exceeded for %s: tier '%s' limit is %d, attempted %d",
			collection, p.Tier(), limit, newCount)
	}
	return nil
}

func (s *service) GetUsage(ctx context.Context, counts Counts) *UsageReport {
	p := s.Policy()
	return &UsageReport{
		Tier:                 p.Tier(),
		FleetsCount:          counts.FleetsCount,
		MaxFleets:            p.MaxFleets(),
		ShipsCount:           counts.ShipsCount,
		MaxShips:             p.MaxShips(),
		SquadsCount:          counts.SquadsCount,
		MaxSquadsTotal:       p.MaxSquadsTotal(),
		CrewsCount:           counts.CrewsCount,
		MaxCrewsPerShip:      p.MaxCrewsPerShip(),
		ConcurrentVoyages:    counts.ConcurrentVoyages,
		MaxConcurrentVoyages: p.MaxConcurrentVoyages(),
		DelegationDepthCap:   p.MaxDelegationDepth(),
	}
}
