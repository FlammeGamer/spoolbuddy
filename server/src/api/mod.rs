mod spools;
mod printers;
pub mod device;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

/// Build the API router
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/spools", spools::router())
        .nest("/printers", printers::router())
        .nest("/device", device::router())
}
