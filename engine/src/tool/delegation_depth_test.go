package tool_test

import (
	"context"
	"testing"

	"github.com/diezy-realm/galleon-fleet/engine/src/tool"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestDelegationDepthLimitEnforcement(t *testing.T) {
	delegate := &tool.DelegateTaskTool{}
	ctx := context.Background()

	maxDepth := 2

	// 1. Within depth limit (depth 1 < max 2) -> allowed
	execCtxAllowed := &tool.ExecutionContext{
		ActorID:            "agent_lvl_1",
		Capabilities:       []string{"*"},
		DelegationDepth:    1,
		MaxDelegationDepth: &maxDepth,
	}

	payload := `{"subagent_role":"Researcher","task_prompt":"find docs"}`
	out, _, err := delegate.ExecuteWithContext(ctx, execCtxAllowed, payload)
	require.NoError(t, err)
	assert.NotEmpty(t, out)
	assert.Contains(t, out, "delegated")

	// 2. Exceeding depth limit (depth 2 >= max 2) -> rejected
	execCtxExceeded := &tool.ExecutionContext{
		ActorID:            "agent_lvl_2",
		Capabilities:       []string{"*"},
		DelegationDepth:    2,
		MaxDelegationDepth: &maxDepth,
	}

	out, _, err = delegate.ExecuteWithContext(ctx, execCtxExceeded, payload)
	require.Error(t, err)
	assert.Contains(t, err.Error(), "delegation depth limit exceeded")
	assert.Empty(t, out)
}
