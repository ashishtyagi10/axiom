//! API route handlers

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axiom_core::{Command, Notification, WorkspaceId, WorkspaceView};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::state::AppState;

// ========== Workspace Routes ==========

/// List all workspaces
pub async fn list_workspaces(State(state): State<AppState>) -> impl IntoResponse {
    let manager = state.workspace_manager.read().await;
    let workspaces = manager.list_workspaces();
    let active_id = manager.active_workspace_id();

    Json(serde_json::json!({
        "workspaces": workspaces,
        "active_id": active_id
    }))
}

#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    name: String,
    path: String,
}

/// Create a new workspace
pub async fn create_workspace(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> impl IntoResponse {
    let manager = state.workspace_manager.read().await;

    match manager.create_workspace(&req.name, PathBuf::from(&req.path)) {
        Ok(workspace) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "workspace": {
                    "id": workspace.id,
                    "name": workspace.name,
                    "path": workspace.path
                }
            })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })),
        ),
    }
}

/// Get workspace by ID
pub async fn get_workspace(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid workspace ID" })),
            )
        }
    };

    let manager = state.workspace_manager.read().await;

    match manager.get_workspace(workspace_id) {
        Some(workspace) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "workspace": {
                    "id": workspace.id,
                    "name": workspace.name,
                    "path": workspace.path,
                    "workspace_type": workspace.workspace_type,
                    "is_active": workspace.is_active,
                    "created_at": workspace.created_at,
                    "last_accessed": workspace.last_accessed
                }
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Workspace not found" })),
        ),
    }
}

/// Activate a workspace
pub async fn activate_workspace(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid workspace ID" })),
            )
        }
    };

    let manager = state.workspace_manager.read().await;

    match manager.activate_workspace(workspace_id) {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

/// Delete a workspace
pub async fn delete_workspace(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "success": false, "error": "Invalid workspace ID" })),
            )
        }
    };

    let manager = state.workspace_manager.read().await;

    match manager.delete_workspace(workspace_id) {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

// ========== File Routes ==========

#[derive(Deserialize)]
pub struct ListFilesQuery {
    path: Option<String>,
    #[serde(default)]
    include_hidden: bool,
}

/// List files in a workspace
pub async fn list_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ListFilesQuery>,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid workspace ID" })),
            )
        }
    };

    let manager = state.workspace_manager.read().await;

    let workspace = match manager.get_workspace(workspace_id) {
        Some(ws) => ws,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Workspace not found" })),
            )
        }
    };

    let base_path = workspace.path.clone();
    let target_path = match &query.path {
        Some(p) => base_path.join(p),
        None => base_path,
    };

    let mut entries = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(&target_path) {
        for entry in read_dir.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files if not requested
            if !query.include_hidden && file_name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata().ok();
            let is_directory = metadata.as_ref().map_or(false, |m| m.is_dir());
            let size = metadata.as_ref().map_or(0, |m| m.len());

            entries.push(serde_json::json!({
                "name": file_name,
                "path": entry.path().strip_prefix(&workspace.path).unwrap_or(entry.path().as_path()),
                "is_directory": is_directory,
                "size": size
            }));
        }
    }

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a["is_directory"].as_bool().unwrap_or(false);
        let b_is_dir = b["is_directory"].as_bool().unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_name = a["name"].as_str().unwrap_or("");
                let b_name = b["name"].as_str().unwrap_or("");
                a_name.to_lowercase().cmp(&b_name.to_lowercase())
            }
        }
    });

    (StatusCode::OK, Json(serde_json::json!({ "entries": entries })))
}

#[derive(Deserialize)]
pub struct ReadFileQuery {
    path: String,
}

/// Read a file from a workspace
pub async fn read_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ReadFileQuery>,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid workspace ID" })),
            )
        }
    };

    let manager = state.workspace_manager.read().await;

    let workspace = match manager.get_workspace(workspace_id) {
        Some(ws) => ws,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Workspace not found" })),
            )
        }
    };

    let file_path = workspace.path.join(&query.path);

    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => (
            StatusCode::OK,
            Json(serde_json::json!({ "content": content })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to read file: {}", e) })),
        ),
    }
}

#[derive(Deserialize)]
pub struct WriteFileRequest {
    path: String,
    content: String,
}

/// Write a file to a workspace
pub async fn write_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<WriteFileRequest>,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "success": false, "error": "Invalid workspace ID" })),
            )
        }
    };

    let manager = state.workspace_manager.read().await;

    let workspace = match manager.get_workspace(workspace_id) {
        Some(ws) => ws,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "success": false, "error": "Workspace not found" })),
            )
        }
    };

    let file_path = workspace.path.join(&req.path);

    // Ensure parent directory exists
    if let Some(parent) = file_path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "success": false, "error": format!("Failed to create directory: {}", e) })),
            );
        }
    }

    match tokio::fs::write(&file_path, &req.content).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "error": format!("Failed to write file: {}", e) })),
        ),
    }
}

#[derive(Deserialize)]
pub struct RunCommandRequest {
    command: String,
}

/// Run a command in a workspace
pub async fn run_command(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RunCommandRequest>,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "stdout": "",
                    "stderr": "Invalid workspace ID",
                    "exit_code": 1
                })),
            )
        }
    };

    let manager = state.workspace_manager.read().await;

    let workspace = match manager.get_workspace(workspace_id) {
        Some(ws) => ws,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "stdout": "",
                    "stderr": "Workspace not found",
                    "exit_code": 1
                })),
            )
        }
    };

    // Execute command using shell
    let output = TokioCommand::new("sh")
        .arg("-c")
        .arg(&req.command)
        .current_dir(&workspace.path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(1);

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": exit_code
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "stdout": "",
                "stderr": format!("Failed to execute command: {}", e),
                "exit_code": 1
            })),
        ),
    }
}

// ========== WebSocket Handler ==========

/// WebSocket handler for real-time communication
pub async fn websocket_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid workspace ID").into_response();
        }
    };

    ws.on_upgrade(move |socket| handle_websocket(socket, state, workspace_id))
}

async fn handle_websocket(socket: WebSocket, state: AppState, workspace_id: WorkspaceId) {
    let (mut sender, mut receiver) = socket.split();

    tracing::info!("WebSocket connected for workspace: {}", workspace_id);

    // Get or create service for this workspace
    let service = {
        let manager = state.workspace_manager.read().await;
        match manager.get_or_create_service(workspace_id) {
            Ok(s) => s,
            Err(e) => {
                let _ = sender
                    .send(Message::Text(
                        serde_json::json!({
                            "type": "Error",
                            "message": e.to_string()
                        })
                        .to_string(),
                    ))
                    .await;
                return;
            }
        }
    };

    // Send initial state
    let _ = sender
        .send(Message::Text(
            serde_json::json!({
                "type": "Connected",
                "workspace_id": workspace_id.to_string(),
                "cwd": service.cwd()
            })
            .to_string(),
        ))
        .await;

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                tracing::debug!("Received: {}", text);

                // Parse command
                match serde_json::from_str::<Command>(&text) {
                    Ok(command) => {
                        // Note: In a real implementation, we'd need mutable access to service
                        // For now, just echo back
                        let response = serde_json::json!({
                            "type": "CommandReceived",
                            "command": text
                        });
                        let _ = sender.send(Message::Text(response.to_string())).await;
                    }
                    Err(e) => {
                        let error = serde_json::json!({
                            "type": "Error",
                            "message": format!("Invalid command: {}", e)
                        });
                        let _ = sender.send(Message::Text(error.to_string())).await;
                    }
                }
            }
            Ok(Message::Close(_)) => {
                tracing::info!("WebSocket closed for workspace: {}", workspace_id);
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

// ========== Orchestration Routes ==========

#[derive(Deserialize)]
pub struct OrchestrationRequest {
    messages: Vec<OrchestrationMessage>,
}

#[derive(Deserialize)]
pub struct OrchestrationMessage {
    role: String,
    content: String,
}

/// Run the orchestrator to decide next action
pub async fn orchestrate(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<OrchestrationRequest>,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid workspace ID" })),
            )
        }
    };

    let manager = state.workspace_manager.read().await;

    let workspace = match manager.get_workspace(workspace_id) {
        Some(ws) => ws,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Workspace not found" })),
            )
        }
    };

    // Convert messages to orchestration ChatMessage format
    let chat_messages: Vec<axiom_core::orchestration::ChatMessage> = req
        .messages
        .iter()
        .map(|m| match m.role.as_str() {
            "system" => axiom_core::orchestration::ChatMessage::system(&m.content),
            "assistant" => axiom_core::orchestration::ChatMessage::assistant(&m.content),
            _ => axiom_core::orchestration::ChatMessage::user(&m.content),
        })
        .collect();

    // Create orchestration service with shared config settings
    let llm_settings = axiom_core::LlmSettings::from_axiom_config(&state.config);
    let service = axiom_core::OrchestrationService::with_settings(workspace.path.clone(), llm_settings);

    match service.orchestrate(&chat_messages) {
        Ok(decision) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "next_agent": format!("{:?}", decision.next_agent).to_lowercase(),
                "reasoning": decision.reasoning,
                "task": decision.task
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

#[derive(Deserialize)]
pub struct DeveloperRequest {
    task: String,
}

/// Run the developer agent
pub async fn run_developer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<DeveloperRequest>,
) -> impl IntoResponse {
    let workspace_id: WorkspaceId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid workspace ID" })),
            )
        }
    };

    let manager = state.workspace_manager.read().await;

    let workspace = match manager.get_workspace(workspace_id) {
        Some(ws) => ws,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Workspace not found" })),
            )
        }
    };

    // Create orchestration service with shared config settings
    let llm_settings = axiom_core::LlmSettings::from_axiom_config(&state.config);
    let service = axiom_core::OrchestrationService::with_settings(workspace.path.clone(), llm_settings);

    match service.run_developer(&req.task) {
        Ok(response) => {
            // Execute write operations immediately
            let operations: Vec<serde_json::Value> = response
                .operations
                .iter()
                .map(|op| match op {
                    axiom_core::orchestration::AgentOperation::Write { path, content } => {
                        // Execute write
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let result = std::fs::write(path, content);
                        serde_json::json!({
                            "type": "write",
                            "path": path.to_string_lossy(),
                            "success": result.is_ok(),
                            "error": result.err().map(|e| e.to_string())
                        })
                    }
                    axiom_core::orchestration::AgentOperation::Delete { path } => {
                        let result = std::fs::remove_file(path);
                        serde_json::json!({
                            "type": "delete",
                            "path": path.to_string_lossy(),
                            "success": result.is_ok(),
                            "error": result.err().map(|e| e.to_string())
                        })
                    }
                    axiom_core::orchestration::AgentOperation::Execute { command } => {
                        serde_json::json!({
                            "type": "execute",
                            "command": command,
                            "note": "Execute operations returned to client"
                        })
                    }
                })
                .collect();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "reasoning": response.reasoning,
                    "operations": operations,
                    "message": response.message
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

/// Get LLM settings
pub async fn get_llm_settings(
    State(state): State<AppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // Use shared config settings (same as TUI)
    let settings = axiom_core::LlmSettings::from_axiom_config(&state.config);

    let providers: Vec<serde_json::Value> = settings
        .providers
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "base_url": p.base_url,
                "default_model": p.default_model,
                "enabled": p.enabled,
                // Don't expose API keys
                "has_api_key": !p.api_key.is_empty()
            })
        })
        .collect();

    let mappings: Vec<serde_json::Value> = settings
        .agent_mappings
        .iter()
        .map(|m| {
            serde_json::json!({
                "agent_id": format!("{:?}", m.agent_id).to_lowercase(),
                "provider_id": m.provider_id,
                "model_id": m.model_id
            })
        })
        .collect();

    Json(serde_json::json!({
        "providers": providers,
        "agent_mappings": mappings
    }))
}

#[derive(Deserialize)]
pub struct UpdateProviderRequest {
    api_key: Option<String>,
    base_url: Option<String>,
    default_model: Option<String>,
    enabled: Option<bool>,
}

/// Update a provider configuration
pub async fn update_provider(
    State(_state): State<AppState>,
    Path((_workspace_id, _provider_id)): Path<(String, String)>,
    Json(_req): Json<UpdateProviderRequest>,
) -> impl IntoResponse {
    // In a full implementation, this would persist the settings
    // For now, return success as a placeholder
    Json(serde_json::json!({
        "success": true,
        "message": "Provider settings update not yet implemented - settings are in-memory only"
    }))
}

#[derive(Deserialize)]
pub struct UpdateMappingRequest {
    provider_id: String,
    model_id: String,
}

/// Update an agent mapping
pub async fn update_agent_mapping(
    State(_state): State<AppState>,
    Path((_workspace_id, _agent_id)): Path<(String, String)>,
    Json(_req): Json<UpdateMappingRequest>,
) -> impl IntoResponse {
    // In a full implementation, this would persist the settings
    Json(serde_json::json!({
        "success": true,
        "message": "Agent mapping update not yet implemented - settings are in-memory only"
    }))
}
