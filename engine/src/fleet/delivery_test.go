package fleet

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/diezy-realm/galleon-fleet/engine/core/metrics"
)

func TestHandleNetworkQRCode(t *testing.T) {
	handler := NewHTTPHandler(nil)

	req := httptest.NewRequest(http.MethodGet, "/api/network/qrcode?url=http://127.0.0.1:9090", nil)
	rec := httptest.NewRecorder()

	handler.handleNetworkQRCode(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", rec.Code)
	}

	var resp struct {
		URL     string `json:"url"`
		DataURL string `json:"dataUrl"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if resp.URL != "http://127.0.0.1:9090" {
		t.Errorf("expected url 'http://127.0.0.1:9090', got %q", resp.URL)
	}
	if !strings.HasPrefix(resp.DataURL, "data:image/png;base64,") {
		t.Errorf("expected dataUrl to start with 'data:image/png;base64,', got %q", resp.DataURL)
	}
}

func TestHandleSystemMetrics(t *testing.T) {
	handler := NewHTTPHandler(nil)

	req := httptest.NewRequest(http.MethodGet, "/api/system/metrics", nil)
	rec := httptest.NewRecorder()

	handler.handleSystemMetrics(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", rec.Code)
	}

	var resp map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	// Verify Web-2 shape
	if _, ok := resp["cpuUsage"]; !ok {
		t.Errorf("expected cpuUsage field in response")
	}
	if _, ok := resp["memoryUsage"]; !ok {
		t.Errorf("expected memoryUsage field in response")
	}
	if _, ok := resp["activeProcesses"]; !ok {
		t.Errorf("expected activeProcesses field in response")
	}
	if _, ok := resp["uptime"]; !ok {
		t.Errorf("expected uptime field in response")
	}

	// Verify Tauri / Desktop shell shape (ADR 0003)
	if _, ok := resp["gateway_latency_ms"]; !ok {
		t.Errorf("expected gateway_latency_ms field in response")
	}
	if _, ok := resp["active_threads"]; !ok {
		t.Errorf("expected active_threads field in response")
	}
	if _, ok := resp["memory_db_mb"]; !ok {
		t.Errorf("expected memory_db_mb field in response")
	}
}

func TestHandleSystemMetrics_RealGatewayDelegation(t *testing.T) {
	mockGw := &mockGatewayClient{
		respSuccess: true,
	}
	svc := NewService(mockGw)
	handler := NewHTTPHandler(svc)

	req := httptest.NewRequest(http.MethodGet, "/api/system/metrics", nil)
	rec := httptest.NewRecorder()

	handler.handleSystemMetrics(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", rec.Code)
	}

	var resp map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	// Verify CPU and Memory values came from Rust gateway mock (3.25% CPU, 128MB used, 4096MB total)
	cpuUsage, ok := resp["cpuUsage"].(float64)
	if !ok || cpuUsage != 3.25 {
		t.Errorf("expected cpuUsage from Rust gateway 3.25, got %v", resp["cpuUsage"])
	}

	memUsage, ok := resp["memoryUsage"].(map[string]any)
	if !ok {
		t.Fatalf("expected memoryUsage map in response")
	}
	if usedMB, ok := memUsage["usedMB"].(float64); !ok || usedMB != 128 {
		t.Errorf("expected usedMB from Rust gateway 128, got %v", memUsage["usedMB"])
	}
	if totalMB, ok := memUsage["totalMB"].(float64); !ok || totalMB != 4096 {
		t.Errorf("expected totalMB from Rust gateway 4096, got %v", memUsage["totalMB"])
	}
}

func TestHandleExtendedSystemEndpoints(t *testing.T) {
	mockGw := &mockGatewayClient{
		respSuccess: true,
		respOutput:  `{"stdout":"test output","exit_code":0}`,
	}
	svc := NewService(mockGw)
	h := NewHTTPHandler(svc)
	server := metrics.NewServer(0)
	h.RegisterHTTP(server)

	// 1. /api/health
	req := httptest.NewRequest(http.MethodGet, "/api/health", nil)
	rec := httptest.NewRecorder()
	server.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from /api/health, got %d", rec.Code)
	}
	var health map[string]any
	_ = json.Unmarshal(rec.Body.Bytes(), &health)
	if health["status"] != "ok" {
		t.Errorf("expected health status 'ok', got %v", health["status"])
	}

	// 2. /api/system/network
	req = httptest.NewRequest(http.MethodGet, "/api/system/network", nil)
	rec = httptest.NewRecorder()
	server.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from /api/system/network, got %d", rec.Code)
	}

	// 3. /api/engine/processes
	req = httptest.NewRequest(http.MethodGet, "/api/engine/processes", nil)
	rec = httptest.NewRecorder()
	server.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from /api/engine/processes, got %d", rec.Code)
	}
	var processes []map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &processes); err != nil || len(processes) == 0 {
		t.Fatalf("expected non-empty process list, got error: %v", err)
	}

	// 4. /api/engine/execute
	req = httptest.NewRequest(http.MethodPost, "/api/engine/execute", strings.NewReader(`{"command":"echo test"}`))
	rec = httptest.NewRecorder()
	server.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from /api/engine/execute, got %d", rec.Code)
	}
}

func TestCollectionPathTraversalSanitization(t *testing.T) {
	tempDir := t.TempDir()
	t.Setenv("GALLEON_DATA_DIR", tempDir)

	svc := NewService(nil)
	handler := NewHTTPHandler(svc)
	server := metrics.NewServer(0)
	handler.RegisterHTTP(server)

	// Save collection with path traversal in URL: /api/collections/../../evil
	req := httptest.NewRequest(http.MethodPost, "/api/collections/../../evil", strings.NewReader(`[{"name":"test"}]`))
	rec := httptest.NewRecorder()
	handler.handleCollections(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from save collection, got %d", rec.Code)
	}

	// Verify that the file was written INSIDE tempDir and sanitized to evil.json, NOT outside tempDir
	expectedFile := filepath.Join(tempDir, "evil.json")
	if _, err := os.Stat(expectedFile); os.IsNotExist(err) {
		t.Fatalf("expected file to be safely saved at %s", expectedFile)
	}

	// Verify that reading with path traversal retrieves the sanitized collection
	req = httptest.NewRequest(http.MethodGet, "/api/collections/../../evil", nil)
	rec = httptest.NewRecorder()
	handler.handleCollections(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from get collection, got %d", rec.Code)
	}
	var items []map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &items); err != nil || len(items) == 0 {
		t.Fatalf("expected non-empty items, got error %v", err)
	}
}
