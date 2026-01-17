//! Agent Orchestration System
//!
//! Provides a multi-agent system for software development tasks:
//! - Orchestrator: Coordinates and delegates tasks
//! - Product Owner: Defines requirements
//! - Architect: Designs technical solutions
//! - Developer: Writes and modifies code
//!
//! # Example
//!
//! ```no_run
//! use axiom_core::orchestration::{OrchestrationService, ChatMessage};
//! use std::path::PathBuf;
//!
//! let service = OrchestrationService::new(PathBuf::from("/my/workspace"));
//!
//! // Ask the orchestrator what to do
//! let messages = vec![
//!     ChatMessage::user("Add a login button to the navbar")
//! ];
//! let decision = service.orchestrate(&messages).unwrap();
//! println!("Next agent: {:?}", decision.next_agent);
//! ```

mod developer;
mod orchestrator;
mod service;
mod types;

// Re-export types
pub use types::{
    AgentMapping, AgentOperation, AgentRole, AgentState, AgentStateStatus, ChatMessage,
    DeveloperResponse, LlmSettings, MessageRole, NextAgent, OrchestratorDecision, ProviderConfig,
};

// Re-export service
pub use service::{OperationResult, OrchestrationService, ProviderConfigUpdate};

// Re-export helper functions
pub use developer::{build_developer_messages, get_file_tree, parse_developer_response};
pub use orchestrator::{build_orchestrator_messages, parse_orchestrator_response};
