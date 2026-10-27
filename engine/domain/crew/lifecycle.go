package crew

import (
	"context"
	"fmt"
	"io"
	"log/slog"
	"sync"
	"time"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
	"github.com/diezy-realm/galleon-fleet/engine/core/logger"
	"github.com/diezy-realm/galleon-fleet/engine/core/metrics"
)

const (
	DefaultMaxIterations = 10
	DefaultModelName     = "default-engine"
	DefaultProviderName  = "galleon-runtime"
)

// TurnEngine implements TurnLifecycle orchestrating iterative turn execution,
// tool dispatching, context assembly, and token reconciliation.
type TurnEngine struct {
	mu              sync.RWMutex
	activeTurns     map[string]context.CancelFunc
	resolver        ContextResolver
	tokenCounter    TokenCounter
	llmProvider     LLMProvider
	toolExecutor    ToolExecutor
	maxIterations   int
}

// NewTurnEngine constructs a new TurnEngine instance
func NewTurnEngine(
	resolver ContextResolver,
	tokenCounter TokenCounter,
	llmProvider LLMProvider,
	toolExecutor ToolExecutor,
) *TurnEngine {
	if tokenCounter == nil {
		tokenCounter = NewStandardTokenCounter()
	}
	if resolver == nil {
		resolver = NewContextResolver(tokenCounter)
	}

	return &TurnEngine{
		activeTurns:   make(map[string]context.CancelFunc),
		resolver:      resolver,
		tokenCounter:  tokenCounter,
		llmProvider:   llmProvider,
		toolExecutor:  toolExecutor,
		maxIterations: DefaultMaxIterations,
	}
}

// SetMaxIterations overrides default iteration ceiling
func (e *TurnEngine) SetMaxIterations(max int) {
	e.mu.Lock()
	defer e.mu.Unlock()
	if max > 0 {
		e.maxIterations = max
	}
}

// RunTurn executes an end-to-end turn lifecycle loop with tool executions and streaming events
func (e *TurnEngine) RunTurn(
	ctx context.Context,
	req *TurnRequest,
	eventCh chan<- *TurnEvent,
) (*TurnOutcome, error) {
	log := logger.Get()

	if req == nil {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "request cannot be nil", appErrors.LayerService)
	}
	if req.SessionID == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "session_id cannot be empty", appErrors.LayerService)
	}
	if req.Prompt == "" && len(req.History) == 0 {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "prompt cannot be empty", appErrors.LayerService)
	}

	turnCtx, cancel := context.WithCancel(ctx)
	e.mu.Lock()
	e.activeTurns[req.SessionID] = cancel
	e.mu.Unlock()

	defer func() {
		e.mu.Lock()
		delete(e.activeTurns, req.SessionID)
		e.mu.Unlock()
		cancel()
	}()

	start := time.Now()
	metricAgent := req.AgentID
	if metricAgent == "" {
		metricAgent = "primary_agent"
	}

	metrics.ActiveAgents.Inc()
	defer func() {
		metrics.ActiveAgents.Dec()
		duration := time.Since(start).Seconds()
		metrics.AgentTurnDuration.WithLabelValues(metricAgent, "completed").Observe(duration)
	}()

	log.InfoContext(ctx, "initiating native go turn execution",
		slog.String("session_id", req.SessionID),
		slog.String("agent_id", metricAgent),
	)

	// 1. Resolve initial tools
	var availableTools []ToolDefinition
	if e.toolExecutor != nil {
		availableTools = e.toolExecutor.GetAvailableTools()
	}

	// 2. Resolve Context & Prompt Assembly
	resolvedContext, err := e.resolver.ResolveContext(turnCtx, req, availableTools)
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to resolve prompt context", appErrors.LayerService)
	}

	currentMessages := make([]ChatMessage, len(resolvedContext.Messages))
	copy(currentMessages, resolvedContext.Messages)

	maxIters := e.maxIterations
	if req.MaxIterations > 0 {
		maxIters = req.MaxIterations
	}

	modelName := req.Model
	if modelName == "" {
		modelName = DefaultModelName
	}
	providerName := req.Provider
	if providerName == "" {
		providerName = DefaultProviderName
	}

	var accumulatedEvents []*TurnEvent
	var fullFinalResponse string
	var totalPromptTokens int
	var totalCompletionTokens int
	var loopDetectionHistory = make(map[string]int)

	// Helper to send events both to caller channel and collect into outcome
	emitEvent := func(ev *TurnEvent) error {
		accumulatedEvents = append(accumulatedEvents, ev)
		if eventCh != nil {
			select {
			case <-turnCtx.Done():
				return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
			case eventCh <- ev:
			}
		}
		return nil
	}

	// 3. Iterative Turn Loop
	iteration := 0
	finishReason := "stop"

	for iteration < maxIters {
		iteration++

		select {
		case <-turnCtx.Done():
			return nil, appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
		default:
		}

		if e.llmProvider == nil {
			return nil, appErrors.New(appErrors.CodeUnavailable, "llm provider is not configured", appErrors.LayerService)
		}

		stream, err := e.llmProvider.ExecuteTurn(
			turnCtx,
			req.SessionID,
			modelName,
			resolvedContext.SystemPrompt,
			req.AgentID,
			currentMessages,
			resolvedContext.Tools,
		)
		if err != nil {
			return nil, appErrors.Wrap(err, appErrors.CodeUnavailable, "failed to invoke model provider", appErrors.LayerService)
		}

		var iterText string
		var iterThought string
		var iterToolCalls []*ToolCall
		iterPromptTokens := 0
		iterCompletionTokens := 0

		// Consume Stream
		for {
			token, err := stream.Recv()
			if err != nil {
				if err == io.EOF {
					break
				}
				select {
				case <-turnCtx.Done():
					_ = stream.Close()
					return nil, appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
				default:
					_ = stream.Close()
					return nil, appErrors.Wrap(err, appErrors.CodeInternal, "stream consumption error", appErrors.LayerService)
				}
			}

			if token.ErrorMessage != "" {
				_ = stream.Close()
				_ = emitEvent(&TurnEvent{
					Type:         EventError,
					Content:      token.ErrorMessage,
					ErrorMessage: token.ErrorMessage,
				})
				return nil, appErrors.New(appErrors.CodeInternal, token.ErrorMessage, appErrors.LayerService)
			}

			if token.IsThought && token.DeltaText != "" {
				iterThought += token.DeltaText
				if err := emitEvent(&TurnEvent{
					Type:    EventThoughtChunk,
					Content: token.DeltaText,
				}); err != nil {
					_ = stream.Close()
					return nil, err
				}
			} else if token.DeltaText != "" {
				iterText += token.DeltaText
				if err := emitEvent(&TurnEvent{
					Type:    EventTextChunk,
					Content: token.DeltaText,
				}); err != nil {
					_ = stream.Close()
					return nil, err
				}
			}

			if token.ToolCall != nil {
				iterToolCalls = append(iterToolCalls, token.ToolCall)
			}

			if token.FinishReason != "" {
				finishReason = token.FinishReason
			}
			if token.PromptTokens > 0 {
				iterPromptTokens = token.PromptTokens
			}
			if token.CompletionTokens > 0 {
				iterCompletionTokens = token.CompletionTokens
			}
		}
		_ = stream.Close()

		// Reconcile iteration token usage
		if iterPromptTokens == 0 {
			iterPromptTokens = e.tokenCounter.EstimateContextTokens(&ResolvedContext{
				SystemPrompt: resolvedContext.SystemPrompt,
				Messages:     currentMessages,
				Tools:        resolvedContext.Tools,
			})
		}
		if iterCompletionTokens == 0 {
			iterCompletionTokens = e.tokenCounter.CountTokens(iterText + iterThought)
		}

		totalPromptTokens += iterPromptTokens
		totalCompletionTokens += iterCompletionTokens

		// Record Prometheus LLM usage
		metrics.LLMTokenUsage.WithLabelValues(providerName, modelName, "prompt").Add(float64(iterPromptTokens))
		metrics.LLMTokenUsage.WithLabelValues(providerName, modelName, "completion").Add(float64(iterCompletionTokens))

		// Check if tools were called
		if len(iterToolCalls) > 0 {
			// Append assistant response to messages
			assistantMsg := ChatMessage{
				Role:    RoleAssistant,
				Content: iterText,
			}
			currentMessages = append(currentMessages, assistantMsg)

			for _, call := range iterToolCalls {
				// Loop detector: check for identical calls
				callSig := fmt.Sprintf("%s:%s", call.Name, call.Arguments)
				loopDetectionHistory[callSig]++
				if loopDetectionHistory[callSig] > 3 {
					log.WarnContext(turnCtx, "loop detector triggered on repeated tool call",
						slog.String("tool", call.Name),
						slog.Int("repetitions", loopDetectionHistory[callSig]),
					)
					_ = emitEvent(&TurnEvent{
						Type:         EventError,
						Content:      fmt.Sprintf("Loop detector aborted execution: repeated tool call '%s'", call.Name),
						ErrorMessage: "loop detector triggered",
					})
					finishReason = "loop_detected"
					break
				}

				// Emit tool call started
				if err := emitEvent(&TurnEvent{
					Type:    EventToolCallStarted,
					Content: fmt.Sprintf("Executing tool '%s' with arguments: %s", call.Name, call.Arguments),
				}); err != nil {
					return nil, err
				}

				if e.toolExecutor == nil {
					_ = emitEvent(&TurnEvent{
						Type:         EventError,
						Content:      fmt.Sprintf("Tool '%s' failed: tool executor not configured", call.Name),
						ErrorMessage: "tool executor not configured",
					})
					continue
				}

				// Execute Tool
				toolRes, err := e.toolExecutor.ExecuteTool(turnCtx, call)
				if err != nil {
					_ = emitEvent(&TurnEvent{
						Type:         EventError,
						Content:      fmt.Sprintf("Tool '%s' error: %s", call.Name, err.Error()),
						ErrorMessage: err.Error(),
					})
					currentMessages = append(currentMessages, ChatMessage{
						Role:       RoleTool,
						ToolCallID: call.ID,
						Name:       call.Name,
						Content:    fmt.Sprintf("Error: %s", err.Error()),
					})
					continue
				}

				// Handle Subagent Action
				if toolRes.IsSubagentAction {
					if err := emitEvent(&TurnEvent{
						Type:       EventSubagentSpawned,
						Content:    fmt.Sprintf("Spawned subagent '%s' for delegated task: %s", toolRes.SubagentID, toolRes.SubagentTask),
						SubagentID: toolRes.SubagentID,
					}); err != nil {
						return nil, err
					}
				}

				// Emit tool call finished
				if err := emitEvent(&TurnEvent{
					Type:       EventToolCallFinished,
					Content:    fmt.Sprintf("Tool '%s' finished: %s", call.Name, toolRes.Output),
					SubagentID: toolRes.SubagentID,
				}); err != nil {
					return nil, err
				}

				currentMessages = append(currentMessages, ChatMessage{
					Role:       RoleTool,
					ToolCallID: call.ID,
					Name:       call.Name,
					Content:    toolRes.Output,
				})
			}

			if finishReason == "loop_detected" {
				break
			}

			// Continue next iteration with updated conversation messages
			continue
		}

		// No tool calls: this iteration completed the generation
		fullFinalResponse = iterText
		break
	}

	if iteration >= maxIters && fullFinalResponse == "" {
		finishReason = "max_iterations"
		fullFinalResponse = "Maximum tool iteration limit reached."
	}

	// 4. Emit Turn Completed Event
	completedContent := "Turn completed successfully."
	if fullFinalResponse != "" {
		completedContent = fullFinalResponse
	}

	if err := emitEvent(&TurnEvent{
		Type:    EventTurnCompleted,
		Content: completedContent,
	}); err != nil {
		return nil, err
	}

	// 5. Final Token Usage & Cost Reconciliation
	calculatedCost := e.tokenCounter.CalculateCost(totalPromptTokens, totalCompletionTokens, modelName)
	finalUsage := TokenUsage{
		PromptTokens:     totalPromptTokens,
		CompletionTokens: totalCompletionTokens,
		TotalTokens:      totalPromptTokens + totalCompletionTokens,
		CostUSD:          calculatedCost,
	}

	log.InfoContext(ctx, "agent turn successfully completed in go engine",
		slog.String("session_id", req.SessionID),
		slog.String("agent_id", req.AgentID),
		slog.Int("iterations", iteration),
		slog.Int("total_tokens", finalUsage.TotalTokens),
	)

	return &TurnOutcome{
		SessionID:     req.SessionID,
		AgentID:       req.AgentID,
		FinalResponse: fullFinalResponse,
		Events:        accumulatedEvents,
		TokenUsage:    finalUsage,
		Iterations:    iteration,
		FinishReason:  finishReason,
		Messages:      currentMessages,
	}, nil
}

// CancelTurn cancels an active in-flight turn by session ID
func (e *TurnEngine) CancelTurn(ctx context.Context, sessionID string) error {
	e.mu.Lock()
	defer e.mu.Unlock()

	cancel, found := e.activeTurns[sessionID]
	if !found {
		return appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("no active turn found for session %s", sessionID), appErrors.LayerService)
	}

	cancel()
	delete(e.activeTurns, sessionID)
	return nil
}
