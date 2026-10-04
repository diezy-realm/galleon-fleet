package tool_test

import (
	"context"
	"testing"

	appErrors "github.com/diezy-labs/claw-crew/engine/core/errors"
	"github.com/diezy-labs/claw-crew/engine/src/run"
	"github.com/diezy-labs/claw-crew/engine/src/tool"
)

func TestDirectDispatchDenial(t *testing.T) {
	runStore := run.NewMemoryStore()
	runHub := run.NewEventHub()
	runSvc := run.NewService(runStore, runHub)

	registry := tool.NewRegistry()
	gate := tool.NewApprovalGate(runSvc)
	toolSvc := tool.NewService(registry, gate, runSvc)

	ctx := context.Background()

	t.Run("RejectEmptyActorID", func(t *testing.T) {
		_, err := toolSvc.ExecuteWithContext(ctx, &tool.ExecutionContext{
			ActorID: "",
			RunID:   "run_anon",
		}, "read_file", `{"path":"test.txt"}`, ".", false)
		if err == nil {
			t.Fatalf("expected error for empty ActorID, got nil")
		}
		if appErr, ok := err.(*appErrors.AppError); ok {
			if appErr.Code != appErrors.CodePermissionDenied {
				t.Errorf("expected CodePermissionDenied, got %v", appErr.Code)
			}
		}
	})

	t.Run("RejectNilExecutionContext", func(t *testing.T) {
		_, err := toolSvc.ExecuteWithContext(ctx, nil, "read_file", `{"path":"test.txt"}`, ".", false)
		if err == nil {
			t.Fatalf("expected error for nil ExecutionContext, got nil")
		}
		if appErr, ok := err.(*appErrors.AppError); ok {
			if appErr.Code != appErrors.CodePermissionDenied {
				t.Errorf("expected CodePermissionDenied, got %v", appErr.Code)
			}
		}
	})

	t.Run("RejectCancelledRunDirectExecution", func(t *testing.T) {
		activeRun, err := runSvc.CreateRun(ctx, &run.CreateRunRequest{
			CrewID: "crew_sec",
			Input:  run.RunInput{Prompt: "Security test"},
		}, "req_sec", "")
		if err != nil {
			t.Fatalf("failed to create run: %v", err)
		}

		if err := runSvc.CancelRun(ctx, activeRun.ID); err != nil {
			t.Fatalf("failed to cancel run: %v", err)
		}

		_, err = toolSvc.ExecuteWithContext(ctx, &tool.ExecutionContext{
			ActorID: "agent_qa",
			RunID:   activeRun.ID,
		}, "read_file", `{"path":"test.txt"}`, ".", false)
		if err == nil {
			t.Fatalf("expected rejection on cancelled run, got nil")
		}
		if appErr, ok := err.(*appErrors.AppError); ok {
			if appErr.Code != appErrors.CodeFailedPrecondition {
				t.Errorf("expected CodeFailedPrecondition, got %v", appErr.Code)
			}
		}
	})

	t.Run("RejectNativeToolServiceWithoutGateway", func(t *testing.T) {
		activeRun, err := runSvc.CreateRun(ctx, &run.CreateRunRequest{
			CrewID: "crew_sec",
			Input:  run.RunInput{Prompt: "Fail-closed gateway test"},
		}, "req_sec_2", "")
		if err != nil {
			t.Fatalf("failed to create run: %v", err)
		}

		// toolSvc has NO gateway wired (gateway is nil).
		// Under fail-closed policy (Lane D1), any native tool execution MUST be rejected.
		_, err = toolSvc.ExecuteWithContext(ctx, &tool.ExecutionContext{
			ActorID: "agent_qa",
			RunID:   activeRun.ID,
		}, "write_file", `{"path":"test.txt","content":"hello"}`, ".", false)
		if err == nil {
			t.Fatalf("expected error when gateway is nil, got nil")
		}
		if appErr, ok := err.(*appErrors.AppError); ok {
			if appErr.Code != appErrors.CodeUnavailable {
				t.Errorf("expected CodeUnavailable, got %v", appErr.Code)
			}
		} else {
			t.Errorf("expected *appErrors.AppError, got %T: %v", err, err)
		}
	})

	t.Run("RejectDirectBuiltinExecutionWithoutGateway", func(t *testing.T) {
		execCtx := &tool.ExecutionContext{
			ActorID: "agent_qa",
			RunID:   "run_test",
		}

		builtins := []tool.Tool{
			&tool.RunLinterTool{},
			&tool.RunTestsTool{},
			&tool.GitStatusTool{},
			&tool.GitCreateBranchTool{},
			&tool.GitCommitTool{},
			tool.NewWebFetchTool(),
			&tool.WriteFileTool{},
			&tool.EditFileTool{},
			&tool.ExecuteCommandTool{},
			&tool.GitDiffTool{},
			&tool.SandboxedCodeRunnerTool{},
		}

		for _, b := range builtins {
			t.Run(b.Name(), func(t *testing.T) {
				_, _, err := b.ExecuteWithContext(ctx, execCtx, `{}`)
				if err == nil {
					t.Fatalf("expected fail-closed rejection for %s when gateway is nil, got nil", b.Name())
				}
				if appErr, ok := err.(*appErrors.AppError); ok {
					if appErr.Code != appErrors.CodeUnavailable {
						t.Errorf("expected CodeUnavailable for %s, got %v", b.Name(), appErr.Code)
					}
				} else {
					t.Errorf("expected *appErrors.AppError for %s, got %T: %v", b.Name(), err, err)
				}
			})
		}
	})
}

