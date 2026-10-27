package crew

import (
	"context"
	"encoding/json"
	"fmt"
	"io"

	"github.com/diezy-realm/galleon-fleet/engine/pkg/client"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
)

// SystemGatewayLLMAdapter adapts client.SystemGatewayClient to domain.LLMProvider
type SystemGatewayLLMAdapter struct {
	gateway client.SystemGatewayClient
}

// NewSystemGatewayLLMAdapter constructs a new adapter instance
func NewSystemGatewayLLMAdapter(gateway client.SystemGatewayClient) LLMProvider {
	return &SystemGatewayLLMAdapter{gateway: gateway}
}

func (a *SystemGatewayLLMAdapter) ExecuteTurn(
	ctx context.Context,
	sessionID, model, systemPrompt, agentAlias string,
	messages []ChatMessage,
	tools []ToolDefinition,
) (LLMStream, error) {
	protoMessages := make([]*pb.TurnChatMessage, 0, len(messages))
	for _, m := range messages {
		protoMsg := &pb.TurnChatMessage{
			Role:    m.Role,
			Content: m.Content,
		}
		if m.ToolCallID != "" {
			protoMsg.ToolCallId = &m.ToolCallID
		}
		if m.Name != "" {
			protoMsg.Name = &m.Name
		}
		protoMessages = append(protoMessages, protoMsg)
	}

	protoTools := make([]*pb.TurnToolDefinition, 0, len(tools))
	for _, t := range tools {
		protoTools = append(protoTools, &pb.TurnToolDefinition{
			Name:           t.Name,
			Description:    t.Description,
			ParametersJson: t.ParametersJSON,
		})
	}

	protoReq := &pb.ExecuteTurnRequest{
		SessionId:    sessionID,
		Model:        model,
		SystemPrompt: systemPrompt,
		Messages:     protoMessages,
		Tools:        protoTools,
	}
	if agentAlias != "" {
		protoReq.AgentAlias = &agentAlias
	}

	stream, err := a.gateway.ExecuteTurn(ctx, protoReq)
	if err != nil {
		return nil, fmt.Errorf("failed to call system gateway ExecuteTurn: %w", err)
	}

	return &grpcLLMStream{stream: stream}, nil
}

type grpcLLMStream struct {
	stream pb.SystemGateway_ExecuteTurnClient
}

func (s *grpcLLMStream) Recv() (*StreamToken, error) {
	resp, err := s.stream.Recv()
	if err != nil {
		return nil, err
	}

	switch ev := resp.Event.(type) {
	case *pb.TurnToken_Token:
		return &StreamToken{
			DeltaText: ev.Token.Delta,
			IsThought: ev.Token.IsThought,
		}, nil
	case *pb.TurnToken_ToolCall:
		return &StreamToken{
			ToolCall: &ToolCall{
				ID:        ev.ToolCall.Id,
				Name:      ev.ToolCall.Name,
				Arguments: ev.ToolCall.ArgumentsJson,
			},
		}, nil
	case *pb.TurnToken_Complete:
		return &StreamToken{
			FinishReason:     ev.Complete.FinishReason,
			PromptTokens:     int(ev.Complete.PromptTokens),
			CompletionTokens: int(ev.Complete.CompletionTokens),
			TotalTokens:      int(ev.Complete.TotalTokens),
		}, nil
	case *pb.TurnToken_Error:
		return &StreamToken{
			ErrorMessage: ev.Error.Message,
		}, nil
	default:
		return &StreamToken{}, nil
	}
}

func (s *grpcLLMStream) Close() error {
	return nil
}

// GatewayToolExecutor executes platform tools via SystemGatewayClient
type GatewayToolExecutor struct {
	gateway        client.SystemGatewayClient
	availableTools []ToolDefinition
}

// NewGatewayToolExecutor constructs a tool executor utilizing the SystemGatewayClient
func NewGatewayToolExecutor(gateway client.SystemGatewayClient, tools []ToolDefinition) ToolExecutor {
	return &GatewayToolExecutor{
		gateway:        gateway,
		availableTools: tools,
	}
}

func (e *GatewayToolExecutor) GetAvailableTools() []ToolDefinition {
	return e.availableTools
}

func (e *GatewayToolExecutor) ExecuteTool(ctx context.Context, call *ToolCall) (*ToolResult, error) {
	if call == nil {
		return nil, fmt.Errorf("tool call cannot be nil")
	}

	// Handle sub-agent delegation keywords natively
	if call.Name == "spawn_subagent" || call.Name == "delegate_task" {
		var args struct {
			SubagentID string `json:"subagent_id"`
			AgentID    string `json:"agent_id"`
			Task       string `json:"task"`
		}
		_ = json.Unmarshal([]byte(call.Arguments), &args)

		subID := args.SubagentID
		if subID == "" {
			subID = args.AgentID
		}
		if subID == "" {
			subID = "subagent-1"
		}

		return &ToolResult{
			ToolCallID:       call.ID,
			Output:           fmt.Sprintf("Subagent '%s' dispatched successfully for task: %s", subID, args.Task),
			IsSubagentAction: true,
			SubagentID:       subID,
			SubagentTask:     args.Task,
		}, nil
	}

	if e.gateway == nil {
		return &ToolResult{
			ToolCallID: call.ID,
			Output:     fmt.Sprintf("Simulated tool '%s' response", call.Name),
		}, nil
	}

	out, err := e.gateway.ExecuteNativeTool(ctx, call.Name, call.Arguments)
	if err != nil {
		return nil, fmt.Errorf("native tool execution failed: %w", err)
	}

	return &ToolResult{
		ToolCallID: call.ID,
		Output:     out,
	}, nil
}

// MockLLMStream provides deterministic streaming for testing and replays
type MockLLMStream struct {
	Tokens []*StreamToken
	idx    int
}

func (m *MockLLMStream) Recv() (*StreamToken, error) {
	if m.idx >= len(m.Tokens) {
		return nil, io.EOF
	}
	t := m.Tokens[m.idx]
	m.idx++
	return t, nil
}

func (m *MockLLMStream) Close() error {
	return nil
}

// MockLLMProvider simulates an LLM provider with canned or step-driven responses
type MockLLMProvider struct {
	ResponsesFunc func(messages []ChatMessage, tools []ToolDefinition) []*StreamToken
}

func (p *MockLLMProvider) ExecuteTurn(
	ctx context.Context,
	sessionID, model, systemPrompt, agentAlias string,
	messages []ChatMessage,
	tools []ToolDefinition,
) (LLMStream, error) {
	if p.ResponsesFunc != nil {
		return &MockLLMStream{Tokens: p.ResponsesFunc(messages, tools)}, nil
	}
	return &MockLLMStream{
		Tokens: []*StreamToken{
			{DeltaText: "Mock LLM default response"},
			{FinishReason: "stop", PromptTokens: 10, CompletionTokens: 5, TotalTokens: 15},
		},
	}, nil
}

// MockToolExecutor simulates tool execution for testing
type MockToolExecutor struct {
	AvailableTools []ToolDefinition
	ExecuteFunc    func(ctx context.Context, call *ToolCall) (*ToolResult, error)
}

func (m *MockToolExecutor) GetAvailableTools() []ToolDefinition {
	return m.AvailableTools
}

func (m *MockToolExecutor) ExecuteTool(ctx context.Context, call *ToolCall) (*ToolResult, error) {
	if m.ExecuteFunc != nil {
		return m.ExecuteFunc(ctx, call)
	}
	return &ToolResult{
		ToolCallID: call.ID,
		Output:     fmt.Sprintf("Mock result for tool %s", call.Name),
	}, nil
}
