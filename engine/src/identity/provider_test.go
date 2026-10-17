package identity

import (
	"context"
	"testing"
)

func TestCallerAuthProvider_BR_B6(t *testing.T) {
	provider := NewLoopbackAuthProvider()
	ctx := context.Background()

	token := provider.LoopbackToken()
	if token == "" {
		t.Fatalf("expected non-empty loopback token")
	}

	t.Run("verify valid loopback token succeeds as Admin", func(t *testing.T) {
		p, err := provider.VerifyLoopbackToken(ctx, token)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if p == nil || !p.IsAuthenticated() {
			t.Fatalf("expected authenticated principal")
		}
		if p.Type != PrincipalTypeLoopback {
			t.Fatalf("expected type loopback, got %s", p.Type)
		}
		if p.Role != RoleAdmin {
			t.Fatalf("expected role admin, got %s", p.Role)
		}
		if !p.CanApprove() {
			t.Fatalf("loopback admin principal must be able to approve")
		}
	})

	t.Run("verify invalid loopback token rejected", func(t *testing.T) {
		p, err := provider.VerifyLoopbackToken(ctx, "invalid-token-here")
		if err != ErrInvalidCredentials || p != nil {
			t.Fatalf("expected ErrInvalidCredentials, got p=%v, err=%v", p, err)
		}
	})

	t.Run("verify empty loopback token rejected", func(t *testing.T) {
		p, err := provider.VerifyLoopbackToken(ctx, "")
		if err != ErrUnauthenticated || p != nil {
			t.Fatalf("expected ErrUnauthenticated, got p=%v, err=%v", p, err)
		}
	})

	t.Run("verify peer token authorization", func(t *testing.T) {
		loopbackProv, ok := provider.(*loopbackAuthProvider)
		if !ok {
			t.Fatalf("expected *loopbackAuthProvider")
		}
		peerID := "peer-node-99"
		peerToken := "shared-secret-peer-xyz"
		loopbackProv.RegisterPeer(peerID, peerToken)

		// 1. Valid peer credentials
		p, err := provider.VerifyPeerToken(ctx, peerID, peerToken)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if p == nil || p.ID != "peer-peer-node-99" {
			t.Fatalf("expected peer principal, got %v", p)
		}

		// 2. Invalid peer credentials rejected
		p2, err2 := provider.VerifyPeerToken(ctx, peerID, "wrong-secret")
		if err2 != ErrInvalidCredentials || p2 != nil {
			t.Fatalf("expected ErrInvalidCredentials, got p=%v, err=%v", p2, err2)
		}

		// 3. Unknown peer rejected
		p3, err3 := provider.VerifyPeerToken(ctx, "unknown-peer", peerToken)
		if err3 != ErrInvalidCredentials || p3 != nil {
			t.Fatalf("expected ErrInvalidCredentials, got p=%v, err=%v", p3, err3)
		}
	})

	t.Run("AutoRegisterToStore integrates with TokenStore and DenyByDefaultResolver", func(t *testing.T) {
		store := NewMemoryTokenStore()
		loopbackProv, _ := provider.(*loopbackAuthProvider)
		loopbackProv.AutoRegisterToStore(store)

		resolver := NewDenyByDefaultResolver(store)
		resolved, err := resolver.ResolveToken(ctx, token)
		if err != nil {
			t.Fatalf("unexpected error resolving loopback token via resolver: %v", err)
		}
		if resolved.Type != PrincipalTypeLoopback || resolved.Role != RoleAdmin {
			t.Fatalf("unexpected principal properties: %v", resolved)
		}
	})
}
