//! Architecture gate: enforce strict 3-tier boundaries (ODR-01, ODR-2026-10-29-rust-single-language, ADR-0001, ADR-0002, GW-4).
//!
//! Tier 1: Desktop Shell (Tauri) — OS packaging & IPC only; no direct shell execution, no fake metrics.
//! Tier 2: Rust Core / Gateway — Security microkernel, sandbox, native tools, LLM execution, fleet governance & single backend runtime.
//! Tier 3: Web UI (web/) — Rendering & HTTP client only; zero backend logic, no direct LLM SDKs, no direct execution endpoints.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn test_ui_tier_boundaries() {
    let root = workspace_root();
    let web_src = root.join("web").join("src");
    let mut violations = Vec::new();

    // 1. web/src must not import vendor LLM SDKs or direct Rust gRPC clients
    fn scan_web_src(dir: &Path, violations: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_web_src(&path, violations);
                continue;
            }
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if ext != "ts" && ext != "tsx" && ext != "js" && ext != "jsx" {
                continue;
            }

            let Ok(src) = fs::read_to_string(&path) else {
                continue;
            };
            for (lineno, line) in src.lines().enumerate() {
                let trimmed = line.trim();
                // Forbid direct vendor LLM SDKs in UI tier
                if trimmed.contains("@google/genai")
                    || trimmed.contains("@anthropic-ai/sdk")
                    || (trimmed.starts_with("import") && trimmed.contains("from 'openai'"))
                {
                    violations.push(format!(
                        "  {}:{}: Direct vendor LLM import in UI tier: {}",
                        path.display(),
                        lineno + 1,
                        trimmed
                    ));
                }

                // Forbid direct Rust gRPC gateway access from UI tier
                if trimmed.contains("SystemGatewayClient") || trimmed.contains(":50052") {
                    violations.push(format!(
                        "  {}:{}: Direct Rust gRPC gateway reference (:50052) in UI tier: {}",
                        path.display(),
                        lineno + 1,
                        trimmed
                    ));
                }

                // Forbid direct fetch(...) calls in UI components outside apiClient.ts (U-11 / ADR 0002)
                let is_api_client =
                    path.ends_with("apiClient.ts") || path.ends_with("apiClient.tsx");
                if !is_api_client && (trimmed.contains("fetch(") || trimmed.starts_with("fetch(")) {
                    violations.push(format!(
                        "  {}:{}: Direct fetch() call in UI tier outside apiClient.ts (U-11 violation): {}",
                        path.display(),
                        lineno + 1,
                        trimmed
                    ));
                }
            }
        }
    }

    if web_src.exists() {
        scan_web_src(&web_src, &mut violations);
    }

    // 2. web/server.ts must not have RCE shell execute endpoints
    let server_ts = root.join("web").join("server.ts");
    if server_ts.exists() {
        if let Ok(src) = fs::read_to_string(&server_ts) {
            for (lineno, line) in src.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.contains("/api/engine/execute") {
                    violations.push(format!(
                        "  {}:{}: RCE shell execute endpoint /api/engine/execute in web/server.ts: {}",
                        server_ts.display(),
                        lineno + 1,
                        trimmed
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Tier 3 (Web UI) boundary violations detected (ADR 0002 / GW-4):\n{}\n\
         UI tier owns rendering only; no direct LLM SDKs, no direct Rust calls, and no server execute endpoints.",
        violations.join("\n")
    );
}

#[test]
fn test_desktop_tier_boundaries() {
    let root = workspace_root();
    let main_rs = root.join("apps").join("tauri").join("src").join("main.rs");
    if !main_rs.exists() {
        return;
    }

    let src = fs::read_to_string(&main_rs).expect("Failed to read apps/tauri/src/main.rs");
    let mut violations = Vec::new();

    for (lineno, line) in src.lines().enumerate() {
        let trimmed = line.trim();

        // 1. Forbid execute_terminal_command in Tauri (ODR-07, PR-04)
        if trimmed.contains("fn execute_terminal_command")
            || (trimmed.contains("execute_terminal_command") && trimmed.contains("invoke_handler!"))
        {
            violations.push(format!(
                "  apps/tauri/src/main.rs:{}: Direct execute_terminal_command forbidden in Desktop shell: {}",
                lineno + 1,
                trimmed
            ));
        }

        // 2. Forbid direct Rust gRPC :50052 connections from desktop shell
        if trimmed.contains(":50052") {
            violations.push(format!(
                "  apps/tauri/src/main.rs:{}: Direct Rust gRPC connection (:50052) forbidden in Desktop shell: {}",
                lineno + 1,
                trimmed
            ));
        }

        // 3. Forbid hardcoded fake telemetry metrics (ADR 0003, U-07)
        if trimmed.contains("gateway_latency_ms: 8") {
            violations.push(format!(
                "  apps/tauri/src/main.rs:{}: Fake telemetry metrics hardcoded in Desktop shell: {}",
                lineno + 1,
                trimmed
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Tier 1 (Desktop Shell) boundary violations detected (ODR-07 / ADR 0003 / GW-4):\n{}\n\
         Tauri shell must forward operations to Rust Gateway / Core and must not host shell execution or fake metrics.",
        violations.join("\n")
    );
}

#[test]
fn test_no_go_sources() {
    let root = workspace_root();
    let mut violations = Vec::new();

    fn scan_for_go(dir: &Path, violations: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();

            if path.is_dir() {
                if name_str == "target" || name_str == ".git" || name_str == "node_modules" {
                    continue;
                }
                scan_for_go(&path, violations);
                continue;
            }

            if name_str.ends_with(".go")
                || name_str == "go.mod"
                || name_str == "go.sum"
                || name_str == "go.work"
            {
                violations.push(path.display().to_string());
            }
        }
    }

    scan_for_go(&root, &mut violations);

    assert!(
        violations.is_empty(),
        "Go source or module files found in repository (ODR-2026-10-29-rust-single-language / BL-090 / SPEC-015):\n{}\n\
         The project is a unified Rust/TS 3-tier codebase; all Go files have been decommissioned.",
        violations.join("\n")
    );
}
