package identity

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/diezy-realm/galleon-fleet/engine/core/metrics"
)

// PairingHTTPHandler handles `/api/pairing/*` endpoints (BG-C2)
type PairingHTTPHandler struct {
	pairingService PairingService
	resolver       PrincipalResolver
}

// NewPairingHTTPHandler creates a handler for pairing endpoints
func NewPairingHTTPHandler(pairingService PairingService, resolver PrincipalResolver) *PairingHTTPHandler {
	return &PairingHTTPHandler{
		pairingService: pairingService,
		resolver:       resolver,
	}
}

// RegisterHTTP mounts pairing endpoints onto the metrics.Server router
func (h *PairingHTTPHandler) RegisterHTTP(server *metrics.Server) {
	server.RegisterRouteFunc("/api/pairing/initiate", h.handleInitiate)
	server.RegisterRouteFunc("/api/pairing/claim", h.handleClaim)
	server.RegisterRouteFunc("/api/pairing/devices", h.handleDevices)
	server.RegisterRouteFunc("/api/pairing/revoke", h.handleRevoke)
}

func (h *PairingHTTPHandler) handleInitiate(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req InitiatePairingRequest
	if r.Body != nil {
		_ = json.NewDecoder(r.Body).Decode(&req)
	}

	session, err := h.pairingService.Initiate(r.Context(), req)
	if err != nil {
		writeJSONResponse(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
		return
	}

	writeJSONResponse(w, http.StatusOK, session)
}

func (h *PairingHTTPHandler) handleClaim(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	clientIP := r.RemoteAddr
	if h.pairingService.IsRateLimited(clientIP) {
		writeJSONResponse(w, http.StatusTooManyRequests, map[string]any{
			"error": "rate limited: too many failed pairing attempts, try again later",
		})
		return
	}

	var req ClaimPairingRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSONResponse(w, http.StatusBadRequest, map[string]any{"error": "invalid request body"})
		return
	}

	resp, err := h.pairingService.Claim(r.Context(), req)
	if err != nil {
		if err == ErrPairingCodeNotFound || err == ErrInvalidCredentials {
			writeJSONResponse(w, http.StatusUnauthorized, map[string]any{"error": err.Error()})
			return
		}
		if err == ErrTokenExpired {
			writeJSONResponse(w, http.StatusGone, map[string]any{"error": err.Error()})
			return
		}
		writeJSONResponse(w, http.StatusBadRequest, map[string]any{"error": err.Error()})
		return
	}

	writeJSONResponse(w, http.StatusOK, resp)
}

func (h *PairingHTTPHandler) handleDevices(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Strictly require authenticated caller with Operator/Admin authority
	principal, err := h.resolver.Resolve(r.Context(), r)
	if err != nil || principal == nil || !principal.IsAuthenticated() {
		writeJSONResponse(w, http.StatusUnauthorized, map[string]any{
			"error": "unauthorized: authentication required to view paired devices",
		})
		return
	}

	if !principal.HasRole(RoleOperator) {
		writeJSONResponse(w, http.StatusForbidden, map[string]any{
			"error": "forbidden: operator role required",
		})
		return
	}

	devices, err := h.pairingService.ListDevices(r.Context())
	if err != nil {
		writeJSONResponse(w, http.StatusInternalServerError, map[string]any{"error": err.Error()})
		return
	}

	writeJSONResponse(w, http.StatusOK, devices)
}

func (h *PairingHTTPHandler) handleRevoke(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Strictly require authenticated caller with Operator/Admin authority
	principal, err := h.resolver.Resolve(r.Context(), r)
	if err != nil || principal == nil || !principal.IsAuthenticated() {
		writeJSONResponse(w, http.StatusUnauthorized, map[string]any{
			"error": "unauthorized: authentication required to revoke device",
		})
		return
	}

	if !principal.HasRole(RoleOperator) {
		writeJSONResponse(w, http.StatusForbidden, map[string]any{
			"error": "forbidden: operator role required",
		})
		return
	}

	var body struct {
		DeviceID string `json:"device_id"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil || strings.TrimSpace(body.DeviceID) == "" {
		writeJSONResponse(w, http.StatusBadRequest, map[string]any{"error": "device_id is required"})
		return
	}

	if err := h.pairingService.RevokeDevice(r.Context(), body.DeviceID); err != nil {
		writeJSONResponse(w, http.StatusNotFound, map[string]any{"error": err.Error()})
		return
	}

	writeJSONResponse(w, http.StatusOK, map[string]any{
		"success":   true,
		"device_id": body.DeviceID,
		"status":    "revoked",
	})
}

func writeJSONResponse(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(data)
}
