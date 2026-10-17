#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use serde_json::Value;

/// Base URL of the Go engine HTTP gateway. Override with GALLEON_ENGINE_URL.
/// SSOT: the engine is the single source for all fleet-domain data; Tauri
/// commands proxy to it rather than holding a divergent second copy.
fn engine_url() -> String {
    std::env::var("GALLEON_ENGINE_URL").unwrap_or_else(|_| "http://127.0.0.1:9090".into())
}

/// GET <engine>/api/<path> and return the parsed JSON body.
async fn engine_get(path: &str) -> Result<Value, String> {
    let url = format!("{}/api/{}", engine_url(), path);
    let res = reqwest::get(&url).await.map_err(|e| format!("engine unreachable: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("engine {} for /api/{}", res.status(), path));
    }
    res.json::<Value>().await.map_err(|e| format!("invalid engine response: {e}"))
}

/// POST <engine>/api/<path> with an optional JSON body; return the parsed JSON response.
async fn engine_post(path: &str, body: Value) -> Result<Value, String> {
    let url = format!("{}/api/{}", engine_url(), path);
    let res = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("engine unreachable: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("engine {} for /api/{}", res.status(), path));
    }
    res.json::<Value>().await.map_err(|e| format!("invalid engine response: {e}"))
}

// Commands
#[tauri::command]
async fn get_fleet_metrics() -> Result<Value, String> {
    engine_get("fleet/metrics").await
}

#[tauri::command]
async fn ring_deck_bell() -> Result<Value, String> {
    engine_post("fleet/deck-bell", Value::Null).await
}

#[tauri::command]
async fn get_collection(name: String) -> Result<Value, String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("invalid collection name".into());
    }
    engine_get(&format!("collections/{}", name)).await
}

#[tauri::command]
async fn save_collection(name: String, data: Value) -> Result<(), String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("invalid collection name".into());
    }
    engine_post(&format!("collections/{}", name), data).await.map(|_| ())
}

// SSOT: Tauri delegates system metrics to Go engine (/api/system/metrics).
// Gracefully degrades to reporting engine error + local threads if unreachable (ADR 0003).
#[tauri::command]
async fn get_system_metrics() -> Result<Value, String> {
    match engine_get("system/metrics").await {
        Ok(metrics) => Ok(metrics),
        Err(e) => Ok(serde_json::json!({
            "error": "Engine unreachable",
            "message": e,
            "active_threads": std::thread::available_parallelism()
                .map(|n| n.get() as u32 * 2)
                .unwrap_or(16)
        })),
    }
}

#[tauri::command]
async fn get_executive_briefing() -> Result<Value, String> {
    engine_get("system/executive-briefing").await
}

#[tauri::command]
async fn get_harbor_providers() -> Result<Value, String> {
    engine_get("providers/harbor").await
}

#[tauri::command]
async fn get_diagnostics() -> Result<Value, String> {
    engine_get("diagnostics").await
}

#[tauri::command]
async fn apply_remedy() -> Result<Value, String> {
    engine_post("diagnostics/remedy", Value::Null).await
}

#[tauri::command]
async fn get_snapshots() -> Result<Value, String> {
    engine_get("snapshots").await
}

#[tauri::command]
async fn create_snapshot(title: Option<String>) -> Result<Value, String> {
    engine_post("snapshots", serde_json::json!({ "label": title })).await
}

#[tauri::command]
async fn get_engine_processes() -> Result<Value, String> {
    engine_get("engine/processes").await
}

#[tauri::command]
async fn get_fleet_policies() -> Result<Value, String> {
    engine_get("fleet/policies").await
}

#[tauri::command]
async fn chat_quartermaster(message: String, _context: Option<Value>) -> Result<Value, String> {
    engine_post("chat/quartermaster", serde_json::json!({ "message": message })).await
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
            match child.try_wait() {
                Ok(None) => true,
                _ => false,
            }
        } else {
            false
        };
        (lock.is_some(), alive)
    };

    let engine_online = engine_get("health").await.is_ok();

    Ok(serde_json::json!({
        "sidecar_spawned": sidecar_spawned,
        "sidecar_alive": sidecar_alive,
        "engine_online": engine_online,
        "engine_url": engine_url(),
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
                    if let Ok(child) = std::process::Command::new(candidate)
                        .arg("daemon")
                        .spawn()
                    {
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
        .invoke_handler(tauri::generate_handler![
            get_fleet_metrics, 
            ring_deck_bell, 
            get_collection,
            save_collection,
            get_system_metrics,
            get_executive_briefing,
            get_harbor_providers,
            get_diagnostics,
            apply_remedy,
            get_snapshots,
            create_snapshot,
            get_engine_processes,
            get_fleet_policies,
            chat_quartermaster,
            get_sidecar_status
        ])
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit Galleon Fleet", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Open Galleon Fleet Deck", true, None::<&str>)?;
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

