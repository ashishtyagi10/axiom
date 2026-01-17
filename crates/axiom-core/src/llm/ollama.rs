//! Ollama LLM provider
//!
//! Connects to local Ollama instance for chat completions.

use super::{ChatMessage, LlmError, LlmProvider, ProviderCapabilities, ProviderStatus};
use crate::events::Event;
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use std::io::{BufRead, BufReader};

/// Ollama provider for local LLM inference
pub struct OllamaProvider {
    /// Base URL for Ollama API
    base_url: String,

    /// Model to use
    model: RwLock<String>,

    /// Cached model list
    cached_models: RwLock<Option<Vec<String>>>,

    /// Current status
    status: RwLock<ProviderStatus>,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new("http://localhost:11434", "gemma3:4b")
    }
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: RwLock::new(model.to_string()),
            cached_models: RwLock::new(None),
            status: RwLock::new(ProviderStatus::Ready),
        }
    }

    /// Create with default URL
    pub fn with_model(model: &str) -> Self {
        Self::new("http://localhost:11434", model)
    }

    /// Fetch models from Ollama API
    fn fetch_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/api/tags", self.base_url);

        let response = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .call()?;

        let json: serde_json::Value = response.into_json()?;

        let models: Vec<String> = json
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Cache the models
        *self.cached_models.write() = Some(models.clone());

        Ok(models)
    }

    /// Check if Ollama is running
    fn check_connection(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        ureq::get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .call()
            .is_ok()
    }
}

impl LlmProvider for OllamaProvider {
    fn id(&self) -> &str {
        "ollama"
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    fn model(&self) -> String {
        self.model.read().clone()
    }

    fn set_model(&self, model: &str) -> Result<(), LlmError> {
        *self.model.write() = model.to_string();
        Ok(())
    }

    fn list_models(&self) -> Result<Vec<String>, LlmError> {
        // Return cached models if available
        if let Some(models) = self.cached_models.read().as_ref() {
            return Ok(models.clone());
        }

        self.fetch_models()
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            function_calling: false,
            vision: false,
            file_context: true,
            max_context: 8192,
            max_output: 4096,
        }
    }

    fn status(&self) -> ProviderStatus {
        if self.check_connection() {
            ProviderStatus::Ready
        } else {
            ProviderStatus::Unavailable("Cannot connect to Ollama".to_string())
        }
    }

    fn send_message(&self, messages: Vec<ChatMessage>, event_tx: Sender<Event>) {
        let base_url = self.base_url.clone();
        let model = self.model.read().clone();

        std::thread::spawn(move || {
            if let Err(e) = send_ollama_request(&base_url, &model, messages, &event_tx) {
                let _ = event_tx.send(Event::LlmError(e.to_string()));
            }
            let _ = event_tx.send(Event::LlmDone);
        });
    }
}

/// Send request to Ollama and stream response
fn send_ollama_request(
    base_url: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    event_tx: &Sender<Event>,
) -> Result<(), LlmError> {
    let body = serde_json::json!({
        "model": model,
        "messages": messages.iter().map(|m| {
            serde_json::json!({
                "role": m.role_str(),
                "content": m.text()
            })
        }).collect::<Vec<_>>(),
        "stream": true
    });

    let url = format!("{}/api/chat", base_url);

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(&body)?;

    let reader = BufReader::new(response.into_reader());

    for line in reader.lines() {
        let line = line?;

        if line.is_empty() {
            continue;
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                break;
            }

            if let Some(content) = json
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
            {
                if !content.is_empty() {
                    if event_tx.send(Event::LlmChunk(content.to_string())).is_err() {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
