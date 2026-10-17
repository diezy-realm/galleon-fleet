package identity

import (
	"crypto/rand"
	"encoding/hex"
	"sync"
	"time"
)

// TokenRecord holds token state, metadata, and associated principal
type TokenRecord struct {
	Token     string     `json:"token"`
	Principal *Principal `json:"principal"`
	ExpiresAt time.Time  `json:"expires_at"`
	Revoked   bool       `json:"revoked"`
	CreatedAt time.Time  `json:"created_at"`
}

// TokenStore defines operations for storing, resolving, and revoking tokens
type TokenStore interface {
	Register(token string, principal *Principal, ttl time.Duration) *TokenRecord
	Get(token string) (*TokenRecord, error)
	Revoke(token string) error
	RevokeByDeviceID(deviceID string) int
	ListActive() []*TokenRecord
}

type memoryTokenStore struct {
	mu     sync.RWMutex
	tokens map[string]*TokenRecord
}

// NewMemoryTokenStore constructs a thread-safe in-memory token store
func NewMemoryTokenStore() TokenStore {
	return &memoryTokenStore{
		tokens: make(map[string]*TokenRecord),
	}
}

func (s *memoryTokenStore) Register(token string, principal *Principal, ttl time.Duration) *TokenRecord {
	s.mu.Lock()
	defer s.mu.Unlock()

	var expiresAt time.Time
	if ttl > 0 {
		expiresAt = time.Now().Add(ttl)
	}

	rec := &TokenRecord{
		Token:     token,
		Principal: principal,
		ExpiresAt: expiresAt,
		Revoked:   false,
		CreatedAt: time.Now(),
	}
	s.tokens[token] = rec
	return rec
}

func (s *memoryTokenStore) Get(token string) (*TokenRecord, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	rec, ok := s.tokens[token]
	if !ok {
		return nil, ErrInvalidCredentials
	}
	if rec.Revoked {
		return nil, ErrTokenRevoked
	}
	if !rec.ExpiresAt.IsZero() && time.Now().After(rec.ExpiresAt) {
		return nil, ErrTokenExpired
	}
	return rec, nil
}

func (s *memoryTokenStore) Revoke(token string) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	rec, ok := s.tokens[token]
	if !ok {
		return ErrInvalidCredentials
	}
	rec.Revoked = true
	return nil
}

func (s *memoryTokenStore) RevokeByDeviceID(deviceID string) int {
	s.mu.Lock()
	defer s.mu.Unlock()

	count := 0
	for _, rec := range s.tokens {
		if rec.Principal != nil && rec.Principal.DeviceID == deviceID && !rec.Revoked {
			rec.Revoked = true
			count++
		}
	}
	return count
}

func (s *memoryTokenStore) ListActive() []*TokenRecord {
	s.mu.RLock()
	defer s.mu.RUnlock()

	now := time.Now()
	var result []*TokenRecord
	for _, rec := range s.tokens {
		if !rec.Revoked && (rec.ExpiresAt.IsZero() || now.Before(rec.ExpiresAt)) {
			result = append(result, rec)
		}
	}
	return result
}

// GenerateSecureToken creates a cryptographically strong random hex token
func GenerateSecureToken(byteLength int) string {
	b := make([]byte, byteLength)
	if _, err := rand.Read(b); err != nil {
		// Fallback should never happen in normal OS environment
		return hex.EncodeToString([]byte(time.Now().String()))
	}
	return hex.EncodeToString(b)
}
