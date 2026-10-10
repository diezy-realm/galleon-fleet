# Native Tool Catalog & SSOT Contract (Go ↔ Rust)

**Status:** Normative (SSOT)  
**Ticket:** G-29 / T3-26 (P0)  
**Protocol:** gRPC :50052 (`clawcrew.agent.SystemGateway`)  
**Protobuf Definition:** `proto/agent_service.proto`  

---

## 1. Overview & Single Source of Truth (SSOT)

Di arsitektur 4-Tier Galleon:
- **Go Engine (Tier 3)** bertindak sebagai otak orkestrator yang menginisiasi pemanggilan tool.
- **Rust SystemGateway (Tier 2)** bertindak sebagai security microkernel yang mengeksekusi native platform tools (`bash`, `git`, file I/O) di dalam sandbox OS.

Sebelum penyelarasan G-29 / T3-26, terjadi ketidaksinkronan antara 19 tool di Go dan 4 tool di Rust, termasuk mismatch nama argumen `write_file` (`content` di Go vs `contents` di Rust) serta field `workspace_path` di Protobuf yang tidak diisi. Dokumen ini mendefinisikan kontrak SSOT tunggal untuk seluruh native tool.

---

## 2. Inventarisasi & Pemetaan Tool (Go ↔ Rust)

| Tool Canonical (SSOT) | Alias / Sub-action | Tier 3 (Go Tool) | Tier 2 (Rust SystemGateway) | Skema Argumen Wajib | Risk Tier |
|---|---|---|---|---|---|
| `write_file` | — | `WriteFileTool` | `execute_native_tool` | `{"path": string, "content": string}` | `write` |
| `read_file` | `workspace.read_file` | `ReadFileTool` | `execute_native_tool` | `{"path": string}` (opsional: `max_bytes`, `offset`) | `read` |
| `bash` | `execute_command` | `ExecuteCommandTool` | `execute_native_tool` | `{"command": string}` | `execute` |
| `git` | `git.status`, `git.commit`, `git.create_branch`, `git_diff` | `GitStatusTool`, `GitCommitTool`, dll. | `execute_native_tool` | `{"args": string}` | `write` / `read` |
| `edit_file` | — | `EditFileTool` | *delegated* | `{"path": string, "target": string, "replacement": string}` | `write` |
| `workspace.list_files` | — | `ListFilesTool` | *delegated* | `{"path": string}` | `read` |
| `workspace.search_code` | — | `SearchCodeTool` | *delegated* | `{"query": string, "path": string}` | `read` |
| `web.fetch` | — | `WebFetchTool` | *in-process / delegated* | `{"url": string}` | `read` |

---

## 3. Resolusi Mismatch Kunci

### A. Konsistensi `write_file` (`content` vs `contents`)
- **Kanonikal (SSOT):** Argumen resmi di Protobuf (`WriteFileToolArgs`) dan Go adalah **`content`**.
- **Kompatibilitas Rust:** Rust `SystemGatewayService` memprioritaskan kunci `"content"` dan tetap mendukung `"contents"` sebagai fallback kompatibel:
  ```rust
  let contents = args
      .get("content")
      .or_else(|| args.get("contents"))
      .and_then(|v| v.as_str())
      .unwrap_or("");
  ```

### B. Populasi `workspace_path`
- Setiap pemanggilan gRPC `ExecuteNativeTool` dari Go kini **wajib mengisi field `workspace_path`** di `ToolCallRequest`.
- Client Go mengekstrak `workspacePath` dari parameter eksplisit atau dari envelope `__workspace_root`/`workspace_path` di JSON.
- Rust memvalidasi bahwa seluruh path operasi file berada di dalam `workspace_root` yang diotorisasi (tanpa path traversal dan symlink escape).

---

## 4. Protobuf Contract

Definisi pada `proto/agent_service.proto`:

```protobuf
service SystemGateway {
    // Requests Rust to execute a native platform tool
    rpc ExecuteNativeTool(ToolCallRequest) returns (ToolCallResponse);

    // Returns the canonical SSOT catalog of native tools and their argument schemas (G-29 / T3-26)
    rpc GetNativeToolCatalog(GetNativeToolCatalogRequest) returns (GetNativeToolCatalogResponse);
    ...
}

message ToolCallRequest {
    string tool_name = 1;
    string arguments_json = 2;
    optional string workspace_path = 3; // Wajib diisi per-request workspace override
    optional int32 timeout_seconds = 4;
}

message WriteFileToolArgs {
    string path = 1;
    string content = 2; // Canonical SSOT argument name: content
}

message ReadFileToolArgs {
    string path = 1;
    optional int64 max_bytes = 2;
    optional int64 offset = 3;
}

message BashToolArgs {
    string command = 1;
}

message GitToolArgs {
    string args = 1;
}
```

---

## 5. Bukti Verifikasi Kontrak (DoD)

1. **Go Unit & Contract Test:** `engine/src/tool/native_tool_contract_test.go`
   - `TestNativeToolSSOT_WriteFileContentSchema`: Memastikan skema Go menggunakan `content` dan bukan `contents`.
   - `TestNativeToolSSOT_WorkspacePathPopulated`: Memastikan `workspace_path` selalu terisi di gRPC `ToolCallRequest`.
   - `TestNativeToolSSOT_CatalogQuery`: Memverifikasi kueri `GetNativeToolCatalog`.
2. **Rust Integration Test:** `crates/clawcrew-gateway/tests/system_gateway_integration_test.rs`
   - `go_callable_grpc_surface_executes_write_file_with_content_key`: Lolos uji roundtrip tulis file dengan `"content"` melalui wire gRPC nyata.
   - `go_callable_grpc_surface_accepts_alias_execute_command`: Lolos uji alias eksekusi perintah shell.
   - `go_callable_grpc_surface_returns_native_tool_catalog`: Lolos uji pemanggilan `GetNativeToolCatalog`.
