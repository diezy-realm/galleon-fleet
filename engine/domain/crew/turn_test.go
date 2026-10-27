package crew_test

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"sync"
	"testing"
	"time"

	domainCrew "github.com/diezy-realm/galleon-fleet/engine/domain/crew"
)

func TestTurnLifecycle_SimpleTextTurn(t *testing.T) {
	mockLLM := &domainCrew.MockLLMProvider{
		ResponsesFunc: func(messages []domainCrew.ChatMessage, tools []domainCrew.ToolDefinition) []*domainCrew.StreamToken {
			return []*domainCrew.StreamToken{
				{DeltaText: "Analyzing task requirements...", IsThought: true},
				{DeltaText: "Hello, world! I have completed the assigned task."},
				{FinishReason: "stop", PromptTokens: 42, CompletionTokens: 18, TotalTokens: 60},
			}
		},
	}

	tokenCounter := domainCrew.NewStandardTokenCounter()
	resolver := domainCrew.NewContextResolver(tokenCounter)
	engine := domainCrew.NewTurnEngine(resolver, tokenCounter, mockLLM, nil)

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	req := &domainCrew.TurnRequest{
		SessionID: "sess-simple-text-1",
		AgentID:   "coder",
		Prompt:    "Write a hello world program",
	}

	eventCh := make(chan *domainCrew.TurnEvent, 16)
	var events []*domainCrew.TurnEvent
	var wg sync.WaitGroup
	wg.Add(1)

	go func() {
		defer wg.Done()
		for ev := range eventCh {
			events = append(events, ev)
		}
	}()

	outcome, err := engine.RunTurn(ctx, req, eventCh)
	close(eventCh)
	wg.Wait()

	if err != nil {
		t.Fatalf("unexpected turn error: %v", err)
	}

	if outcome.FinalResponse != "Hello, world! I have completed the assigned task." {
		t.Errorf("unexpected final response: %q", outcome.FinalResponse)
	}

	if outcome.Iterations != 1 {
		t.Errorf("expected 1 iteration, got %d", outcome.Iterations)
	}

	if outcome.TokenUsage.TotalTokens != 60 {
		t.Errorf("expected 60 tokens, got %d", outcome.TokenUsage.TotalTokens)
	}

	if outcome.TokenUsage.CostUSD <= 0 {
		t.Errorf("expected non-zero cost calculation, got %f", outcome.TokenUsage.CostUSD)
	}

	var hasThought, hasText, hasCompleted bool
	for _, ev := range events {
		switch ev.Type {
		case domainCrew.EventThoughtChunk:
			hasThought = true
		case domainCrew.EventTextChunk:
			hasText = true
		case domainCrew.EventTurnCompleted:
			hasCompleted = true
		}
	}

	if !hasThought {
		t.Errorf("expected thought chunk event")
	}
	if !hasText {
		t.Errorf("expected text chunk event")
	}
	if !hasCompleted {
		t.Errorf("expected turn completed event")
	}
}

func TestTurnLifecycle_MultiStepToolCall(t *testing.T) {
	callCount := 0

	mockLLM := &domainCrew.MockLLMProvider{
		ResponsesFunc: func(messages []domainCrew.ChatMessage, tools []domainCrew.ToolDefinition) []*domainCrew.StreamToken {
			callCount++
			if callCount == 1 {
				// First iteration: model decides to call tool
				return []*domainCrew.StreamToken{
					{DeltaText: "Need to compute factorial of 5", IsThought: true},
					{
						ToolCall: &domainCrew.ToolCall{
							ID:        "call_calc_123",
							Name:      "calculator",
							Arguments: `{"operation":"factorial","value":5}`,
						},
					},
					{FinishReason: "tool_calls", PromptTokens: 30, CompletionTokens: 10, TotalTokens: 40},
				}
			}

			// Second iteration: model receives tool output and gives final response
			return []*domainCrew.StreamToken{
				{DeltaText: "The result of 5! is 120."},
				{FinishReason: "stop", PromptTokens: 50, CompletionTokens: 8, TotalTokens: 58},
			}
		},
	}

	mockTools := &domainCrew.MockToolExecutor{
		AvailableTools: []domainCrew.ToolDefinition{
			{Name: "calculator", Description: "Math calculation utility", ParametersJSON: `{"type":"object"}`},
		},
		ExecuteFunc: func(ctx context.Context, call *domainCrew.ToolCall) (*domainCrew.ToolResult, error) {
			if call.Name == "calculator" {
				return &domainCrew.ToolResult{
					ToolCallID: call.ID,
					Output:     "120",
				}, nil
			}
			return nil, fmt.Errorf("unknown tool %s", call.Name)
		},
	}

	tokenCounter := domainCrew.NewStandardTokenCounter()
	resolver := domainCrew.NewContextResolver(tokenCounter)
	engine := domainCrew.NewTurnEngine(resolver, tokenCounter, mockLLM, mockTools)

	ctx := context.Background()
	req := &domainCrew.TurnRequest{
		SessionID: "sess-tool-loop-1",
		AgentID:   "planner",
		Prompt:    "Calculate 5!",
	}

	eventCh := make(chan *domainCrew.TurnEvent, 32)
	var events []*domainCrew.TurnEvent
	var wg sync.WaitGroup
	wg.Add(1)

	go func() {
		defer wg.Done()
		for ev := range eventCh {
			events = append(events, ev)
		}
	}()

	outcome, err := engine.RunTurn(ctx, req, eventCh)
	close(eventCh)
	wg.Wait()

	if err != nil {
		t.Fatalf("unexpected turn error: %v", err)
	}

	if outcome.Iterations != 2 {
		t.Errorf("expected 2 iterations for tool loop, got %d", outcome.Iterations)
	}

	if outcome.FinalResponse != "The result of 5! is 120." {
		t.Errorf("unexpected final response: %q", outcome.FinalResponse)
	}

	var hasStarted, hasFinished bool
	for _, ev := range events {
		switch ev.Type {
		case domainCrew.EventToolCallStarted:
			hasStarted = true
			if !strings.Contains(ev.Content, "calculator") {
				t.Errorf("expected tool name in start event, got %q", ev.Content)
			}
		case domainCrew.EventToolCallFinished:
			hasFinished = true
			if !strings.Contains(ev.Content, "120") {
				t.Errorf("expected tool output in finish event, got %q", ev.Content)
			}
		}
	}

	if !hasStarted {
		t.Errorf("expected EventToolCallStarted")
	}
	if !hasFinished {
		t.Errorf("expected EventToolCallFinished")
	}
}

func TestTurnLifecycle_SubagentDelegation(t *testing.T) {
	mockLLM := &domainCrew.MockLLMProvider{
		ResponsesFunc: func(messages []domainCrew.ChatMessage, tools []domainCrew.ToolDefinition) []*domainCrew.StreamToken {
			if len(messages) <= 2 {
				return []*domainCrew.StreamToken{
					{
						ToolCall: &domainCrew.ToolCall{
							ID:        "call_subagent_1",
							Name:      "spawn_subagent",
							Arguments: `{"subagent_id":"researcher-1","task":"Find benchmark numbers"}`,
						},
					},
					{FinishReason: "tool_calls"},
				}
			}
			return []*domainCrew.StreamToken{
				{DeltaText: "Delegation completed."},
				{FinishReason: "stop"},
			}
		},
	}

	executor := domainCrew.NewGatewayToolExecutor(nil, []domainCrew.ToolDefinition{
		{Name: "spawn_subagent", Description: "Dispatches delegated subagent"},
	})

	tokenCounter := domainCrew.NewStandardTokenCounter()
	resolver := domainCrew.NewContextResolver(tokenCounter)
	engine := domainCrew.NewTurnEngine(resolver, tokenCounter, mockLLM, executor)

	eventCh := make(chan *domainCrew.TurnEvent, 16)
	var events []*domainCrew.TurnEvent
	var wg sync.WaitGroup
	wg.Add(1)
	go func() {
		defer wg.Done()
		for ev := range eventCh {
			events = append(events, ev)
		}
	}()

	outcome, err := engine.RunTurn(context.Background(), &domainCrew.TurnRequest{
		SessionID: "sess-subagent-1",
		AgentID:   "lead",
		Prompt:    "Delegate benchmark research",
	}, eventCh)
	close(eventCh)
	wg.Wait()

	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if outcome.Iterations != 2 {
		t.Errorf("expected 2 iterations, got %d", outcome.Iterations)
	}

	var hasSubagentSpawned bool
	for _, ev := range events {
		if ev.Type == domainCrew.EventSubagentSpawned {
			hasSubagentSpawned = true
			if ev.SubagentID != "researcher-1" {
				t.Errorf("expected subagent_id researcher-1, got %q", ev.SubagentID)
			}
		}
	}

	if !hasSubagentSpawned {
		t.Errorf("expected EventSubagentSpawned")
	}
}

func TestContextResolver_AssemblyAndPersona(t *testing.T) {
	tokenCounter := domainCrew.NewStandardTokenCounter()
	resolver := domainCrew.NewContextResolver(tokenCounter)

	ctxWindow, _ := json.Marshal(domainCrew.ContextWindowPayload{
		SystemPrompt: "Custom master prompt.",
		LearnedRules: []string{"Always return JSON", "Enforce strict types"},
		History: []domainCrew.ChatMessage{
			{Role: domainCrew.RoleUser, Content: "prior question"},
			{Role: domainCrew.RoleAssistant, Content: "prior answer"},
		},
	})

	req := &domainCrew.TurnRequest{
		SessionID:     "s-1",
		AgentID:       "qa",
		Prompt:        "Run regression checks",
		ContextWindow: ctxWindow,
	}

	tools := []domainCrew.ToolDefinition{
		{Name: "run_test", Description: "runs tests"},
		{Name: "run_test", Description: "duplicate"}, // should be deduped
	}

	resolved, err := resolver.ResolveContext(context.Background(), req, tools)
	if err != nil {
		t.Fatalf("failed to resolve context: %v", err)
	}

	if !strings.Contains(resolved.SystemPrompt, "Custom master prompt.") {
		t.Errorf("missing custom master prompt: %s", resolved.SystemPrompt)
	}
	if !strings.Contains(resolved.SystemPrompt, "Always return JSON") {
		t.Errorf("missing learned rule: %s", resolved.SystemPrompt)
	}
	if !strings.Contains(resolved.SystemPrompt, "Quality Assurance Specialist") {
		t.Errorf("missing QA persona directive: %s", resolved.SystemPrompt)
	}

	if len(resolved.Tools) != 1 {
		t.Errorf("expected 1 deduplicated tool, got %d", len(resolved.Tools))
	}

	if len(resolved.Messages) != 3 {
		t.Fatalf("expected 3 messages, got %d", len(resolved.Messages))
	}

	if resolved.Messages[2].Content != "Run regression checks" {
		t.Errorf("last message should be user prompt, got: %q", resolved.Messages[2].Content)
	}

	if resolved.EstimatedTokens <= 0 {
		t.Errorf("expected positive estimated tokens, got %d", resolved.EstimatedTokens)
	}
}

func TestContextResolver_SlidingWindowPruning(t *testing.T) {
	tokenCounter := domainCrew.NewStandardTokenCounter()
	resolver := domainCrew.NewContextResolver(tokenCounter)

	// Create long conversation history
	var longHistory []domainCrew.ChatMessage
	for i := 0; i < 50; i++ {
		longHistory = append(longHistory, domainCrew.ChatMessage{
			Role:    domainCrew.RoleUser,
			Content: fmt.Sprintf("Detailed long message %d with extra words explaining various scenarios to take up substantial tokens in context.", i),
		})
	}

	ctxWindow, _ := json.Marshal(domainCrew.ContextWindowPayload{
		MaxTokens: 2200, // Small context limit to force truncation
		History:   longHistory,
	})

	req := &domainCrew.TurnRequest{
		SessionID:     "sess-prune-1",
		Prompt:        "Final latest prompt",
		ContextWindow: ctxWindow,
	}

	resolved, err := resolver.ResolveContext(context.Background(), req, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(resolved.Messages) >= len(longHistory) {
		t.Errorf("expected history to be pruned, got %d messages", len(resolved.Messages))
	}

	hasBreadcrumb := false
	for _, m := range resolved.Messages {
		if strings.Contains(m.Content, "truncated") {
			hasBreadcrumb = true
			break
		}
	}

	if !hasBreadcrumb {
		t.Errorf("expected truncation breadcrumb in pruned messages")
	}

	lastMsg := resolved.Messages[len(resolved.Messages)-1]
	if lastMsg.Content != "Final latest prompt" {
		t.Errorf("expected latest prompt preserved, got: %q", lastMsg.Content)
	}
}

func TestTokenCounter_CalculationAndCost(t *testing.T) {
	counter := domainCrew.NewStandardTokenCounter()

	tokens := counter.CountTokens("The quick brown fox jumps over the lazy dog.")
	if tokens < 8 || tokens > 14 {
		t.Errorf("expected ~10 tokens for test sentence, got %d", tokens)
	}

	costSonnet := counter.CalculateCost(1000, 500, "claude-3-5-sonnet-20241022")
	expectedCost := (1000.0/1_000_000.0)*2.50 + (500.0/1_000_000.0)*10.00
	if fmt.Sprintf("%.6f", costSonnet) != fmt.Sprintf("%.6f", expectedCost) {
		t.Errorf("expected cost %f, got %f", expectedCost, costSonnet)
	}

	costFlash := counter.CalculateCost(1000, 500, "gemini-1.5-flash")
	if costFlash >= costSonnet {
		t.Errorf("expected flash cost to be lower than sonnet")
	}

	// Reconciliation
	reconciled := counter.Reconcile(
		domainCrew.TokenUsage{PromptTokens: 10, CompletionTokens: 5, TotalTokens: 15},
		domainCrew.TokenUsage{PromptTokens: 12, CompletionTokens: 6, TotalTokens: 18},
	)
	if reconciled.TotalTokens != 18 {
		t.Errorf("expected reconciled tokens 18, got %d", reconciled.TotalTokens)
	}
}

func TestTurnLifecycle_Cancellation(t *testing.T) {
	mockLLM := &domainCrew.MockLLMProvider{
		ResponsesFunc: func(messages []domainCrew.ChatMessage, tools []domainCrew.ToolDefinition) []*domainCrew.StreamToken {
			time.Sleep(100 * time.Millisecond)
			return []*domainCrew.StreamToken{
				{DeltaText: "delayed"},
				{FinishReason: "stop"},
			}
		},
	}

	tokenCounter := domainCrew.NewStandardTokenCounter()
	resolver := domainCrew.NewContextResolver(tokenCounter)
	engine := domainCrew.NewTurnEngine(resolver, tokenCounter, mockLLM, nil)

	ctx, cancel := context.WithCancel(context.Background())
	cancel() // Cancel immediately

	_, err := engine.RunTurn(ctx, &domainCrew.TurnRequest{
		SessionID: "sess-cancel-1",
		Prompt:    "should cancel",
	}, nil)

	if err == nil {
		t.Fatalf("expected turn cancellation error, got nil")
	}
}

func TestTurnLifecycle_LoopDetector(t *testing.T) {
	mockLLM := &domainCrew.MockLLMProvider{
		ResponsesFunc: func(messages []domainCrew.ChatMessage, tools []domainCrew.ToolDefinition) []*domainCrew.StreamToken {
			// Model gets stuck calling the exact same tool endlessly
			return []*domainCrew.StreamToken{
				{
					ToolCall: &domainCrew.ToolCall{
						ID:        "stuck_call",
						Name:      "read_file",
						Arguments: `{"path":"stuck.txt"}`,
					},
				},
				{FinishReason: "tool_calls"},
			}
		},
	}

	executor := &domainCrew.MockToolExecutor{
		ExecuteFunc: func(ctx context.Context, call *domainCrew.ToolCall) (*domainCrew.ToolResult, error) {
			return &domainCrew.ToolResult{ToolCallID: call.ID, Output: "empty"}, nil
		},
	}

	tokenCounter := domainCrew.NewStandardTokenCounter()
	resolver := domainCrew.NewContextResolver(tokenCounter)
	engine := domainCrew.NewTurnEngine(resolver, tokenCounter, mockLLM, executor)
	engine.SetMaxIterations(10)

	outcome, err := engine.RunTurn(context.Background(), &domainCrew.TurnRequest{
		SessionID: "sess-loop-detector",
		Prompt:    "trigger loop",
	}, nil)

	if err != nil {
		t.Fatalf("turn should complete gracefully on loop detection: %v", err)
	}

	if outcome.FinishReason != "loop_detected" {
		t.Errorf("expected finish_reason loop_detected, got %q", outcome.FinishReason)
	}
}
