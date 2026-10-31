# Rust Conventions

**Status:** Normative (code MUST follow this)  
**Last updated:** 2026-10-29
**Rust edition:** 2024  
**MSRV:** 1.96.0

---

## Cargo Workspace Structure

```
crates/
├── galleon-gateway/         # HTTP API gateway, vault, secret management
├── galleon-runtime/         # Agent turn execution, tool dispatch
├── galleon-providers/       # LLM vendor APIs (SSOT ADR 0001)
├── galleon-channel-*/       # Channel plugins (Discord, Telegram, etc.)
├── galleon-tools/           # Native tools (filesystem, shell, network)
└── [33 core workspace crates + fixtures & tools; limit: <50]
```

**Root `Cargo.toml`:**
```toml
[workspace]
members = [
    "crates/galleon-gateway",
    "crates/galleon-runtime",
    "crates/galleon-providers",
    "crates/galleon-channel-*",
    "crates/galleon-tools",
]
resolver = "2"

[workspace.package]
edition = "2024"
rust-version = "1.96.0"
license = "Apache-2.0"

[workspace.dependencies]
# Shared versions (single SSOT for deps)
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

**Convention:** New workspace dependencies go in `[workspace.dependencies]`, crates reference with `dep.workspace = true`.

---

## Crate Boundaries (Clean Architecture)

### Gateway Crate (`galleon-gateway`)

**Owns:**
- HTTP API gateway (`axum`)
- Vault (secrets encryption/decryption)
- WebSocket server (real-time message streaming)
- mDNS A2A discovery

**Exports:**
```rust
pub struct Gateway {
    vault: Arc<Vault>,
    runtime: Arc<Runtime>, // galleon-runtime
}

pub trait VaultService {
    async fn get_secret(&self, key: &str) -> Result<String>;
    async fn set_secret(&self, key: &str, value: &str) -> Result<()>;
}
```

**Dependencies:**
```toml
[dependencies]
galleon-runtime.workspace = true
galleon-providers.workspace = true
tokio.workspace = true
```

---

### Runtime Crate (`galleon-runtime`)

**Owns:**
- Agent turn execution (`execute_turn`)
- Tool dispatch (`ToolDispatcher`)
- Message streaming
- WASI plugin runtime

**Exports:**
```rust
pub async fn execute_turn(
    model: &str,
    messages: Vec<Message>,
    tools: Vec<Tool>,
) -> Result<TurnResponse>;

pub trait ToolExecutor {
    async fn execute(&self, tool: &str, args: Value) -> Result<Value>;
}
```

**Dependencies:**
```toml
[dependencies]
galleon-providers.workspace = true
galleon-tools.workspace = true
tokio.workspace = true
serde_json.workspace = true
```

---

### Providers Crate (`galleon-providers`)

**Owns:**
- LLM vendor APIs (OpenAI, Gemini, Bedrock, Anthropic) — **SSOT (ADR 0001)**
- Provider routing + fallback
- Token streaming
- Circuit breaker + retry logic

**Exports:**
```rust
pub trait Provider {
    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<TokenStream>;
}

pub struct ProviderDispatch {
    openai: OpenAIProvider,
    gemini: GeminiProvider,
    bedrock: BedrockProvider,
    anthropic: AnthropicProvider,
}

impl ProviderDispatch {
    pub fn dispatch(&self, model: &str) -> Result<&dyn Provider>;
}
```

**Dependencies:**
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio.workspace = true
serde.workspace = true
# NO dependency on galleon-runtime (providers = leaf crate)
```

**Convention:** Providers crate is **leaf** (no internal crate deps), runtime calls it.

---

### Channel Crates (`galleon-channel-*`)

**Pattern:** One crate per channel (Discord, Telegram, Slack, etc.)

**Example: `galleon-channel-discord`**
```rust
pub struct DiscordChannel {
    client: Arc<DiscordClient>,
}

impl Channel for DiscordChannel {
    async fn send_message(&self, msg: &Message) -> Result<()>;
    async fn receive_messages(&self) -> Result<MessageStream>;
}
```

**Naming:** `galleon-channel-{service}` (e.g., `galleon-channel-telegram`, `galleon-channel-slack`)

---

### Tool Crates (`galleon-tools`)

**Pattern:** Native tools (filesystem, shell, network, etc.) under `galleon-tools`

**Example: `ShellTool` in `galleon-tools`**
```rust
pub struct ShellTool;

impl ToolExecutor for ShellTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        let command = args["command"].as_str().ok_or("Missing command")?;
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await?;
        
        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code(),
        }))
    }
}
```

**Naming:** `galleon-tools` (or domain-specific tools if partitioned)

---

## Crate Modularity

1. Domain baru = crate baru `galleon-<domain>-core` tanpa `axum`, `galleon-runtime`, `galleon-gateway`. Adapter HTTP di `galleon-gateway/src/api_<domain>/`.
2. Arah dependensi: `gateway → runtime → leaf`; `gateway → <domain>-core → leaf`. Leaf tidak boleh mengimpor `runtime`/`gateway`. Cek: `cargo tree -p <leaf> -e normal | rg 'galleon-(runtime|gateway)'` harus kosong.
3. Tidak membuat crate untuk fitur tanpa konsumen (YAGNI). Stub/kerangka kosong dilarang.
4. Nama crate tidak boleh `galleon-fleet` (nama package root).
5. Dependensi pihak ketiga dideklarasikan di `[workspace.dependencies]` bila dipakai ≥ 2 crate.

---

## Trait Boundaries

### Why Traits?

- **Extensibility:** New provider/channel/tool = implement trait, no core change
- **Testability:** Mock implementations for unit tests
- **Plugin architecture:** WASI components implement same traits

### Example: Provider Trait

```rust
// galleon-providers/src/lib.rs
#[async_trait]
pub trait Provider: Send + Sync {
    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<TokenStream>;
    
    fn name(&self) -> &str;
    fn supports_tools(&self) -> bool { true }
}

// galleon-providers/src/openai.rs
pub struct OpenAIProvider {
    api_key: String,
    client: reqwest::Client,
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<TokenStream> {
        let request = self.build_request(messages, tools)?;
        let response = self.client.post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;
        
        Ok(self.parse_stream(response).await?)
    }
    
    fn name(&self) -> &str { "openai" }
}
```

**Conventions:**
- Traits use `#[async_trait]` for async methods
- Traits are `Send + Sync` (thread-safe)
- Default implementations for optional methods

---

## Error Handling

### Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    
    #[error("Tool execution failed: {0}")]
    ToolFailed(#[from] ToolError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
```

**Conventions:**
- Use `thiserror` for error enums (derive `Error`, not hand-written `Display`)
- `#[from]` for auto-conversion from upstream errors
- `Result<T>` type alias at crate root
- Error messages are sentence case ("Provider not found", not "provider not found")

### Propagate Errors (Don't Panic)

```rust
// ✅ Correct: propagate error
pub async fn execute_turn(model: &str, messages: Vec<Message>) -> Result<TurnResponse> {
    let provider = get_provider(model)?; // ? propagates error
    let response = provider.stream_chat(messages).await?;
    Ok(response)
}

// ❌ Wrong: panic on error
pub async fn execute_turn(model: &str, messages: Vec<Message>) -> TurnResponse {
    let provider = get_provider(model).unwrap(); // Panics if provider not found
    provider.stream_chat(messages).await.unwrap() // Panics if HTTP fails
}
```

**Convention:** `unwrap()` / `expect()` only in:
- Tests
- `main()` setup (crash early if config invalid)
- Documented invariants (add comment why panic is safe)

---

## Lints, Unsafe, Panic

1. Setiap `Cargo.toml` member: `[lints] workspace = true`.
2. Crate dengan hutang `unwrap/expect`: di `lib.rs`/`main.rs`
   ```rust
   // RATCHET(BL-091): baseline unwrap+expect = <N> pada <commit>. Turunkan, jangan naikkan.
   #![allow(clippy::unwrap_used, clippy::expect_used)]
   ```
   Crate baru **tidak boleh** memakai allow ini. Penambahan `unwrap/expect` baru di jalur produksi dicegah oleh gate diff yang **sudah ada**, `scripts/ci/check_production_unwrap.py` (T-CI-01), dipasang ke gate di TASK-093, bukan membuat script ratchet baru.
3. Crate baru: `#![forbid(unsafe_code)]`. Crate lama yang memakai `unsafe`: setiap blok wajib komentar `// SAFETY:` (clippy `undocumented_unsafe_blocks` diaktifkan per crate saat crate itu disentuh).
4. Test boleh `unwrap()`/`expect()` (`#[cfg(test)]` + `#[allow(clippy::unwrap_used)]` di modul test).
5. Library tidak memanggil `std::process::exit` atau `panic!` untuk kondisi yang bisa dipulihkan.

---

## Async Patterns

### Use Tokio Runtime

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let gateway = Gateway::new().await?;
    gateway.run().await?;
    Ok(())
}
```

**Convention:** Always use `#[tokio::main]` in binaries, `#[tokio::test]` in tests.

### Spawn Background Tasks

```rust
// ✅ Correct: spawn task, handle errors
tokio::spawn(async move {
    if let Err(e) = background_work().await {
        ::galleon_log::record!(
            ERROR,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Fail)
                .with_attrs(::serde_json::json!({ "error": format!("{e}") })),
            "Background task failed"
        );
    }
});

// ❌ Wrong: spawn without error handling
tokio::spawn(async move {
    background_work().await.unwrap(); // Panics in background thread
});
```

**Convention:** Always log or handle errors in spawned tasks (panics are silent).

### Select Multiple Futures

```rust
use tokio::select;

async fn process_with_timeout(work: impl Future<Output = Result<()>>) -> Result<()> {
    select! {
        result = work => result,
        _ = tokio::time::sleep(Duration::from_secs(30)) => {
            Err(RuntimeError::Timeout)
        }
    }
}
```

**Convention:** Use `select!` for timeouts, graceful shutdown, multi-future racing.

---

## Logging

Semua event log lewat `::galleon_log::record!` (dipaksa oleh `clippy.toml` `disallowed-macros` + `-D warnings`). Contoh nyata dari `crates/galleon-gateway/src/api_pairing.rs:453`:

```rust
::galleon_log::record!(
    ERROR,
    ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Fail)
        .with_outcome(::galleon_log::EventOutcome::Failure)
        .with_attrs(::serde_json::json!({ "error": format!("{e}") })),
    "device registry insert failed after successful pairing; rolling back in-process token"
);
```

- Jangan log secret, token, atau body request mentah. Gunakan `galleon-security-scrub` bila perlu menyertakan input pengguna.
- Catatan drift: teks `reason` di `clippy.toml` masih menyebut `::clawcrew_log::record!`; diperbaiki di TASK-093.

---

## Testing

### Test Matrix and Guidelines

| Lapisan | Alat | Perintah |
|---|---|---|
| Unit + integrasi | cargo-nextest | `cargo nextest run -p <crate> --locked` |
| Doctest | cargo test | `cargo test --doc -p <crate> --locked` (nextest tidak menjalankan doctest) |
| Kontrak HTTP | `insta` (sudah di `Cargo.lock`) | snapshot JSON per rute di `crates/galleon-gateway/tests/snapshots/` |
| Negatif keamanan | test biasa | 401 tanpa token, 403 di luar allowlist, 400 field asing, 413 body > limit |

- Profil `ci` nextest boleh retry; profil `default` **tidak**. Test flaky dicatat di backlog, bukan ditutup retry.
- Test yang menulis file memakai `tempfile::TempDir`, tidak menulis ke repo.

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_execute_turn_success() {
        let provider = MockProvider::new();
        let runtime = Runtime::new(provider);
        
        let response = runtime.execute_turn("gpt-4", vec![]).await.unwrap();
        assert!(!response.tokens.is_empty());
    }
    
    #[tokio::test]
    async fn test_execute_turn_provider_not_found() {
        let runtime = Runtime::new(MockProvider::new());
        
        let result = runtime.execute_turn("invalid-model", vec![]).await;
        assert!(matches!(result, Err(RuntimeError::ProviderNotFound(_))));
    }
}
```

**Conventions:**
- `#[cfg(test)]` module in same file
- `#[tokio::test]` for async tests
- Mock external dependencies (HTTP, filesystem)
- Test both success and error paths

### Integration Tests

```rust
// tests/integration_test.rs
use galleon_runtime::execute_turn;

#[tokio::test]
async fn test_turn_execution_e2e() {
    // Real provider (requires API key in env)
    let response = execute_turn("gpt-4", vec![], vec![]).await.unwrap();
    assert!(!response.content.is_empty());
}
```

**Conventions:**
- `tests/*.rs` for integration tests (separate from `src/`)
- Use real dependencies (HTTP, filesystem), not mocks
- Gate expensive tests with `#[ignore]` (run with `cargo test -- --ignored`)

---

## Naming Conventions

| Type | Pattern | Example |
|------|---------|---------|
| Crate | `snake_case` | `galleon_gateway`, `galleon_providers` |
| Module | `snake_case` | `mod api;`, `mod vault;` |
| Struct | `PascalCase` | `Gateway`, `OpenAIProvider` |
| Trait | `PascalCase` | `Provider`, `ToolExecutor` |
| Function | `snake_case` | `execute_turn`, `get_secret` |
| Constant | `UPPER_SNAKE_CASE` | `MAX_RETRIES`, `DEFAULT_TIMEOUT` |
| Type alias | `PascalCase` | `Result<T>`, `TokenStream` |

---

## Code Organization

### Crate Layout

```
galleon-gateway/
├── src/
│   ├── lib.rs            # Public API exports
│   ├── api.rs            # HTTP API router and handlers
│   ├── vault.rs          # Vault service
│   ├── ws.rs             # WebSocket server
│   └── mdns.rs           # mDNS discovery
├── tests/
│   └── integration.rs    # Integration tests
├── Cargo.toml
└── README.md
```

**Convention:** `lib.rs` exports public API, internal modules are private by default.

### Re-exports

```rust
// lib.rs
mod api;
mod vault;

pub use api::build_router;
pub use vault::{Vault, VaultService};
```

**Convention:** Re-export public types at crate root (users import from crate, not internal modules).

---

## Performance

### Use Arc for Shared State

```rust
pub struct Gateway {
    vault: Arc<Vault>,      // Shared across tasks
    runtime: Arc<Runtime>,  // Shared across tasks
}

impl Gateway {
    pub fn new(vault: Vault, runtime: Runtime) -> Self {
        Self {
            vault: Arc::new(vault),
            runtime: Arc::new(runtime),
        }
    }
}
```

**Convention:** Wrap shared state in `Arc` (cheap clone, thread-safe reference counting).

### Avoid Cloning Large Data

```rust
// ✅ Correct: pass by reference
pub async fn process_messages(messages: &[Message]) -> Result<()> {
    for msg in messages {
        // ...
    }
    Ok(())
}

// ❌ Wrong: clone entire vec
pub async fn process_messages(messages: Vec<Message>) -> Result<()> {
    // Clones vec on every call
    Ok(())
}
```

**Convention:** Pass `&T` or `&[T]` for read-only access, `T` only when taking ownership.

---

## Security

### Scrub Secrets from Logs

```rust
pub async fn call_llm(api_key: &str, prompt: &str) -> Result<String> {
    // ❌ Wrong: logs raw secret
    ::galleon_log::record!(
        INFO,
        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Read)
            .with_attrs(::serde_json::json!({ "key": api_key })),
        "Calling LLM"
    );

    // ✅ Correct: redact or scrub secret using galleon-security-scrub
    let scrubbed = ::galleon_security_scrub::scrub_secret(api_key);
    ::galleon_log::record!(
        INFO,
        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Read)
            .with_attrs(::serde_json::json!({ "key": scrubbed })),
        "Calling LLM"
    );
    
    // ...
}
```

**Convention:** Never log full secrets (API keys, tokens, passwords). Redact or scrub using galleon-security-scrub.

### Validate Input at Boundaries

```rust
pub async fn execute_turn(model: &str, messages: Vec<Message>) -> Result<TurnResponse> {
    // Validate model name
    if model.is_empty() {
        return Err(RuntimeError::InvalidInput("Model cannot be empty".into()));
    }
    
    // Validate messages
    if messages.is_empty() {
        return Err(RuntimeError::InvalidInput("Messages cannot be empty".into()));
    }
    
    // ...
}
```

**Convention:** Validate inputs at public API boundaries (reject invalid early).

---

## Build Cache (sccache)

- `rustc-wrapper` **tidak** ditaruh di `.cargo/config.toml`. Aktifkan via env:
  ```bash
  export RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0
  ```
- Alasan: Cargo gagal bila sccache tidak terpasang; sccache tidak dapat meng-cache build incremental, dan crate `bin`/`dylib`/`cdylib`/`proc-macro` tidak di-cache ([sccache Rust docs](https://github.com/mozilla/sccache/blob/main/docs/Rust.md)).

---

## Pola Handler HTTP (gateway)

```rust
pub async fn handle_get_collection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::api::require_auth(&state, &headers).map_err(ApiError::from)?;   // 1. auth dulu
    let name = CollectionName::from_str(&name).map_err(ApiError::bad_request)?; // 2. validasi tipe
    let value = state.fleet.load(name).await.map_err(ApiError::from)?;       // 3. panggil domain
    Ok(Json(value))                                                           // 4. tanpa logika lain
}
```

- `ApiError` memetakan error domain ke status eksplisit; pesan ke klien generik, detail ke `record!`.
- Request body yang mengubah state: `#[serde(deny_unknown_fields)]`.
- Body limit dan timeout dari layer router, bukan per handler (kecuali override eksplisit).

---

## File I/O

- Tulis atomik: `tempfile::NamedTempFile::new_in(dir)` → `write_all` → `as_file().sync_all()` → `persist(path)`; pola rujukan `crates/galleon-infra/src/session_store.rs:223`.
- Unix: permission 0600 untuk data pengguna.
- I/O di handler async lewat `tokio::fs` atau `spawn_blocking`.
- Nama file dari input pengguna **selalu** lewat enum/newtype tertutup, tidak pernah `format!("{dir}/{user_input}.json")`.

---

## Related Documents

- [Architecture Overview](../architecture/00-overview.md) - System design
- [Boundary Policy](../architecture/01-boundary-policy.md) - What Rust owns vs delegates
- [ADR 0001: LLM SSOT Rust](../../decisions/0001-llm-ssot-rust.md) - Rust executes LLM calls
- [ADR-0018: Rust Single Language Architecture](../../decisions/0018-rust-single-language-architecture.md) - Arsitektur bahasa tunggal Rust
- [Rust Conventions Amendment](./rust-conventions-amendment-2026-10-29.md) - Amandemen konvensi Rust single language (Applied)
