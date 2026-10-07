# Galleon Client Protocol Specification (v1)

**Status:** Final  
**Standard:** JSON-RPC 2.0 over WebSocket  
**Target:** Galleon CLI, TUI, Tauri Desktop, and Web Dashboard  

---

## 1. Transport & Connection Lifecycle

- **Protocol:** `ws://` / `wss://` on route `/ws/v1/client`.
- **Authentication:** Must present Bearer token in HTTP upgrade request headers (`Authorization: Bearer <token>`) or query parameter (`?token=<token>`).
- **Fail-Closed:** Unauthenticated or unauthorized connection attempts are rejected during WebSocket handshake with HTTP `401 Unauthorized` or `403 Forbidden`.

---

## 2. JSON-RPC 2.0 Message Envelope

### 2.1 Request Envelope
```json
{
  "jsonrpc": "2.0",
  "id": "req-101",
  "method": "chat",
  "params": {}
}
```

### 2.2 Response Envelope
```json
{
  "jsonrpc": "2.0",
  "id": "req-101",
  "result": {}
}
```

### 2.3 Error Envelope
```json
{
  "jsonrpc": "2.0",
  "id": "req-101",
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": "detailed context"
  }
}
```

Standard Error Codes:
- `-32700`: Parse error
- `-32600`: Invalid Request
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error
- `-32001`: Unauthorized / Permission denied
- `-32002`: Quota exceeded / Resource gate rejection
- `-32003`: Session not found

### 2.4 Streaming Notification Envelope
Server notifications for streaming deltas and progress omit the `id` field:
```json
{
  "jsonrpc": "2.0",
  "method": "stream.event",
  "params": {
    "session_id": "sess_42",
    "type": "text_chunk",
    "content": "Running task...",
    "subagent_id": "sub_eval_1",
    "timestamp": "2026-10-07T12:00:00Z"
  }
}
```

---

## 3. Client Methods

### 3.1 `chat`
Starts a streaming conversation turn with the primary agent / crew.

- **Params:**
  ```json
  {
    "session_id": "sess-42",
    "agent_id": "primary_agent",
    "prompt": "Analyze repository security posture"
  }
  ```
- **Result:**
  ```json
  {
    "status": "started",
    "session_id": "sess-42"
  }
  ```
- **Events Emitted:**
  - `stream.event` with `type: "thought"`
  - `stream.event` with `type: "text_chunk"`
  - `stream.event` with `type: "tool_call_started"`
  - `stream.event` with `type: "tool_call_finished"`
  - `stream.event` with `type: "subagent_spawned"`
  - `stream.event` with `type: "turn_completed"`

### 3.2 `run`
Dispatches a deterministic mission / workflow run.

- **Params:**
  ```json
  {
    "run_id": "run-001",
    "workflow": "engineering_feature_delivery",
    "inputs": {
      "branch": "feat/api-v1"
    }
  }
  ```
- **Result:**
  ```json
  {
    "run_id": "run-001",
    "status": "running"
  }
  ```

### 3.3 `approve`
Submits human approval decision for a pending gate.

- **Params:**
  ```json
  {
    "approval_id": "appr-99",
    "decision": "approve",
    "reason": "Verified safe by captain"
  }
  ```
- **Result:**
  ```json
  {
    "approval_id": "appr-99",
    "status": "resolved",
    "decision": "approve"
  }
  ```

### 3.4 `status`
Inspects current status of session, runs, or resource usage.

- **Params:**
  ```json
  {
    "session_id": "sess-42"
  }
  ```
- **Result:**
  ```json
  {
    "session_id": "sess-42",
    "state": "active",
    "active_agents": 2
  }
  ```
