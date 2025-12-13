mod device;
mod ui;

use std::sync::Arc;

use axum::Router;
use tokio::sync::{mpsc, RwLock};

use crate::{api::device::DeviceCommand, AppState};

pub use device::device_ws;
pub use ui::ui_ws;

/// Build the WebSocket router
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/device", axum::routing::get(device_ws))
        .route("/ui", axum::routing::get(ui_ws))
}

/// State for the connected device
pub struct DeviceStateInner {
    pub connected: bool,
    pub last_weight: Option<f64>,
    pub weight_stable: bool,
    pub current_tag_id: Option<String>,
    pub command_tx: Option<mpsc::Sender<DeviceCommand>>,
}

impl Default for DeviceStateInner {
    fn default() -> Self {
        Self {
            connected: false,
            last_weight: None,
            weight_stable: false,
            current_tag_id: None,
            command_tx: None,
        }
    }
}

/// Thread-safe device state wrapper
pub struct DeviceState(RwLock<DeviceStateInner>);

impl DeviceState {
    pub fn new() -> Self {
        Self(RwLock::new(DeviceStateInner::default()))
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, DeviceStateInner> {
        self.0.read().await
    }

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, DeviceStateInner> {
        self.0.write().await
    }
}
