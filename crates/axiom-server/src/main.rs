//! Axiom Server - HTTP/WebSocket server for Axiom
//!
//! Serves the web UI and provides API endpoints for the frontend.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use axiom_core::{AxiomConfig, Command, Notification, WorkspaceManager};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod state;
mod routes;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "axiom_server=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Axiom Server...");

    // Load configuration
    let cwd = std::env::current_dir()?;
    let config = axiom_core::config::load_config(&cwd).unwrap_or_default();

    // Create workspace manager
    let workspace_manager = WorkspaceManager::new(config.clone())?;

    // Create app state
    let state = AppState::new(config, workspace_manager);

    // Build router
    let app = Router::new()
        // API routes
        .route("/api/health", get(health_check))
        .route("/api/workspaces", get(routes::list_workspaces))
        .route("/api/workspaces", post(routes::create_workspace))
        .route("/api/workspaces/:id", get(routes::get_workspace))
        .route("/api/workspaces/:id", axum::routing::delete(routes::delete_workspace))
        .route("/api/workspaces/:id/activate", post(routes::activate_workspace))
        .route("/api/workspaces/:id/files", get(routes::list_files))
        .route("/api/workspaces/:id/file", get(routes::read_file))
        .route("/api/workspaces/:id/file", axum::routing::put(routes::write_file))
        .route("/api/workspaces/:id/command", post(routes::run_command))
        .route("/api/workspaces/:id/ws", get(routes::websocket_handler))
        // Orchestration routes
        .route("/api/workspaces/:id/orchestrate", post(routes::orchestrate))
        .route("/api/workspaces/:id/agents/developer", post(routes::run_developer))
        .route("/api/workspaces/:id/llm/settings", get(routes::get_llm_settings))
        .route("/api/workspaces/:id/llm/providers/:provider_id", axum::routing::put(routes::update_provider))
        .route("/api/workspaces/:id/llm/mappings/:agent_id", axum::routing::put(routes::update_agent_mapping))
        // Serve static files (UI) - from Next.js build output
        .nest_service("/", ServeDir::new("web/out").fallback(get(index_html)))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Get port from env or default
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on http://{}", addr);
    tracing::info!("WebSocket endpoint: ws://{}/api/workspaces/:id/ws", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": axiom_core::version()
    }))
}

async fn index_html() -> impl IntoResponse {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Axiom</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #1a1a2e;
            color: #eee;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
        }
        .container {
            text-align: center;
            padding: 2rem;
        }
        h1 { color: #00d4ff; }
        .status { color: #4ade80; }
        .endpoints {
            margin-top: 2rem;
            text-align: left;
            background: #16213e;
            padding: 1rem 2rem;
            border-radius: 8px;
        }
        code {
            background: #0f3460;
            padding: 0.2rem 0.5rem;
            border-radius: 4px;
        }
        a { color: #00d4ff; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Axiom Server</h1>
        <p class="status">Server is running</p>
        <div class="endpoints">
            <h3>API Endpoints:</h3>
            <ul>
                <li><code>GET /api/health</code> - Health check</li>
                <li><code>GET /api/workspaces</code> - List workspaces</li>
                <li><code>POST /api/workspaces</code> - Create workspace</li>
                <li><code>GET /api/workspaces/:id</code> - Get workspace</li>
                <li><code>DELETE /api/workspaces/:id</code> - Delete workspace</li>
                <li><code>POST /api/workspaces/:id/activate</code> - Activate workspace</li>
                <li><code>GET /api/workspaces/:id/files</code> - List files</li>
                <li><code>GET /api/workspaces/:id/file?path=...</code> - Read file</li>
                <li><code>PUT /api/workspaces/:id/file</code> - Write file</li>
                <li><code>POST /api/workspaces/:id/command</code> - Run command</li>
                <li><code>WS /api/workspaces/:id/ws</code> - WebSocket stream</li>
            </ul>
        </div>
        <p style="margin-top: 2rem;">
            <a href="/api/health">Check API Health</a>
        </p>
    </div>
</body>
</html>
    "#)
}
