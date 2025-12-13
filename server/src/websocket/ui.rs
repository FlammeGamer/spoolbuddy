use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};

use crate::AppState;

/// WebSocket endpoint for UI clients (browser, tablet, device display)
pub async fn ui_ws(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(|socket| handle_ui_socket(socket, state))
}

async fn handle_ui_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channel
    let mut rx = state.ui_broadcast.subscribe();

    tracing::debug!("UI client connected");

    // Send initial state
    let device_state = state.device_state.read().await;
    let initial_state = serde_json::json!({
        "type": "initial_state",
        "device": {
            "connected": device_state.connected,
            "last_weight": device_state.last_weight,
            "weight_stable": device_state.weight_stable,
            "current_tag_id": device_state.current_tag_id
        }
    });
    drop(device_state);

    if sender
        .send(Message::Text(initial_state.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    // Task to forward broadcasts to this client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Keep connection alive, handle pings
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Ping(data) => {
                // Pong is handled automatically by axum
                tracing::trace!("Ping received: {:?}", data);
            }
            Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }

    send_task.abort();
    tracing::debug!("UI client disconnected");
}
