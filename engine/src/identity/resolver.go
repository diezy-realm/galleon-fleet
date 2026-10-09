package identity

import (
	"context"
	"net/http"
	"strings"
)

// PrincipalResolver resolves and authenticates caller identity from requests or tokens (BG-C1)
type PrincipalResolver interface {
	Resolve(ctx context.Context, r *http.Request) (*Principal, error)
	ResolveToken(ctx context.Context, token string) (*Principal, error)
}

// DenyByDefaultResolver is a secure-by-default resolver that rejects unauthenticated calls
type DenyByDefaultResolver struct {
	tokenStore TokenStore
}

// NewDenyByDefaultResolver constructs a new resolver backed by a TokenStore
func NewDenyByDefaultResolver(store TokenStore) *DenyByDefaultResolver {
	if store == nil {
		store = NewMemoryTokenStore()
	}
	return &DenyByDefaultResolver{
		tokenStore: store,
	}
}

// TokenStore returns the underlying token store
func (r *DenyByDefaultResolver) TokenStore() TokenStore {
	return r.tokenStore
}

// Resolve extracts credentials from an HTTP request and resolves the authenticated Principal
// Strictly deny-by-default: returns ErrUnauthenticated if no credentials exist.
func (r *DenyByDefaultResolver) Resolve(ctx context.Context, req *http.Request) (*Principal, error) {
	if req == nil {
		return nil, ErrUnauthenticated
	}

	token := extractTokenFromRequest(req)
	if token == "" {
		return nil, ErrUnauthenticated
	}

	return r.ResolveToken(ctx, token)
}

// ResolveToken validates a raw token string against the store and returns the Principal
func (r *DenyByDefaultResolver) ResolveToken(ctx context.Context, token string) (*Principal, error) {
	token = strings.TrimSpace(token)
	if token == "" {
		return nil, ErrUnauthenticated
	}

	rec, err := r.tokenStore.Get(token)
	if err != nil {
		return nil, err
	}

	if rec.Principal == nil || !rec.Principal.IsAuthenticated() {
		return nil, ErrInvalidCredentials
	}

	return rec.Principal, nil
}

// extractTokenFromRequest looks for credentials across standard headers
func extractTokenFromRequest(r *http.Request) string {
	// 1. Authorization: Bearer <token>
	authHeader := strings.TrimSpace(r.Header.Get("Authorization"))
	if authHeader != "" {
		if strings.HasPrefix(strings.ToLower(authHeader), "bearer ") {
			token := strings.TrimSpace(authHeader[7:])
			if token != "" {
				return token
			}
		}
	}

	// 2. X-Galleon-Token: <token>
	if tok := strings.TrimSpace(r.Header.Get("X-Galleon-Token")); tok != "" {
		return tok
	}

	// 3. X-Galleon-Loopback-Token: <token>
	if tok := strings.TrimSpace(r.Header.Get("X-Galleon-Loopback-Token")); tok != "" {
		return tok
	}

	return ""
}
