package entitlement_test

import (
	"context"
	"testing"

	"github.com/diezy-realm/galleon-fleet/engine/src/entitlement"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestPoliciesValues(t *testing.T) {
	// 1. Community Policy {1, 1, 2, 6, 8, 2, 1}
	cp := entitlement.NewCommunityPolicy()
	assert.Equal(t, entitlement.TierCommunity, cp.Tier())
	assert.Equal(t, 1, cp.MaxFleets())
	assert.Equal(t, 1, cp.MaxShips())
	assert.Equal(t, 2, cp.MaxSquadsTotal())
	assert.Equal(t, 6, cp.MaxCrewsPerSquad())
	assert.Equal(t, 8, cp.MaxCrewsPerShip())
	assert.Equal(t, 2, cp.MaxConcurrentVoyages())
	require.NotNil(t, cp.MaxDelegationDepth())
	assert.Equal(t, 1, *cp.MaxDelegationDepth())

	// 2. Team Policy {1, 3, 6, 6, 22, 4, 2}
	tp := entitlement.NewTeamPolicy()
	assert.Equal(t, entitlement.TierTeam, tp.Tier())
	assert.Equal(t, 1, tp.MaxFleets())
	assert.Equal(t, 3, tp.MaxShips())
	assert.Equal(t, 6, tp.MaxSquadsTotal())
	assert.Equal(t, 6, tp.MaxCrewsPerSquad())
	assert.Equal(t, 22, tp.MaxCrewsPerShip())
	assert.Equal(t, 4, tp.MaxConcurrentVoyages())
	require.NotNil(t, tp.MaxDelegationDepth())
	assert.Equal(t, 2, *tp.MaxDelegationDepth())

	// 3. Dockyard Policy {1, 6, 12, 8, 100, 8, 3}
	dp := entitlement.NewDockyardPolicy()
	assert.Equal(t, entitlement.TierDockyard, dp.Tier())
	assert.Equal(t, 1, dp.MaxFleets())
	assert.Equal(t, 6, dp.MaxShips())
	assert.Equal(t, 12, dp.MaxSquadsTotal())
	assert.Equal(t, 8, dp.MaxCrewsPerSquad())
	assert.Equal(t, 100, dp.MaxCrewsPerShip())
	assert.Equal(t, 8, dp.MaxConcurrentVoyages())
	require.NotNil(t, dp.MaxDelegationDepth())
	assert.Equal(t, 3, *dp.MaxDelegationDepth())
}

func TestCheckQuotaGrandfathering(t *testing.T) {
	svc := entitlement.NewService() // Community policy: MaxShips: 1

	// Case 1: Create 1st ship -> allowed
	err := svc.CheckQuota("ships", 0, 1)
	assert.NoError(t, err)

	// Case 2: Create 2nd ship on Community tier -> rejected
	err = svc.CheckQuota("ships", 1, 2)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "quota exceeded for ships")

	// Case 3: Grandfathering: already has 3 ships (e.g. from prior tier), saving existing 3 ships -> allowed
	err = svc.CheckQuota("ships", 3, 3)
	assert.NoError(t, err)

	// Case 4: Saving 2 ships (downgrade/delete) -> allowed
	err = svc.CheckQuota("ships", 3, 2)
	assert.NoError(t, err)

	// Case 5: Upgrade to Team tier -> can add up to 3 ships
	svc.SetPolicy(entitlement.NewTeamPolicy())
	err = svc.CheckQuota("ships", 1, 2)
	assert.NoError(t, err)
	err = svc.CheckQuota("ships", 2, 3)
	assert.NoError(t, err)
	err = svc.CheckQuota("ships", 3, 4)
	assert.Error(t, err)
}

func TestUsageReport(t *testing.T) {
	svc := entitlement.NewService()
	counts := entitlement.Counts{
		FleetsCount:       1,
		ShipsCount:        1,
		SquadsCount:       2,
		CrewsCount:        5,
		ConcurrentVoyages: 1,
	}

	report := svc.GetUsage(context.Background(), counts)
	assert.Equal(t, entitlement.TierCommunity, report.Tier)
	assert.Equal(t, 1, report.ShipsCount)
	assert.Equal(t, 1, report.MaxShips)
	assert.Equal(t, 2, report.SquadsCount)
	assert.Equal(t, 2, report.MaxSquadsTotal)
	require.NotNil(t, report.DelegationDepthCap)
	assert.Equal(t, 1, *report.DelegationDepthCap)
}
