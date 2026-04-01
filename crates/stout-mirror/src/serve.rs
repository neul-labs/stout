//! Mirror server functionality

use crate::error::{Error, Result};
use crate::manifest::MirrorManifest;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing::{info, warn};

/// Configuration for serving a mirror
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Path to the mirror directory
    pub mirror_path: PathBuf,

    /// Port to listen on
    pub port: u16,

    /// Address to bind to
    pub bind: String,

    /// Enable access logging
    pub log_access: bool,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            mirror_path: PathBuf::from("./mirror"),
            port: 8080,
            bind: "0.0.0.0".to_string(),
            log_access: false,
        }
    }
}

/// Shared state for the server
struct ServerState {
    manifest: MirrorManifest,
    #[allow(dead_code)]
    mirror_path: PathBuf,
}

/// Serve a mirror via HTTP
pub async fn serve_mirror(config: ServeConfig) -> Result<()> {
    // Validate mirror
    let manifest_path = config.mirror_path.join("manifest.json");
    if !manifest_path.exists() {
        return Err(Error::InvalidMirror);
    }

    let manifest = MirrorManifest::load(&manifest_path)?;

    info!(
        "Serving mirror with {} formulas, {} casks",
        manifest.formulas.count, manifest.casks.count
    );

    let state = Arc::new(ServerState {
        manifest,
        mirror_path: config.mirror_path.clone(),
    });

    // Build router
    let app = Router::new()
        // API endpoints
        .route("/api/v1/manifest", get(get_manifest))
        .route("/api/v1/health", get(health_check))
        // Static file serving for the mirror content
        .nest_service("/", ServeDir::new(&config.mirror_path))
        .with_state(state);

    // Parse bind address
    let addr: SocketAddr = format!("{}:{}", config.bind, config.port)
        .parse()
        .map_err(|e| Error::Server(format!("Invalid bind address: {}", e)))?;

    info!("Starting mirror server at http://{}", addr);
    info!("  Mirror path: {:?}", config.mirror_path);
    info!("  Press Ctrl+C to stop");

    // Start server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| Error::Server(format!("Failed to bind: {}", e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| Error::Server(format!("Server error: {}", e)))?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    "OK"
}

/// Get manifest endpoint
async fn get_manifest(State(state): State<Arc<ServerState>>) -> Response {
    match serde_json::to_string_pretty(&state.manifest) {
        Ok(json) => (StatusCode::OK, [("content-type", "application/json")], json).into_response(),
        Err(e) => {
            warn!("Failed to serialize manifest: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response()
        }
    }
}
