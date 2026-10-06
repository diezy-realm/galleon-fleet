package identity

import (
	"context"
	"net/http"
)

// DenyAllResolver strictly denies all resolution attempts with ErrNoCredential
type DenyAllResolver struct{}

// NewDenyAllResolver creates a resolver that always fails closed
func NewDenyAllResolver() PrincipalResolver {
	return &DenyAllResolver{}
}

func (d *DenyAllResolver) Resolve(ctx context.Context, r *http.Request) (*Principal, error) {
	return nil, ErrNoCredential
}

func (d *DenyAllResolver) ResolveToken(ctx context.Context, token string) (*Principal, error) {
	return nil, ErrNoCredential
}

// SharedOperatorResolver resolves local calls to the unauthenticated shared-operator sentinel
type SharedOperatorResolver struct{}

// NewSharedOperatorResolver creates a resolver representing trusted local unauthenticated access
func NewSharedOperatorResolver() PrincipalResolver {
	return &SharedOperatorResolver{}
}

func (s *SharedOperatorResolver) Resolve(ctx context.Context, r *http.Request) (*Principal, error) {
	return &Principal{
		ID:            SharedOperatorID,
		DisplayID:     "Local Operator",
		Actor:         ActorKindHuman,
		IsHuman:       true,
		AuthMethod:    AuthMethodSharedOperator,
		TrustDomain:   "local",
		Role:          RoleViewer,
		Authenticated: false, // Per RFC 7141: shared-operator is NOT authenticated for privileged actions
	}, nil
}

func (s *SharedOperatorResolver) ResolveToken(ctx context.Context, token string) (*Principal, error) {
	return s.Resolve(ctx, nil)
}
