//! Axiom - A terminal-based IDE with AI integration
//!
//! Built with ratatui for TUI, featuring:
//! - File tree navigation
//! - Text editor with syntax highlighting
//! - Integrated terminal with proper PTY resize
//! - AI chat with streaming support

pub mod core;
pub mod events;
pub mod llm;
pub mod panels;
pub mod state;
pub mod terminal;
pub mod ui;

// Re-export commonly used types
pub use core::{AxiomError, Result};
pub use events::{Event, EventBus};
pub use panels::PanelRegistry;
pub use state::AppState;
