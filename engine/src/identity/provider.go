package identity

import (
	"context"
	"crypto/subtle"
	"sync"
	"time"
)

// CallerAuthProvider abstracts caller authentication (loopback token, peer-uid, mTLS) in Go (BR-B6)
type CallerAuthProvider interface {
	VerifyLoopbackToken(ctx context.Context, token string) (*Principal, error)
	VerifyPeerToken(ctx context.Context, peerID string, token string) (*Principal, error)
	LoopbackToken() string
}

type loopbackAuthProvider struct {
	mu            sync.RWMutex
	loopbackToken string
	peerTokens    map[string]string // peerID -> token
}

// NewLoopbackAuthProvider creates a caller auth provider with an auto-generated loopback token
func NewLoopbackAuthProvider() CallerAuthProvider {
	token := GenerateSecureToken(32)
	return &loopbackAuthProvider{
		loopbackToken: token,
		peerTokens:    make(map[string]string),
	}
}

// NewLoopbackAuthProviderWithToken creates a provider with a designated token
func NewLoopbackAuthProviderWithToken(token string) CallerAuthProvider {
	if token == "" {
		token = GenerateSecureToken(32)
	}
	return &loopbackAuthProvider{
		loopbackToken: token,
		peerTokens:    make(map[string]string),
	}
}

func (p *loopbackAuthProvider) LoopbackToken() string {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.loopbackToken
}

func (p *loopbackAuthProvider) VerifyLoopbackToken(ctx context.Context, token string) (*Principal, error) {
	if token == "" {
		return nil, ErrUnauthenticated
	}

	p.mu.RLock()
	expected := p.loopbackToken
	p.mu.RUnlock()

	if subtle.ConstantTimeCompare([]byte(token), []byte(expected)) != 1 {
		return nil, ErrInvalidCredentials
	}

	return &Principal{
		ID:            "loopback-local",
		Type:          PrincipalTypeLoopback,
		Role:          RoleAdmin,
		Authenticated: true,
		Scopes:        []string{"*"},
		Metadata: map[string]string{
			"source": "loopback-ipc",
		},
	}, nil
}

func (p *loopbackAuthProvider) VerifyPeerToken(ctx context.Context, peerID string, token string) (*Principal, error) {
	if peerID == "" || token == "" {
		return nil, ErrUnauthenticated
	}

	p.mu.RLock()
	expected, ok := p.peerTokens[peerID]
	p.mu.RUnlock()

	if !ok || subtle.ConstantTimeCompare([]byte(token), []byte(expected)) != 1 {
		return nil, ErrInvalidCredentials
	}

	return &Principal{
		ID:            PrincipalId("peer-" + peerID),
		Type:          PrincipalTypePeer,
		Role:          RoleOperator,
		Authenticated: true,
		Scopes:        []string{"fleet:read", "fleet:sync"},
		Metadata: map[string]string{
			"peer_id": peerID,
		},
	}, nil
}

// RegisterPeer authorizes a peer ID with a pre-shared token
func (p *loopbackAuthProvider) RegisterPeer(peerID string, token string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.peerTokens[peerID] = token
}

// AutoRegisterToStore connects this provider to a TokenStore so loopback token is immediately resolvable
func (p *loopbackAuthProvider) AutoRegisterToStore(store TokenStore) {
	principal := &Principal{
		ID:            "loopback-local",
		Type:          PrincipalTypeLoopback,
		Role:          RoleAdmin,
		Authenticated: true,
		Scopes:        []string{"*"},
		Metadata: map[string]string{
			"source": "loopback-ipc",
		},
	}
	// Loopback token valid for 24h or process lifetime
	store.Register(p.loopbackToken, principal, 24*time.Hour)
}
