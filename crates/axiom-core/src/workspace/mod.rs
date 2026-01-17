//! Workspace management module
//!
//! Provides multi-workspace support for Axiom, allowing users to manage
//! multiple projects with per-workspace configurations.
//!
//! # Architecture
//!
//! ```text
//! WorkspaceManager
//!     │
//!     ├── WorkspaceRegistry (persistent)
//!     │   └── HashMap<WorkspaceId, Workspace>
//!     │
//!     ├── WorkspaceStorage (disk I/O)
//!     │   ├── ~/.axiom/workspaces.json (global registry)
//!     │   └── <workspace>/.axiom/config.toml (per-workspace)
//!     │
//!     └── Services (lazy-loaded)
//!         └── HashMap<WorkspaceId, Arc<AxiomService>>
//! ```
//!
//! # Example
//!
//! ```ignore
//! use axiom_core::{AxiomConfig, WorkspaceManager};
//!
//! // Create manager with global config
//! let manager = WorkspaceManager::new(AxiomConfig::default())?;
//!
//! // Create a workspace
//! let ws = manager.create_workspace("my-project", "/path/to/project")?;
//!
//! // Activate it (creates AxiomService)
//! let service = manager.activate_workspace(ws.id)?;
//!
//! // Use the service
//! service.send(Command::ProcessInput { text: "hello".into() })?;
//! ```

mod manager;
mod storage;
mod types;

pub use manager::WorkspaceManager;
pub use storage::{WorkspaceRegistry, WorkspaceStorage};
pub use types::{
    Workspace, WorkspaceCliAgent, WorkspaceConfig, WorkspaceId, WorkspaceLlmConfig,
    WorkspaceProviderConfig, WorkspaceType, WorkspaceView,
};
