//! OpenAI LLM provider
//!
//! Connects to OpenAI's API for chat completions.

use super::{ChatMessage, LlmError, LlmProvider, ProviderCapabilities, ProviderStatus};
use crate::events::Event;
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use std::io::{BufRead, BufReader};

/// OpenAI provider
pub struct OpenAIProvider {
    /// API key
    api_key: String,

    /// Base URL for the API (supports OpenAI-compatible APIs)
    base_url: String,

    /// Current model
    model: RwLock<String>,

    /// Available models
    models: Vec<String>,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: RwLock::new(model.to_string()),
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-4".to_string(),
                "gpt-3.5-turbo".to_string(),
                "o1".to_string(),
                "o1-mini".to_string(),
            ],
        }
    }

    /// Create with a specific base URL (for OpenAI-compatible APIs like Azure, local proxies)
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }
}

impl LlmProvider for OpenAIProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn model(&self) -> String {
        self.model.read().clone()
    }

    fn set_model(&self, model: &str) -> Result<(), LlmError> {
        *self.model.write() = model.to_string();
        Ok(())
    }

    fn list_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(self.models.clone())
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            function_calling: true,
            vision: true,
            file_context: true,
            max_context: 128_000, // GPT-4o context window
            max_output: 16384,
        }
    }

    fn status(&self) -> ProviderStatus {
        if self.api_key.is_empty() {
            ProviderStatus::Unavailable("No API key configured".to_string())
        } else {
            ProviderStatus::Ready
        }
    }

    fn send_message(&self, messages: Vec<ChatMessage>, event_tx: Sender<Event>) {
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let model = self.model.read().clone();

        std::thread::spawn(move || {
            if let Err(e) = send_openai_request(&base_url, &api_key, &model, messages, &event_tx) {
                let _ = event_tx.send(Event::LlmError(e.to_string()));
            }
            let _ = event_tx.send(Event::LlmDone);
        });
    }
}

/// Send request to OpenAI API and stream response
fn send_openai_request(
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    event_tx: &Sender<Event>,
) -> Result<(), LlmError> {
    let conversation: Vec<serde_json::Value> = messages
        .iter()
        .map(|msg| {
            serde_json::json!({
                "role": msg.role_str(),
                "content": msg.text()
            })
        })
        .collect();

    let body = serde_json::json!({
        "model": model,
        "messages": conversation,
        "stream": true,
        "temperature": 0.7
    });

    let url = format!("{}/chat/completions", base_url);

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", api_key))
        .send_json(&body)?;

    let reader = BufReader::new(response.into_reader());

    for line in reader.lines() {
        let line = line?;

        if line.is_empty() || !line.starts_with("data: ") {
            continue;
        }

        let data = &line[6..];

        if data == "[DONE]" {
            break;
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
            // Extract content from delta
            if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                if let Some(choice) = choices.first() {
                    if let Some(delta) = choice.get("delta") {
                        if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                            if !content.is_empty() {
                                if event_tx.send(Event::LlmChunk(content.to_string())).is_err() {
                                    break;
                                }
                            }
                        }
                    }

                    // Check for finish_reason
                    if let Some(finish_reason) = choice.get("finish_reason") {
                        if !finish_reason.is_null() {
                            break;
                        }
                    }
                }
            }

            // Check for errors
            if let Some(error) = json.get("error") {
                let error_msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                return Err(LlmError::Api {
                    status: 500,
                    message: error_msg.to_string(),
                });
            }
        }
    }

    Ok(())
}
