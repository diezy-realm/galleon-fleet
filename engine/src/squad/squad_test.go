package squad_test

import (
	"context"
	"testing"

	"github.com/diezy-realm/galleon-fleet/engine/src/squad"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestSquadLifecycle(t *testing.T) {
	svc := squad.NewService()
	ctx := context.Background()

	// 1. Create Squad
	sq, err := svc.CreateSquad(ctx, "shp_alpha", "flt_main", "Red Squadron")
	require.NoError(t, err)
	assert.NotEmpty(t, sq.ID)
	assert.Equal(t, "shp_alpha", sq.ShipID)
	assert.Equal(t, "flt_main", sq.FleetID)
	assert.Equal(t, "Red Squadron", sq.Name)
	assert.Empty(t, sq.CrewIDs)

	// 2. Add Crew
	err = svc.AddCrewToSquad(ctx, sq.ID, "crw_pilot1")
	require.NoError(t, err)
	err = svc.AddCrewToSquad(ctx, sq.ID, "crw_pilot2")
	require.NoError(t, err)

	// Idempotent duplicate add
	err = svc.AddCrewToSquad(ctx, sq.ID, "crw_pilot1")
	require.NoError(t, err)

	fetched, err := svc.GetSquad(ctx, sq.ID)
	require.NoError(t, err)
	assert.Equal(t, []string{"crw_pilot1", "crw_pilot2"}, fetched.CrewIDs)

	// 3. List squads by ship
	list, err := svc.ListSquadsByShip(ctx, "shp_alpha")
	require.NoError(t, err)
	assert.Len(t, list, 1)

	// 4. Remove Crew
	err = svc.RemoveCrewFromSquad(ctx, sq.ID, "crw_pilot1")
	require.NoError(t, err)

	fetched, err = svc.GetSquad(ctx, sq.ID)
	require.NoError(t, err)
	assert.Equal(t, []string{"crw_pilot2"}, fetched.CrewIDs)
}
