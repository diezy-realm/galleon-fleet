package identity

// ActorKind mirrors clawcrew_api::principal::ActorKind (Rust)
// Distinguishes interactive human callers from headless automated services.
type ActorKind string

const (
	ActorKindHuman   ActorKind = "human"
	ActorKindService ActorKind = "service"
)

// DenyReason mirrors clawcrew_api::principal::DenyReason (Rust)
// Fail-closed: any credential or authorization ambiguity maps to a DenyReason.
type DenyReason string

const (
	DenyReasonNoCredential     DenyReason = "no_credential"
	DenyReasonBadCredential    DenyReason = "bad_credential"
	DenyReasonTokenExpired     DenyReason = "token_expired"
	DenyReasonTokenRevoked     DenyReason = "token_revoked"
	DenyReasonMfaRequired      DenyReason = "mfa_required"
	DenyReasonNotEntitled      DenyReason = "not_entitled"
	DenyReasonAliasNotEntitled DenyReason = "alias_not_entitled"
	DenyReasonUnknownProvider  DenyReason = "unknown_provider"
	DenyReasonMisconfigured    DenyReason = "misconfigured"
)
