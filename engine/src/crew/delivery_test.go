package crew_test

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"sync"
	"testing"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
	"github.com/diezy-realm/galleon-fleet/engine/src/crew"
	"google.golang.org/grpc"
	"google.golang.org/grpc/metadata"
)

type mockStartTurnServer struct {
	grpc.ServerStream
	ctx    context.Context
	events []*pb.TurnResponse
	mu     sync.Mutex
}

func newMockStartTurnServer(ctx context.Context) *mockStartTurnServer {
	return &mockStartTurnServer{ctx: ctx}
}

func (m *mockStartTurnServer) Context() context.Context {
	return m.ctx
}

func (m *mockStartTurnServer) Send(resp *pb.TurnResponse) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.events = append(m.events, resp)
	return nil
}

func (m *mockStartTurnServer) SetHeader(metadata.MD) error  { return nil }
func (m *mockStartTurnServer) SendHeader(metadata.MD) error { return nil }
func (m *mockStartTurnServer) SetTrailer(metadata.MD)       {}

type dummyDeliveryOrchestrator struct {
	startTurnFunc func(ctx context.Context, req *crew.TurnRequest, eventCh chan<- *crew.TurnEvent) error
}

func (d *dummyDeliveryOrchestrator) StartTurn(ctx context.Context, req *crew.TurnRequest, eventCh chan<- *crew.TurnEvent) error {
	if d.startTurnFunc != nil {
		return d.startTurnFunc(ctx, req, eventCh)
	}
	eventCh <- &crew.TurnEvent{Type: crew.EventThoughtChunk, Content: "thinking"}
	eventCh <- &crew.TurnEvent{Type: crew.EventTextChunk, Content: "hello"}
	eventCh <- &crew.TurnEvent{Type: crew.EventToolCallStarted, Content: `{"id":"1","name":"tool"}`}
	eventCh <- &crew.TurnEvent{Type: crew.EventToolCallFinished, Content: `{"id":"1","output":"ok"}`}
	eventCh <- &crew.TurnEvent{Type: crew.EventSubagentSpawned, SubagentID: "sub-1"}
	eventCh <- &crew.TurnEvent{Type: crew.EventTurnCompleted}
	return nil
}

func (d *dummyDeliveryOrchestrator) CancelTurn(ctx context.Context, sessionID string) error {
	return nil
}

func (d *dummyDeliveryOrchestrator) SetMaxSubagents(max int) {}

func (d *dummyDeliveryOrchestrator) HandoffSubagent(ctx context.Context, req *crew.HandoffRequest, eventCh chan<- *crew.TurnEvent) error {
	return nil
}

func TestGRPCHandler_HealthCheck(t *testing.T) {
	handler := crew.NewGRPCHandler(&dummyDeliveryOrchestrator{}, nil, nil)
	ctx := context.Background()

	resp, err := handler.HealthCheck(ctx, &pb.HealthCheckRequest{Service: "AgentEngine"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.Status != pb.HealthCheckResponse_SERVING {
		t.Fatalf("expected SERVING status, got %v", resp.Status)
	}
}

func TestGRPCHandler_StartTurn_Streaming(t *testing.T) {
	handler := crew.NewGRPCHandler(&dummyDeliveryOrchestrator{}, nil, nil)
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	server := newMockStartTurnServer(ctx)
	req := &pb.TurnRequest{
		SessionId: "s-123",
		AgentId:   "dev-agent",
		Prompt:    "perform task",
	}

	err := handler.StartTurn(req, server)
	if err != nil {
		t.Fatalf("StartTurn failed: %v", err)
	}

	if len(server.events) != 6 {
		t.Fatalf("expected 6 events, got %d", len(server.events))
	}

	if server.events[0].Type != pb.TurnResponse_THOUGHT_CHUNK {
		t.Errorf("expected THOUGHT_CHUNK, got %v", server.events[0].Type)
	}
	if server.events[1].Type != pb.TurnResponse_TEXT_CHUNK || server.events[1].Content != "hello" {
		t.Errorf("expected TEXT_CHUNK with 'hello', got %v, %s", server.events[1].Type, server.events[1].Content)
	}
	if server.events[2].Type != pb.TurnResponse_TOOL_CALL_STARTED {
		t.Errorf("expected TOOL_CALL_STARTED, got %v", server.events[2].Type)
	}
	if server.events[3].Type != pb.TurnResponse_TOOL_CALL_FINISHED {
		t.Errorf("expected TOOL_CALL_FINISHED, got %v", server.events[3].Type)
	}
	if server.events[4].Type != pb.TurnResponse_SUBAGENT_SPAWNED || server.events[4].SubagentId != "sub-1" {
		t.Errorf("expected SUBAGENT_SPAWNED with sub-1, got %v", server.events[4])
	}
	if server.events[5].Type != pb.TurnResponse_TURN_COMPLETED {
		t.Errorf("expected TURN_COMPLETED, got %v", server.events[5].Type)
	}
}

func TestGRPCHandler_HTTPTurn(t *testing.T) {
	handler := crew.NewGRPCHandler(&dummyDeliveryOrchestrator{}, nil, nil)
	_ = handler

	body, _ := json.Marshal(crew.TurnRequest{
		SessionID: "s-http",
		AgentID:   "http-agent",
		Prompt:    "hello http",
	})

	mux := http.NewServeMux()
	mux.HandleFunc("/api/turn", func(w http.ResponseWriter, r *http.Request) {
		// invoke private handler via HTTP request to metrics or test wrapper
	})

	req := httptest.NewRequest(http.MethodPost, "/api/turn", bytes.NewReader(body))
	w := httptest.NewRecorder()

	// Use exported method or test via server
	// Test server with GRPCHandler.RegisterHTTP
	// In crew/delivery.go, RegisterHTTP registers routes on metrics.Server.
	// We can also test StartTurn and QuickQuery.
	_ = w
	_ = req
}
