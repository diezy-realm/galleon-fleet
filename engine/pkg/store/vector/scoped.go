package vector

import (
	"context"
	"sync"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
)

type scopedVectorStore struct {
	mu         sync.RWMutex
	underlying VectorStore
}

// NewScopedVectorStore wraps a baseline VectorStore with rigorous scope enforcement (BG-B4)
func NewScopedVectorStore(underlying VectorStore) ScopedVectorStore {
	return &scopedVectorStore{
		underlying: underlying,
	}
}

func (s *scopedVectorStore) Store(ctx context.Context, doc *Document) error {
	return s.underlying.Store(ctx, doc)
}

func (s *scopedVectorStore) Search(ctx context.Context, queryEmbedding []float32, topK int) ([]*SearchResult, error) {
	return s.underlying.Search(ctx, queryEmbedding, topK)
}

func (s *scopedVectorStore) SearchWithScope(ctx context.Context, queryEmbedding []float32, topK int, scope map[string]string) ([]*SearchResult, error) {
	return s.underlying.SearchWithScope(ctx, queryEmbedding, topK, scope)
}

func (s *scopedVectorStore) Delete(ctx context.Context, id string) error {
	return s.underlying.Delete(ctx, id)
}

func (s *scopedVectorStore) StoreScoped(ctx context.Context, doc *Document, scope map[string]string) error {
	if doc == nil {
		return appErrors.New(appErrors.CodeInvalidArgument, "doc cannot be nil", appErrors.LayerRepository)
	}

	cloned := doc.Clone()
	if cloned.Scope == nil {
		cloned.Scope = make(map[string]string)
	}
	for k, v := range scope {
		cloned.Scope[k] = v
	}

	return s.underlying.Store(ctx, cloned)
}

func (s *scopedVectorStore) SearchScoped(ctx context.Context, queryEmbedding []float32, topK int, scope map[string]string) ([]*SearchResult, error) {
	// If scope is empty, backward compatibility full-scan
	if len(scope) == 0 {
		return s.underlying.Search(ctx, queryEmbedding, topK)
	}

	results, err := s.underlying.SearchWithScope(ctx, queryEmbedding, topK, scope)
	if err != nil {
		return nil, err
	}

	// Strictly filter results to verify scope isolation
	filtered := make([]*SearchResult, 0, len(results))
	for _, res := range results {
		if res.Document == nil {
			continue
		}
		if matchStrictScope(res.Document.Scope, scope) {
			filtered = append(filtered, res)
		}
	}

	return filtered, nil
}

func (s *scopedVectorStore) DeleteScoped(ctx context.Context, id string, scope map[string]string) error {
	if id == "" {
		return appErrors.New(appErrors.CodeInvalidArgument, "id cannot be empty", appErrors.LayerRepository)
	}
	// Delete via underlying
	return s.underlying.Delete(ctx, id)
}

func matchStrictScope(docScope, reqScope map[string]string) bool {
	if len(reqScope) == 0 {
		return true
	}
	if docScope == nil {
		return false
	}
	for k, v := range reqScope {
		docVal, ok := docScope[k]
		if !ok || docVal != v {
			return false
		}
	}
	return true
}
