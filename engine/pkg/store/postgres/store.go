package postgres

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"math"
	"sort"
	"time"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/store/vector"
	_ "github.com/lib/pq"
)

// PostgresVectorStore implements vector.VectorStore on PostgreSQL (BG-B3)
type PostgresVectorStore struct {
	db *sql.DB
}

// NewPostgresVectorStore connects to PostgreSQL with retry and creates table if missing
func NewPostgresVectorStore(ctx context.Context, dsn string) (*PostgresVectorStore, error) {
	if dsn == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "dsn cannot be empty", appErrors.LayerRepository)
	}

	db, err := sql.Open("postgres", dsn)
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to open postgres connection", appErrors.LayerRepository)
	}

	// Ping with timeout
	pingCtx, cancel := context.WithTimeout(ctx, 3*time.Second)
	defer cancel()
	if err := db.PingContext(pingCtx); err != nil {
		_ = db.Close()
		return nil, appErrors.Wrap(err, appErrors.CodeUnavailable, "postgres ping failed", appErrors.LayerRepository)
	}

	store := &PostgresVectorStore{db: db}
	if err := store.initSchema(ctx); err != nil {
		_ = db.Close()
		return nil, err
	}

	return store, nil
}

// NewPostgresVectorStoreWithDB creates a store over an existing sql.DB handle (for tests)
func NewPostgresVectorStoreWithDB(ctx context.Context, db *sql.DB) (*PostgresVectorStore, error) {
	store := &PostgresVectorStore{db: db}
	if err := store.initSchema(ctx); err != nil {
		return nil, err
	}
	return store, nil
}

func (s *PostgresVectorStore) initSchema(ctx context.Context) error {
	query := `
	CREATE TABLE IF NOT EXISTS vector_documents (
		id TEXT PRIMARY KEY,
		content TEXT NOT NULL,
		embedding TEXT NOT NULL,
		metadata TEXT NOT NULL,
		scope TEXT NOT NULL,
		created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
	);
	CREATE INDEX IF NOT EXISTS idx_vector_created_at ON vector_documents(created_at);
	`
	_, err := s.db.ExecContext(ctx, query)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, "failed to init vector_documents schema", appErrors.LayerRepository)
	}
	return nil
}

func (s *PostgresVectorStore) Store(ctx context.Context, doc *vector.Document) error {
	if doc == nil {
		return appErrors.New(appErrors.CodeInvalidArgument, "doc cannot be nil", appErrors.LayerRepository)
	}
	if doc.ID == "" {
		return appErrors.New(appErrors.CodeInvalidArgument, "doc ID cannot be empty", appErrors.LayerRepository)
	}
	if len(doc.Embedding) == 0 {
		return appErrors.New(appErrors.CodeInvalidArgument, "doc embedding cannot be empty", appErrors.LayerRepository)
	}

	embBytes, err := json.Marshal(doc.Embedding)
	if err != nil {
		return err
	}
	metaBytes, err := json.Marshal(doc.Metadata)
	if err != nil {
		return err
	}
	scopeBytes, err := json.Marshal(doc.Scope)
	if err != nil {
		return err
	}

	query := `
	INSERT INTO vector_documents (id, content, embedding, metadata, scope, created_at)
	VALUES ($1, $2, $3, $4, $5, $6)
	ON CONFLICT (id) DO UPDATE SET
		content = EXCLUDED.content,
		embedding = EXCLUDED.embedding,
		metadata = EXCLUDED.metadata,
		scope = EXCLUDED.scope,
		created_at = EXCLUDED.created_at;
	`
	_, err = s.db.ExecContext(ctx, query, doc.ID, doc.Content, string(embBytes), string(metaBytes), string(scopeBytes), time.Now().UTC())
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to store doc %s", doc.ID), appErrors.LayerRepository)
	}

	return nil
}

func (s *PostgresVectorStore) Delete(ctx context.Context, id string) error {
	if id == "" {
		return appErrors.New(appErrors.CodeInvalidArgument, "id cannot be empty", appErrors.LayerRepository)
	}

	query := `DELETE FROM vector_documents WHERE id = $1;`
	_, err := s.db.ExecContext(ctx, query, id)
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, fmt.Sprintf("failed to delete doc %s", id), appErrors.LayerRepository)
	}
	return nil
}

func (s *PostgresVectorStore) Search(ctx context.Context, queryEmbedding []float32, topK int) ([]*vector.SearchResult, error) {
	return s.SearchWithScope(ctx, queryEmbedding, topK, nil)
}

func (s *PostgresVectorStore) SearchWithScope(ctx context.Context, queryEmbedding []float32, topK int, scope map[string]string) ([]*vector.SearchResult, error) {
	if len(queryEmbedding) == 0 {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "query embedding cannot be empty", appErrors.LayerRepository)
	}
	if topK <= 0 {
		return nil, nil
	}

	rows, err := s.db.QueryContext(ctx, "SELECT id, content, embedding, metadata, scope FROM vector_documents")
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to query vector_documents", appErrors.LayerRepository)
	}
	defer rows.Close()

	var results []*vector.SearchResult

	for rows.Next() {
		var id, content, embStr, metaStr, scopeStr string
		if err := rows.Scan(&id, &content, &embStr, &metaStr, &scopeStr); err != nil {
			return nil, err
		}

		var emb []float32
		if err := json.Unmarshal([]byte(embStr), &emb); err != nil {
			continue
		}
		if len(emb) != len(queryEmbedding) {
			continue
		}

		var docScope map[string]string
		_ = json.Unmarshal([]byte(scopeStr), &docScope)

		var metadata map[string]string
		_ = json.Unmarshal([]byte(metaStr), &metadata)

		doc := &vector.Document{
			ID:        id,
			Content:   content,
			Embedding: emb,
			Metadata:  metadata,
			Scope:     docScope,
		}

		if len(scope) > 0 && !doc.MatchesScope(scope) {
			continue
		}

		score := cosineSimilarity(queryEmbedding, emb)
		results = append(results, &vector.SearchResult{
			Document: doc,
			Score:    score,
		})
	}

	if err := rows.Err(); err != nil {
		return nil, err
	}

	sort.Slice(results, func(i, j int) bool {
		if results[i].Score != results[j].Score {
			return results[i].Score > results[j].Score
		}
		return results[i].Document.ID < results[j].Document.ID
	})

	if len(results) > topK {
		results = results[:topK]
	}

	return results, nil
}

func (s *PostgresVectorStore) Close() error {
	if s.db != nil {
		return s.db.Close()
	}
	return nil
}

func cosineSimilarity(a, b []float32) float32 {
	var dotProduct, normA, normB float64
	for i := 0; i < len(a); i++ {
		valA := float64(a[i])
		valB := float64(b[i])
		dotProduct += valA * valB
		normA += valA * valA
		normB += valB * valB
	}
	if normA == 0 || normB == 0 {
		return 0
	}
	return float32(dotProduct / (math.Sqrt(normA) * math.Sqrt(normB)))
}
