use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::{db::Printer, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_printers))
        .route("/{serial}", get(get_printer))
}

/// GET /api/printers - List all printers
async fn list_printers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Printer>>, (StatusCode, String)> {
    let printers = sqlx::query_as::<_, Printer>("SELECT * FROM printers ORDER BY name")
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(printers))
}

/// GET /api/printers/:serial - Get a single printer
async fn get_printer(
    State(state): State<Arc<AppState>>,
    Path(serial): Path<String>,
) -> Result<Json<Printer>, (StatusCode, String)> {
    let printer = sqlx::query_as::<_, Printer>("SELECT * FROM printers WHERE serial = ?")
        .bind(&serial)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match printer {
        Some(p) => Ok(Json(p)),
        None => Err((StatusCode::NOT_FOUND, format!("Printer {} not found", serial))),
    }
}
