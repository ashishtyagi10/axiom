//! TUI state management
//!
//! Panel-specific state for the terminal UI.
//! Backend state (agents, LLM) is managed by AxiomService.

mod app;
mod focus;
mod input_mode;

pub use app::{AppState, MessageLevel, StatusMessage};
pub use focus::{FocusState, PanelId};
pub use input_mode::InputMode;

// Re-export core types that TUI frequently uses
pub use axiom_core::{AgentId, OutputContext};
