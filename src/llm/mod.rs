//! LLM integration module
//!
//! Provides streaming chat with various LLM providers.

mod ollama;

pub use ollama::OllamaProvider;

use crate::events::Event;
use crossbeam_channel::Sender;

/// LLM provider trait
pub trait LlmProvider: Send + Sync {
    /// Send a message and stream the response.
    ///
    /// This method initiates a request to the LLM provider and streams the
    /// response chunks back via the provided `event_tx` channel.
    ///
    /// # Arguments
    ///
    /// * `messages` - A vector of chat messages representing the conversation history.
    /// * `event_tx` - The event bus sender to emit `LlmChunk`, `LlmDone`, or `LlmError` events.
    fn send_message(&self, messages: Vec<ChatMessage>, event_tx: Sender<Event>);

    /// Get the provider's display name.
    fn name(&self) -> &str;

    /// Get the name of the currently active model.
    fn model(&self) -> String;
}

/// Chat message for LLM
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    /// Create a new message from the user.
    ///
    /// # Arguments
    ///
    /// * `content` - The text content of the message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create a new message from the assistant (AI).
    ///
    /// # Arguments
    ///
    /// * `content` - The text content of the message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    /// Create a new system message.
    ///
    /// System messages are typically used to set the behavior or context for the AI.
    ///
    /// # Arguments
    ///
    /// * `content` - The text content of the message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}
