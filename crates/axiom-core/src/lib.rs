//! Axiom Core - Backend library for Axiom
//!
//! This crate provides the UI-agnostic backend functionality:
//! - Agent system (Conductor, Executor, Registry)
//! - LLM providers (Ollama, Claude, Gemini)
//! - PTY management for CLI agents
//! - Configuration loading
//! - File watching
//!
//! Any UI (TUI, Web, GUI) can consume this crate through the `AxiomService` interface.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────────┐   Command     ┌──────────────────┐
//! │   Any UI          │ ─────────────→│   axiom-core     │
//! │ (TUI, Web, GUI)   │               │   AxiomService   │
//! │                   │ ←─────────────│                  │
//! └───────────────────┘  Notification └──────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use axiom_core::{AxiomService, Command, Notification};
//!
//! // Create the service
//! let service = AxiomService::new(config, cwd)?;
//!
//! // Send commands
//! service.send(Command::process_input("hello"))?;
//!
//! // Receive notifications
//! while let Ok(notif) = service.notifications().try_recv() {
//!     match notif {
//!         Notification::AgentOutput { id, chunk } => {
//!             println!("Agent {}: {}", id, chunk);
//!         }
//!         _ => {}
//!     }
//! }
//! ```

// Public API modules
pub mod commands;
pub mod error;
pub mod notifications;
pub mod types;

// Backend modules (moved from main axiom crate)
pub mod config;

// Internal event system (used by backend components)
pub(crate) mod events;

// LLM provider system
pub mod llm;

// Workspace management
pub mod workspace;

// Agent orchestration system
pub mod orchestration;

// Re-export commonly used types
pub use commands::Command;
pub use error::{AxiomError, Result};
pub use notifications::{FileEntry, Notification};
pub use types::{
    AgentId, AgentSpawnRequest, AgentStatus, AgentType, AgentView, CliAgentInfo, OutputContext,
    ProviderInfo, ProviderStatus, TerminalCell, TerminalColor, TerminalLine, TerminalScreen,
};

// Re-export config types
pub use config::{AxiomConfig, CliAgentConfig, CliAgentsConfig, LlmConfig, ProviderConfig};

// Re-export workspace types
pub use workspace::{
    Workspace, WorkspaceConfig, WorkspaceId, WorkspaceManager, WorkspaceType, WorkspaceView,
};

// Re-export LLM types
pub use llm::{
    ChatMessage, ClaudeProvider, GeminiProvider, LlmError, LlmProvider, OllamaProvider,
    OpenAIProvider, ProviderCapabilities, ProviderRegistry, SharedProvider,
};

// Re-export orchestration types
pub use orchestration::{
    AgentRole, DeveloperResponse, LlmSettings, NextAgent, OperationResult, OrchestratorDecision,
    OrchestrationService, ProviderConfigUpdate,
};

// Agent system
pub mod agents;

// Re-export agent types
pub use agents::{Agent, AgentRegistry, Conductor, Executor, PtyAgent, PtyAgentManager};

// Main service facade
pub mod service;
pub use service::AxiomService;

// TODO: File watcher will be added later
// pub mod watcher;

/// Get the crate version
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
