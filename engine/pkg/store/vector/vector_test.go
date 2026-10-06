package vector

import (
	"context"
	"strings"
	"testing"
	"time"

	"github.com/diezy-labs/claw-crew/engine/src/memory"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// mockMemoryVectorStore adapts memory.VectorStore to vector.VectorStore
type mockVectorStoreAdapter struct {
	inner memory.VectorStore
}

func (m *mockVectorStoreAdapter) Store(ctx context.Context, doc *Document) error {
	return m.inner.Store(ctx, doc)
}

func (m *mockVectorStoreAdapter) Search(ctx context.Context, queryEmbedding []float32, topK int) ([]*SearchResult, error) {
	return m.inner.Search(ctx, queryEmbedding, topK)
}

func (m *mockVectorStoreAdapter) SearchWithScope(ctx context.Context, queryEmbedding []float32, topK int, scope map[string]string) ([]*SearchResult, error) {
	return m.inner.SearchWithScope(ctx, queryEmbedding, topK, scope)
}

func (m *mockVectorStoreAdapter) Delete(ctx context.Context, id string) error {
	if del, ok := m.inner.(interface{ Delete(context.Context, string) error }); ok {
		return del.Delete(ctx, id)
	}
	return nil
}

func TestScopedVectorStore_Isolation(t *testing.T) {
	ctx := context.Background()
	inner := memory.NewVectorStore()
	store := NewScopedVectorStore(&mockVectorStoreAdapter{inner: inner})

	// 1. Store Fleet doc
	err := store.StoreScoped(ctx, &Document{
		ID:        "doc-fleet",
		Content:   "Fleet navigation strategy",
		Embedding: []float32{1.0, 0.0},
	}, map[string]string{"fleet": "caribbean"})
	require.NoError(t, err)

	// 2. Store Ship doc
	err = store.StoreScoped(ctx, &Document{
		ID:        "doc-ship",
		Content:   "Black Pearl ship repair",
		Embedding: []float32{1.0, 0.0},
	}, map[string]string{"fleet": "caribbean", "ship": "black-pearl"})
	require.NoError(t, err)

	// 3. Store Squad doc
	err = store.StoreScoped(ctx, &Document{
		ID:        "doc-squad",
		Content:   "Boarding squad tactics",
		Embedding: []float32{1.0, 0.0},
	}, map[string]string{"fleet": "caribbean", "squad": "boarding"})
	require.NoError(t, err)

	// Test A: Search with Ship scope returns only that ship
	shipResults, err := store.SearchScoped(ctx, []float32{1.0, 0.0}, 10, map[string]string{"ship": "black-pearl"})
	require.NoError(t, err)
	require.Len(t, shipResults, 1)
	assert.Equal(t, "doc-ship", shipResults[0].Document.ID)

	// Test B: Search with Squad scope does not return other squad or general ship
	squadResults, err := store.SearchScoped(ctx, []float32{1.0, 0.0}, 10, map[string]string{"squad": "boarding"})
	require.NoError(t, err)
	require.Len(t, squadResults, 1)
	assert.Equal(t, "doc-squad", squadResults[0].Document.ID)

	// Test C: Empty scope returns all matching embedding docs
	allResults, err := store.SearchScoped(ctx, []float32{1.0, 0.0}, 10, nil)
	require.NoError(t, err)
	assert.Len(t, allResults, 3)
}

func TestRetentionAndExport(t *testing.T) {
	retManager := NewRetentionManager(RetentionConfig{
		RunRetentionDays:   3,
		CrewRetentionDays:  7,
		FleetRetentionDays: 30,
	})

	now := time.Now()
	expiredRunTime := now.AddDate(0, 0, -4)
	validRunTime := now.AddDate(0, 0, -2)

	assert.True(t, retManager.IsExpired(map[string]string{ScopeRunID: "run-1"}, expiredRunTime, now))
	assert.False(t, retManager.IsExpired(map[string]string{ScopeRunID: "run-2"}, validRunTime, now))

	// Test Export JSON
	docs := []*TimedDocument{
		{
			Document: &Document{
				ID:      "doc-1",
				Content: "Hello World",
				Scope:   map[string]string{"fleet": "main"},
			},
			CreatedAt: now,
		},
	}

	jsonBytes, err := ExportDocuments(docs, ExportRequest{
		Scope:  map[string]string{"fleet": "main"},
		Format: "json",
	})
	require.NoError(t, err)
	assert.True(t, strings.Contains(string(jsonBytes), "Hello World"))

	// Test Export CSV
	csvBytes, err := ExportDocuments(docs, ExportRequest{
		Scope:  map[string]string{"fleet": "main"},
		Format: "csv",
	})
	require.NoError(t, err)
	assert.True(t, strings.Contains(string(csvBytes), "Hello World"))
	assert.True(t, strings.Contains(string(csvBytes), "id,content,created_at,scope"))

	// Test Pseudonymization
	pseudo := PseudonymizeIdentifier("pirate-jack-sparrow")
	assert.True(t, strings.HasPrefix(pseudo, "pseudo-"))
}
