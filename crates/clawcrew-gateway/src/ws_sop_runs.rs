//! Live SOP-runs WebSocket: pushes run summaries as the engine transitions.
//!
//! - `WS /ws/sops/runs`: initial snapshot then a live run-change feed.
//!
//! The snapshot and every subsequent frame come from the engine directly (its
//! in-memory active set plus retained terminal runs and its run-change
//! broadcast), never from polling. The engine is the source of truth; this
//! handler is a thin bridge from its `subscribe_run_changes` feed to the
//! browser.

use super::AppState;
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

const WS_PROTOCOL: &str = "clawcrew.v1";

/// WS /ws/sops/runs, real-time SOP run summaries.
pub async fn handle_ws_sop_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let token = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|auth| auth.strip_prefix("Bearer "))
            .or_else(|| {
                headers
                    .get("sec-websocket-protocol")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|protos| {
                        protos
                            .split(',')
                            .map(|p| p.trim())
                            .find_map(|p| p.strip_prefix("bearer."))
                    })
            })
            .unwrap_or("");

        if !state.pairing.is_authenticated(token) {
            return (
                StatusCode::UNAUTHORIZED,
                "Unauthorized: provide Authorization header or Sec-WebSocket-Protocol bearer",
            )
                .into_response();
        }
    }

    let ws = if headers
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|protos| protos.split(',').any(|p| p.trim() == WS_PROTOCOL))
    {
        ws.protocols([WS_PROTOCOL])
    } else {
        ws
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state))
        .into_response()
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // The SOP subsystem may be disabled; tell the client and close.
    let Some(runner) = state.sop_runner.as_ref() else {
        let msg = serde_json::json!({ "type": "disabled" });
        let _ = sender.send(Message::Text(msg.to_string().into())).await;
        return;
    };

    let (snapshot, rx) = match runner.subscribe_run_changes() {
        Ok(v) => v,
        Err(_) => {
            let msg = serde_json::json!({ "type": "error", "error": "engine lock poisoned" });
            let _ = sender.send(Message::Text(msg.to_string().into())).await;
            return;
        }
    };

    let msg = serde_json::json!({ "type": "snapshot", "runs": snapshot });
    if sender
        .send(Message::Text(msg.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    // No notifier attached (headless embedder): the snapshot stands; keep the
    // socket open until the client leaves rather than closing abruptly.
    let Some(mut rx) = rx else {
        while let Some(m) = receiver.next().await {
            match m {
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        return;
    };

    let send_task = clawcrew_spawn::spawn!(async move {
        loop {
            match rx.recv().await {
                Ok(run) => {
                    let msg = serde_json::json!({ "type": "run", "run": run });
                    if sender
                        .send(Message::Text(msg.to_string().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    let msg = serde_json::json!({ "type": "lagged", "missed": n });
                    let _ = sender.send(Message::Text(msg.to_string().into())).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    while let Some(m) = receiver.next().await {
        match m {
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    send_task.abort();
}
