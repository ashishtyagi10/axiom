//! Application state management
//!
//! Contains the central state container and input mode state machine.

mod app;
mod context;
mod focus;
mod input_mode;

pub use app::AppState;
pub use context::{AgentId, OutputContext};
pub use focus::{FocusState, PanelId};
pub use input_mode::InputMode;

// Re-export workspace types from axiom-core
pub use axiom_core::{Workspace, WorkspaceId, WorkspaceManager, WorkspaceType, WorkspaceView};
