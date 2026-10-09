package identity

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"sync"
	"testing"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
)

func TestPairingService_BG_C2(t *testing.T) {
	tokenStore := NewMemoryTokenStore()
	pairingSvc := NewPairingService(tokenStore)
	resolver := NewDenyByDefaultResolver(tokenStore)
	ctx := context.Background()

	// 1. Initiate pairing session
	session, err := pairingSvc.Initiate(ctx, InitiatePairingRequest{
		DeviceName: "iPad Terminal",
		TTL:        100 * time.Millisecond,
	})
	if err != nil {
		t.Fatalf("initiate failed: %v", err)
	}

	if session.Code == "" || session.Token == "" {
		t.Fatalf("session missing code or token")
	}
	if !bytes.HasPrefix([]byte(session.QRCodeData), []byte("data:image/png;base64,")) {
		t.Fatalf("expected valid QR code data URI, got %s", session.QRCodeData)
	}

	// 2. Claim pairing session successfully
	claimResp, err := pairingSvc.Claim(ctx, ClaimPairingRequest{
		Code:       session.Code,
		Token:      session.Token,
		DeviceID:   "device-ipad-001",
		DeviceName: "iPad Terminal Pro",
		Platform:   "iOS",
	})
	if err != nil {
		t.Fatalf("claim failed: %v", err)
	}

	if claimResp.SessionToken == "" {
		t.Fatalf("expected issued session token")
	}
	if claimResp.Principal == nil || !claimResp.Principal.IsAuthenticated() {
		t.Fatalf("expected authenticated principal on claim response")
	}
	if claimResp.Principal.DeviceID != "device-ipad-001" {
		t.Fatalf("expected device id bound to principal")
	}

	// Verify the issued session token is resolvable by DenyByDefaultResolver!
	resolvedPrincipal, err := resolver.ResolveToken(ctx, claimResp.SessionToken)
	if err != nil {
		t.Fatalf("resolver failed to resolve issued device session token: %v", err)
	}
	if resolvedPrincipal.ID != "device-device-ipad-001" {
		t.Fatalf("unexpected principal ID: %s", resolvedPrincipal.ID)
	}

	// 3. Single-use enforcement: re-claiming the same code must fail
	_, errSecondClaim := pairingSvc.Claim(ctx, ClaimPairingRequest{
		Code:       session.Code,
		Token:      session.Token,
		DeviceID:   "device-attacker",
		DeviceName: "Attacker",
	})
	if errSecondClaim != ErrPairingCodeNotFound {
		t.Fatalf("expected ErrPairingCodeNotFound on re-use, got %v", errSecondClaim)
	}

	// 4. Test expiration
	expSession, err := pairingSvc.Initiate(ctx, InitiatePairingRequest{
		TTL: 10 * time.Millisecond,
	})
	if err != nil {
		t.Fatalf("initiate expiring session failed: %v", err)
	}
	time.Sleep(20 * time.Millisecond)

	_, errExpClaim := pairingSvc.Claim(ctx, ClaimPairingRequest{
		Code:     expSession.Code,
		Token:    expSession.Token,
		DeviceID: "device-late",
	})
	if errExpClaim != ErrTokenExpired {
		t.Fatalf("expected ErrTokenExpired, got %v", errExpClaim)
	}

	// 5. Test revocation
	devices, err := pairingSvc.ListDevices(ctx)
	if err != nil {
		t.Fatalf("list devices failed: %v", err)
	}
	if len(devices) == 0 {
		t.Fatalf("expected at least 1 paired device")
	}

	errRevoke := pairingSvc.RevokeDevice(ctx, "device-ipad-001")
	if errRevoke != nil {
		t.Fatalf("revoke failed: %v", errRevoke)
	}

	// The session token of the revoked device should now be rejected as revoked
	_, errAfterRevoke := resolver.ResolveToken(ctx, claimResp.SessionToken)
	if errAfterRevoke != ErrTokenRevoked {
		t.Fatalf("expected ErrTokenRevoked after device revocation, got %v", errAfterRevoke)
	}
}

func TestPairingDeliveryHTTP_BG_C2(t *testing.T) {
	tokenStore := NewMemoryTokenStore()
	pairingSvc := NewPairingService(tokenStore)
	resolver := NewDenyByDefaultResolver(tokenStore)
	handler := NewPairingHTTPHandler(pairingSvc, resolver)

	adminToken := "admin-super-key"
	tokenStore.Register(adminToken, &Principal{
		ID:            "admin",
		Role:          RoleAdmin,
		Authenticated: true,
	}, 1*time.Hour)

	// 1. POST /api/pairing/initiate
	initReq := httptest.NewRequest(http.MethodPost, "/api/pairing/initiate", bytes.NewBufferString(`{}`))
	initRec := httptest.NewRecorder()
	handler.handleInitiate(initRec, initReq)
	if initRec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK for initiate, got %d", initRec.Code)
	}

	var sess PairingSession
	if err := json.Unmarshal(initRec.Body.Bytes(), &sess); err != nil {
		t.Fatalf("failed to decode initiate response: %v", err)
	}

	// 2. POST /api/pairing/claim
	claimBody, _ := json.Marshal(ClaimPairingRequest{
		Code:       sess.Code,
		Token:      sess.Token,
		DeviceID:   "dev-phone-123",
		DeviceName: "Pixel Phone",
	})
	claimReq := httptest.NewRequest(http.MethodPost, "/api/pairing/claim", bytes.NewBuffer(claimBody))
	claimRec := httptest.NewRecorder()
	handler.handleClaim(claimRec, claimReq)
	if claimRec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK for claim, got %d: %s", claimRec.Code, claimRec.Body.String())
	}

	// 3. GET /api/pairing/devices without auth rejected with 401
	unauthReq := httptest.NewRequest(http.MethodGet, "/api/pairing/devices", nil)
	unauthRec := httptest.NewRecorder()
	handler.handleDevices(unauthRec, unauthReq)
	if unauthRec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401 Unauthorized without credentials, got %d", unauthRec.Code)
	}

	// 4. GET /api/pairing/devices with admin auth succeeds with 200
	authedReq := httptest.NewRequest(http.MethodGet, "/api/pairing/devices", nil)
	authedReq.Header.Set("Authorization", "Bearer "+adminToken)
	authedRec := httptest.NewRecorder()
	handler.handleDevices(authedRec, authedReq)
	if authedRec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK with admin credentials, got %d", authedRec.Code)
	}

	// 5. POST /api/pairing/revoke with admin auth succeeds
	revokeBody, _ := json.Marshal(map[string]string{"device_id": "dev-phone-123"})
	revokeReq := httptest.NewRequest(http.MethodPost, "/api/pairing/revoke", bytes.NewBuffer(revokeBody))
	revokeReq.Header.Set("Authorization", "Bearer "+adminToken)
	revokeRec := httptest.NewRecorder()
	handler.handleRevoke(revokeRec, revokeReq)
	if revokeRec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK for revoke, got %d", revokeRec.Code)
	}
}

type mockDeviceSyncProvider struct {
	mu      sync.Mutex
	devices map[string]*pb.DeviceRecord
}

func newMockDeviceSyncProvider() *mockDeviceSyncProvider {
	return &mockDeviceSyncProvider{
		devices: make(map[string]*pb.DeviceRecord),
	}
}

func (m *mockDeviceSyncProvider) SyncPairedDevice(ctx context.Context, devID, devName, platform, tokenHash string) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.devices[devID] = &pb.DeviceRecord{
		Id:         devID,
		Name:       devName,
		DeviceType: platform,
		PairedAt:   time.Now().UTC().Format(time.RFC3339),
		LastSeen:   time.Now().UTC().Format(time.RFC3339),
	}
	return nil
}

func (m *mockDeviceSyncProvider) ListPairedDevices(ctx context.Context) ([]*pb.DeviceRecord, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	out := make([]*pb.DeviceRecord, 0, len(m.devices))
	for _, d := range m.devices {
		out = append(out, d)
	}
	return out, nil
}

func (m *mockDeviceSyncProvider) RevokePairedDevice(ctx context.Context, devID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	delete(m.devices, devID)
	return nil
}

func TestPairingService_SynchronizeWithRustRegistrySSOT(t *testing.T) {
	ctx := context.Background()
	tokenStore := NewMemoryTokenStore()
	mockSSOT := newMockDeviceSyncProvider()

	// 1. Inisialisasi pairing service dengan mock sync provider ke Rust SSOT
	svc := NewPairingServiceWithSync(tokenStore, mockSSOT)

	sess, err := svc.Initiate(ctx, InitiatePairingRequest{DeviceName: "MacBook Pro M3", TTL: 5 * time.Minute})
	if err != nil {
		t.Fatalf("Initiate failed: %v", err)
	}

	// 2. Claim device di Go engine
	claimResp, err := svc.Claim(ctx, ClaimPairingRequest{
		Code:       sess.Code,
		Token:      sess.Token,
		DeviceID:   "dev-mac-007",
		DeviceName: "MacBook Pro M3",
		Platform:   "darwin",
	})
	if err != nil {
		t.Fatalf("Claim failed: %v", err)
	}
	if claimResp.DeviceID != "dev-mac-007" {
		t.Fatalf("expected device dev-mac-007, got %s", claimResp.DeviceID)
	}

	// 3. Verifikasi device tersinkronisasi ke SSOT (Rust DeviceRegistry)
	ssotDevices, err := mockSSOT.ListPairedDevices(ctx)
	if err != nil || len(ssotDevices) != 1 {
		t.Fatalf("expected 1 device in SSOT, got %d (err: %v)", len(ssotDevices), err)
	}
	if ssotDevices[0].Id != "dev-mac-007" {
		t.Fatalf("expected SSOT device dev-mac-007, got %s", ssotDevices[0].Id)
	}

	// 4. Simulasi Go engine restart:
	// In-memory cache kosong, tokenStore baru, tetapi syncProvider tetap mengarah ke SSOT yang sama
	newEngineTokenStore := NewMemoryTokenStore()
	restartedSvc := NewPairingServiceWithSync(newEngineTokenStore, mockSSOT)

	// ListDevices harus merehidrasi state dari SSOT tanpa requiring re-pairing
	rehydratedDevices, err := restartedSvc.ListDevices(ctx)
	if err != nil {
		t.Fatalf("ListDevices after restart failed: %v", err)
	}
	if len(rehydratedDevices) != 1 {
		t.Fatalf("expected 1 rehydrated device after restart, got %d", len(rehydratedDevices))
	}
	if rehydratedDevices[0].DeviceID != "dev-mac-007" {
		t.Fatalf("expected rehydrated device dev-mac-007, got %s", rehydratedDevices[0].DeviceID)
	}

	// 5. Revoke device dan verifikasi propagasi ke SSOT
	if err := restartedSvc.RevokeDevice(ctx, "dev-mac-007"); err != nil {
		t.Fatalf("RevokeDevice failed: %v", err)
	}
	ssotAfterRevoke, _ := mockSSOT.ListPairedDevices(ctx)
	if len(ssotAfterRevoke) != 0 {
		t.Fatalf("expected 0 devices in SSOT after revoke, got %d", len(ssotAfterRevoke))
	}
}
