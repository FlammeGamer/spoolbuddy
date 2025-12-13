use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{api::device::DeviceCommand, AppState};

/// WebSocket endpoint for device connection
pub async fn device_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_device_socket(socket, state))
}

/// Messages from device to server
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeviceMessage {
    /// NFC tag detected
    TagDetected {
        tag_id: String,
        tag_type: String,
        data: Option<serde_json::Value>,
    },
    /// NFC tag removed
    TagRemoved,
    /// Weight update from scale
    Weight { grams: f64, stable: bool },
    /// Heartbeat
    Heartbeat { uptime: u64 },
    /// Command response
    CommandResult {
        request_id: String,
        success: bool,
        error: Option<String>,
    },
}

/// Messages from server to device
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Write tag command
    WriteTag {
        request_id: String,
        data: serde_json::Value,
    },
    /// Tare scale command
    TareScale,
    /// Calibrate scale command
    CalibrateScale { known_weight: f64 },
    /// Notification to show on device
    Notification { message: String, duration: u32 },
}

async fn handle_device_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Create command channel
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<DeviceCommand>(32);

    // Mark device as connected
    {
        let mut device_state = state.device_state.write().await;
        device_state.connected = true;
        device_state.command_tx = Some(cmd_tx);
    }

    tracing::info!("Device connected");

    // Broadcast device connected to UI
    let _ = state.ui_broadcast.send(
        serde_json::json!({
            "type": "device_connected"
        })
        .to_string(),
    );

    // Task for sending commands to device
    let state_clone = state.clone();
    let send_task = tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let msg = match cmd {
                DeviceCommand::TareScale => ServerMessage::TareScale,
                DeviceCommand::CalibrateScale { known_weight } => {
                    ServerMessage::CalibrateScale { known_weight }
                }
                DeviceCommand::WriteTag { spool } => ServerMessage::WriteTag {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    data: serde_json::to_value(&spool).unwrap_or_default(),
                },
            };

            let json = serde_json::to_string(&msg).unwrap_or_default();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Process incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<DeviceMessage>(&text) {
                Ok(device_msg) => {
                    handle_device_message(&state, device_msg).await;
                }
                Err(e) => {
                    tracing::warn!("Invalid device message: {}", e);
                }
            }
        }
    }

    // Cleanup on disconnect
    send_task.abort();

    {
        let mut device_state = state.device_state.write().await;
        device_state.connected = false;
        device_state.command_tx = None;
        device_state.current_tag_id = None;
    }

    tracing::info!("Device disconnected");

    // Broadcast device disconnected to UI
    let _ = state.ui_broadcast.send(
        serde_json::json!({
            "type": "device_disconnected"
        })
        .to_string(),
    );
}

async fn handle_device_message(state: &Arc<AppState>, msg: DeviceMessage) {
    match msg {
        DeviceMessage::TagDetected { tag_id, tag_type, data } => {
            tracing::info!("Tag detected: {} ({})", tag_id, tag_type);

            // Update device state
            {
                let mut device_state = state.device_state.write().await;
                device_state.current_tag_id = Some(tag_id.clone());
            }

            // Lookup spool by tag_id
            let spool = sqlx::query_as::<_, crate::db::Spool>(
                "SELECT * FROM spools WHERE tag_id = ?",
            )
            .bind(&tag_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();

            // Broadcast to UI
            let _ = state.ui_broadcast.send(
                serde_json::json!({
                    "type": "tag_detected",
                    "tag_id": tag_id,
                    "tag_type": tag_type,
                    "spool": spool,
                    "raw_data": data
                })
                .to_string(),
            );
        }

        DeviceMessage::TagRemoved => {
            tracing::info!("Tag removed");

            {
                let mut device_state = state.device_state.write().await;
                device_state.current_tag_id = None;
            }

            let _ = state.ui_broadcast.send(
                serde_json::json!({
                    "type": "tag_removed"
                })
                .to_string(),
            );
        }

        DeviceMessage::Weight { grams, stable } => {
            {
                let mut device_state = state.device_state.write().await;
                device_state.last_weight = Some(grams);
                device_state.weight_stable = stable;
            }

            let _ = state.ui_broadcast.send(
                serde_json::json!({
                    "type": "weight",
                    "grams": grams,
                    "stable": stable
                })
                .to_string(),
            );
        }

        DeviceMessage::Heartbeat { uptime } => {
            tracing::debug!("Device heartbeat: {}s uptime", uptime);
        }

        DeviceMessage::CommandResult { request_id, success, error } => {
            let _ = state.ui_broadcast.send(
                serde_json::json!({
                    "type": "command_result",
                    "request_id": request_id,
                    "success": success,
                    "error": error
                })
                .to_string(),
            );
        }
    }
}
