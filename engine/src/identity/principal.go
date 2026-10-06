package identity

import (
	"context"
	"slices"
)

type contextKey string

const principalContextKey = contextKey("galleon.identity.principal")

// PrincipalId mirrors clawcrew_api::principal::PrincipalId (Rust)
type PrincipalId string

const SharedOperatorID PrincipalId = "shared-operator"

// AuthMethod mirrors clawcrew_api::principal::AuthMethod
type AuthMethod string

const (
	AuthMethodNone           AuthMethod = "none"
	AuthMethodSharedOperator AuthMethod = "shared_operator"
	AuthMethodOIDC           AuthMethod = "oidc"
	AuthMethodPeercred       AuthMethod = "peercred"
	AuthMethodNative         AuthMethod = "native"
)

// PrincipalType defines the source entity of an identity
type PrincipalType string

const (
	PrincipalTypeUser     PrincipalType = "user"
	PrincipalTypeDevice   PrincipalType = "device"
	PrincipalTypeLoopback PrincipalType = "loopback"
	PrincipalTypePeer     PrincipalType = "peer"
)

// Role defines the authorization tier of a principal
type Role string

const (
	RoleAdmin    Role = "admin"
	RoleOperator Role = "operator"
	RoleMember   Role = "member"
	RoleViewer   Role = "viewer"
)

// Principal represents a verified caller identity in the Go engine (BG-C1, RFC 7141 parity)
type Principal struct {
	ID            PrincipalId       `json:"id"`
	DisplayID     string            `json:"display_id"`
	IsHuman       bool              `json:"is_human"`
	AuthMethod    AuthMethod        `json:"auth_method"`
	ProviderAlias *string           `json:"provider_alias,omitempty"`
	TrustDomain   string            `json:"trust_domain"`
	MFAVerified   bool              `json:"mfa_verified"`
	Type          PrincipalType     `json:"type"`
	Role          Role              `json:"role"`
	Authenticated bool              `json:"authenticated"`
	Scopes        []string          `json:"scopes"`
	DeviceID      string            `json:"device_id,omitempty"`
	Metadata      map[string]string `json:"metadata,omitempty"`
}

// IsAuthenticated reports whether the principal has been cryptographically or credentials-verified
func (p *Principal) IsAuthenticated() bool {
	if p == nil {
		return false
	}
	if p.AuthMethod != "" && (p.AuthMethod == AuthMethodNone || p.ID == SharedOperatorID) {
		return false
	}
	return p.Authenticated && p.ID != ""
}

// CanApprove reports whether this principal has sufficient authority to approve HIGH_RISK operations (S2-SEC-01)
func (p *Principal) CanApprove() bool {
	if !p.IsAuthenticated() {
		return false
	}
	if p.Role != RoleAdmin && p.Role != RoleOperator {
		return false
	}
	if len(p.Scopes) == 0 {
		return true // Default scopes for admin/operator allow approval
	}
	return slices.Contains(p.Scopes, "*") || slices.Contains(p.Scopes, "approval") || slices.Contains(p.Scopes, "terminal:approve")
}

// HasRole checks if the principal possesses the required role or higher
func (p *Principal) HasRole(role Role) bool {
	if !p.IsAuthenticated() {
		return false
	}
	if p.Role == RoleAdmin {
		return true // Admin satisfies all roles
	}
	if p.Role == RoleOperator && (role == RoleOperator || role == RoleMember || role == RoleViewer) {
		return true
	}
	if p.Role == RoleMember && (role == RoleMember || role == RoleViewer) {
		return true
	}
	return p.Role == role
}

// HasScope checks if the principal has a specific permission scope
func (p *Principal) HasScope(scope string) bool {
	if !p.IsAuthenticated() {
		return false
	}
	if slices.Contains(p.Scopes, "*") {
		return true
	}
	return slices.Contains(p.Scopes, scope)
}

// WithPrincipal returns a new context containing the verified principal
func WithPrincipal(ctx context.Context, p *Principal) context.Context {
	return context.WithValue(ctx, principalContextKey, p)
}

// FromContext extracts the verified principal from context, if present
func FromContext(ctx context.Context) (*Principal, bool) {
	if ctx == nil {
		return nil, false
	}
	p, ok := ctx.Value(principalContextKey).(*Principal)
	return p, ok && p != nil && p.IsAuthenticated()
}

// PrincipalFromContext extracts the verified principal from context, returning ok=false if not found
func PrincipalFromContext(ctx context.Context) (Principal, bool) {
	p, ok := FromContext(ctx)
	if !ok || p == nil {
		return Principal{}, false
	}
	return *p, true
}
