package crew_test

import (
	"context"
	"net"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/pkg/client"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
	"google.golang.org/grpc"
)

type mockGatewayTurnServer struct {
	pb.UnimplementedSystemGatewayServer
	mu      sync.Mutex
	lastReq *pb.ExecuteTurnRequest
}

func (s *mockGatewayTurnServer) getLastRequest() *pb.ExecuteTurnRequest {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.lastReq
}

func (s *mockGatewayTurnServer) ExecuteTurn(req *pb.ExecuteTurnRequest, stream pb.SystemGateway_ExecuteTurnServer) error {
	s.mu.Lock()
	s.lastReq = req
	s.mu.Unlock()
	prompt := ""
	if len(req.Messages) > 0 {
		prompt = req.Messages[len(req.Messages)-1].Content
	}

	// 1. Emit thought chunk
	if err := stream.Send(&pb.TurnToken{
		Event: &pb.TurnToken_Token{
			Token: &pb.TokenChunk{
				Delta:     "Reasoning about: " + prompt,
				IsThought: true,
			},
		},
	}); err != nil {
		return err
	}

	// 2. Emit text chunk
	if err := stream.Send(&pb.TurnToken{
		Event: &pb.TurnToken_Token{
			Token: &pb.TokenChunk{
				Delta:     "I am executing the turn autonomously.",
				IsThought: false,
			},
		},
	}); err != nil {
		return err
	}

	lower := strings.ToLower(prompt)
	if strings.Contains(lower, "long running") {
		select {
		case <-stream.Context().Done():
			return stream.Context().Err()
		case <-time.After(300 * time.Millisecond):
		}
	}

	// 3. If prompt requests subagent delegation, emit tool call
	if strings.Contains(lower, "subagent") || strings.Contains(lower, "delegate") {
		if err := stream.Send(&pb.TurnToken{
			Event: &pb.TurnToken_ToolCall{
				ToolCall: &pb.TurnToolCall{
					Id:            "call_delegate_subagent_1",
					Name:          "spawn_subagent",
					ArgumentsJson: `{"role":"tester","task":"verify results"}`,
				},
			},
		}); err != nil {
			return err
		}
	}

	// 4. Emit complete
	return stream.Send(&pb.TurnToken{
		Event: &pb.TurnToken_Complete{
			Complete: &pb.TurnComplete{
				FinishReason:     "stop",
				PromptTokens:     10,
				CompletionTokens: 20,
				TotalTokens:      30,
			},
		},
	})
}

func (s *mockGatewayTurnServer) ExecuteNativeTool(ctx context.Context, req *pb.ToolCallRequest) (*pb.ToolCallResponse, error) {
	return &pb.ToolCallResponse{
		Success: true,
		Output:  "mock tool execution output",
	}, nil
}

func startMockGatewayServer(t testing.TB) (client.SystemGatewayClient, func()) {
	client, _, cleanup := startMockGatewayServerWithRecorder(t)
	return client, cleanup
}

func startMockGatewayServerWithRecorder(t testing.TB) (client.SystemGatewayClient, *mockGatewayTurnServer, func()) {
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on ephemeral port: %v", err)
	}
	server := grpc.NewServer()
	mockSrv := &mockGatewayTurnServer{}
	pb.RegisterSystemGatewayServer(server, mockSrv)

	go func() {
		_ = server.Serve(lis)
	}()

	gwClient, err := client.NewSystemGatewayClient(lis.Addr().String())
	if err != nil {
		server.Stop()
		t.Fatalf("failed to create gateway client: %v", err)
	}

	cleanup := func() {
		_ = gwClient.Close()
		server.Stop()
	}

	return gwClient, mockSrv, cleanup
}
