use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(device_status))
        .route("/tare", post(tare_scale))
        .route("/write-tag", post(write_tag))
}

#[derive(Debug, Serialize)]
pub struct DeviceStatus {
    connected: bool,
    last_weight: Option<f64>,
    weight_stable: bool,
    current_tag_id: Option<String>,
}

/// GET /api/device/status - Get device connection status
async fn device_status(State(state): State<Arc<AppState>>) -> Json<DeviceStatus> {
    let device_state = state.device_state.read().await;
    Json(DeviceStatus {
        connected: device_state.connected,
        last_weight: device_state.last_weight,
        weight_stable: device_state.weight_stable,
        current_tag_id: device_state.current_tag_id.clone(),
    })
}

/// POST /api/device/tare - Tare the scale
async fn tare_scale(State(state): State<Arc<AppState>>) -> Result<StatusCode, (StatusCode, String)> {
    let device_state = state.device_state.read().await;
    if !device_state.connected {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "Device not connected".into()));
    }

    // Send tare command via device channel
    if let Some(tx) = &device_state.command_tx {
        let _ = tx.send(DeviceCommand::TareScale).await;
    }

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct WriteTagRequest {
    pub spool_id: String,
}

/// POST /api/device/write-tag - Write NFC tag
async fn write_tag(
    State(state): State<Arc<AppState>>,
    Json(request): Json<WriteTagRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let device_state = state.device_state.read().await;
    if !device_state.connected {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "Device not connected".into()));
    }

    if device_state.current_tag_id.is_none() {
        return Err((StatusCode::BAD_REQUEST, "No tag present".into()));
    }

    // Fetch spool data
    let spool = sqlx::query_as::<_, crate::db::Spool>("SELECT * FROM spools WHERE id = ?")
        .bind(&request.spool_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let spool = spool.ok_or((StatusCode::NOT_FOUND, "Spool not found".into()))?;

    // Send write command via device channel
    if let Some(tx) = &device_state.command_tx {
        let _ = tx.send(DeviceCommand::WriteTag { spool }).await;
    }

    Ok(StatusCode::OK)
}

/// Commands that can be sent to the device
#[derive(Debug, Clone)]
pub enum DeviceCommand {
    TareScale,
    CalibrateScale { known_weight: f64 },
    WriteTag { spool: crate::db::Spool },
}
