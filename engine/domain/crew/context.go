package crew

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
)

const (
	DefaultMaxContextTokens = 8192
	DefaultReservedTokens   = 2048
	TruncationBreadcrumb    = "[... earlier conversation history truncated to fit context limits ...]"
	DefaultSystemPrompt     = "You are Galleon AI Agent, an advanced collaborative engineering intelligence."
)

// ContextWindowPayload represents deserialized context metadata from gateway wire
type ContextWindowPayload struct {
	SystemPrompt string        `json:"system_prompt,omitempty"`
	Persona      string        `json:"persona,omitempty"`
	LearnedRules []string      `json:"learned_rules,omitempty"`
	History      []ChatMessage `json:"history,omitempty"`
	MaxTokens    int           `json:"max_tokens,omitempty"`
}

// DefaultContextResolver implements ContextResolver
type DefaultContextResolver struct {
	tokenCounter TokenCounter
	maxTokens    int
}

// NewContextResolver creates a new ContextResolver instance
func NewContextResolver(tokenCounter TokenCounter) ContextResolver {
	if tokenCounter == nil {
		tokenCounter = NewStandardTokenCounter()
	}
	return &DefaultContextResolver{
		tokenCounter: tokenCounter,
		maxTokens:    DefaultMaxContextTokens,
	}
}

// ResolveContext resolves system prompts, persona directives, learned memory, history, and tools
func (r *DefaultContextResolver) ResolveContext(
	ctx context.Context,
	req *TurnRequest,
	availableTools []ToolDefinition,
) (*ResolvedContext, error) {
	if req == nil {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "turn request cannot be nil", appErrors.LayerService)
	}

	systemPrompt := req.SystemPrompt
	if systemPrompt == "" {
		systemPrompt = DefaultSystemPrompt
	}

	var parsedPayload ContextWindowPayload
	if len(req.ContextWindow) > 0 {
		if err := json.Unmarshal(req.ContextWindow, &parsedPayload); err == nil {
			if parsedPayload.SystemPrompt != "" {
				systemPrompt = parsedPayload.SystemPrompt
			}
		} else {
			// If not JSON, treat raw bytes as memory preamble or extra instructions
			systemPrompt = fmt.Sprintf("%s\n\n[Injected Context]:\n%s", systemPrompt, string(req.ContextWindow))
		}
	}

	// 1. Resolve agent persona and role specialization
	persona := parsedPayload.Persona
	if persona == "" && req.AgentID != "" {
		persona = r.resolveDefaultPersona(req.AgentID)
	}
	if persona != "" {
		systemPrompt = fmt.Sprintf("%s\n\n[Agent Role & Persona Directive]:\n%s", systemPrompt, persona)
	}

	// 2. Append learned rules and memory recall
	if len(parsedPayload.LearnedRules) > 0 {
		var sb strings.Builder
		sb.WriteString("\n\n[Learned Context & Memory Preamble]:")
		for _, rule := range parsedPayload.LearnedRules {
			sb.WriteString(fmt.Sprintf("\n- %s", rule))
		}
		systemPrompt += sb.String()
	}

	// 3. Assemble history messages
	var allMessages []ChatMessage

	// Merge history from context window and request
	if len(parsedPayload.History) > 0 {
		allMessages = append(allMessages, parsedPayload.History...)
	}
	if len(req.History) > 0 {
		allMessages = append(allMessages, req.History...)
	}

	// Append the user's initiating prompt if non-empty
	if req.Prompt != "" {
		// Avoid duplicate if the last message in history is already the exact same prompt
		isDuplicate := false
		if len(allMessages) > 0 {
			last := allMessages[len(allMessages)-1]
			if last.Role == RoleUser && last.Content == req.Prompt {
				isDuplicate = true
			}
		}
		if !isDuplicate {
			allMessages = append(allMessages, ChatMessage{
				Role:    RoleUser,
				Content: req.Prompt,
			})
		}
	}

	// 4. Enforce Context Limits & Sliding Window Pruning
	maxBudget := r.maxTokens - DefaultReservedTokens
	if parsedPayload.MaxTokens > DefaultReservedTokens {
		maxBudget = parsedPayload.MaxTokens - DefaultReservedTokens
	}

	prunedMessages := r.pruneHistoryToFit(systemPrompt, allMessages, availableTools, maxBudget)

	// 5. Deduplicate and collect tools
	dedupTools := make([]ToolDefinition, 0, len(availableTools))
	seenToolNames := make(map[string]bool)
	for _, tool := range availableTools {
		if !seenToolNames[tool.Name] && tool.Name != "" {
			seenToolNames[tool.Name] = true
			dedupTools = append(dedupTools, tool)
		}
	}

	resolved := &ResolvedContext{
		SystemPrompt: systemPrompt,
		Messages:     prunedMessages,
		Tools:        dedupTools,
	}

	resolved.EstimatedTokens = r.tokenCounter.EstimateContextTokens(resolved)

	return resolved, nil
}

// pruneHistoryToFit preserves system context and recent messages within the token budget
func (r *DefaultContextResolver) pruneHistoryToFit(
	systemPrompt string,
	messages []ChatMessage,
	tools []ToolDefinition,
	maxBudget int,
) []ChatMessage {
	if len(messages) <= 1 {
		return messages
	}

	// Calculate base tokens consumed by system prompt and tools
	baseTokens := 4 + r.tokenCounter.CountTokens(systemPrompt)
	for _, tool := range tools {
		baseTokens += 8 + r.tokenCounter.CountTokens(tool.Name) + r.tokenCounter.CountTokens(tool.Description) + r.tokenCounter.CountTokens(tool.ParametersJSON)
	}

	availableForMessages := maxBudget - baseTokens
	if availableForMessages <= 0 {
		// Extreme limit: keep only the last message
		return messages[len(messages)-1:]
	}

	// Work backwards from the newest message to the oldest
	var kept []ChatMessage
	consumedTokens := 0
	truncated := false

	for i := len(messages) - 1; i >= 0; i-- {
		msgTokens := r.tokenCounter.EstimateMessageTokens(messages[i])
		if consumedTokens+msgTokens > availableForMessages {
			truncated = true
			break
		}
		consumedTokens += msgTokens
		kept = append([]ChatMessage{messages[i]}, kept...)
	}

	if truncated && len(kept) < len(messages) {
		breadcrumbMsg := ChatMessage{
			Role:    RoleSystem,
			Content: TruncationBreadcrumb,
		}
		kept = append([]ChatMessage{breadcrumbMsg}, kept...)
	}

	return kept
}

func (r *DefaultContextResolver) resolveDefaultPersona(agentID string) string {
	switch strings.ToLower(agentID) {
	case "planner":
		return "Role: Strategic Task Planner. Decompose complex multi-step objectives into structured acyclic execution graphs with clear acceptance criteria."
	case "coder":
		return "Role: Senior Systems Software Engineer. Implement clean, idiomatic, and highly efficient code adhering strictly to zero-unwrap and error propagation standards."
	case "reviewer":
		return "Role: Principal Code Reviewer & Gatekeeper. Verify adherence to single-source-of-truth invariants, performance constraints, and rigorous test coverage."
	case "qa", "qa_agent":
		return "Role: Quality Assurance Specialist. Execute comprehensive end-to-end integration verifications, edge-case regressions, and concurrency stress testing."
	case "navigator":
		return "Role: Autonomous Fleet Navigator. Route requests across bounded contexts, coordinate sub-agents, and supervise task execution lifecycles."
	default:
		return fmt.Sprintf("Role: Specialized Agent '%s'. Follow user instructions with rigor and clarity.", agentID)
	}
}
