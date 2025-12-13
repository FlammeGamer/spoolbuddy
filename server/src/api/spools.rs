use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    db::{Spool, SpoolInput},
    AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_spools).post(create_spool))
        .route("/{id}", get(get_spool).put(update_spool).delete(delete_spool))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Filter by material type
    material: Option<String>,
    /// Filter by brand
    brand: Option<String>,
    /// Search in color_name, brand, material
    search: Option<String>,
}

/// GET /api/spools - List all spools
async fn list_spools(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Spool>>, (StatusCode, String)> {
    let mut sql = String::from("SELECT * FROM spools WHERE 1=1");
    let mut bindings: Vec<String> = Vec::new();

    if let Some(material) = &query.material {
        sql.push_str(" AND material = ?");
        bindings.push(material.clone());
    }

    if let Some(brand) = &query.brand {
        sql.push_str(" AND brand = ?");
        bindings.push(brand.clone());
    }

    if let Some(search) = &query.search {
        sql.push_str(" AND (color_name LIKE ? OR brand LIKE ? OR material LIKE ?)");
        let pattern = format!("%{}%", search);
        bindings.push(pattern.clone());
        bindings.push(pattern.clone());
        bindings.push(pattern);
    }

    sql.push_str(" ORDER BY updated_at DESC");

    // Build query dynamically
    let mut query_builder = sqlx::query_as::<_, Spool>(&sql);
    for binding in &bindings {
        query_builder = query_builder.bind(binding);
    }

    let spools = query_builder
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(spools))
}

/// GET /api/spools/:id - Get a single spool
async fn get_spool(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Spool>, (StatusCode, String)> {
    let spool = sqlx::query_as::<_, Spool>("SELECT * FROM spools WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match spool {
        Some(s) => Ok(Json(s)),
        None => Err((StatusCode::NOT_FOUND, format!("Spool {} not found", id))),
    }
}

/// POST /api/spools - Create a new spool
async fn create_spool(
    State(state): State<Arc<AppState>>,
    Json(input): Json<SpoolInput>,
) -> Result<(StatusCode, Json<Spool>), (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT INTO spools (
            id, tag_id, material, subtype, color_name, rgba, brand,
            label_weight, core_weight, weight_new, weight_current,
            slicer_filament, note, data_origin, tag_type,
            added_time, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&input.tag_id)
    .bind(&input.material)
    .bind(&input.subtype)
    .bind(&input.color_name)
    .bind(&input.rgba)
    .bind(&input.brand)
    .bind(input.label_weight)
    .bind(input.core_weight)
    .bind(input.weight_new)
    .bind(input.weight_current)
    .bind(&input.slicer_filament)
    .bind(&input.note)
    .bind(&input.data_origin)
    .bind(&input.tag_type)
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Fetch the created spool
    let spool = sqlx::query_as::<_, Spool>("SELECT * FROM spools WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Broadcast update to UI clients
    let _ = state.ui_broadcast.send(serde_json::json!({
        "type": "spool_created",
        "spool": spool
    }).to_string());

    Ok((StatusCode::CREATED, Json(spool)))
}

/// PUT /api/spools/:id - Update a spool
async fn update_spool(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<SpoolInput>,
) -> Result<Json<Spool>, (StatusCode, String)> {
    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query(
        r#"
        UPDATE spools SET
            tag_id = ?, material = ?, subtype = ?, color_name = ?, rgba = ?,
            brand = ?, label_weight = ?, core_weight = ?, weight_new = ?,
            weight_current = ?, slicer_filament = ?, note = ?,
            data_origin = ?, tag_type = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&input.tag_id)
    .bind(&input.material)
    .bind(&input.subtype)
    .bind(&input.color_name)
    .bind(&input.rgba)
    .bind(&input.brand)
    .bind(input.label_weight)
    .bind(input.core_weight)
    .bind(input.weight_new)
    .bind(input.weight_current)
    .bind(&input.slicer_filament)
    .bind(&input.note)
    .bind(&input.data_origin)
    .bind(&input.tag_type)
    .bind(now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, format!("Spool {} not found", id)));
    }

    // Fetch updated spool
    let spool = sqlx::query_as::<_, Spool>("SELECT * FROM spools WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Broadcast update
    let _ = state.ui_broadcast.send(serde_json::json!({
        "type": "spool_updated",
        "spool": spool
    }).to_string());

    Ok(Json(spool))
}

/// DELETE /api/spools/:id - Delete a spool
async fn delete_spool(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM spools WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, format!("Spool {} not found", id)));
    }

    // Broadcast deletion
    let _ = state.ui_broadcast.send(serde_json::json!({
        "type": "spool_deleted",
        "id": id
    }).to_string());

    Ok(StatusCode::NO_CONTENT)
}
