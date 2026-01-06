//! Claude (Anthropic) LLM provider
//!
//! Connects to Anthropic's Claude API for chat completions.

use super::{ChatMessage, LlmError, LlmProvider, ProviderCapabilities, ProviderStatus};
use crate::events::Event;
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use std::io::{BufRead, BufReader};

/// Claude provider for Anthropic's API
pub struct ClaudeProvider {
    /// API key
    api_key: String,

    /// Base URL for the API
    base_url: String,

    /// Current model
    model: RwLock<String>,

    /// Available models
    models: Vec<String>,
}

impl ClaudeProvider {
    /// Create a new Claude provider
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            model: RwLock::new(model.to_string()),
            models: vec![
                "claude-sonnet-4-20250514".to_string(),
                "claude-opus-4-20250514".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
            ],
        }
    }

    /// Create with a specific base URL (for proxies)
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }
}

impl LlmProvider for ClaudeProvider {
    fn id(&self) -> &str {
        "claude"
    }

    fn name(&self) -> &str {
        "Claude"
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
            max_context: 200_000,
            max_output: 8192,
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
            if let Err(e) = send_claude_request(&base_url, &api_key, &model, messages, &event_tx) {
                let _ = event_tx.send(Event::LlmError(e.to_string()));
            }
            let _ = event_tx.send(Event::LlmDone);
        });
    }
}

/// Send request to Claude API and stream response
fn send_claude_request(
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    event_tx: &Sender<Event>,
) -> Result<(), LlmError> {
    // Separate system message from conversation
    let mut system_content = String::new();
    let mut conversation: Vec<serde_json::Value> = Vec::new();

    for msg in messages {
        if msg.role_str() == "system" {
            system_content.push_str(&msg.text());
            system_content.push('\n');
        } else {
            conversation.push(serde_json::json!({
                "role": msg.role_str(),
                "content": msg.text()
            }));
        }
    }

    // Build request body
    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": 8192,
        "stream": true,
        "messages": conversation
    });

    // Add system message if present
    if !system_content.is_empty() {
        body["system"] = serde_json::Value::String(system_content.trim().to_string());
    }

    let url = format!("{}/v1/messages", base_url);

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .send_json(&body)?;

    // Read SSE streaming response
    let reader = BufReader::new(response.into_reader());

    for line in reader.lines() {
        let line = line?;

        if line.is_empty() || !line.starts_with("data: ") {
            continue;
        }

        let data = &line[6..]; // Skip "data: " prefix

        if data == "[DONE]" {
            break;
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
            // Check event type
            let event_type = json.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match event_type {
                "content_block_delta" => {
                    if let Some(delta) = json.get("delta") {
                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() {
                                if event_tx.send(Event::LlmChunk(text.to_string())).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
                "message_stop" => break,
                "error" => {
                    let error_msg = json
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error");
                    return Err(LlmError::Api {
                        status: 500,
                        message: error_msg.to_string(),
                    });
                }
                _ => {}
            }
        }
    }

    Ok(())
}
