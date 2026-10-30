#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::Value;
use std::collections::HashMap;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

/// Base URL of the Rust HTTP gateway. Override with GALLEON_GATEWAY_URL.
/// SSOT: the gateway is the single source for all fleet-domain data; Tauri
/// commands proxy to it rather than holding a divergent second copy.
fn gateway_url() -> String {
    std::env::var("GALLEON_GATEWAY_URL").unwrap_or_else(|_| "http://127.0.0.1:42617".into())
}

/// Validate path for `api_request` to prevent SSRF and path traversal.
/// Path must start with "/api/" or exactly match "/pair" or "/pair/code",
/// and must not contain "://" or "..".
pub fn validate_path(path: &str) -> Result<(), &'static str> {
    if path.contains("://") || path.contains("..") {
        return Err("path not allowed");
    }
    if path == "/pair" || path == "/pair/code" || path.starts_with("/api/") {
        Ok(())
    } else {
        Err("path not allowed")
    }
}

#[derive(serde::Serialize)]
pub struct ApiResponse {
    pub status: u16,
    pub body: Value,
}

#[tauri::command]
async fn api_request(
    method: String,
    path: String,
    body: Option<Value>,
    token: Option<String>,
    headers: Option<HashMap<String, String>>,
) -> Result<ApiResponse, String> {
    validate_path(&path).map_err(|e| e.to_string())?;

    let url = format!("{}{}", gateway_url(), path);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("gateway unreachable: {e}"))?;

    let req_method = match method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "PATCH" => reqwest::Method::PATCH,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        _ => return Err(format!("unsupported method: {method}")),
    };

    let mut builder = client.request(req_method, &url);

    if let Some(ref t) = token {
        if !t.is_empty() {
            builder = builder.header("Authorization", format!("Bearer {t}"));
        }
    }

    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            builder = builder.header(k, v);
        }
    }

    if let Some(ref b) = body {
        if !b.is_null() {
            builder = builder.json(b);
        }
    }

    let res = builder
        .send()
        .await
        .map_err(|e| format!("gateway unreachable: {e}"))?;

    let status = res.status().as_u16();
    let text = res
        .text()
        .await
        .map_err(|e| format!("invalid gateway response: {e}"))?;

    let body_json: Value = if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or(Value::String(text))
    };

    Ok(ApiResponse {
        status,
        body: body_json,
    })
}

#[derive(Default)]
struct SidecarState {
    child: std::sync::Mutex<Option<std::process::Child>>,
}

#[tauri::command]
async fn get_sidecar_status(state: tauri::State<'_, SidecarState>) -> Result<Value, String> {
    let (sidecar_spawned, sidecar_alive) = {
        let mut lock = state.child.lock().map_err(|e| e.to_string())?;
        let alive = if let Some(ref mut child) = *lock {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        };
        (lock.is_some(), alive)
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let gateway_online = client
        .get(format!("{}/health", gateway_url()))
        .send()
        .await
        .map(|res| res.status().is_success())
        .unwrap_or(false);

    Ok(serde_json::json!({
        "sidecar_spawned": sidecar_spawned,
        "sidecar_alive": sidecar_alive,
        "gateway_online": gateway_online,
        "gateway_url": gateway_url(),
        "engine_online": gateway_online,
        "engine_url": gateway_url(),
    }))
}

fn try_spawn_sidecar() -> Option<std::process::Child> {
    // Look for sidecar binary in the current executable directory or binaries/
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let candidates = [
                parent.join("galleon.exe"),
                parent.join("galleon"),
                parent.join("binaries").join("galleon.exe"),
                parent.join("binaries").join("galleon"),
            ];
            for candidate in &candidates {
                if candidate.exists() {
                    if let Ok(child) = std::process::Command::new(candidate).arg("daemon").spawn() {
                        return Some(child);
                    }
                }
            }
        }
    }
    None
}

fn main() {
    let sidecar_child = try_spawn_sidecar();

    tauri::Builder::default()
        .manage(SidecarState {
            child: std::sync::Mutex::new(sidecar_child),
        })
        .invoke_handler(tauri::generate_handler![api_request, get_sidecar_status])
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit Galleon Fleet", true, None::<&str>)?;
            let show_i =
                MenuItem::with_id(app, "show", "Open Galleon Fleet Deck", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        let state = app.state::<SidecarState>();
                        if let Ok(mut lock) = state.child.lock() {
                            if let Some(mut child) = lock.take() {
                                let _ = child.kill();
                            }
                        }
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Galleon Fleet desktop application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<SidecarState>() {
                    let child = state.child.lock().ok().and_then(|mut g| g.take());
                    if let Some(mut c) = child {
                        let _ = c.kill();
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_allowed() {
        assert!(validate_path("/api/x").is_ok());
        assert!(validate_path("/api/devices").is_ok());
        assert!(validate_path("/api/v1/stream").is_ok());
        assert!(validate_path("/pair").is_ok());
        assert!(validate_path("/pair/code").is_ok());
    }

    #[test]
    fn test_validate_path_rejected() {
        assert_eq!(validate_path("http://evil"), Err("path not allowed"));
        assert_eq!(validate_path("/../"), Err("path not allowed"));
        assert_eq!(validate_path("/admin/shutdown"), Err("path not allowed"));
        assert_eq!(validate_path("/api/../admin"), Err("path not allowed"));
        assert_eq!(
            validate_path("https://evil.com/api/x"),
            Err("path not allowed")
        );
        assert_eq!(
            validate_path("ftp://evil.com/pair"),
            Err("path not allowed")
        );
        assert_eq!(
            validate_path("javascript:alert(1)"),
            Err("path not allowed")
        );
        assert_eq!(validate_path("/pair/other"), Err("path not allowed"));
        assert_eq!(validate_path(""), Err("path not allowed"));
    }
}
