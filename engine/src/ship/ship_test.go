package ship_test

import (
	"context"
	"testing"

	"github.com/diezy-realm/galleon-fleet/engine/src/ship"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestShipUniqueIDsInSameFleet(t *testing.T) {
	svc := ship.NewService()
	ctx := context.Background()

	// BL-81 AC #1: Verify two ships on the same fleet get unique, non-colliding IDs
	ship1, err := svc.CreateShip(ctx, "flt_royal", "charter_1")
	require.NoError(t, err)

	ship2, err := svc.CreateShip(ctx, "flt_royal", "charter_2")
	require.NoError(t, err)

	assert.NotEqual(t, ship1.ID, ship2.ID, "two ships on same fleet must not collide IDs")
	assert.Equal(t, "flt_royal", ship1.FleetID)
	assert.Equal(t, "flt_royal", ship2.FleetID)
	assert.Empty(t, ship1.CrewIDs)
	assert.Empty(t, ship2.CrewIDs)

	// Can retrieve each independently
	got1, err := svc.GetShip(ctx, ship1.ID)
	require.NoError(t, err)
	assert.Equal(t, ship1.ID, got1.ID)

	got2, err := svc.GetShip(ctx, ship2.ID)
	require.NoError(t, err)
	assert.Equal(t, ship2.ID, got2.ID)
}
