package memory

import (
	"context"
	"testing"

	"github.com/diezy-labs/claw-crew/engine/pkg/client"
	"github.com/diezy-labs/claw-crew/engine/pkg/pb"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

type mockGatewayClient struct {
	client.SystemGatewayClient
	storedRules []*pb.LearnedRule
}

func (m *mockGatewayClient) StoreLearnedRule(ctx context.Context, rule string, scope map[string]string, reversible bool) (*pb.StoreLearnedRuleResponse, error) {
	id := "rust-rule-1"
	m.storedRules = append(m.storedRules, &pb.LearnedRule{
		Id:         id,
		Rule:       rule,
		Scope:      scope,
		Reversible: reversible,
		Approved:   true,
	})
	return &pb.StoreLearnedRuleResponse{
		Id:      id,
		Success: true,
	}, nil
}

func (m *mockGatewayClient) RetrieveLearnedRules(ctx context.Context, scopeKey, scopeValue string) ([]*pb.LearnedRule, error) {
	return m.storedRules, nil
}

func TestProposalStoreWithGateway(t *testing.T) {
	ctx := context.Background()
	vec := NewVectorStore()
	mockGW := &mockGatewayClient{}

	store := NewProposalStoreWithGateway(vec, mockGW)

	prop, err := store.Propose(ctx, "Never write secrets to git", map[string]string{"fleet": "alpha"})
	require.NoError(t, err)

	approved, err := store.Approve(ctx, prop.ID)
	require.NoError(t, err)
	assert.Equal(t, ProposalApproved, approved.Status)
	assert.Equal(t, "rust-rule-1", approved.DocumentID)

	// Verify rule was forwarded to Rust gateway
	require.Len(t, mockGW.storedRules, 1)
	assert.Equal(t, "Never write secrets to git", mockGW.storedRules[0].Rule)

	// Verify sync back from gateway
	freshStore := NewProposalStoreWithGateway(vec, mockGW)
	err = freshStore.SyncFromGateway(ctx)
	require.NoError(t, err)

	list, err := freshStore.List(ctx, ProposalApproved)
	require.NoError(t, err)
	require.Len(t, list, 1)
	assert.Equal(t, "rust-rule-1", list[0].ID)
	assert.Equal(t, "Never write secrets to git", list[0].Rule)
}
