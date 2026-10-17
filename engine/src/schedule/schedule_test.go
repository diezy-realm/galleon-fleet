package schedule_test

import (
	"context"
	"testing"

	"github.com/diezy-realm/galleon-fleet/engine/src/schedule"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestScheduleLifecycle(t *testing.T) {
	ctx := context.Background()
	svc := schedule.NewService()

	// 1. Error when not bound to squad or crew
	err := svc.CreateSchedule(ctx, &schedule.Schedule{
		CronExpression: "0 * * * *",
		Prompt:         "Review log anomalies",
	})
	assert.Error(t, err)

	// 2. Bound to Squad
	schedSquad := &schedule.Schedule{
		SquadID:        "sqd_alpha",
		CronExpression: "*/15 * * * *",
		Prompt:         "Check squad tasks",
		Enabled:        true,
	}
	err = svc.CreateSchedule(ctx, schedSquad)
	require.NoError(t, err)
	assert.NotEmpty(t, schedSquad.ID)

	// 3. Bound to Crew
	schedCrew := &schedule.Schedule{
		CrewID:         "crw_navigator",
		CronExpression: "0 0 * * *",
		Prompt:         "Daily navigation summary",
		Enabled:        true,
	}
	err = svc.CreateSchedule(ctx, schedCrew)
	require.NoError(t, err)

	// 4. Query by Squad and Crew
	squadScheds, err := svc.ListSchedulesBySquad(ctx, "sqd_alpha")
	require.NoError(t, err)
	assert.Len(t, squadScheds, 1)

	crewScheds, err := svc.ListSchedulesByCrew(ctx, "crw_navigator")
	require.NoError(t, err)
	assert.Len(t, crewScheds, 1)

	// 5. Toggle and Delete
	err = svc.ToggleSchedule(ctx, schedSquad.ID, false)
	require.NoError(t, err)
	fetched, err := svc.GetSchedule(ctx, schedSquad.ID)
	require.NoError(t, err)
	assert.False(t, fetched.Enabled)

	err = svc.DeleteSchedule(ctx, schedCrew.ID)
	require.NoError(t, err)
	_, err = svc.GetSchedule(ctx, schedCrew.ID)
	assert.Error(t, err)
}
