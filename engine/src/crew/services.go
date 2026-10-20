package crew

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"sync"
	"time"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
	"github.com/diezy-realm/galleon-fleet/engine/core/logger"
	"github.com/diezy-realm/galleon-fleet/engine/core/metrics"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/client"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
)

type service struct {
	mu              sync.RWMutex
	activeTurns     map[string]context.CancelFunc
	dispatcher      ToolDispatcher
	gateway         client.SystemGatewayClient
	activeSubagents int
	maxSubagents    int
}

// NewService creates a new crew.Orchestrator instance with tool dispatcher and Rust SystemGateway client
func NewService(
	dispatcher ToolDispatcher,
	gateway client.SystemGatewayClient,
) Orchestrator {
	return &service{
		activeTurns:  make(map[string]context.CancelFunc),
		dispatcher:   dispatcher,
		gateway:      gateway,
		maxSubagents: 4, // Default sub-agent concurrency ceiling (BG-D1 / BL-18)
	}
}

func (s *service) SetMaxSubagents(max int) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.maxSubagents = max
}

func (s *service) StartTurn(ctx context.Context, req *TurnRequest, eventCh chan<- *TurnEvent) error {
	log := logger.Get()

	if req.SessionID == "" {
		return appErrors.New(appErrors.CodeInvalidArgument, "session_id cannot be empty", appErrors.LayerService)
	}
	if req.Prompt == "" {
		return appErrors.New(appErrors.CodeInvalidArgument, "prompt cannot be empty", appErrors.LayerService)
	}

	turnCtx, cancel := context.WithCancel(ctx)
	s.mu.Lock()
	s.activeTurns[req.SessionID] = cancel
	s.mu.Unlock()

	var subagentWg sync.WaitGroup
	defer func() {
		s.mu.Lock()
		delete(s.activeTurns, req.SessionID)
		s.mu.Unlock()
		cancel()
		subagentWg.Wait() // Ensure all subagents wind down before eventCh can be closed by caller (BUG-001)
	}()

	metrics.ActiveAgents.Inc()
	defer metrics.ActiveAgents.Dec()

	start := time.Now()
	agentID := req.AgentID
	if agentID == "" {
		agentID = "primary_agent"
	}

	defer func() {
		duration := time.Since(start).Seconds()
		metrics.AgentTurnDuration.WithLabelValues(agentID, "completed").Observe(duration)
	}()

	log.InfoContext(ctx, "starting agent turn",
		slog.String("session_id", req.SessionID),
		slog.String("agent_id", agentID),
	)

	// 1. Prepare protobuf ExecuteTurnRequest for Rust SystemGateway
	protoReq := &pb.ExecuteTurnRequest{
		SessionId:  req.SessionID,
		AgentAlias: &agentID,
		Messages: []*pb.TurnChatMessage{
			{Role: "user", Content: req.Prompt},
		},
	}
	if s.dispatcher != nil {
		for _, t := range s.dispatcher.GetAvailableTools() {
			paramBytes, _ := json.Marshal(t.Parameters)
			protoReq.Tools = append(protoReq.Tools, &pb.TurnToolDefinition{
				Name:           t.Name,
				Description:    t.Description,
				ParametersJson: string(paramBytes),
			})
		}
	}

	// 2. Concurrently invoke Rust SystemGateway ExecuteTurn stream (Rust LLM SSOT)
	stream, err := s.gateway.ExecuteTurn(turnCtx, protoReq)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeUnavailable, "failed to initiate ExecuteTurn with Rust gateway", appErrors.LayerService)
	}

	// 3. Process incoming streaming tokens from Rust Gateway
	for {
		token, err := stream.Recv()
		if err != nil {
			if err == io.EOF {
				break
			}
			select {
			case <-turnCtx.Done():
				return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
			default:
				return appErrors.Wrap(err, appErrors.CodeInternal, "error receiving stream token from Rust gateway", appErrors.LayerService)
			}
		}

		switch ev := token.Event.(type) {
		case *pb.TurnToken_Token:
			if ev.Token.IsThought {
				select {
				case <-turnCtx.Done():
					return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
				case eventCh <- &TurnEvent{
					Type:    EventThoughtChunk,
					Content: ev.Token.Delta,
				}:
				}
			} else {
				select {
				case <-turnCtx.Done():
					return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
				case eventCh <- &TurnEvent{
					Type:    EventTextChunk,
					Content: ev.Token.Delta,
				}:
				}
			}

		case *pb.TurnToken_ToolCall:
			tc := ev.ToolCall
			select {
			case <-turnCtx.Done():
				return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
			case eventCh <- &TurnEvent{
				Type:    EventToolCallStarted,
				Content: fmt.Sprintf("Executing tool '%s' with arguments: %s", tc.Name, tc.ArgumentsJson),
			}:
			}

			if s.dispatcher != nil {
				res, err := s.dispatcher.Dispatch(turnCtx, &ToolCall{
					ID:        tc.Id,
					Name:      tc.Name,
					Arguments: tc.ArgumentsJson,
				})
				if err != nil {
					log.WarnContext(ctx, "tool execution failed",
						slog.String("tool", tc.Name),
						slog.String("error", err.Error()),
					)
					select {
					case <-turnCtx.Done():
						return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
					case eventCh <- &TurnEvent{
						Type:         EventError,
						Content:      fmt.Sprintf("Tool '%s' error: %s", tc.Name, err.Error()),
						ErrorMessage: err.Error(),
					}:
					}
					continue
				}

				if res.IsSubagentAction {
					// BG-D1 / BL-18: Subagent resource-gate (cap active concurrent sub-agents)
					// Must be checked BEFORE subagentWg.Add(1) and goroutine spawn!
					s.mu.Lock()
					if s.maxSubagents > 0 && s.activeSubagents >= s.maxSubagents {
						s.mu.Unlock()
						select {
						case <-turnCtx.Done():
							return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
						case eventCh <- &TurnEvent{
							Type:       EventToolCallFinished,
							Content:    fmt.Sprintf("Resource gate rejection: maximum concurrent sub-agents reached (%d/%d)", s.activeSubagents, s.maxSubagents),
							SubagentID: res.SubagentID,
						}:
						}
						continue
					}
					s.activeSubagents++
					s.mu.Unlock()

					select {
					case <-turnCtx.Done():
						s.mu.Lock()
						s.activeSubagents--
						s.mu.Unlock()
						return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
					case eventCh <- &TurnEvent{
						Type:       EventSubagentSpawned,
						Content:    fmt.Sprintf("Spawned subagent '%s' for delegated task: %s", res.SubagentID, res.SubagentTask),
						SubagentID: res.SubagentID,
					}:
					}

					subagentWg.Add(1)
					go func(subID, subTask string) {
						defer subagentWg.Done()
						defer func() {
							s.mu.Lock()
							s.activeSubagents--
							s.mu.Unlock()
						}()
						s.runSubagent(turnCtx, subID, subTask, eventCh)
					}(res.SubagentID, res.SubagentTask)
				} else {
					select {
					case <-turnCtx.Done():
						return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
					case eventCh <- &TurnEvent{
						Type:    EventToolCallFinished,
						Content: fmt.Sprintf("Tool '%s' finished: %s", tc.Name, res.Output),
					}:
					}
				}
			}

		case *pb.TurnToken_Error:
			return appErrors.New(appErrors.CodeInternal, ev.Error.Message, appErrors.LayerService)

		case *pb.TurnToken_Complete:
			// Turn generation complete
		}
	}

	// 4. Wait for any running subagent goroutines to finish
	subagentWg.Wait()

	// 5. Emit Turn Completed event
	select {
	case <-turnCtx.Done():
		return appErrors.New(appErrors.CodeTimeout, "turn cancelled or timed out", appErrors.LayerService)
	case eventCh <- &TurnEvent{
		Type:    EventTurnCompleted,
		Content: "Turn completed successfully.",
	}:
	}

	log.InfoContext(ctx, "agent turn successfully completed",
		slog.String("session_id", req.SessionID),
		slog.String("agent_id", agentID),
	)

	return nil
}

// runSubagent handles parallel subagent turn execution via Rust SystemGateway
func (s *service) runSubagent(ctx context.Context, subagentID, task string, eventCh chan<- *TurnEvent) {
	metrics.ActiveAgents.Inc()
	defer metrics.ActiveAgents.Dec()

	stream, err := s.gateway.ExecuteTurn(ctx, &pb.ExecuteTurnRequest{
		SessionId:  fmt.Sprintf("subagent-%s", subagentID),
		AgentAlias: &subagentID,
		Messages: []*pb.TurnChatMessage{
			{Role: "system", Content: fmt.Sprintf("You are sub-agent '%s' tasked with: %s", subagentID, task)},
			{Role: "user", Content: task},
		},
	})
	if err != nil {
		return
	}

	for {
		token, err := stream.Recv()
		if err != nil {
			break
		}
		if ev, ok := token.Event.(*pb.TurnToken_Token); ok && ev.Token.Delta != "" {
			select {
			case <-ctx.Done():
				return
			case eventCh <- &TurnEvent{
				Type:       EventTextChunk,
				Content:    ev.Token.Delta,
				SubagentID: subagentID,
			}:
			}
		}
	}
}

func (s *service) CancelTurn(ctx context.Context, sessionID string) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	cancel, found := s.activeTurns[sessionID]
	if !found {
		return appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("no active turn found for session %s", sessionID), appErrors.LayerService)
	}

	cancel()
	delete(s.activeTurns, sessionID)
	return nil
}

// HandoffSubagent performs subagent handoff / A2A delegation carrying an artifact reference
// with the resource-gate check inherited from BG-D1 (BG-D4 / eks-BL-18 part 2).
func (s *service) HandoffSubagent(ctx context.Context, req *HandoffRequest, eventCh chan<- *TurnEvent) error {
	s.mu.Lock()
	if s.maxSubagents > 0 && s.activeSubagents >= s.maxSubagents {
		s.mu.Unlock()
		return appErrors.New(appErrors.CodeFailedPrecondition,
			fmt.Sprintf("resource gate rejection: maximum concurrent sub-agents reached (%d/%d)", s.activeSubagents, s.maxSubagents),
			appErrors.LayerService)
	}
	s.activeSubagents++
	s.mu.Unlock()

	defer func() {
		s.mu.Lock()
		s.activeSubagents--
		s.mu.Unlock()
	}()

	if eventCh != nil {
		eventCh <- &TurnEvent{
			Type:       EventSubagentSpawned,
			Content:    fmt.Sprintf("Handoff to subagent '%s' with artifact '%s': %s", req.ToAgentID, req.ArtifactRef, req.TaskDescription),
			SubagentID: req.ToAgentID,
		}
	}

	taskWithArtifact := req.TaskDescription
	if req.ArtifactRef != "" {
		taskWithArtifact = fmt.Sprintf("[Handoff Artifact: %s] %s", req.ArtifactRef, req.TaskDescription)
	}

	s.runSubagent(ctx, req.ToAgentID, taskWithArtifact, eventCh)
	return nil
}

// MemoryRegistry provides persistent storage for crews and agents
type MemoryRegistry struct {
	mu    sync.RWMutex
	crews map[string]*Squad
}

// NewRegistry creates a new Registry preloaded with standard default crews
func NewRegistry() Registry {
	r := &MemoryRegistry{
		crews: make(map[string]*Squad),
	}

	// Register default Research & Engineering Crew
	defaultCrew := &Squad{
		ID:              "crew_research_dev",
		Name:            "Research & Engineering Crew",
		Description:     "Multi-agent crew for code analysis, planning, implementation, and review.",
		CrewMemberCount: 3,
		Agents: []*CrewMember{
			{
				ID:           "planner",
				Name:         "Planner Agent",
				Role:         "Decompose high-level tasks into DAG task graph.",
				Status:       CrewMemberStatusIdle,
				Capabilities: []string{"planning", "task_breakdown"},
			},
			{
				ID:           "coder",
				Name:         "Code Specialist",
				Role:         "Implements code, refactors, and edits files.",
				Status:       CrewMemberStatusIdle,
				Capabilities: []string{"write_file", "edit_file", "read_file"},
			},
			{
				ID:           "reviewer",
				Name:         "Code Reviewer",
				Role:         "Validates code changes, generates git diffs, runs test suites.",
				Status:       CrewMemberStatusIdle,
				Capabilities: []string{"run_tests", "generate_diff"},
			},
		},
	}

	r.crews[defaultCrew.ID] = defaultCrew
	return r
}

func (r *MemoryRegistry) ListSquads(ctx context.Context) ([]*Squad, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	list := make([]*Squad, 0, len(r.crews))
	for _, c := range r.crews {
		list = append(list, c)
	}
	return list, nil
}

func (r *MemoryRegistry) GetSquad(ctx context.Context, id string) (*Squad, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	c, ok := r.crews[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("crew not found: %s", id), appErrors.LayerService)
	}
	return c, nil
}

func (r *MemoryRegistry) RegisterSquad(ctx context.Context, crew *Squad) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	crew.CrewMemberCount = len(crew.Agents)
	r.crews[crew.ID] = crew
	return nil
}

func (r *MemoryRegistry) UpdateCrewMemberStatus(ctx context.Context, crewID, agentID string, status CrewMemberStatus) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	c, ok := r.crews[crewID]
	if !ok {
		return appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("crew not found: %s", crewID), appErrors.LayerService)
	}
	for _, a := range c.Agents {
		if a.ID == agentID {
			a.Status = status
			return nil
		}
	}
	return appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("agent %s not found in crew %s", agentID, crewID), appErrors.LayerService)
}
