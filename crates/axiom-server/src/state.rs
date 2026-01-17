//! Server state management

use axiom_core::{AxiomConfig, WorkspaceManager};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: AxiomConfig,
    pub workspace_manager: Arc<RwLock<WorkspaceManager>>,
}

impl AppState {
    pub fn new(config: AxiomConfig, workspace_manager: WorkspaceManager) -> Self {
        Self {
            config,
            workspace_manager: Arc::new(RwLock::new(workspace_manager)),
        }
    }
}
