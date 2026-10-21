package crew_test

import (
	"context"
	"encoding/json"
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
		expectSent         bool
	}{
		{
			name:               "explicit agentID passed as agent_alias",
			inputAgentID:       "specialist_researcher",
			expectedAgentAlias: "specialist_researcher",
			expectSent:         true,
		},
		{
			name:               "empty agentID does not send agent_alias",
			inputAgentID:       "",
			expectedAgentAlias: "",
			expectSent:         false,
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

			if tc.expectSent {
				if lastReq.AgentAlias == nil {
					t.Fatalf("expected AgentAlias to be sent, got nil")
				}
				if lastReq.GetAgentAlias() != tc.expectedAgentAlias {
					t.Errorf("expected AgentAlias %q, got %q", tc.expectedAgentAlias, lastReq.GetAgentAlias())
				}
			} else {
				if lastReq.AgentAlias != nil {
					t.Errorf("expected AgentAlias to NOT be sent (nil), got %q", *lastReq.AgentAlias)
				}
			}

			if lastReq.GetModel() == "gpt-4o" {
				t.Errorf("expected hardcoded Model 'gpt-4o' to be removed, but found %q", lastReq.GetModel())
			}
		})
	}
}

func TestCrewMemberAgentAliasTurnExecution(t *testing.T) {
	tests := []struct {
		name               string
		member             *crew.CrewMember
		expectedProtoAlias string
		expectSent         bool
	}{
		{
			name: "CrewMember dengan AgentAlias terisi -> field proto terkirim",
			member: &crew.CrewMember{
				ID:           "member_planner",
				Name:         "Planner Agent",
				Role:         "Planning",
				Status:       crew.CrewMemberStatusIdle,
				Capabilities: []string{"plan"},
				AgentAlias:   "planner_agent",
			},
			expectedProtoAlias: "planner_agent",
			expectSent:         true,
		},
		{
			name: "CrewMember dengan AgentAlias kosong -> field proto tidak terkirim, perilaku lama tidak berubah",
			member: &crew.CrewMember{
				ID:           "member_coder",
				Name:         "Code Specialist",
				Role:         "Coding",
				Status:       crew.CrewMemberStatusIdle,
				Capabilities: []string{"code"},
				AgentAlias:   "",
			},
			expectedProtoAlias: "",
			expectSent:         false,
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

			turnReq := crew.NewTurnRequestForMember(tc.member, "session_member_test", "Execute assigned task")

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

			if tc.expectSent {
				if lastReq.AgentAlias == nil {
					t.Fatalf("expected proto AgentAlias field to be sent, but got nil")
				}
				if lastReq.GetAgentAlias() != tc.expectedProtoAlias {
					t.Errorf("expected proto AgentAlias %q, got %q", tc.expectedProtoAlias, lastReq.GetAgentAlias())
				}
			} else {
				if lastReq.AgentAlias != nil {
					t.Errorf("expected proto AgentAlias field to NOT be sent (nil), but got %q", *lastReq.AgentAlias)
				}
			}
		})
	}
}

func TestCrewMemberSerializationBackwardCompatibility(t *testing.T) {
	// 1. Unmarshal legacy JSON without agent_alias
	legacyJSON := `{"id":"legacy_01","name":"Legacy Agent","role":"Worker","status":"idle","capabilities":["read"]}`
	var member crew.CrewMember
	if err := json.Unmarshal([]byte(legacyJSON), &member); err != nil {
		t.Fatalf("failed to unmarshal legacy json: %v", err)
	}
	if member.AgentAlias != "" {
		t.Errorf("expected empty AgentAlias for legacy JSON, got %q", member.AgentAlias)
	}

	// 2. Marshal with empty AgentAlias -> omitempty excludes agent_alias
	data, err := json.Marshal(member)
	if err != nil {
		t.Fatalf("failed to marshal member: %v", err)
	}
	var rawMap map[string]any
	if err := json.Unmarshal(data, &rawMap); err != nil {
		t.Fatalf("failed to unmarshal into map: %v", err)
	}
	if _, exists := rawMap["agent_alias"]; exists {
		t.Errorf("expected agent_alias to be omitted in JSON when empty, but was present: %s", string(data))
	}

	// 3. Marshal with populated AgentAlias -> agent_alias included
	member.AgentAlias = "worker_alias"
	dataWithAlias, err := json.Marshal(member)
	if err != nil {
		t.Fatalf("failed to marshal member with alias: %v", err)
	}
	var rawMapWithAlias map[string]any
	if err := json.Unmarshal(dataWithAlias, &rawMapWithAlias); err != nil {
		t.Fatalf("failed to unmarshal into map: %v", err)
	}
	if rawMapWithAlias["agent_alias"] != "worker_alias" {
		t.Errorf("expected agent_alias 'worker_alias', got %v", rawMapWithAlias["agent_alias"])
	}
}
