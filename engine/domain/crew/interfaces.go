package crew

import (
	"context"
)

// Standard chat message roles
const (
	RoleSystem    = "system"
	RoleUser      = "user"
	RoleAssistant = "assistant"
	RoleTool      = "tool"
)

// EventType defines streaming turn event types matching proto/wire contracts
type EventType int

const (
	EventThoughtChunk EventType = iota
	EventTextChunk
	EventToolCallStarted
	EventToolCallFinished
	EventSubagentSpawned
	EventTurnCompleted
	EventError
)

func (e EventType) String() string {
	switch e {
	case EventThoughtChunk:
		return "THOUGHT_CHUNK"
	case EventTextChunk:
		return "TEXT_CHUNK"
	case EventToolCallStarted:
		return "TOOL_CALL_STARTED"
	case EventToolCallFinished:
		return "TOOL_CALL_FINISHED"
	case EventSubagentSpawned:
		return "SUBAGENT_SPAWNED"
	case EventTurnCompleted:
		return "TURN_COMPLETED"
	case EventError:
		return "ERROR"
	default:
		return "UNKNOWN"
	}
}

// TurnEvent represents an event emitted during turn execution
type TurnEvent struct {
	Type         EventType `json:"type"`
	Content      string    `json:"content"`
	SubagentID   string    `json:"subagent_id,omitempty"`
	ErrorMessage string    `json:"error_message,omitempty"`
}

// ChatMessage represents a structured conversation message
type ChatMessage struct {
	Role       string `json:"role"`
	Content    string `json:"content"`
	ToolCallID string `json:"tool_call_id,omitempty"`
	Name       string `json:"name,omitempty"`
}

// ToolDefinition describes a callable tool schema
type ToolDefinition struct {
	Name           string `json:"name"`
	Description    string `json:"description"`
	ParametersJSON string `json:"parameters_json"`
}

// ToolCall describes an invocation of a tool by the LLM
type ToolCall struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Arguments string `json:"arguments"`
}

// ToolResult contains the output or error resulting from a tool execution
type ToolResult struct {
	ToolCallID       string `json:"tool_call_id"`
	Output           string `json:"output"`
	Error            error  `json:"-"`
	IsSubagentAction bool   `json:"is_subagent_action"`
	SubagentID       string `json:"subagent_id,omitempty"`
	SubagentTask     string `json:"subagent_task,omitempty"`
}

// TokenUsage contains counted prompt, completion, and total tokens + cost
type TokenUsage struct {
	PromptTokens     int     `json:"prompt_tokens"`
	CompletionTokens int     `json:"completion_tokens"`
	TotalTokens      int     `json:"total_tokens"`
	CostUSD          float64 `json:"cost_usd"`
}

// TurnRequest defines the domain request to execute an agent turn
type TurnRequest struct {
	SessionID     string        `json:"session_id"`
	AgentID       string        `json:"agent_id"`
	Prompt        string        `json:"prompt"`
	ContextWindow []byte        `json:"context_window,omitempty"`
	History       []ChatMessage `json:"history,omitempty"`
	SystemPrompt  string        `json:"system_prompt,omitempty"`
	MaxIterations int           `json:"max_iterations,omitempty"`
	Model         string        `json:"model,omitempty"`
	Provider      string        `json:"provider,omitempty"`
}

// TurnOutcome represents the final result of a completed turn
type TurnOutcome struct {
	SessionID     string        `json:"session_id"`
	AgentID       string        `json:"agent_id"`
	FinalResponse string        `json:"final_response"`
	Events        []*TurnEvent  `json:"events"`
	TokenUsage    TokenUsage    `json:"token_usage"`
	Iterations    int           `json:"iterations"`
	FinishReason  string        `json:"finish_reason"`
	Messages      []ChatMessage `json:"messages"`
}

// ResolvedContext represents assembled prompt context ready for LLM invocation
type ResolvedContext struct {
	SystemPrompt    string
	Messages        []ChatMessage
	Tools           []ToolDefinition
	EstimatedTokens int
}

// ContextResolver resolves and prunes context prompt, persona, memory, and history
type ContextResolver interface {
	ResolveContext(ctx context.Context, req *TurnRequest, availableTools []ToolDefinition) (*ResolvedContext, error)
}

// TokenCounter counts tokens, calculates costs, and tracks usage
type TokenCounter interface {
	CountTokens(text string) int
	EstimateMessageTokens(msg ChatMessage) int
	EstimateContextTokens(resolved *ResolvedContext) int
	CalculateCost(promptTokens, completionTokens int, model string) float64
	Reconcile(estimated, actual TokenUsage) TokenUsage
}

// LLMStream yields streaming tokens from the model provider
type LLMStream interface {
	Recv() (*StreamToken, error)
	Close() error
}

// StreamToken represents an event chunk from the model provider
type StreamToken struct {
	DeltaText        string
	IsThought        bool
	ToolCall         *ToolCall
	FinishReason     string
	PromptTokens     int
	CompletionTokens int
	TotalTokens      int
	ErrorMessage     string
}

// LLMProvider abstracts the model LLM provider invocation
type LLMProvider interface {
	ExecuteTurn(ctx context.Context, sessionID, model, systemPrompt, agentAlias string, messages []ChatMessage, tools []ToolDefinition) (LLMStream, error)
}

// ToolExecutor executes native tools and returns structured results
type ToolExecutor interface {
	ExecuteTool(ctx context.Context, call *ToolCall) (*ToolResult, error)
	GetAvailableTools() []ToolDefinition
}

// TurnLifecycle manages the full iterative turn execution loop
type TurnLifecycle interface {
	RunTurn(ctx context.Context, req *TurnRequest, eventCh chan<- *TurnEvent) (*TurnOutcome, error)
	CancelTurn(ctx context.Context, sessionID string) error
}
