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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_axiom_error_io_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let axiom_err: AxiomError = io_err.into();
        assert!(matches!(axiom_err, AxiomError::Io(_)));
        assert!(axiom_err.to_string().contains("IO error"));
    }

    #[test]
    fn test_axiom_error_pty_conversion() {
        let pty_err = PtyError::Create("failed to create".to_string());
        let axiom_err: AxiomError = pty_err.into();
        assert!(matches!(axiom_err, AxiomError::Pty(_)));
        assert!(axiom_err.to_string().contains("PTY error"));
    }

    #[test]
    fn test_axiom_error_llm_conversion() {
        let llm_err = LlmError::NotConfigured("claude".to_string());
        let axiom_err: AxiomError = llm_err.into();
        assert!(matches!(axiom_err, AxiomError::Llm(_)));
        assert!(axiom_err.to_string().contains("LLM error"));
    }

    #[test]
    fn test_axiom_error_config_display() {
        let err = AxiomError::Config("invalid setting".to_string());
        assert_eq!(err.to_string(), "Configuration error: invalid setting");
    }

    #[test]
    fn test_axiom_error_channel_variants() {
        let send_err = AxiomError::ChannelSend;
        assert_eq!(send_err.to_string(), "Channel send error");

        let recv_err = AxiomError::ChannelRecv;
        assert_eq!(recv_err.to_string(), "Channel receive error");
    }

    #[test]
    fn test_pty_error_variants() {
        let create_err = PtyError::Create("create failed".to_string());
        assert_eq!(create_err.to_string(), "Failed to create PTY: create failed");

        let spawn_err = PtyError::Spawn("spawn failed".to_string());
        assert_eq!(spawn_err.to_string(), "Failed to spawn shell: spawn failed");

        let resize_err = PtyError::Resize("resize failed".to_string());
        assert_eq!(resize_err.to_string(), "Failed to resize PTY: resize failed");

        let write_err = PtyError::Write("write failed".to_string());
        assert_eq!(write_err.to_string(), "PTY write error: write failed");

        let read_err = PtyError::Read("read failed".to_string());
        assert_eq!(read_err.to_string(), "PTY read error: read failed");
    }

    #[test]
    fn test_pty_error_io_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken");
        let pty_err: PtyError = io_err.into();
        assert!(matches!(pty_err, PtyError::Io(_)));
        assert!(pty_err.to_string().contains("PTY IO error"));
    }

    #[test]
    fn test_llm_error_variants() {
        let not_configured = LlmError::NotConfigured("ollama".to_string());
        assert_eq!(not_configured.to_string(), "Provider not configured: ollama");

        let network = LlmError::Network("connection refused".to_string());
        assert_eq!(network.to_string(), "Network error: connection refused");

        let api = LlmError::Api {
            status: 401,
            message: "unauthorized".to_string(),
        };
        assert_eq!(api.to_string(), "API error (401): unauthorized");

        let stream = LlmError::Stream("stream interrupted".to_string());
        assert_eq!(stream.to_string(), "Streaming error: stream interrupted");

        let parse = LlmError::Parse("invalid json".to_string());
        assert_eq!(parse.to_string(), "Parse error: invalid json");

        let unavailable = LlmError::Unavailable("provider offline".to_string());
        assert_eq!(unavailable.to_string(), "Provider not available: provider offline");
    }

    #[test]
    fn test_error_source_chain() {
        // Test that error source chain works correctly
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "underlying error");
        let pty_err = PtyError::Io(io_err);
        let axiom_err = AxiomError::Pty(pty_err);

        // Check that source() returns the underlying error
        let source = axiom_err.source();
        assert!(source.is_some());
    }
}
