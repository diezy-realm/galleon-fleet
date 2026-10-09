package identity

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func TestDenyByDefaultResolver_NegativeTests(t *testing.T) {
	store := NewMemoryTokenStore()
	resolver := NewDenyByDefaultResolver(store)
	ctx := context.Background()

	t.Run("nil request rejected with ErrUnauthenticated", func(t *testing.T) {
		p, err := resolver.Resolve(ctx, nil)
		if p != nil || err != ErrUnauthenticated {
			t.Fatalf("expected ErrUnauthenticated, got p=%v, err=%v", p, err)
		}
	})

	t.Run("request without credentials rejected with ErrUnauthenticated", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodGet, "/api/terminal/exec", nil)
		p, err := resolver.Resolve(ctx, req)
		if p != nil || err != ErrUnauthenticated {
			t.Fatalf("expected ErrUnauthenticated, got p=%v, err=%v", p, err)
		}
	})

	t.Run("empty Authorization header rejected with ErrUnauthenticated", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodGet, "/api/terminal/exec", nil)
		req.Header.Set("Authorization", "   ")
		p, err := resolver.Resolve(ctx, req)
		if p != nil || err != ErrUnauthenticated {
			t.Fatalf("expected ErrUnauthenticated, got p=%v, err=%v", p, err)
		}
	})

	t.Run("malformed Bearer header rejected with ErrUnauthenticated", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodGet, "/api/terminal/exec", nil)
		req.Header.Set("Authorization", "Bearer  ")
		p, err := resolver.Resolve(ctx, req)
		if p != nil || err != ErrUnauthenticated {
			t.Fatalf("expected ErrUnauthenticated, got p=%v, err=%v", p, err)
		}
	})

	t.Run("non-bearer unknown scheme rejected with ErrUnauthenticated", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodGet, "/api/terminal/exec", nil)
		req.Header.Set("Authorization", "Basic dXNlcjpwYXNz")
		p, err := resolver.Resolve(ctx, req)
		if p != nil || err != ErrUnauthenticated {
			t.Fatalf("expected ErrUnauthenticated, got p=%v, err=%v", p, err)
		}
	})

	t.Run("unregistered token rejected with ErrInvalidCredentials", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodGet, "/api/terminal/exec", nil)
		req.Header.Set("Authorization", "Bearer random-unregistered-token-1234")
		p, err := resolver.Resolve(ctx, req)
		if p != nil || err != ErrInvalidCredentials {
			t.Fatalf("expected ErrInvalidCredentials, got p=%v, err=%v", p, err)
		}
	})

	t.Run("expired token rejected with ErrTokenExpired", func(t *testing.T) {
		token := "expiring-token-abc"
		store.Register(token, &Principal{
			ID:            "user-1",
			Type:          PrincipalTypeUser,
			Role:          RoleAdmin,
			Authenticated: true,
		}, 10*time.Millisecond)

		// Wait for token to expire
		time.Sleep(20 * time.Millisecond)

		req := httptest.NewRequest(http.MethodGet, "/api/terminal/exec", nil)
		req.Header.Set("Authorization", "Bearer "+token)
		p, err := resolver.Resolve(ctx, req)
		if p != nil || err != ErrTokenExpired {
			t.Fatalf("expected ErrTokenExpired, got p=%v, err=%v", p, err)
		}
	})

	t.Run("revoked token rejected with ErrTokenRevoked", func(t *testing.T) {
		token := "revokable-token-xyz"
		store.Register(token, &Principal{
			ID:            "user-2",
			Type:          PrincipalTypeUser,
			Role:          RoleAdmin,
			Authenticated: true,
		}, 1*time.Hour)

		if err := store.Revoke(token); err != nil {
			t.Fatalf("revoke failed: %v", err)
		}

		req := httptest.NewRequest(http.MethodGet, "/api/terminal/exec", nil)
		req.Header.Set("Authorization", "Bearer "+token)
		p, err := resolver.Resolve(ctx, req)
		if p != nil || err != ErrTokenRevoked {
			t.Fatalf("expected ErrTokenRevoked, got p=%v, err=%v", p, err)
		}
	})
}

func TestDenyByDefaultResolver_PositiveTests(t *testing.T) {
	store := NewMemoryTokenStore()
	resolver := NewDenyByDefaultResolver(store)
	ctx := context.Background()

	adminPrincipal := &Principal{
		ID:            "admin-root",
		Type:          PrincipalTypeUser,
		Role:          RoleAdmin,
		Authenticated: true,
		Scopes:        []string{"*"},
	}
	token := "valid-secret-token-admin"
	store.Register(token, adminPrincipal, 1*time.Hour)

	t.Run("valid Bearer Authorization header resolves principal", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodPost, "/api/terminal/exec", nil)
		req.Header.Set("Authorization", "Bearer "+token)

		p, err := resolver.Resolve(ctx, req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if p == nil || p.ID != "admin-root" {
			t.Fatalf("expected admin-root principal, got %v", p)
		}
		if !p.IsAuthenticated() {
			t.Fatalf("principal should be authenticated")
		}
		if !p.CanApprove() {
			t.Fatalf("admin principal should have CanApprove() == true")
		}
		if !p.HasRole(RoleAdmin) || !p.HasRole(RoleOperator) {
			t.Fatalf("admin should satisfy Admin and Operator role checks")
		}
	})

	t.Run("valid X-Galleon-Token header resolves principal", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodPost, "/api/terminal/exec", nil)
		req.Header.Set("X-Galleon-Token", token)

		p, err := resolver.Resolve(ctx, req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if p == nil || p.ID != "admin-root" {
			t.Fatalf("expected admin-root principal, got %v", p)
		}
	})

	t.Run("valid X-Galleon-Loopback-Token header resolves principal", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodPost, "/api/terminal/exec", nil)
		req.Header.Set("X-Galleon-Loopback-Token", token)

		p, err := resolver.Resolve(ctx, req)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if p == nil || p.ID != "admin-root" {
			t.Fatalf("expected admin-root principal, got %v", p)
		}
	})
}

func TestPrincipal_Permissions(t *testing.T) {
	admin := &Principal{
		ID:            "u-admin",
		Role:          RoleAdmin,
		Authenticated: true,
		Scopes:        []string{"*"},
	}
	operator := &Principal{
		ID:            "u-op",
		Role:          RoleOperator,
		Authenticated: true,
		Scopes:        []string{"terminal:approve", "fleet:read"},
	}
	viewer := &Principal{
		ID:            "u-view",
		Role:          RoleViewer,
		Authenticated: true,
		Scopes:        []string{"fleet:read"},
	}
	unauthenticated := &Principal{
		ID:            "u-anon",
		Role:          RoleAdmin,
		Authenticated: false,
	}

	if !admin.CanApprove() {
		t.Fatalf("admin must be able to approve")
	}
	if !operator.CanApprove() {
		t.Fatalf("operator with terminal:approve must be able to approve")
	}
	if viewer.CanApprove() {
		t.Fatalf("viewer must NOT be able to approve")
	}
	if unauthenticated.CanApprove() {
		t.Fatalf("unauthenticated principal must NOT be able to approve")
	}
}
