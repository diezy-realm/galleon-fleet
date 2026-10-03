package tool

import (
	"context"
	"encoding/json"
	"fmt"

	appErrors "github.com/diezy-labs/claw-crew/engine/core/errors"
)

// GitStatusTool implements git.status
type GitStatusTool struct{}

func (t *GitStatusTool) Name() string { return "git.status" }
func (t *GitStatusTool) Description() string {
	return "Returns modified files and current branch in the workspace"
}
func (t *GitStatusTool) RiskTier() RiskTier { return RiskTierRead }
func (t *GitStatusTool) Definition() *ToolDefinition {
	return &ToolDefinition{
		ID:               t.Name(),
		Version:          "1.0.0",
		DisplayName:      "Git Workspace Status",
		Description:      t.Description(),
		RiskTier:         RiskTierRead,
		RiskClass:        RiskClassRead,
		Capabilities:     []string{"git.read"},
		RequiresApproval: false,
		TimeoutSeconds:   15,
		MaxOutputBytes:   50000,
		IdempotencyMode:  IdempotencySafe,
		Source:           "native",
		InputSchema:      json.RawMessage(`{"type": "object"}`),
	}
}

func (t *GitStatusTool) Execute(ctx context.Context, args string, workspaceRoot string) (string, error) {
	out, _, err := t.ExecuteWithContext(ctx, nil, args)
	return out, err
}

func (t *GitStatusTool) ExecuteWithContext(ctx context.Context, execCtx *ExecutionContext, args string) (string, []string, error) {
	if execCtx == nil || execCtx.Gateway == nil {
		return "", nil, appErrors.New(appErrors.CodeUnavailable, fmt.Sprintf("native tool %s execution rejected: SystemGateway is required (fail-closed boundary enforced)", t.Name()), appErrors.LayerService)
	}
	resp, err := execCtx.Gateway.ExecuteNativeTool(ctx, t.Name(), args)
	if err != nil {
		return "", nil, err
	}
	var res struct {
		Success bool   `json:"success"`
		Output  string `json:"output"`
		Error   string `json:"error"`
	}
	if err := json.Unmarshal([]byte(resp), &res); err != nil {
		return "", nil, fmt.Errorf("invalid gateway response: %w", err)
	}
	if !res.Success {
		return "", nil, fmt.Errorf("gateway execution failed: %s", res.Error)
	}
	return res.Output, nil, nil
}

// GitCreateBranchTool implements git.create_branch
type GitCreateBranchTool struct{}

func (t *GitCreateBranchTool) Name() string        { return "git.create_branch" }
func (t *GitCreateBranchTool) Description() string { return "Creates and switches to a new Git branch" }
func (t *GitCreateBranchTool) RiskTier() RiskTier  { return RiskTierWrite }
func (t *GitCreateBranchTool) Definition() *ToolDefinition {
	return &ToolDefinition{
		ID:               t.Name(),
		Version:          "1.0.0",
		DisplayName:      "Git Create Branch",
		Description:      t.Description(),
		RiskTier:         RiskTierWrite,
		RiskClass:        RiskClassWriteWorkspace,
		Capabilities:     []string{"git.write"},
		RequiresApproval: false,
		TimeoutSeconds:   15,
		MaxOutputBytes:   50000,
		IdempotencyMode:  IdempotencySafe,
		Source:           "native",
		InputSchema: json.RawMessage(`{
			"type": "object",
			"properties": {
				"branch_name": {"type": "string", "description": "Name of branch to create"}
			},
			"required": ["branch_name"]
		}`),
	}
}

func (t *GitCreateBranchTool) Execute(ctx context.Context, args string, workspaceRoot string) (string, error) {
	out, _, err := t.ExecuteWithContext(ctx, nil, args)
	return out, err
}

func (t *GitCreateBranchTool) ExecuteWithContext(ctx context.Context, execCtx *ExecutionContext, args string) (string, []string, error) {
	if execCtx == nil || execCtx.Gateway == nil {
		return "", nil, appErrors.New(appErrors.CodeUnavailable, fmt.Sprintf("native tool %s execution rejected: SystemGateway is required (fail-closed boundary enforced)", t.Name()), appErrors.LayerService)
	}
	resp, err := execCtx.Gateway.ExecuteNativeTool(ctx, t.Name(), args)
	if err != nil {
		return "", nil, err
	}
	var res struct {
		Success bool   `json:"success"`
		Output  string `json:"output"`
		Error   string `json:"error"`
	}
	if err := json.Unmarshal([]byte(resp), &res); err != nil {
		return "", nil, fmt.Errorf("invalid gateway response: %w", err)
	}
	if !res.Success {
		return "", nil, fmt.Errorf("gateway execution failed: %s", res.Error)
	}
	return res.Output, nil, nil
}

// GitCommitTool implements git.commit requiring explicit human approval
type GitCommitTool struct{}

func (t *GitCommitTool) Name() string { return "git.commit" }
func (t *GitCommitTool) Description() string {
	return "Commits staged changes to the repository. Requires human approval"
}
func (t *GitCommitTool) RiskTier() RiskTier { return RiskTierWrite }
func (t *GitCommitTool) Definition() *ToolDefinition {
	return &ToolDefinition{
		ID:               t.Name(),
		Version:          "1.0.0",
		DisplayName:      "Git Commit",
		Description:      t.Description(),
		RiskTier:         RiskTierWrite,
		RiskClass:        RiskClassWriteWorkspace,
		Capabilities:     []string{"git.write"},
		RequiresApproval: true,
		TimeoutSeconds:   20,
		MaxOutputBytes:   50000,
		IdempotencyMode:  IdempotencyAtMostOnce,
		Source:           "native",
		InputSchema: json.RawMessage(`{
			"type": "object",
			"properties": {
				"message": {"type": "string", "description": "Commit message"},
				"files": {"type": "array", "items": {"type": "string"}, "description": "Files to stage before committing (optional, default all modified)"}
			},
			"required": ["message"]
		}`),
	}
}

func (t *GitCommitTool) Execute(ctx context.Context, args string, workspaceRoot string) (string, error) {
	out, _, err := t.ExecuteWithContext(ctx, nil, args)
	return out, err
}

func (t *GitCommitTool) ExecuteWithContext(ctx context.Context, execCtx *ExecutionContext, args string) (string, []string, error) {
	if execCtx == nil || execCtx.Gateway == nil {
		return "", nil, appErrors.New(appErrors.CodeUnavailable, fmt.Sprintf("native tool %s execution rejected: SystemGateway is required (fail-closed boundary enforced)", t.Name()), appErrors.LayerService)
	}
	resp, err := execCtx.Gateway.ExecuteNativeTool(ctx, t.Name(), args)
	if err != nil {
		return "", nil, err
	}
	var res struct {
		Success bool   `json:"success"`
		Output  string `json:"output"`
		Error   string `json:"error"`
	}
	if err := json.Unmarshal([]byte(resp), &res); err != nil {
		return "", nil, fmt.Errorf("invalid gateway response: %w", err)
	}
	if !res.Success {
		return "", nil, fmt.Errorf("gateway execution failed: %s", res.Error)
	}
	return res.Output, nil, nil
}
