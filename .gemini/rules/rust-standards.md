# Rust Standards & Global Steering

Normative reference: [`standard-docs/tech-standart/tech-stack/rust-conventions.md`](../../standard-docs/tech-standart/tech-stack/rust-conventions.md)
Edition: **Rust 2024** | MSRV: **1.82+**

## 1. Branding & Crate Naming
- The project has rebranded from `zeroclaw` / `clawcrew` to `Galleon`.
- Any new crates, files, documents, or modules MUST use `galleon-` prefix instead of `clawcrew-` (e.g., `galleon-channel-matrix`).
- Legacy crates retain `clawcrew-*` unless refactored.

## 2. Workspace & Dependency Governance
- Workspace dependencies are declared centrally in root `Cargo.toml` under `[workspace.dependencies]`.
- Crates reference dependencies via `dep.workspace = true`.
- Shared versions prevent duplicate builds and drift.

## 3. Crate Boundaries & Clean Architecture
- `clawcrew-gateway`: gRPC server (`SystemGatewayServer` :50052), Vault, WebSocket server, mDNS discovery.
- `clawcrew-runtime`: Agent turn execution (`execute_turn`), tool dispatch (`ToolDispatcher`), WASI plugin host.
- `clawcrew-providers`: LLM vendor APIs (OpenAI, Gemini, Bedrock, Anthropic) — **SSOT (ADR 0001)**. Must remain a **leaf crate** (no dependency on `clawcrew-runtime`).
- `clawcrew-channel-*`: Channel plugins (one crate per domain/service).
- `clawcrew-tools-*`: Native tool implementations (filesystem, shell, network).

## 4. Trait-Driven Architecture
- Extensibility and testing via traits: `Provider`, `ToolExecutor`, `Channel`.
- Use `#[async_trait]`, require `Send + Sync`.
- Default trait implementations for optional functionality.

## 5. Strict Error Handling
- Use `thiserror` for error enums; define `pub type Result<T> = std::result::Result<T, Error>` at crate roots.
- **NEVER use `unwrap()` or `expect()` in production paths.** Always propagate errors with `?`.
- `unwrap()` / `expect()` are permitted **exclusively** inside test scopes (`#[cfg(test)] mod tests { ... }`, standalone `tests.rs`, integration `tests/` directory) or with explicit documented safe invariants (`// INVARIANT: ...`). Test helpers and fixtures must never be placed outside `#[cfg(test)]`.

## 6. Concurrency & Async Patterns
- Tokio runtime standard across binaries (`#[tokio::main]`) and tests (`#[tokio::test]`).
- Spawning background tasks must handle and log errors (no unhandled futures).
- Use `tokio::select!` for timeouts and cancellation.

## 7. Logging & Observability
- Use `tracing` with structured fields: `#[instrument(skip(self))]`.
- **Secret Scrubbing:** Never log full API keys or auth tokens. Redact credentials at the log boundary.

## 8. Memory & Performance
- Wrap shared state in `Arc` across tasks.
- Pass by reference (`&T`, `&[T]`) instead of unnecessary `.clone()` calls.

## 9. Compilation & Testing Optimization
- **Mandatory Fast Test Runner (`cargo-nextest`):** Seluruh pengujian Rust WAJIB menggunakan `cargo-nextest` (`cargo nextest run`) untuk eksekusi paralel, terisolasi, dan cepat (Ref: https://nexte.st/docs/installation/from-source/):
  ```powershell
  # Test spesifik per crate
  cargo nextest run -p <crate-name>
  # Test spesifik fungsi test
  cargo nextest run -p <crate-name> -E 'test(<test_fn>)'
  # Workspace testing saat pre-merge gate
  cargo nextest run --workspace
  ```
- **Targeted Checking (Fast Inner Loop):** In large workspaces, never run whole-workspace checks for routine edits. Check specific crates:
  ```powershell
  cargo check -p <crate-name>
  cargo check -p <crate-name> --lib
  cargo clippy -p <crate-name> -- -D warnings
  ```
- **Build Lock Contention on Windows:** If background analyzers lock `target/`:
  ```powershell
  $env:CARGO_TARGET_DIR="target/agent"; cargo check -p <crate-name>
  ```
- **Compiler Caching (`sccache`):** `sccache` diaktifkan di `.cargo/config.toml` (`rustc-wrapper = "sccache"`). Gunakan `sccache --show-stats` untuk memantau cache hit. Di CI runner, pastikan binary `sccache` diinstal via action sebelum Cargo dijalankan, atau gunakan `RUSTC_WRAPPER=""` jika job tidak menggunakan cache. Dilarang menghapus baris wrapper dari `.cargo/config.toml`.



