package identity

import (
	"crypto/rand"
	"encoding/hex"
	"errors"
	"sync"
	"time"
)

var (
	ErrWebAuthnChallengeNotFound = errors.New("webauthn challenge not found or expired")
	ErrWebAuthnInvalidSignature = errors.New("webauthn assertion signature invalid")
)

// WebAuthnCredential represents a registered FIDO2 / WebAuthn passkey credential
type WebAuthnCredential struct {
	ID          string    `json:"id"`
	PrincipalID string    `json:"principal_id"`
	PublicKey   []byte    `json:"public_key"`
	SignCount   uint32    `json:"sign_count"`
	CreatedAt   time.Time `json:"created_at"`
}

// WebAuthnChallenge represents a cryptographic challenge issued during WebAuthn ceremonies
type WebAuthnChallenge struct {
	Challenge string    `json:"challenge"`
	ExpiresAt time.Time `json:"expires_at"`
}

// WebAuthnStore manages credentials and challenge lifetimes
type WebAuthnStore interface {
	CreateChallenge(ttl time.Duration) (*WebAuthnChallenge, error)
	VerifyChallenge(challenge string) bool
	RegisterCredential(cred *WebAuthnCredential) error
	GetCredential(id string) (*WebAuthnCredential, error)
}

type memoryWebAuthnStore struct {
	mu          sync.RWMutex
	challenges  map[string]time.Time
	credentials map[string]*WebAuthnCredential
}

// NewMemoryWebAuthnStore creates an in-memory WebAuthn store
func NewMemoryWebAuthnStore() WebAuthnStore {
	return &memoryWebAuthnStore{
		challenges:  make(map[string]time.Time),
		credentials: make(map[string]*WebAuthnCredential),
	}
}

func (s *memoryWebAuthnStore) CreateChallenge(ttl time.Duration) (*WebAuthnChallenge, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return nil, err
	}
	challenge := hex.EncodeToString(b)
	exp := time.Now().Add(ttl)

	s.mu.Lock()
	defer s.mu.Unlock()
	s.challenges[challenge] = exp

	return &WebAuthnChallenge{
		Challenge: challenge,
		ExpiresAt: exp,
	}, nil
}

func (s *memoryWebAuthnStore) VerifyChallenge(challenge string) bool {
	s.mu.Lock()
	defer s.mu.Unlock()

	exp, ok := s.challenges[challenge]
	if !ok {
		return false
	}
	delete(s.challenges, challenge) // Single-use challenge
	return time.Now().Before(exp)
}

func (s *memoryWebAuthnStore) RegisterCredential(cred *WebAuthnCredential) error {
	if cred == nil || cred.ID == "" {
		return errors.New("invalid credential")
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	s.credentials[cred.ID] = cred
	return nil
}

func (s *memoryWebAuthnStore) GetCredential(id string) (*WebAuthnCredential, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	cred, ok := s.credentials[id]
	if !ok {
		return nil, errors.New("credential not found")
	}
	return cred, nil
}
