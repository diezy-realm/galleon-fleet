package run

import (
	"context"
	"encoding/json/v2"
	"fmt"
	"log/slog"
	"net/http"
	"strings"

	"github.com/diezy-realm/galleon-fleet/engine/core/logger"
	"github.com/diezy-realm/galleon-fleet/engine/core/metrics"
	"github.com/diezy-realm/galleon-fleet/engine/core/tracing"
)

// HTTPHandler handles HTTP REST and SSE endpoints for Runs
type HTTPHandler struct {
	service Service
}

// NewHTTPHandler creates a new HTTPHandler
func NewHTTPHandler(service Service) *HTTPHandler {
	return &HTTPHandler{
		service: service,
	}
}

// Service exposes the underlying run service (used for startup resume, F1-3).
func (h *HTTPHandler) Service() Service { return h.service }

// RegisterHTTP registers run endpoints on the metrics/HTTP server
func (h *HTTPHandler) RegisterHTTP(server *metrics.Server) {
	server.RegisterRouteFunc("/api/v1/runs", h.handleRunsRoot)
	server.RegisterRouteFunc("/api/v1/runs/", h.handleRunByID)
}

func (h *HTTPHandler) handleRunsRoot(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, `{"error":{"code":"METHOD_NOT_ALLOWED","message":"Method not allowed"}}`, http.StatusMethodNotAllowed)
		return
	}

	reqID := r.Header.Get("X-Request-ID")
	ctx, span := tracing.StartSpan(r.Context(), "HTTP POST /api/v1/runs")
	defer span.End()

	if reqID != "" {
		span.TraceID = reqID
		span.SetAttribute("request_id", reqID)
	} else {
		reqID = span.TraceID
	}
	span.SetAttribute("http.method", r.Method)
	span.SetAttribute("http.url", r.URL.Path)

	ctx = context.WithValue(ctx, "request_id", reqID)
	ctx = context.WithValue(ctx, "trace_id", reqID)
	r = r.WithContext(ctx)

	log := logger.Get()
	log.InfoContext(ctx, "handling create run HTTP request",
		slog.String("trace_id", reqID),
		slog.String("method", r.Method),
		slog.String("url", r.URL.Path),
	)

	idempotencyKey := r.Header.Get("Idempotency-Key")

	var req CreateRunRequest
	if err := json.UnmarshalRead(r.Body, &req); err != nil {
		log.WarnContext(ctx, "failed to parse create run json body",
			slog.String("trace_id", reqID),
			slog.String("error", err.Error()),
		)
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("X-Request-ID", reqID)
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte(`{"error":{"code":"INVALID_ARGUMENT","message":"invalid JSON body"}}`))
		return
	}

	run, err := h.service.CreateRun(r.Context(), &req, reqID, idempotencyKey)
	if err != nil {
		log.ErrorContext(ctx, "create run failed",
			slog.String("trace_id", reqID),
			slog.String("error", err.Error()),
		)
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("X-Request-ID", reqID)
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte(fmt.Sprintf(`{"error":{"code":"FAILED","message":%q}}`, err.Error())))
		return
	}

	resp := CreateRunResponse{
		ID:        run.ID,
		CrewID:    run.CrewID,
		Status:    run.Status,
		EventsURL: fmt.Sprintf("/api/v1/runs/%s/events", run.ID),
		CreatedAt: run.CreatedAt,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("X-Request-ID", reqID)
	w.WriteHeader(http.StatusAccepted)
	data, _ := json.Marshal(resp)
	_, _ = w.Write(data)
}

func (h *HTTPHandler) handleRunByID(w http.ResponseWriter, r *http.Request) {
	path := strings.TrimPrefix(r.URL.Path, "/api/v1/runs/")
	parts := strings.Split(path, "/")
	if len(parts) == 0 || parts[0] == "" {
		http.NotFound(w, r)
		return
	}

	runID := parts[0]

	reqID := r.Header.Get("X-Request-ID")
	ctx, span := tracing.StartSpan(r.Context(), fmt.Sprintf("HTTP %s /api/v1/runs/%s", r.Method, runID))
	defer span.End()

	if reqID != "" {
		span.TraceID = reqID
		span.SetAttribute("request_id", reqID)
	} else {
		reqID = span.TraceID
	}
	span.SetAttribute("http.method", r.Method)
	span.SetAttribute("http.url", r.URL.Path)
	span.SetAttribute("run_id", runID)

	ctx = context.WithValue(ctx, "request_id", reqID)
	ctx = context.WithValue(ctx, "trace_id", reqID)
	r = r.WithContext(ctx)

	w.Header().Set("X-Request-ID", reqID)

	log := logger.Get()
	log.DebugContext(ctx, "handling run by id HTTP request",
		slog.String("trace_id", reqID),
		slog.String("run_id", runID),
		slog.String("method", r.Method),
	)

	// Check sub-routes: /events or /cancel
	if len(parts) > 1 {
		switch parts[1] {
		case "events":
			h.handleEvents(w, r, runID)
			return
		case "cancel":
			h.handleCancel(w, r, runID)
			return
		default:
			http.NotFound(w, r)
			return
		}
	}

	// GET /api/v1/runs/{run_id}
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	run, err := h.service.GetRun(r.Context(), runID)
	if err != nil {
		log.WarnContext(ctx, "run not found",
			slog.String("trace_id", reqID),
			slog.String("run_id", runID),
			slog.String("error", err.Error()),
		)
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		_, _ = w.Write([]byte(fmt.Sprintf(`{"error":{"code":"RUN_NOT_FOUND","message":%q}}`, err.Error())))
		return
	}

	w.Header().Set("Content-Type", "application/json")
	data, _ := json.Marshal(run)
	_, _ = w.Write(data)
}

func (h *HTTPHandler) handleEvents(w http.ResponseWriter, r *http.Request, runID string) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Verify run exists
	if _, err := h.service.GetRun(r.Context(), runID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		_, _ = w.Write([]byte(`{"error":{"code":"RUN_NOT_FOUND","message":"run not found","layer":"DELIVERY"}}`))
		return
	}

	flusher, ok := w.(http.Flusher)
	if !ok {
		http.Error(w, "streaming unsupported", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	if reqID := r.Header.Get("X-Request-ID"); reqID != "" {
		w.Header().Set("X-Request-ID", reqID)
	}
	w.WriteHeader(http.StatusOK)
	flusher.Flush()

	// Parse Last-Event-ID for reconnection deduplication (BUG-006)
	lastEventID := r.Header.Get("Last-Event-ID")
	eventCh, unsubscribe := h.service.SubscribeEventsSince(runID, lastEventID)
	defer unsubscribe()

	ctx := r.Context()
	for {
		select {
		case <-ctx.Done():
			return
		case evt, ok := <-eventCh:
			if !ok {
				return
			}
			data, err := json.Marshal(evt)
			if err == nil {
				fmt.Fprintf(w, "id: %s\nevent: %s\ndata: %s\n\n", evt.EventID, evt.Type, data)
				flusher.Flush()
			}
		}
	}
}

func (h *HTTPHandler) handleCancel(w http.ResponseWriter, r *http.Request, runID string) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if err := h.service.CancelRun(r.Context(), runID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		if reqID := r.Header.Get("X-Request-ID"); reqID != "" {
			w.Header().Set("X-Request-ID", reqID)
		}
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte(fmt.Sprintf(`{"error":{"code":"FAILED_PRECONDITION","message":%q}}`, err.Error())))
		return
	}

	w.Header().Set("Content-Type", "application/json")
	if reqID := r.Header.Get("X-Request-ID"); reqID != "" {
		w.Header().Set("X-Request-ID", reqID)
	}
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write([]byte(`{"status":"cancelled"}`))
}
