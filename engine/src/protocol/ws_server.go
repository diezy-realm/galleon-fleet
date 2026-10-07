package protocol

import (
	"bufio"
	"context"
	"crypto/sha1"
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/src/crew"
	"github.com/diezy-realm/galleon-fleet/engine/src/identity"
)

// JSON-RPC 2.0 structures
type Request struct {
	JSONRPC string          `json:"jsonrpc"`
	ID      any             `json:"id,omitempty"`
	Method  string          `json:"method"`
	Params  json.RawMessage `json:"params,omitempty"`
}

type Response struct {
	JSONRPC string `json:"jsonrpc"`
	ID      any    `json:"id,omitempty"`
	Result  any    `json:"result,omitempty"`
	Error   *Error `json:"error,omitempty"`
}

type Notification struct {
	JSONRPC string `json:"jsonrpc"`
	Method  string `json:"method"`
	Params  any    `json:"params"`
}

type Error struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
	Data    any    `json:"data,omitempty"`
}

const (
	ErrParse          = -32700
	ErrInvalidRequest = -32600
	ErrMethodNotFound = -32601
	ErrInvalidParams  = -32602
	ErrInternal       = -32603
	ErrUnauthorized   = -32001
	ErrQuotaExceeded  = -32002
)

// Server implements Galleon Client Protocol v1 over WebSocket JSON-RPC 2.0 (BG-E2)
type Server struct {
	orchestrator crew.Orchestrator
	resolver     identity.PrincipalResolver
}

func NewServer(orch crew.Orchestrator, resolver identity.PrincipalResolver) *Server {
	return &Server{
		orchestrator: orch,
		resolver:     resolver,
	}
}

// Handler returns the HTTP HandlerFunc protected by RequireAuthenticated
func (s *Server) Handler() http.HandlerFunc {
	return identity.RequireAuthenticated(s.resolver, s.handleUpgrade)
}

func (s *Server) handleUpgrade(w http.ResponseWriter, r *http.Request) {
	// 1. Verify WebSocket upgrade headers
	if !strings.EqualFold(r.Header.Get("Upgrade"), "websocket") ||
		!strings.Contains(strings.ToLower(r.Header.Get("Connection")), "upgrade") {
		http.Error(w, "expected websocket upgrade", http.StatusBadRequest)
		return
	}

	key := r.Header.Get("Sec-WebSocket-Key")
	if key == "" {
		http.Error(w, "missing Sec-WebSocket-Key", http.StatusBadRequest)
		return
	}

	// 2. Compute Sec-WebSocket-Accept = base64(sha1(key + GUID))
	const wsGUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
	h := sha1.New()
	h.Write([]byte(key + wsGUID))
	accept := base64.StdEncoding.EncodeToString(h.Sum(nil))

	// 3. Hijack connection
	hj, ok := w.(http.Hijacker)
	if !ok {
		http.Error(w, "webserver doesn't support hijacking", http.StatusInternalServerError)
		return
	}

	conn, bufrw, err := hj.Hijack()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	defer conn.Close()

	// 4. Send 101 Switching Protocols response
	resp := "HTTP/1.1 101 Switching Protocols\r\n" +
		"Upgrade: websocket\r\n" +
		"Connection: Upgrade\r\n" +
		"Sec-WebSocket-Accept: " + accept + "\r\n\r\n"
	if _, err := bufrw.WriteString(resp); err != nil {
		return
	}
	if err := bufrw.Flush(); err != nil {
		return
	}

	// 5. Handle WebSocket connection frame loop
	s.handleConn(conn, bufrw.Reader, r.Context())
}

func (s *Server) handleConn(conn net.Conn, reader *bufio.Reader, ctx context.Context) {
	var writeMu sync.Mutex
	sendFrame := func(payload []byte) error {
		writeMu.Lock()
		defer writeMu.Unlock()
		return writeTextFrame(conn, payload)
	}

	sendNotification := func(method string, params any) {
		msg := Notification{
			JSONRPC: "2.0",
			Method:  method,
			Params:  params,
		}
		if b, err := json.Marshal(msg); err == nil {
			_ = sendFrame(b)
		}
	}

	for {
		payload, err := readFrame(reader)
		if err != nil {
			break // Connection closed or frame read failed
		}

		var req Request
		if err := json.Unmarshal(payload, &req); err != nil {
			errResp, _ := json.Marshal(Response{
				JSONRPC: "2.0",
				Error:   &Error{Code: ErrParse, Message: "Parse error"},
			})
			_ = sendFrame(errResp)
			continue
		}

		s.dispatchMethod(ctx, req, sendFrame, sendNotification)
	}
}

func (s *Server) dispatchMethod(
	ctx context.Context,
	req Request,
	sendFrame func([]byte) error,
	sendNotification func(string, any),
) {
	switch req.Method {
	case "chat":
		var params struct {
			SessionID string `json:"session_id"`
			AgentID   string `json:"agent_id"`
			Prompt    string `json:"prompt"`
		}
		if err := json.Unmarshal(req.Params, &params); err != nil || params.SessionID == "" || params.Prompt == "" {
			errResp, _ := json.Marshal(Response{
				JSONRPC: "2.0",
				ID:      req.ID,
				Error:   &Error{Code: ErrInvalidParams, Message: "session_id and prompt required"},
			})
			_ = sendFrame(errResp)
			return
		}

		// Immediate acknowledgment result
		res, _ := json.Marshal(Response{
			JSONRPC: "2.0",
			ID:      req.ID,
			Result:  map[string]any{"status": "started", "session_id": params.SessionID},
		})
		_ = sendFrame(res)

		// Stream turn asynchronously
		go func() {
			if s.orchestrator == nil {
				return
			}
			eventCh := make(chan *crew.TurnEvent, 64)
			turnCtx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
			defer cancel()

			go func() {
				for ev := range eventCh {
					sendNotification("stream.event", map[string]any{
						"session_id":   params.SessionID,
						"type":         ev.Type,
						"content":      ev.Content,
						"subagent_id":  ev.SubagentID,
						"error":        ev.ErrorMessage,
						"timestamp":    time.Now().UTC().Format(time.RFC3339),
					})
				}
			}()

			_ = s.orchestrator.StartTurn(turnCtx, &crew.TurnRequest{
				SessionID: params.SessionID,
				AgentID:   params.AgentID,
				Prompt:    params.Prompt,
			}, eventCh)
			close(eventCh)
		}()

	case "status":
		res, _ := json.Marshal(Response{
			JSONRPC: "2.0",
			ID:      req.ID,
			Result:  map[string]any{"status": "online", "version": "v1.0.0"},
		})
		_ = sendFrame(res)

	case "run":
		res, _ := json.Marshal(Response{
			JSONRPC: "2.0",
			ID:      req.ID,
			Result:  map[string]any{"status": "queued"},
		})
		_ = sendFrame(res)

	case "approve":
		res, _ := json.Marshal(Response{
			JSONRPC: "2.0",
			ID:      req.ID,
			Result:  map[string]any{"status": "resolved"},
		})
		_ = sendFrame(res)

	default:
		errResp, _ := json.Marshal(Response{
			JSONRPC: "2.0",
			ID:      req.ID,
			Error:   &Error{Code: ErrMethodNotFound, Message: fmt.Sprintf("Method '%s' not found", req.Method)},
		})
		_ = sendFrame(errResp)
	}
}

// Minimal RFC 6455 WebSocket frame helpers (zero external dependencies)
func readFrame(r *bufio.Reader) ([]byte, error) {
	b0, err := r.ReadByte()
	if err != nil {
		return nil, err
	}
	opcode := b0 & 0x0f
	if opcode == 8 { // Close frame
		return nil, io.EOF
	}

	b1, err := r.ReadByte()
	if err != nil {
		return nil, err
	}
	isMasked := (b1 & 0x80) != 0
	length := int(b1 & 0x7f)

	if length == 126 {
		var l uint16
		if err := binary.Read(r, binary.BigEndian, &l); err != nil {
			return nil, err
		}
		length = int(l)
	} else if length == 127 {
		var l uint64
		if err := binary.Read(r, binary.BigEndian, &l); err != nil {
			return nil, err
		}
		length = int(l)
	}

	var mask [4]byte
	if isMasked {
		if _, err := io.ReadFull(r, mask[:]); err != nil {
			return nil, err
		}
	}

	payload := make([]byte, length)
	if _, err := io.ReadFull(r, payload); err != nil {
		return nil, err
	}

	if isMasked {
		for i := 0; i < length; i++ {
			payload[i] ^= mask[i%4]
		}
	}
	return payload, nil
}

func writeTextFrame(w io.Writer, payload []byte) error {
	length := len(payload)
	var header []byte

	if length <= 125 {
		header = []byte{0x81, byte(length)}
	} else if length <= 65535 {
		header = []byte{0x81, 126, byte(length >> 8), byte(length & 0xff)}
	} else {
		header = make([]byte, 10)
		header[0] = 0x81
		header[1] = 127
		binary.BigEndian.PutUint64(header[2:], uint64(length))
	}

	if _, err := w.Write(header); err != nil {
		return err
	}
	_, err := w.Write(payload)
	return err
}
