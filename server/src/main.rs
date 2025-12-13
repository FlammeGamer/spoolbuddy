mod api;
mod config;
mod db;
mod websocket;

use std::sync::Arc;

use axum::Router;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::websocket::DeviceState;

/// Shared application state
pub struct AppState {
    pub db: SqlitePool,
    pub config: Config,
    pub device_state: DeviceState,
    /// Broadcast channel for UI updates
    pub ui_broadcast: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spoolstation_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env();

    // Connect to database
    let db = db::connect(&config.database_url).await?;

    // Run migrations
    db::migrate(&db).await?;

    // Create broadcast channel for UI updates
    let (ui_broadcast, _) = broadcast::channel(100);

    // Create shared state
    let state = Arc::new(AppState {
        db,
        config: config.clone(),
        device_state: DeviceState::new(),
        ui_broadcast,
    });

    // Build router
    let app = Router::new()
        .nest("/api", api::router())
        .nest("/ws", websocket::router())
        .fallback_service(ServeDir::new(&config.static_dir))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
    tracing::info!("SpoolStation server listening on {}", config.bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}
