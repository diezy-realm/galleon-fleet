package identity

import (
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func TestAuthMiddleware_BG_C3(t *testing.T) {
	store := NewMemoryTokenStore()
	resolver := NewDenyByDefaultResolver(store)

	operatorToken := "op-token-123"
	store.Register(operatorToken, &Principal{
		ID:            "operator-1",
		Role:          RoleOperator,
		Authenticated: true,
		Scopes:        []string{"terminal:approve"},
	}, 1*time.Hour)

	viewerToken := "viewer-token-456"
	store.Register(viewerToken, &Principal{
		ID:            "viewer-1",
		Role:          RoleViewer,
		Authenticated: true,
	}, 1*time.Hour)

	dummyHandler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte("success"))
	})

	authMiddleware := AuthMiddleware(resolver)

	t.Run("unauthenticated request to RequireAuth rejected with 401", func(t *testing.T) {
		stack := authMiddleware(RequireAuth(dummyHandler))
		req := httptest.NewRequest(http.MethodGet, "/protected", nil)
		rec := httptest.NewRecorder()

		stack.ServeHTTP(rec, req)
		if rec.Code != http.StatusUnauthorized {
			t.Fatalf("expected 401, got %d", rec.Code)
		}
	})

	t.Run("authenticated request to RequireAuth succeeds with 200", func(t *testing.T) {
		stack := authMiddleware(RequireAuth(dummyHandler))
		req := httptest.NewRequest(http.MethodGet, "/protected", nil)
		req.Header.Set("Authorization", "Bearer "+operatorToken)
		rec := httptest.NewRecorder()

		stack.ServeHTTP(rec, req)
		if rec.Code != http.StatusOK {
			t.Fatalf("expected 200, got %d", rec.Code)
		}
	})

	t.Run("viewer request to RequireApprovalAuthority rejected with 403", func(t *testing.T) {
		stack := authMiddleware(RequireApprovalAuthority(dummyHandler))
		req := httptest.NewRequest(http.MethodPost, "/approve", nil)
		req.Header.Set("Authorization", "Bearer "+viewerToken)
		rec := httptest.NewRecorder()

		stack.ServeHTTP(rec, req)
		if rec.Code != http.StatusForbidden {
			t.Fatalf("expected 403 Forbidden for viewer without approval authority, got %d", rec.Code)
		}
	})

	t.Run("operator request to RequireApprovalAuthority succeeds with 200", func(t *testing.T) {
		stack := authMiddleware(RequireApprovalAuthority(dummyHandler))
		req := httptest.NewRequest(http.MethodPost, "/approve", nil)
		req.Header.Set("Authorization", "Bearer "+operatorToken)
		rec := httptest.NewRecorder()

		stack.ServeHTTP(rec, req)
		if rec.Code != http.StatusOK {
			t.Fatalf("expected 200 OK for operator with approval authority, got %d", rec.Code)
		}
	})
}
