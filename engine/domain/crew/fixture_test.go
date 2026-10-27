package crew_test

import (
	"context"
	"encoding/json"
	"testing"

	domainCrew "github.com/diezy-realm/galleon-fleet/engine/domain/crew"
)

// FixtureTrace represents the standard eval / regression trace schema
type FixtureTrace struct {
	ID        string `json:"id"`
	UserInput string `json:"user_input"`
	Steps     []struct {
		ToolCall *struct {
			Name      string `json:"name"`
			Arguments string `json:"arguments"`
		} `json:"tool_call,omitempty"`
		ToolOutput *string `json:"tool_output,omitempty"`
		Response   *struct {
			Type    string `json:"type"`
			Content string `json:"content"`
		} `json:"response,omitempty"`
	} `json:"steps"`
	FinalResponse string `json:"final_response"`
}

func TestFixtureCompatibility_EvalTraceReplay(t *testing.T) {
	// Standard trace JSON fixture matching clawcrew-eval trace schema
	rawJSON := `{
		"id": "trace-eval-001",
		"user_input": "Check git repository status and summarize",
		"steps": [
			{
				"tool_call": {
					"name": "git_status",
					"arguments": "{\"branch\":\"testing\"}"
				},
				"tool_output": "On branch testing. Nothing to commit."
			},
			{
				"response": {
					"type": "text",
					"content": "Repository is clean on branch testing."
				}
			}
		],
		"final_response": "Repository is clean on branch testing."
	}`

	var trace FixtureTrace
	if err := json.Unmarshal([]byte(rawJSON), &trace); err != nil {
		t.Fatalf("failed to unmarshal trace fixture: %v", err)
	}

	stepIdx := 0
	mockLLM := &domainCrew.MockLLMProvider{
		ResponsesFunc: func(messages []domainCrew.ChatMessage, tools []domainCrew.ToolDefinition) []*domainCrew.StreamToken {
			if stepIdx == 0 {
				stepIdx++
				return []*domainCrew.StreamToken{
					{
						ToolCall: &domainCrew.ToolCall{
							ID:        "fixture_step_1",
							Name:      trace.Steps[0].ToolCall.Name,
							Arguments: trace.Steps[0].ToolCall.Arguments,
						},
					},
					{FinishReason: "tool_calls"},
				}
			}

			// Final step
			return []*domainCrew.StreamToken{
				{DeltaText: trace.Steps[1].Response.Content},
				{FinishReason: "stop"},
			}
		},
	}

	toolExecutor := &domainCrew.MockToolExecutor{
		AvailableTools: []domainCrew.ToolDefinition{
			{Name: "git_status", Description: "Returns git repo status"},
		},
		ExecuteFunc: func(ctx context.Context, call *domainCrew.ToolCall) (*domainCrew.ToolResult, error) {
			if call.Name == "git_status" {
				return &domainCrew.ToolResult{
					ToolCallID: call.ID,
					Output:     *trace.Steps[0].ToolOutput,
				}, nil
			}
			return &domainCrew.ToolResult{ToolCallID: call.ID, Output: "ok"}, nil
		},
	}

	tokenCounter := domainCrew.NewStandardTokenCounter()
	resolver := domainCrew.NewContextResolver(tokenCounter)
	engine := domainCrew.NewTurnEngine(resolver, tokenCounter, mockLLM, toolExecutor)

	eventCh := make(chan *domainCrew.TurnEvent, 32)
	var events []*domainCrew.TurnEvent
	doneCh := make(chan struct{})

	go func() {
		defer close(doneCh)
		for ev := range eventCh {
			events = append(events, ev)
		}
	}()

	outcome, err := engine.RunTurn(context.Background(), &domainCrew.TurnRequest{
		SessionID: "trace-session-eval-1",
		AgentID:   "eval",
		Prompt:    trace.UserInput,
	}, eventCh)
	close(eventCh)
	<-doneCh

	if err != nil {
		t.Fatalf("turn execution failed on fixture replay: %v", err)
	}

	if outcome.FinalResponse != trace.FinalResponse {
		t.Errorf("expected final response %q, got %q", trace.FinalResponse, outcome.FinalResponse)
	}

	// Verify standard event ordering: ToolCallStarted -> ToolCallFinished -> TextChunk -> TurnCompleted
	var eventTypes []domainCrew.EventType
	for _, ev := range events {
		eventTypes = append(eventTypes, ev.Type)
	}

	if len(eventTypes) < 4 {
		t.Fatalf("expected at least 4 events in trace replay, got %d", len(eventTypes))
	}

	if eventTypes[0] != domainCrew.EventToolCallStarted {
		t.Errorf("expected first event to be EventToolCallStarted, got %v", eventTypes[0])
	}
	if eventTypes[1] != domainCrew.EventToolCallFinished {
		t.Errorf("expected second event to be EventToolCallFinished, got %v", eventTypes[1])
	}
	if eventTypes[2] != domainCrew.EventTextChunk {
		t.Errorf("expected third event to be EventTextChunk, got %v", eventTypes[2])
	}
	if eventTypes[3] != domainCrew.EventTurnCompleted {
		t.Errorf("expected fourth event to be EventTurnCompleted, got %v", eventTypes[3])
	}
}
