//go:build wireinject
// +build wireinject

package app

import (
	"github.com/diezy-realm/galleon-fleet/engine/core/config"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/client"
	"github.com/diezy-realm/galleon-fleet/engine/src/artifact"
	"github.com/diezy-realm/galleon-fleet/engine/src/crew"
	"github.com/diezy-realm/galleon-fleet/engine/src/fleet"
	"github.com/diezy-realm/galleon-fleet/engine/src/memory"
	"github.com/diezy-realm/galleon-fleet/engine/src/run"
	"github.com/diezy-realm/galleon-fleet/engine/src/task"
	"github.com/diezy-realm/galleon-fleet/engine/src/tool"
	"github.com/diezy-realm/galleon-fleet/engine/src/workflow"
	"github.com/google/wire"
)

// InitializeApp builds the dependency injection graph via Google Wire.
//
// NOTE: fleet's objective proposer (orchestrator → fleet.ObjectiveProposer) is
// wired post-construction in wire_gen.go via fleetService.SetObjectiveProposer,
// because orchestrator imports fleet (dependency inversion breaks the cycle) and
// Wire cannot express a setter call. If you regenerate wire_gen.go, re-add that
// manual step (construct qmorch.NewService(systemGatewayClient, fleetService) and inject it).
func InitializeApp(cfg *config.AppConfig) (*App, error) {
	wire.Build(
		NewGRPCServer,
		ProvideMetricsServer,
		client.Set,
		memory.Set,
		crew.Set,
		run.Set,
		task.Set,
		tool.Set,
		artifact.Set,
		workflow.Set,
		fleet.Set,
		NewApp,
	)
	return &App{}, nil
}
