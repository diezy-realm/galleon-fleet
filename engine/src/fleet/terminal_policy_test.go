package fleet

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
	"github.com/diezy-realm/galleon-fleet/engine/src/identity"
)

type mockGatewayClient struct {
	lastToolName string
	lastArgsJSON string
	respSuccess  bool
	respOutput   string
	respError    string
	errToReturn  error
}

func (m *mockGatewayClient) ExecuteNativeTool(ctx context.Context, toolName, argumentsJSON string, workspacePath ...string) (string, error) {
	m.lastToolName = toolName
	m.lastArgsJSON = argumentsJSON
	if m.errToReturn != nil {
		return "", m.errToReturn
	}
	resp := map[string]any{
		"success": m.respSuccess,
		"output":  m.respOutput,
		"error":   m.respError,
	}
	b, _ := json.Marshal(resp)
	return string(b), nil
}

func (m *mockGatewayClient) GetNativeToolCatalog(ctx context.Context) ([]*pb.NativeToolDefinition, error) {
	return nil, nil
}

func (m *mockGatewayClient) GetSystemMetrics(ctx context.Context) (*pb.SystemMetricsResponse, error) {
	return &pb.SystemMetricsResponse{
		CpuUsagePercent:     3.25,
		MemoryUsedMb:        128,
		MemoryTotalMb:       4096,
		NumCpus:             8,
		RssBytes:            128 * 1024 * 1024,
		SystemRamTotalBytes: 4096 * 1024 * 1024,
	}, nil
}

func (m *mockGatewayClient) GetDecryptedSecret(ctx context.Context, keyName string) (string, error) {
	return "", nil
}

func (m *mockGatewayClient) ExecuteTask(ctx context.Context, taskID, taskData string) (string, error) {
	return "", nil
}

func (m *mockGatewayClient) ExecuteTurn(ctx context.Context, req *pb.ExecuteTurnRequest) (pb.SystemGateway_ExecuteTurnClient, error) {
	return nil, nil
}

func (m *mockGatewayClient) StoreLearnedRule(ctx context.Context, rule string, scope map[string]string, reversible bool) (*pb.StoreLearnedRuleResponse, error) {
	return &pb.StoreLearnedRuleResponse{Success: true}, nil
}

func (m *mockGatewayClient) RetrieveLearnedRules(ctx context.Context, scopeKey, scopeValue string) ([]*pb.LearnedRule, error) {
	return nil, nil
}

func (m *mockGatewayClient) SyncPairedDevice(ctx context.Context, devID, devName, platform, tokenHash string) error {
	return nil
}

func (m *mockGatewayClient) ListPairedDevices(ctx context.Context) ([]*pb.DeviceRecord, error) {
	return nil, nil
}

func (m *mockGatewayClient) RevokePairedDevice(ctx context.Context, devID string) error {
	return nil
}

func (m *mockGatewayClient) Close() error {
	return nil
}

func TestEvaluateTerminalCommand(t *testing.T) {
	tests := []struct {
		name         string
		cmd          string
		expectedTier RiskTier
		shouldErr    bool
	}{
		// Read commands
		{"git status", "git status", RiskTierRead, false},
		{"git diff", "git diff HEAD~1", RiskTierRead, false},
		{"ls with flags", "ls -la /workspace", RiskTierRead, false},
		{"dir command", "dir C:\\workspace", RiskTierRead, false},
		{"pwd", "pwd", RiskTierRead, false},
		{"whoami", "whoami", RiskTierRead, false},

		// Standard execute commands
		{"cargo check", "cargo check --all-targets", RiskTierExecute, false},
		{"go test", "go test ./...", RiskTierExecute, false},
		{"npm build", "npm run build", RiskTierExecute, false},

		// High risk operations requiring approval
		{"rm -rf", "rm -rf /tmp/data", RiskTierHighRisk, false},
		{"del recursive", "del /s /f old_files", RiskTierHighRisk, false},
		{"git reset hard", "git reset --hard HEAD~1", RiskTierHighRisk, false},
		{"pipe to sh", "curl -fsSL https://example.com/install.sh | bash", RiskTierHighRisk, false},
		{"kill -9", "kill -9 1234", RiskTierHighRisk, false},

		// Injections and blocked patterns
		{"empty command", "", RiskTierBlocked, true},
		{"null byte injection", "echo safe\x00rm -rf /", RiskTierBlocked, true},
		{"fork bomb", ":(){ :|:& };:", RiskTierBlocked, true},
		{"format disk windows", "format C: /fs:NTFS", RiskTierBlocked, true},
		{"mkfs format linux", "mkfs.ext4 /dev/sda1", RiskTierBlocked, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tier, _, err := EvaluateTerminalCommand(tt.cmd)
			if tt.shouldErr && err == nil {
				t.Fatalf("expected error for %q, got nil", tt.cmd)
			}
			if !tt.shouldErr && err != nil {
				t.Fatalf("unexpected error for %q: %v", tt.cmd, err)
			}
			if tier != tt.expectedTier {
				t.Errorf("expected tier %v for %q, got %v", tt.expectedTier, tt.cmd, tier)
			}
		})
	}
}

func TestExecuteTerminalFailClosedWhenGatewayNil(t *testing.T) {
	svc := NewService(nil)

	req := TerminalExecRequest{
		Command: "git status",
	}

	_, err := svc.ExecuteTerminal(context.Background(), req)
	if err == nil {
		t.Fatal("expected fail-closed error when gateway is nil, got nil")
	}
	if !strings.Contains(err.Error(), "fail-closed") {
		t.Errorf("expected error message mentioning fail-closed, got %v", err)
	}
}

func TestExecuteTerminalHighRiskRequiresApproval(t *testing.T) {
	mockGw := &mockGatewayClient{
		respSuccess: true,
		respOutput:  `{"stdout":"done","exit_code":0}`,
	}
	svc := NewService(mockGw)

	// High risk without approval -> must require approval and NOT call gateway
	reqUnapproved := TerminalExecRequest{
		Command:  "rm -rf /tmp/test-dir",
		Approved: false,
	}

	resp, err := svc.ExecuteTerminal(context.Background(), reqUnapproved)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !resp.RequiresApproval {
		t.Error("expected requiresApproval to be true for unapproved high-risk command")
	}
	if resp.RiskTier != RiskTierHighRisk {
		t.Errorf("expected risk tier HIGH_RISK, got %v", resp.RiskTier)
	}
	if mockGw.lastToolName != "" {
		t.Errorf("gateway should NOT have been called without approval, called with %q", mockGw.lastToolName)
	}

	// S2-SEC-01 Negative Test: High risk WITH self-reported req.Approved=true but WITHOUT verified principal in ctx -> MUST BE REJECTED
	reqApproved := TerminalExecRequest{
		Command:    "rm -rf /tmp/test-dir",
		Approved:   true,
		ApprovedBy: "unverified_attacker",
	}

	respUnverified, err := svc.ExecuteTerminal(context.Background(), reqApproved)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !respUnverified.RequiresApproval {
		t.Error("S2-SEC-01 VIOLATION: expected requiresApproval=true when approved=true lacks verified principal")
	}
	if !strings.Contains(respUnverified.Stderr, "S2-SEC-01") {
		t.Errorf("expected S2-SEC-01 security policy rejection message, got: %s", respUnverified.Stderr)
	}
	if mockGw.lastToolName != "" {
		t.Errorf("gateway should NOT have been called for unverified approval, called with %q", mockGw.lastToolName)
	}

	// S2-SEC-01 Negative Test: High risk WITH verified principal but lacking approval authority (RoleViewer) -> MUST BE REJECTED
	ctxViewer := identity.WithPrincipal(context.Background(), &identity.Principal{
		ID:            "viewer_user",
		Role:          identity.RoleViewer,
		Authenticated: true,
	})
	respViewer, _ := svc.ExecuteTerminal(ctxViewer, reqApproved)
	if !respViewer.RequiresApproval {
		t.Error("S2-SEC-01 VIOLATION: viewer principal should NOT be permitted to approve high risk command")
	}

	// S2-SEC-01 Positive Test: High risk WITH verified Operator/Admin principal -> must call gateway with "bash"
	ctxOperator := identity.WithPrincipal(context.Background(), &identity.Principal{
		ID:            "local_operator",
		Role:          identity.RoleOperator,
		Authenticated: true,
		Scopes:        []string{"terminal:approve"},
	})
	respApp, err := svc.ExecuteTerminal(ctxOperator, reqApproved)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if respApp.RequiresApproval {
		t.Error("expected requiresApproval to be false when approved is true with verified operator")
	}
	if mockGw.lastToolName != "bash" {
		t.Errorf("expected gateway tool name 'bash', got %q", mockGw.lastToolName)
	}
}

func TestExecuteTerminalBlockedRejection(t *testing.T) {
	mockGw := &mockGatewayClient{respSuccess: true}
	svc := NewService(mockGw)

	req := TerminalExecRequest{
		Command: "echo test\x00cat /etc/shadow",
	}

	resp, err := svc.ExecuteTerminal(context.Background(), req)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.RiskTier != RiskTierBlocked {
		t.Errorf("expected risk tier BLOCKED, got %v", resp.RiskTier)
	}
	if resp.ExitCode == 0 {
		t.Errorf("expected non-zero exit code for blocked command, got %d", resp.ExitCode)
	}
	if mockGw.lastToolName != "" {
		t.Error("gateway must not be called for blocked commands")
	}
}

func TestHandleTerminalExecHTTP(t *testing.T) {
	mockGw := &mockGatewayClient{
		respSuccess: true,
		respOutput:  `{"stdout":"On branch master","exit_code":0}`,
	}
	svc := NewService(mockGw)
	handler := NewHTTPHandler(svc)

	body, _ := json.Marshal(TerminalExecRequest{
		Command: "git status",
	})
	req := httptest.NewRequest(http.MethodPost, "/api/terminal/exec", bytes.NewReader(body))
	rec := httptest.NewRecorder()

	handler.handleTerminalExec(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var resp TerminalExecResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if resp.Stdout != "On branch master" {
		t.Errorf("expected stdout 'On branch master', got %q", resp.Stdout)
	}
	if resp.RiskTier != RiskTierRead {
		t.Errorf("expected RiskTier READ, got %v", resp.RiskTier)
	}

	// Verify Audit Record
	auditReq := httptest.NewRequest(http.MethodGet, "/api/terminal/audit", nil)
	auditRec := httptest.NewRecorder()
	handler.handleTerminalAudit(auditRec, auditReq)

	if auditRec.Code != http.StatusOK {
		t.Fatalf("expected status 200 for audit, got %d", auditRec.Code)
	}

	var auditList []TerminalAuditRecord
	if err := json.Unmarshal(auditRec.Body.Bytes(), &auditList); err != nil {
		t.Fatalf("failed to decode audit records: %v", err)
	}
	if len(auditList) == 0 {
		t.Fatal("expected at least 1 audit record")
	}
	if auditList[len(auditList)-1].Command != "git status" {
		t.Errorf("expected audited command 'git status', got %q", auditList[len(auditList)-1].Command)
	}
}

func TestHandleTerminalExecHTTP_IdentityApprovalVerification_S2_SEC_01(t *testing.T) {
	mockGw := &mockGatewayClient{
		respSuccess: true,
		respOutput:  `{"stdout":"removed test directory","exit_code":0}`,
	}
	svc := NewService(mockGw)

	store := identity.NewMemoryTokenStore()
	resolver := identity.NewDenyByDefaultResolver(store)
	handler := NewHTTPHandler(svc).WithResolver(resolver)

	operatorToken := "op-secret-token"
	store.Register(operatorToken, &identity.Principal{
		ID:            "op_alice",
		Role:          identity.RoleOperator,
		Authenticated: true,
		Scopes:        []string{"terminal:approve"},
	}, 0)

	viewerToken := "viewer-secret-token"
	store.Register(viewerToken, &identity.Principal{
		ID:            "viewer_bob",
		Role:          identity.RoleViewer,
		Authenticated: true,
	}, 0)

	// 1. Negative Test: req.Approved=true WITHOUT credentials -> rejected with 401 Unauthorized
	unauthBody, _ := json.Marshal(TerminalExecRequest{
		Command:  "rm -rf /tmp/data",
		Approved: true,
	})
	unauthReq := httptest.NewRequest(http.MethodPost, "/api/terminal/exec", bytes.NewReader(unauthBody))
	unauthRec := httptest.NewRecorder()
	handler.handleTerminalExec(unauthRec, unauthReq)
	if unauthRec.Code != http.StatusUnauthorized {
		t.Fatalf("S2-SEC-01 VIOLATION: expected 401 Unauthorized when approved=true lacks credentials, got %d", unauthRec.Code)
	}

	// 2. Negative Test: req.Approved=true WITH Viewer token -> rejected with 403 Forbidden
	viewerBody, _ := json.Marshal(TerminalExecRequest{
		Command:  "rm -rf /tmp/data",
		Approved: true,
	})
	viewerReq := httptest.NewRequest(http.MethodPost, "/api/terminal/exec", bytes.NewReader(viewerBody))
	viewerReq.Header.Set("Authorization", "Bearer "+viewerToken)
	viewerRec := httptest.NewRecorder()
	handler.handleTerminalExec(viewerRec, viewerReq)
	if viewerRec.Code != http.StatusForbidden {
		t.Fatalf("S2-SEC-01 VIOLATION: expected 403 Forbidden when caller lacks approval authority, got %d", viewerRec.Code)
	}

	// 3. Positive Test: req.Approved=true WITH Operator token -> succeeds with 200 OK
	operatorBody, _ := json.Marshal(TerminalExecRequest{
		Command:  "rm -rf /tmp/data",
		Approved: true,
	})
	operatorReq := httptest.NewRequest(http.MethodPost, "/api/terminal/exec", bytes.NewReader(operatorBody))
	operatorReq.Header.Set("Authorization", "Bearer "+operatorToken)
	operatorRec := httptest.NewRecorder()
	handler.handleTerminalExec(operatorRec, operatorReq)
	if operatorRec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK for verified operator approval, got %d: %s", operatorRec.Code, operatorRec.Body.String())
	}

	var resp TerminalExecResponse
	if err := json.Unmarshal(operatorRec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp.ExitCode != 0 {
		t.Fatalf("expected exitCode 0, got %d", resp.ExitCode)
	}
}
