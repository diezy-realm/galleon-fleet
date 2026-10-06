package identity

import (
	"context"
	"crypto/subtle"
	"encoding/base64"
	"fmt"
	"strings"
	"sync"
	"time"

	qrcode "github.com/skip2/go-qrcode"
)

// PairingSession represents an active pending one-time pairing ceremony
type PairingSession struct {
	Code        string    `json:"code"`
	Token       string    `json:"token"`
	QRCodeData  string    `json:"qr_code_data"` // data:image/png;base64,...
	PairingURI  string    `json:"pairing_uri"`  // galleon://pair?code=...&token=...
	ExpiresAt   time.Time `json:"expires_at"`
	Claimed     bool      `json:"claimed"`
	CreatedAt   time.Time `json:"created_at"`
}

// PairedDevice records a successfully paired remote client device
type PairedDevice struct {
	DeviceID   string    `json:"device_id"`
	DeviceName string    `json:"device_name"`
	Platform   string    `json:"platform"`
	PairedAt   time.Time `json:"paired_at"`
	LastSeenAt time.Time `json:"last_seen_at"`
	Revoked    bool      `json:"revoked"`
}

// InitiatePairingRequest holds initiation options
type InitiatePairingRequest struct {
	DeviceName string        `json:"device_name"`
	TTL        time.Duration `json:"ttl"` // Defaults to 5 minutes if 0
}

// ClaimPairingRequest holds payload for completing pairing
type ClaimPairingRequest struct {
	Code       string `json:"code"`
	Token      string `json:"token"`
	DeviceID   string `json:"device_id"`
	DeviceName string `json:"device_name"`
	Platform   string `json:"platform"`
}

// ClaimPairingResponse returned on successful claim
type ClaimPairingResponse struct {
	SessionToken string        `json:"session_token"`
	DeviceID     string        `json:"device_id"`
	Principal    *Principal    `json:"principal"`
	ExpiresAt    time.Time     `json:"expires_at"`
}

// PairingService manages authorized device pairing ceremonies via QR code (BG-C2)
type PairingService interface {
	Initiate(ctx context.Context, req InitiatePairingRequest) (*PairingSession, error)
	Claim(ctx context.Context, req ClaimPairingRequest) (*ClaimPairingResponse, error)
	ListDevices(ctx context.Context) ([]*PairedDevice, error)
	RevokeDevice(ctx context.Context, deviceID string) error
	IsRateLimited(clientIP string) bool
}

type memoryPairingService struct {
	mu           sync.RWMutex
	tokenStore   TokenStore
	sessions     map[string]*PairingSession // code -> session
	devices      map[string]*PairedDevice   // deviceID -> device
	failedCounts map[string]int             // clientIP -> failures
	lockoutUntil map[string]time.Time       // clientIP -> lockout
}

// NewPairingService creates a pairing service integrated with the identity token store
func NewPairingService(tokenStore TokenStore) PairingService {
	return &memoryPairingService{
		tokenStore:   tokenStore,
		sessions:     make(map[string]*PairingSession),
		devices:      make(map[string]*PairedDevice),
		failedCounts: make(map[string]int),
		lockoutUntil: make(map[string]time.Time),
	}
}

func (s *memoryPairingService) IsRateLimited(clientIP string) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()

	if lock, ok := s.lockoutUntil[clientIP]; ok {
		if time.Now().Before(lock) {
			return true
		}
	}
	return false
}

func (s *memoryPairingService) recordFailure(clientIP string) {
	if clientIP == "" {
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()

	s.failedCounts[clientIP]++
	if s.failedCounts[clientIP] >= 5 {
		s.lockoutUntil[clientIP] = time.Now().Add(5 * time.Minute)
		s.failedCounts[clientIP] = 0
	}
}

func (s *memoryPairingService) Initiate(ctx context.Context, req InitiatePairingRequest) (*PairingSession, error) {
	ttl := req.TTL
	if ttl <= 0 {
		ttl = 5 * time.Minute
	}

	codeRaw := GenerateSecureToken(4) // 8 hex characters
	code := fmt.Sprintf("PAIR-%s", strings.ToUpper(codeRaw))
	token := GenerateSecureToken(24) // 48 hex characters one-time secret

	pairingURI := fmt.Sprintf("galleon://pair?code=%s&token=%s", code, token)

	pngBytes, err := qrcode.Encode(pairingURI, qrcode.Medium, 256)
	if err != nil {
		return nil, fmt.Errorf("failed to generate QR code: %w", err)
	}

	qrDataURL := fmt.Sprintf("data:image/png;base64,%s", base64.StdEncoding.EncodeToString(pngBytes))

	now := time.Now()
	session := &PairingSession{
		Code:       code,
		Token:      token,
		QRCodeData: qrDataURL,
		PairingURI: pairingURI,
		ExpiresAt:  now.Add(ttl),
		Claimed:    false,
		CreatedAt:  now,
	}

	s.mu.Lock()
	s.sessions[code] = session
	s.mu.Unlock()

	return session, nil
}

func (s *memoryPairingService) Claim(ctx context.Context, req ClaimPairingRequest) (*ClaimPairingResponse, error) {
	if req.Code == "" || req.Token == "" || req.DeviceID == "" {
		return nil, ErrInvalidCredentials
	}

	s.mu.Lock()
	session, ok := s.sessions[req.Code]
	if !ok {
		s.mu.Unlock()
		return nil, ErrPairingCodeNotFound
	}

	if session.Claimed {
		s.mu.Unlock()
		return nil, ErrPairingCodeNotFound // Already consumed (single use)
	}

	if time.Now().After(session.ExpiresAt) {
		s.mu.Unlock()
		return nil, ErrTokenExpired
	}

	// Constant-time compare one-time token
	if subtle.ConstantTimeCompare([]byte(session.Token), []byte(req.Token)) != 1 {
		s.mu.Unlock()
		return nil, ErrInvalidCredentials
	}

	// Mark consumed immediately
	session.Claimed = true

	// Record paired device
	deviceName := req.DeviceName
	if deviceName == "" {
		deviceName = "Remote Device (" + req.DeviceID + ")"
	}
	device := &PairedDevice{
		DeviceID:   req.DeviceID,
		DeviceName: deviceName,
		Platform:   req.Platform,
		PairedAt:   time.Now(),
		LastSeenAt: time.Now(),
		Revoked:    false,
	}
	s.devices[req.DeviceID] = device
	s.mu.Unlock()

	// Issue long-lived device session token in TokenStore
	sessionToken := GenerateSecureToken(32)
	devicePrincipal := &Principal{
		ID:            PrincipalId("device-" + req.DeviceID),
		Type:          PrincipalTypeDevice,
		Role:          RoleOperator,
		Authenticated: true,
		Scopes:        []string{"fleet:read", "fleet:sync", "terminal:approve"},
		DeviceID:      req.DeviceID,
		Metadata: map[string]string{
			"device_name": deviceName,
			"platform":    req.Platform,
		},
	}

	// 30 days session validity
	ttl := 30 * 24 * time.Hour
	rec := s.tokenStore.Register(sessionToken, devicePrincipal, ttl)

	return &ClaimPairingResponse{
		SessionToken: sessionToken,
		DeviceID:     req.DeviceID,
		Principal:    devicePrincipal,
		ExpiresAt:    rec.ExpiresAt,
	}, nil
}

func (s *memoryPairingService) ListDevices(ctx context.Context) ([]*PairedDevice, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	devices := make([]*PairedDevice, 0, len(s.devices))
	for _, dev := range s.devices {
		devices = append(devices, dev)
	}
	return devices, nil
}

func (s *memoryPairingService) RevokeDevice(ctx context.Context, deviceID string) error {
	s.mu.Lock()
	device, ok := s.devices[deviceID]
	if !ok {
		s.mu.Unlock()
		return ErrInvalidCredentials
	}
	device.Revoked = true
	s.mu.Unlock()

	// Invalidate any active session tokens belonging to this device
	s.tokenStore.RevokeByDeviceID(deviceID)
	return nil
}
