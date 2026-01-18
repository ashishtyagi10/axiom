//! Axiom Server Library
//!
//! Provides the HTTP/WebSocket server for Axiom's web interface.

pub mod routes;
pub mod state;

use axum::{
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use axiom_core::WorkspaceManager;
use std::net::SocketAddr;
use std::sync::Once;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub use state::AppState;

static TRACING_INIT: Once = Once::new();

/// Initialize tracing subscriber (only once)
fn init_tracing() {
    TRACING_INIT.call_once(|| {
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG")
                    .unwrap_or_else(|_| "axiom_server=debug,tower_http=debug".into()),
            ))
            .with(tracing_subscriber::fmt::layer())
            .init();
    });
}

/// Build the Axum router with all routes
fn build_router(state: AppState) -> Router {
    Router::new()
        // API routes
        .route("/api/health", get(health_check))
        .route("/api/workspaces", get(routes::list_workspaces))
        .route("/api/workspaces", axum::routing::post(routes::create_workspace))
        .route("/api/workspaces/:id", get(routes::get_workspace))
        .route(
            "/api/workspaces/:id",
            axum::routing::delete(routes::delete_workspace),
        )
        .route(
            "/api/workspaces/:id/activate",
            axum::routing::post(routes::activate_workspace),
        )
        .route("/api/workspaces/:id/files", get(routes::list_files))
        .route("/api/workspaces/:id/file", get(routes::read_file))
        .route(
            "/api/workspaces/:id/file",
            axum::routing::put(routes::write_file),
        )
        .route(
            "/api/workspaces/:id/command",
            axum::routing::post(routes::run_command),
        )
        .route("/api/workspaces/:id/ws", get(routes::websocket_handler))
        // Orchestration routes
        .route(
            "/api/workspaces/:id/orchestrate",
            axum::routing::post(routes::orchestrate),
        )
        .route(
            "/api/workspaces/:id/agents/developer",
            axum::routing::post(routes::run_developer),
        )
        .route(
            "/api/workspaces/:id/llm/settings",
            get(routes::get_llm_settings),
        )
        .route(
            "/api/workspaces/:id/llm/providers/:provider_id",
            axum::routing::put(routes::update_provider),
        )
        .route(
            "/api/workspaces/:id/llm/mappings/:agent_id",
            axum::routing::put(routes::update_agent_mapping),
        )
        // Slash command route
        .route(
            "/api/workspaces/:id/slash",
            axum::routing::post(routes::execute_slash_command),
        )
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
        .with_state(state)
}

/// Run the Axiom web server on the specified port
pub async fn run_server(port: u16) -> anyhow::Result<()> {
    // Initialize tracing (if not already done)
    init_tracing();

    tracing::info!("Starting Axiom Server...");

    // Load configuration
    let cwd = std::env::current_dir()?;
    let config = axiom_core::config::load_config(&cwd).unwrap_or_default();

    // Create workspace manager
    let workspace_manager = WorkspaceManager::new(config.clone())?;

    // Create app state
    let state = AppState::new(config, workspace_manager);

    // Build router
    let app = build_router(state);

    // Bind and serve
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
    Html(
        r#"
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
    "#,
    )
}
