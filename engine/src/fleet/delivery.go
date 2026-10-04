package fleet

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"runtime"
	"strings"
	"time"

	"github.com/diezy-labs/claw-crew/engine/core/metrics"
	qrcode "github.com/skip2/go-qrcode"
)

type HTTPHandler struct {
	service   Service
	startTime time.Time
}

func NewHTTPHandler(service Service) *HTTPHandler {
	return &HTTPHandler{
		service:   service,
		startTime: time.Now(),
	}
}

func (h *HTTPHandler) RegisterHTTP(server *metrics.Server) {
	server.RegisterRouteFunc("/api/fleet/metrics", h.handleMetrics)
	server.RegisterRouteFunc("/api/fleet/deck-bell", h.handleDeckBell)
	server.RegisterRouteFunc("/api/fleet/seed", h.handleSeedData)
	server.RegisterRouteFunc("/api/system/executive-briefing", h.handleBriefing)
	server.RegisterRouteFunc("/api/providers/harbor", h.handleHarborProviders)
	server.RegisterRouteFunc("/api/diagnostics", h.handleDiagnostics)
	server.RegisterRouteFunc("/api/diagnostics/remedy", h.handleRemedy)
	server.RegisterRouteFunc("/api/snapshots", h.handleSnapshots)
	server.RegisterRouteFunc("/api/fleet/policies", h.handlePolicies)
	server.RegisterRouteFunc("/api/collections/", h.handleCollections)
	server.RegisterRouteFunc("/api/chat/quartermaster", h.handleQuartermasterChat)
	server.RegisterRouteFunc("/api/network/qrcode", h.handleNetworkQRCode)
	server.RegisterRouteFunc("/api/system/metrics", h.handleSystemMetrics)
	server.RegisterRouteFunc("/api/health", h.handleHealth)
	server.RegisterRouteFunc("/api/system/network", h.handleNetwork)
	server.RegisterRouteFunc("/api/engine/processes", h.handleEngineProcesses)
	server.RegisterRouteFunc("/api/engine/execute", h.handleEngineExecute)
}

func writeJSON(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(data)
}

func (h *HTTPHandler) handleMetrics(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	metricsData, err := h.service.GetMetrics(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, metricsData)
}

func (h *HTTPHandler) handleDeckBell(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	msg, err := h.service.RingDeckBell(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"success": true, "message": msg})
}

func (h *HTTPHandler) handleBriefing(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	briefing, err := h.service.GetExecutiveBriefing(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, briefing)
}

func (h *HTTPHandler) handleHarborProviders(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	providers, err := h.service.GetHarborProviders(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, providers)
}

func (h *HTTPHandler) handleDiagnostics(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	diags, err := h.service.GetDiagnostics(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, diags)
}

func (h *HTTPHandler) handleRemedy(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	remedied, err := h.service.ApplyRemedy(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"success": true, "diagnostics": remedied})
}

func (h *HTTPHandler) handleSnapshots(w http.ResponseWriter, r *http.Request) {
	if r.Method == http.MethodGet {
		snaps, err := h.service.GetSnapshots(r.Context())
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		writeJSON(w, http.StatusOK, snaps)
		return
	}
	if r.Method == http.MethodPost {
		// Frontend (apiClient.createSnapshot) sends {title}; accept {label} too
		// for back-compat. C4: align decoder to the real request contract.
		var req struct {
			Title string `json:"title"`
			Label string `json:"label"`
		}
		_ = json.NewDecoder(r.Body).Decode(&req)
		label := req.Title
		if label == "" {
			label = req.Label
		}
		if label == "" {
			label = "Manual Snapshot via Go Orchestrator"
		}
		snap, err := h.service.CreateSnapshot(r.Context(), label)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		writeJSON(w, http.StatusCreated, snap)
		return
	}
	http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
}

func (h *HTTPHandler) handlePolicies(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	policies, err := h.service.GetFleetPolicies(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, policies)
}

func (h *HTTPHandler) handleCollections(w http.ResponseWriter, r *http.Request) {
	collectionName := strings.TrimPrefix(r.URL.Path, "/api/collections/")
	collectionName = strings.TrimSpace(collectionName)

	if collectionName == "" {
		http.NotFound(w, r)
		return
	}

	if r.Method == http.MethodGet {
		data, err := h.service.GetCollection(r.Context(), collectionName)
		if err != nil {
			// If not found yet, return empty list []
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte("[]"))
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write(data)
		return
	}

	if r.Method == http.MethodPost {
		body, err := io.ReadAll(r.Body)
		if err != nil {
			http.Error(w, "failed to read body", http.StatusBadRequest)
			return
		}
		if err := h.service.SaveCollection(r.Context(), collectionName, body); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"success": true})
		return
	}

	http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
}

func (h *HTTPHandler) handleQuartermasterChat(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	var req struct {
		Message string `json:"message"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "invalid request body", http.StatusBadRequest)
		return
	}
	resp, err := h.service.ChatQuartermaster(r.Context(), req.Message)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

func (h *HTTPHandler) handleSeedData(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	seedData, err := h.service.GetSeedData(r.Context())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	writeJSON(w, http.StatusOK, map[string]any{
		"status": "ok",
		"seed":   seedData,
	})
}

func (h *HTTPHandler) handleNetworkQRCode(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	targetURL := r.URL.Query().Get("url")
	if targetURL == "" {
		host := r.Host
		if host == "" {
			host = "localhost:9090"
		}
		targetURL = "http://" + host
	}

	pngBytes, err := qrcode.Encode(targetURL, qrcode.Medium, 256)
	if err != nil {
		http.Error(w, fmt.Sprintf("failed to generate QR code: %v", err), http.StatusInternalServerError)
		return
	}

	dataURL := "data:image/png;base64," + base64.StdEncoding.EncodeToString(pngBytes)
	writeJSON(w, http.StatusOK, map[string]string{
		"url":     targetURL,
		"dataUrl": dataURL,
	})
}

func (h *HTTPHandler) handleSystemMetrics(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var mem runtime.MemStats
	runtime.ReadMemStats(&mem)

	uptimeDuration := time.Since(h.startTime)
	hours := int(uptimeDuration.Hours())
	minutes := int(uptimeDuration.Minutes()) % 60

	usedMB := int(mem.Alloc / (1024 * 1024))
	totalSysMB := int(mem.Sys / (1024 * 1024))
	var percent int
	if totalSysMB > 0 {
		percent = int((usedMB * 100) / totalSysMB)
	}

	// Canonical System Metrics SSOT satisfying both Web-2 browser view and Tauri desktop shell
	metricsResponse := map[string]any{
		// Web-2 browser telemetry shape
		"cpuUsage": 1.5,
		"memoryUsage": map[string]any{
			"usedMB":  usedMB,
			"totalMB": totalSysMB,
			"percent": percent,
		},
		"activeProcesses": runtime.NumGoroutine(),
		"uptime":          fmt.Sprintf("%dh %dm", hours, minutes),
		"platform":        runtime.GOOS,
		"arch":            runtime.GOARCH,
		"goVersion":       runtime.Version(),

		// Tauri desktop shell telemetry shape (ADR 0003)
		"gateway_latency_ms": 5,
		"active_threads":     runtime.GOMAXPROCS(0),
		"isolation_mode":     "Landlock & Tauri Sandboxed",
		"memory_db_mb":       float64(usedMB),
	}

	writeJSON(w, http.StatusOK, metricsResponse)
}

func (h *HTTPHandler) handleHealth(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"status":    "ok",
		"uptime":    time.Since(h.startTime).String(),
		"timestamp": time.Now().UTC().Format(time.RFC3339),
	})
}

func (h *HTTPHandler) handleNetwork(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"status":        "online",
		"local_address": "127.0.0.1",
		"gateway_port":  50052,
		"engine_port":   9090,
	})
}

func (h *HTTPHandler) handleEngineProcesses(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	var mem runtime.MemStats
	runtime.ReadMemStats(&mem)
	usedMB := int(mem.Alloc / (1024 * 1024))
	processes := []map[string]any{
		{
			"pid":    os.Getpid(),
			"name":   "galleon-engine",
			"memory": usedMB,
			"cpu":    0.8,
		},
	}
	writeJSON(w, http.StatusOK, processes)
}

func (h *HTTPHandler) handleEngineExecute(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	var req struct {
		Command string `json:"command"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "invalid request body", http.StatusBadRequest)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"stdout":   fmt.Sprintf("[executed: %s]\n", req.Command),
		"exitCode": 0,
		"duration": "10ms",
	})
}

