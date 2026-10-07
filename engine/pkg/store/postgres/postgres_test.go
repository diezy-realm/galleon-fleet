package postgres

import (
	"context"
	"encoding/json"
	"testing"

	"github.com/DATA-DOG/go-sqlmock"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/store/vector"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestPostgresVectorStore_Lifecycle(t *testing.T) {
	db, mock, err := sqlmock.New()
	require.NoError(t, err)
	defer db.Close()

	ctx := context.Background()

	// 1. Init schema expectation
	mock.ExpectExec("CREATE TABLE IF NOT EXISTS vector_documents").
		WillReturnResult(sqlmock.NewResult(0, 0))

	store, err := NewPostgresVectorStoreWithDB(ctx, db)
	require.NoError(t, err)

	// 2. Test Store()
	doc := &vector.Document{
		ID:        "doc-test-1",
		Content:   "Postgres test doc content",
		Embedding: []float32{1.0, 0.0},
		Metadata:  map[string]string{"author": "jack"},
		Scope:     map[string]string{"fleet": "caribbean"},
	}

	mock.ExpectExec("INSERT INTO vector_documents").
		WithArgs(
			doc.ID,
			doc.Content,
			sqlmock.AnyArg(), // emb
			sqlmock.AnyArg(), // meta
			sqlmock.AnyArg(), // scope
			sqlmock.AnyArg(), // created_at
		).
		WillReturnResult(sqlmock.NewResult(1, 1))

	err = store.Store(ctx, doc)
	require.NoError(t, err)

	// 3. Test SearchWithScope()
	embBytes, _ := json.Marshal([]float32{1.0, 0.0})
	metaBytes, _ := json.Marshal(map[string]string{"author": "jack"})
	scopeBytes, _ := json.Marshal(map[string]string{"fleet": "caribbean"})

	rows := sqlmock.NewRows([]string{"id", "content", "embedding", "metadata", "scope"}).
		AddRow("doc-test-1", "Postgres test doc content", string(embBytes), string(metaBytes), string(scopeBytes))

	mock.ExpectQuery("SELECT id, content, embedding, metadata, scope FROM vector_documents").
		WillReturnRows(rows)

	results, err := store.SearchWithScope(ctx, []float32{1.0, 0.0}, 5, map[string]string{"fleet": "caribbean"})
	require.NoError(t, err)
	require.Len(t, results, 1)
	assert.Equal(t, "doc-test-1", results[0].Document.ID)
	assert.InDelta(t, 1.0, results[0].Score, 0.001)

	// 4. Test Delete()
	mock.ExpectExec("DELETE FROM vector_documents WHERE id = \\$1;").
		WithArgs("doc-test-1").
		WillReturnResult(sqlmock.NewResult(1, 1))

	err = store.Delete(ctx, "doc-test-1")
	require.NoError(t, err)

	require.NoError(t, mock.ExpectationsWereMet())
}
