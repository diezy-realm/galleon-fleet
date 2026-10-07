package crew_test

import (
	"context"
	"net"
	"testing"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/pkg/client"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
	"github.com/diezy-realm/galleon-fleet/engine/src/crew"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"google.golang.org/grpc"
)

type resourceGateServer struct {
	pb.UnimplementedSystemGatewayServer
}

func (s *resourceGateServer) ExecuteTurn(req *pb.ExecuteTurnRequest, stream pb.SystemGateway_ExecuteTurnServer) error {
	// Emit 3 tool calls for subagents
	for i := 1; i <= 3; i++ {
		toolName := "spawn_subagent"
		if i == 3 {
			toolName = "delegate_task"
		}
		if err := stream.Send(&pb.TurnToken{
			Event: &pb.TurnToken_ToolCall{
				ToolCall: &pb.TurnToolCall{
					Id:            "call_subagent_" + string(rune('0'+i)),
					Name:          toolName,
					ArgumentsJson: `{"role":"Reviewer","task":"review code"}`,
				},
			},
		}); err != nil {
			return err
		}
	}

	return stream.Send(&pb.TurnToken{
		Event: &pb.TurnToken_Complete{
			Complete: &pb.TurnComplete{
				FinishReason: "stop",
			},
		},
	})
}

func (s *resourceGateServer) ExecuteNativeTool(ctx context.Context, req *pb.ToolCallRequest) (*pb.ToolCallResponse, error) {
	return &pb.ToolCallResponse{Success: true, Output: "ok"}, nil
}

func TestSubagentResourceGateRejection(t *testing.T) {
	// BG-D1 / BL-18: Kirim N spawn request melebihi cap, assert DITOLAK (bukan dieksekusi semua tanpa batas)
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	require.NoError(t, err)

	grpcServer := grpc.NewServer()
	pb.RegisterSystemGatewayServer(grpcServer, &resourceGateServer{})
	go func() {
		_ = grpcServer.Serve(lis)
	}()
	defer grpcServer.Stop()

	gwClient, err := client.NewSystemGatewayClient(lis.Addr().String())
	require.NoError(t, err)
	defer func() { _ = gwClient.Close() }()

	dispatcher := crew.NewToolDispatcher(gwClient)
	orch := crew.NewService(dispatcher, gwClient)

	// Set concurrency ceiling to 1
	orch.SetMaxSubagents(1)

	eventCh := make(chan *crew.TurnEvent, 50)
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()

	err = orch.StartTurn(ctx, &crew.TurnRequest{
		SessionID: "sess-resource-gate-test",
		Prompt:    "delegate to multiple subagents",
	}, eventCh)
	require.NoError(t, err)

	close(eventCh)

	var hasSpawned bool
	var hasRejection bool
	for ev := range eventCh {
		if ev.Type == crew.EventSubagentSpawned {
			hasSpawned = true
		}
		if ev.Type == crew.EventToolCallFinished && assert.ObjectsAreEqual(ev.Type, crew.EventToolCallFinished) {
			if assert.Contains(t, ev.Content, "Resource gate rejection") {
				hasRejection = true
			}
		}
	}

	assert.True(t, hasSpawned, "at least 1 subagent must be spawned up to the cap")
	assert.True(t, hasRejection, "excess subagent requests beyond cap (1) must be rejected")
}
