//! Error types for Axiom Core
//!
//! Provides a unified error type for all backend operations.

use thiserror::Error;

/// Result type for Axiom Core operations
pub type Result<T> = std::result::Result<T, AxiomError>;

/// Unified error type for Axiom Core
#[derive(Error, Debug)]
pub enum AxiomError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// LLM provider error
    #[error("LLM error: {0}")]
    Llm(String),

    /// Agent error
    #[error("Agent error: {0}")]
    Agent(String),

    /// PTY error
    #[error("PTY error: {0}")]
    Pty(String),

    /// Channel error (communication failure)
    #[error("Channel error: {0}")]
    Channel(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

impl AxiomError {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        AxiomError::Config(msg.into())
    }

    /// Create an LLM error
    pub fn llm(msg: impl Into<String>) -> Self {
        AxiomError::Llm(msg.into())
    }

    /// Create an agent error
    pub fn agent(msg: impl Into<String>) -> Self {
        AxiomError::Agent(msg.into())
    }

    /// Create a PTY error
    pub fn pty(msg: impl Into<String>) -> Self {
        AxiomError::Pty(msg.into())
    }

    /// Create a channel error
    pub fn channel(msg: impl Into<String>) -> Self {
        AxiomError::Channel(msg.into())
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        AxiomError::NotFound(msg.into())
    }

    /// Create an invalid operation error
    pub fn invalid_operation(msg: impl Into<String>) -> Self {
        AxiomError::InvalidOperation(msg.into())
    }
}

impl From<serde_json::Error> for AxiomError {
    fn from(err: serde_json::Error) -> Self {
        AxiomError::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for AxiomError {
    fn from(err: toml::de::Error) -> Self {
        AxiomError::Config(err.to_string())
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for AxiomError {
    fn from(err: crossbeam_channel::SendError<T>) -> Self {
        AxiomError::Channel(format!("Send error: {}", err))
    }
}
