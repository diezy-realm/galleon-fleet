package tool

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	appErrors "github.com/diezy-labs/claw-crew/engine/core/errors"
)

// RunLinterTool implements code.run_linter with environment scrubbing
type RunLinterTool struct{}

func (t *RunLinterTool) Name() string { return "code.run_linter" }
func (t *RunLinterTool) Description() string {
	return "Runs a code linter in an isolated, environment-scrubbed workspace process"
}
func (t *RunLinterTool) RiskTier() RiskTier { return RiskTierRead }
func (t *RunLinterTool) Definition() *ToolDefinition {
	return &ToolDefinition{
		ID:               t.Name(),
		Version:          "1.0.0",
		DisplayName:      "Run Code Linter",
		Description:      t.Description(),
		RiskTier:         RiskTierRead,
		RiskClass:        RiskClassCompute,
		Capabilities:     []string{"code.lint"},
		RequiresApproval: false,
		TimeoutSeconds:   60,
		MaxOutputBytes:   50000,
		IdempotencyMode:  IdempotencySafe,
		Source:           "native",
		InputSchema: json.RawMessage(`{
			"type": "object",
			"properties": {
				"linter": {"type": "string", "description": "Linter executable (e.g. golangci-lint, gofmt, cargo clippy)"},
				"args": {"type": "array", "items": {"type": "string"}},
				"path": {"type": "string", "description": "Working directory relative to workspace root"}
			},
			"required": ["linter"]
		}`),
	}
}

func (t *RunLinterTool) Execute(ctx context.Context, args string, workspaceRoot string) (string, error) {
	out, _, err := t.ExecuteWithContext(ctx, nil, args)
	return out, err
}

func (t *RunLinterTool) ExecuteWithContext(ctx context.Context, execCtx *ExecutionContext, args string) (string, []string, error) {
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

// RunTestsTool implements code.run_tests with process sandboxing and artifact spillover
type RunTestsTool struct{}

func (t *RunTestsTool) Name() string { return "code.run_tests" }
func (t *RunTestsTool) Description() string {
	return "Executes unit tests in an isolated, sandboxed process with artifact spillover for large outputs"
}
func (t *RunTestsTool) RiskTier() RiskTier { return RiskTierExecute }
func (t *RunTestsTool) Definition() *ToolDefinition {
	return &ToolDefinition{
		ID:               t.Name(),
		Version:          "1.0.0",
		DisplayName:      "Run Unit Tests",
		Description:      t.Description(),
		RiskTier:         RiskTierExecute,
		RiskClass:        RiskClassExecuteSandboxed,
		Capabilities:     []string{"code.test"},
		RequiresApproval: false,
		TimeoutSeconds:   120,
		MaxOutputBytes:   50000,
		IdempotencyMode:  IdempotencySafe,
		Source:           "native",
		InputSchema: json.RawMessage(`{
			"type": "object",
			"properties": {
				"command": {"type": "string", "description": "Test command (e.g. go, cargo, npm, pytest)"},
				"args": {"type": "array", "items": {"type": "string"}},
				"path": {"type": "string", "description": "Working directory relative to workspace root"},
				"timeout_seconds": {"type": "integer", "default": 60, "maximum": 300}
			},
			"required": ["command"]
		}`),
	}
}

func (t *RunTestsTool) Execute(ctx context.Context, args string, workspaceRoot string) (string, error) {
	out, _, err := t.ExecuteWithContext(ctx, nil, args)
	return out, err
}

func (t *RunTestsTool) ExecuteWithContext(ctx context.Context, execCtx *ExecutionContext, args string) (string, []string, error) {
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

// scrubbedEnvironment returns host environment stripped of all sensitive secrets and tokens
func scrubbedEnvironment() []string {
	safeKeys := map[string]bool{
		"PATH":        true,
		"HOME":        true,
		"USER":        true,
		"GOROOT":      true,
		"GOPATH":      true,
		"CARGO_HOME":  true,
		"RUSTUP_HOME": true,
		"SYSTEMROOT":  true,
		"WINDIR":      true,
		"COMSPEC":     true,
		"TEMP":        true,
		"TMP":         true,
		"OS":          true,
		"LANG":        true,
		"LC_ALL":      true,
	}

	var env []string
	for _, e := range os.Environ() {
		parts := strings.SplitN(e, "=", 2)
		if len(parts) == 0 {
			continue
		}
		key := strings.ToUpper(parts[0])

		// Deny any secret/credential patterns
		if strings.Contains(key, "SECRET") || strings.Contains(key, "TOKEN") ||
			strings.Contains(key, "KEY") || strings.Contains(key, "PASSWORD") ||
			strings.Contains(key, "AUTH") || strings.HasPrefix(key, "AWS_") ||
			strings.HasPrefix(key, "GITHUB_") || strings.HasPrefix(key, "OPENAI_") ||
			strings.HasPrefix(key, "ANTHROPIC_") || strings.HasPrefix(key, "GEMINI_") {
			continue
		}

		if safeKeys[key] {
			env = append(env, e)
		}
	}
	return env
}
