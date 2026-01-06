//! Error types for axiom using thiserror
//!
//! All errors are typed - no .unwrap() or .expect() in production code.

use thiserror::Error;

/// Top-level application error
#[derive(Error, Debug)]
pub enum AxiomError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),

    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Channel send error")]
    ChannelSend,

    #[error("Channel receive error")]
    ChannelRecv,
}

/// PTY-specific errors
#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Failed to create PTY: {0}")]
    Create(String),

    #[error("Failed to spawn shell: {0}")]
    Spawn(String),

    #[error("Failed to resize PTY: {0}")]
    Resize(String),

    #[error("PTY write error: {0}")]
    Write(String),

    #[error("PTY read error: {0}")]
    Read(String),

    #[error("PTY IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// LLM provider errors
#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Provider not configured: {0}")]
    NotConfigured(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Streaming error: {0}")]
    Stream(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Provider not available: {0}")]
    Unavailable(String),
}

/// Convenience Result type for axiom
pub type Result<T> = std::result::Result<T, AxiomError>;
