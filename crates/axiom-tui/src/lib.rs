//! Axiom TUI - Terminal User Interface for Axiom
//!
//! This crate provides the Ratatui-based terminal interface.
//! It consumes the axiom-core crate for backend functionality.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────────┐   Command     ┌──────────────────┐
//! │   axiom-tui       │ ─────────────→│   axiom-core     │
//! │   TuiApp          │               │   AxiomService   │
//! │                   │ ←─────────────│                  │
//! └───────────────────┘  Notification └──────────────────┘
//! ```
//!
//! # Modules
//!
//! - `events` - TUI-specific events (keyboard, mouse, resize)
//! - `state` - TUI state (focus, input mode, app state)
//! - (Future) `panels` - Panel implementations
//! - (Future) `ui` - Layout and rendering

pub mod app;
pub mod events;
pub mod panels;
pub mod state;
pub mod ui;

// Re-export key types
pub use app::TuiApp;
pub use events::{TuiEvent, TuiEventBus};
pub use state::{AppState, FocusState, InputMode, PanelId};

// Re-export axiom-core types for convenience
pub use axiom_core::{
    AgentId, AgentStatus, AgentType, AgentView, AxiomConfig, AxiomService, Command, Notification,
    OutputContext, ProviderInfo, Result, TerminalScreen,
};

/// Get the TUI crate version
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
