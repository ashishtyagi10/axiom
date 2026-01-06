//! LLM integration module
//!
//! Provides streaming chat with various LLM providers.

mod ollama;

pub use ollama::OllamaProvider;

use crate::events::Event;
use crossbeam_channel::Sender;

/// LLM provider trait
pub trait LlmProvider: Send + Sync {
    /// Send a message and stream the response
    fn send_message(&self, messages: Vec<ChatMessage>, event_tx: Sender<Event>);

    /// Get provider name
    fn name(&self) -> &str;

    /// Get current model name
    fn model(&self) -> String;
}

/// Chat message for LLM
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}
