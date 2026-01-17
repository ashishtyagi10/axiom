//! LLM integration module
//!
//! Provides multi-provider LLM support with streaming chat.
//!
//! Supported providers:
//! - OpenAI (GPT-4, GPT-3.5)
//! - Claude (Anthropic API)
//! - Gemini (Google AI)
//! - Ollama (local inference)

mod claude;
mod error;
mod gemini;
mod message;
mod ollama;
mod openai;
mod provider;
mod registry;

pub use error::LlmError;
pub use message::{
    build_prompt_with_context, format_file_context, ChatMessage, ContentPart, MessageContent, Role,
};
pub use provider::{LlmProvider, ProviderCapabilities, ProviderStatus, SharedProvider};
pub use registry::{ProviderInfo, ProviderRegistry};

// Provider implementations
pub use claude::ClaudeProvider;
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
