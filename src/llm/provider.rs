//! LLM provider trait and capabilities

use super::{ChatMessage, LlmError};
use crate::events::Event;
use crossbeam_channel::Sender;

/// Provider capabilities flags
#[derive(Debug, Clone, Default)]
pub struct ProviderCapabilities {
    /// Supports streaming responses
    pub streaming: bool,

    /// Supports function/tool calling
    pub function_calling: bool,

    /// Supports vision/image input
    pub vision: bool,

    /// Supports file attachments in context
    pub file_context: bool,

    /// Maximum context window size (tokens)
    pub max_context: usize,

    /// Maximum output tokens
    pub max_output: usize,
}

/// Provider status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderStatus {
    /// Ready to accept requests
    Ready,

    /// Currently processing a request
    Busy,

    /// Not available (no API key, server down, etc.)
    Unavailable(String),

    /// Rate limited
    RateLimited,
}

impl ProviderStatus {
    pub fn is_ready(&self) -> bool {
        matches!(self, ProviderStatus::Ready)
    }
}

/// LLM provider trait
///
/// Defines the interface for LLM providers (Claude, Gemini, Ollama, OpenAI).
pub trait LlmProvider: Send + Sync {
    /// Get the provider's unique identifier (e.g., "claude", "gemini", "ollama")
    fn id(&self) -> &str;

    /// Get the provider's display name (e.g., "Claude", "Gemini", "Ollama")
    fn name(&self) -> &str;

    /// Get the currently active model name
    fn model(&self) -> String;

    /// Set the active model
    fn set_model(&self, model: &str) -> Result<(), LlmError>;

    /// List available models for this provider
    fn list_models(&self) -> Result<Vec<String>, LlmError>;

    /// Get provider capabilities
    fn capabilities(&self) -> ProviderCapabilities;

    /// Get current provider status
    fn status(&self) -> ProviderStatus;

    /// Send a message and stream the response
    ///
    /// This method initiates a request to the LLM provider and streams the
    /// response chunks back via the provided `event_tx` channel.
    ///
    /// Events emitted:
    /// - `LlmChunk(String)` - A chunk of the response
    /// - `LlmDone` - Response complete
    /// - `LlmError(String)` - An error occurred
    fn send_message(&self, messages: Vec<ChatMessage>, event_tx: Sender<Event>);

    /// Check if the provider is currently available
    fn is_available(&self) -> bool {
        self.status().is_ready()
    }
}

/// Wrapper to make Box<dyn LlmProvider> cloneable via Arc
pub type SharedProvider = std::sync::Arc<dyn LlmProvider>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_capabilities_default() {
        let caps = ProviderCapabilities::default();
        assert!(!caps.streaming);
        assert!(!caps.function_calling);
        assert!(!caps.vision);
        assert!(!caps.file_context);
        assert_eq!(caps.max_context, 0);
        assert_eq!(caps.max_output, 0);
    }

    #[test]
    fn test_provider_capabilities_custom() {
        let caps = ProviderCapabilities {
            streaming: true,
            function_calling: true,
            vision: false,
            file_context: true,
            max_context: 100_000,
            max_output: 4096,
        };
        assert!(caps.streaming);
        assert!(caps.function_calling);
        assert_eq!(caps.max_context, 100_000);
    }

    #[test]
    fn test_provider_status_ready() {
        let status = ProviderStatus::Ready;
        assert!(status.is_ready());
    }

    #[test]
    fn test_provider_status_busy() {
        let status = ProviderStatus::Busy;
        assert!(!status.is_ready());
    }

    #[test]
    fn test_provider_status_unavailable() {
        let status = ProviderStatus::Unavailable("No API key".to_string());
        assert!(!status.is_ready());
        assert!(matches!(status, ProviderStatus::Unavailable(msg) if msg == "No API key"));
    }

    #[test]
    fn test_provider_status_rate_limited() {
        let status = ProviderStatus::RateLimited;
        assert!(!status.is_ready());
    }

    #[test]
    fn test_provider_status_equality() {
        assert_eq!(ProviderStatus::Ready, ProviderStatus::Ready);
        assert_eq!(ProviderStatus::Busy, ProviderStatus::Busy);
        assert_eq!(ProviderStatus::RateLimited, ProviderStatus::RateLimited);
        assert_ne!(ProviderStatus::Ready, ProviderStatus::Busy);
    }
}
