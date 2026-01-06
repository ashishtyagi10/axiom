//! LLM integration module
//!
//! Provides multi-provider LLM support with streaming chat.
//!
//! Supported providers:
//! - Ollama (local inference)
//! - Claude (Anthropic API)
//! - Gemini (Google AI)
//! - OpenAI (GPT models)

mod error;
mod message;
mod ollama;
mod provider;
mod registry;

// Provider implementations
mod claude;
mod gemini;

pub use error::LlmError;
pub use message::{
    build_prompt_with_context, format_file_context, ChatMessage, ContentPart, MessageContent, Role,
};
pub use ollama::OllamaProvider;
pub use provider::{LlmProvider, ProviderCapabilities, ProviderStatus, SharedProvider};
pub use registry::{ProviderInfo, ProviderRegistry};

// Provider implementations
pub use claude::ClaudeProvider;
pub use gemini::GeminiProvider;
