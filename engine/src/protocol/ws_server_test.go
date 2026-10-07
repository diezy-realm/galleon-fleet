package protocol_test

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/src/crew"
	"github.com/diezy-realm/galleon-fleet/engine/src/identity"
	"github.com/diezy-realm/galleon-fleet/engine/src/protocol"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

type dummyOrchestrator struct{}

func (d *dummyOrchestrator) StartTurn(ctx context.Context, req *crew.TurnRequest, eventCh chan<- *crew.TurnEvent) error {
	eventCh <- &crew.TurnEvent{
		Type:    crew.EventTextChunk,
		Content: "Hello from Galleon Client Protocol v1",
	}
	eventCh <- &crew.TurnEvent{
		Type:    crew.EventTurnCompleted,
		Content: "Turn complete",
	}
	return nil
}
func (d *dummyOrchestrator) CancelTurn(ctx context.Context, sessionID string) error { return nil }
func (d *dummyOrchestrator) SetMaxSubagents(max int)                                  {}
func (d *dummyOrchestrator) HandoffSubagent(ctx context.Context, req *crew.HandoffRequest, eventCh chan<- *crew.TurnEvent) error {
	return nil
}

func TestWSClientProtocolAuthenticationAndJsonRpc(t *testing.T) {
	tokenStore := identity.NewMemoryTokenStore()
	token := "valid-bearer-token-123"
	tokenStore.Register(token, &identity.Principal{
		ID:            "op_test",
		Role:          identity.RoleAdmin,
		Authenticated: true,
	}, 1*time.Hour)

	resolver := identity.NewDenyByDefaultResolver(tokenStore)
	orch := &dummyOrchestrator{}
	wsServer := protocol.NewServer(orch, resolver)

	ts := httptest.NewServer(wsServer.Handler())
	defer ts.Close()

	// 1. Unauthenticated request -> 401 Unauthorized
	req, err := http.NewRequest(http.MethodGet, ts.URL, nil)
	require.NoError(t, err)
	req.Header.Set("Upgrade", "websocket")
	req.Header.Set("Connection", "Upgrade")
	req.Header.Set("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
	req.Header.Set("Sec-WebSocket-Version", "13")

	resp, err := http.DefaultClient.Do(req)
	require.NoError(t, err)
	assert.Equal(t, http.StatusUnauthorized, resp.StatusCode)
	resp.Body.Close()

	// 2. Authenticated request with Bearer token
	// Direct dial & handshake verification
	rawConn, err := net.Dial("tcp", ts.Listener.Addr().String())
	require.NoError(t, err)
	defer rawConn.Close()

	handshake := fmt.Sprintf(
		"GET / HTTP/1.1\r\nHost: %s\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nAuthorization: Bearer %s\r\n\r\n",
		ts.Listener.Addr().String(), token,
	)
	_, err = rawConn.Write([]byte(handshake))
	require.NoError(t, err)

	reader := bufio.NewReader(rawConn)
	statusLine, err := reader.ReadString('\n')
	require.NoError(t, err)
	assert.Contains(t, statusLine, "101 Switching Protocols")

	// Read remaining HTTP headers until double CRLF
	for {
		line, err := reader.ReadString('\n')
		require.NoError(t, err)
		if strings.TrimSpace(line) == "" {
			break
		}
	}

	// 3. Send JSON-RPC "status" request
	statusReq := `{"jsonrpc":"2.0","id":"req-1","method":"status"}`
	// Masked client frame: opcode 0x1, masked
	mask := []byte{1, 2, 3, 4}
	payloadBytes := []byte(statusReq)
	maskedPayload := make([]byte, len(payloadBytes))
	for i, b := range payloadBytes {
		maskedPayload[i] = b ^ mask[i%4]
	}

	frame := []byte{0x81, byte(0x80 | len(payloadBytes))}
	frame = append(frame, mask...)
	frame = append(frame, maskedPayload...)
	_, err = rawConn.Write(frame)
	require.NoError(t, err)

	// Read response frame
	b0, err := reader.ReadByte()
	require.NoError(t, err)
	assert.Equal(t, byte(0x81), b0) // Text frame
	b1, err := reader.ReadByte()
	require.NoError(t, err)
	lenResp := int(b1 & 0x7f)
	respBytes := make([]byte, lenResp)
	_, err = reader.Read(respBytes)
	require.NoError(t, err)

	var jsonResp protocol.Response
	err = json.Unmarshal(respBytes, &jsonResp)
	require.NoError(t, err)
	assert.Equal(t, "2.0", jsonResp.JSONRPC)
	assert.Equal(t, "req-1", jsonResp.ID)
	resultMap, ok := jsonResp.Result.(map[string]any)
	require.True(t, ok)
	assert.Equal(t, "online", resultMap["status"])
}
