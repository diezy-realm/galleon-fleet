package crew_test

import (
	"context"
	"testing"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/src/crew"
)

func TestCrewOrchestratorTurn(t *testing.T) {
	gateway, cleanup := startMockGatewayServer(t)
	defer cleanup()

	dispatcher := crew.NewToolDispatcher(gateway)
	orchestrator := crew.NewService(dispatcher, gateway)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	turnReq := &crew.TurnRequest{
		SessionID: "test_session_1",
		AgentID:   "qa_agent",
		Prompt:    "Please delegate subagent to verify results",
	}

	eventCh := make(chan *crew.TurnEvent, 32)
	errCh := make(chan error, 1)

	go func() {
		defer close(eventCh)
		errCh <- orchestrator.StartTurn(ctx, turnReq, eventCh)
	}()

	var receivedThought bool
	var receivedText bool
	var receivedSubagent bool
	var receivedCompleted bool

	for event := range eventCh {
		switch event.Type {
		case crew.EventThoughtChunk:
			receivedThought = true
		case crew.EventTextChunk:
			receivedText = true
		case crew.EventSubagentSpawned:
			receivedSubagent = true
		case crew.EventTurnCompleted:
			receivedCompleted = true
		}
	}

	if err := <-errCh; err != nil {
		t.Fatalf("unexpected turn error: %v", err)
	}

	if !receivedThought {
		t.Errorf("expected thought chunk event")
	}
	if !receivedText {
		t.Errorf("expected text chunk event")
	}
	if !receivedSubagent {
		t.Errorf("expected subagent spawned event")
	}
	if !receivedCompleted {
		t.Errorf("expected turn completed event")
	}
}

func TestCrewCancelTurn(t *testing.T) {
	gateway, cleanup := startMockGatewayServer(t)
	defer cleanup()

	dispatcher := crew.NewToolDispatcher(gateway)
	orchestrator := crew.NewService(dispatcher, gateway)

	ctx := context.Background()
	turnReq := &crew.TurnRequest{
		SessionID: "cancel_session",
		AgentID:   "qa_agent",
		Prompt:    "Long running task",
	}

	eventCh := make(chan *crew.TurnEvent, 32)
	go func() {
		defer close(eventCh)
		_ = orchestrator.StartTurn(ctx, turnReq, eventCh)
	}()

	// Allow turn to register
	time.Sleep(10 * time.Millisecond)

	// Cancel the turn
	if err := orchestrator.CancelTurn(ctx, "cancel_session"); err != nil {
		t.Fatalf("failed to cancel turn: %v", err)
	}

	// Drain channel
	for range eventCh {
	}
}

func TestCrewAgentAliasResolution(t *testing.T) {
	tests := []struct {
		name               string
		inputAgentID       string
		expectedAgentAlias string
	}{
		{
			name:               "explicit agentID passed as agent_alias",
			inputAgentID:       "specialist_researcher",
			expectedAgentAlias: "specialist_researcher",
		},
		{
			name:               "empty agentID falls back to primary_agent",
			inputAgentID:       "",
			expectedAgentAlias: "primary_agent",
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			gwClient, mockServer, cleanup := startMockGatewayServerWithRecorder(t)
			defer cleanup()

			dispatcher := crew.NewToolDispatcher(gwClient)
			orchestrator := crew.NewService(dispatcher, gwClient)

			ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
			defer cancel()

			turnReq := &crew.TurnRequest{
				SessionID: "session_alias_test",
				AgentID:   tc.inputAgentID,
				Prompt:    "Verify agent alias forwarding",
			}

			eventCh := make(chan *crew.TurnEvent, 16)
			errCh := make(chan error, 1)

			go func() {
				defer close(eventCh)
				errCh <- orchestrator.StartTurn(ctx, turnReq, eventCh)
			}()

			for range eventCh {
			}

			if err := <-errCh; err != nil {
				t.Fatalf("unexpected turn error: %v", err)
			}

			lastReq := mockServer.getLastRequest()
			if lastReq == nil {
				t.Fatalf("expected ExecuteTurn to be called, but got nil request")
			}

			if lastReq.GetAgentAlias() != tc.expectedAgentAlias {
				t.Errorf("expected AgentAlias %q, got %q", tc.expectedAgentAlias, lastReq.GetAgentAlias())
			}

			if lastReq.GetModel() == "gpt-4o" {
				t.Errorf("expected hardcoded Model 'gpt-4o' to be removed, but found %q", lastReq.GetModel())
			}
		})
	}
}
