package identity

import "errors"

var (
	// ErrUnauthenticated indicates credentials are required but missing
	ErrUnauthenticated = errors.New("unauthenticated: credentials missing or empty")
	ErrNoCredential    = ErrUnauthenticated

	// ErrInvalidCredentials indicates the presented credentials failed verification
	ErrInvalidCredentials = errors.New("unauthenticated: invalid credentials")
	ErrBadCredential       = ErrInvalidCredentials

	// ErrTokenExpired indicates the token or session has expired
	ErrTokenExpired = errors.New("unauthenticated: token has expired")

	// ErrTokenRevoked indicates the token or device has been explicitly revoked
	ErrTokenRevoked = errors.New("unauthenticated: token has been revoked")

	// ErrForbidden indicates principal lacks required role or scope
	ErrForbidden = errors.New("forbidden: insufficient permissions")

	// ErrRateLimited indicates too many attempts
	ErrRateLimited = errors.New("rate limited: too many requests")

	// ErrPairingCodeNotFound indicates the pairing code does not exist or has already been used
	ErrPairingCodeNotFound = errors.New("pairing code not found or already consumed")
)
