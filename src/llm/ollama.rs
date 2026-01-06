//! Ollama LLM provider
//!
//! Connects to local Ollama instance for chat completions.

use super::{ChatMessage, LlmProvider};
use crate::events::Event;
use crossbeam_channel::Sender;
use std::io::{BufRead, BufReader};
use std::sync::RwLock;

/// Ollama provider for local LLM inference
pub struct OllamaProvider {
    /// Base URL for Ollama API
    base_url: String,

    /// Model to use (wrapped in RwLock for interior mutability)
    model: RwLock<String>,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new("gemma3:4b")
    }
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(model: &str) -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: RwLock::new(model.to_string()),
        }
    }

    /// Set the model
    pub fn with_model(self, model: &str) -> Self {
        *self.model.write().unwrap() = model.to_string();
        self
    }

    /// Set the base URL
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    /// Change the current model
    pub fn set_model(&self, model: &str) {
        *self.model.write().unwrap() = model.to_string();
    }

    /// List available models from Ollama
    pub fn list_models(&self) -> Result<Vec<String>, String> {
        let url = format!("{}/api/tags", self.base_url);

        let response = ureq::get(&url)
            .call()
            .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

        let json: serde_json::Value = response
            .into_json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let models = json
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}

impl LlmProvider for OllamaProvider {
    fn send_message(&self, messages: Vec<ChatMessage>, event_tx: Sender<Event>) {
        let base_url = self.base_url.clone();
        let model = self.model.read().unwrap().clone();

        // Spawn a thread to handle the streaming response
        std::thread::spawn(move || {
            if let Err(e) = send_ollama_request(&base_url, &model, messages, &event_tx) {
                let _ = event_tx.send(Event::LlmError(e));
            }
            let _ = event_tx.send(Event::LlmDone);
        });
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    fn model(&self) -> String {
        self.model.read().unwrap().clone()
    }
}

/// Send request to Ollama and stream response
fn send_ollama_request(
    base_url: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    event_tx: &Sender<Event>,
) -> Result<(), String> {
    // Build the request body
    let body = serde_json::json!({
        "model": model,
        "messages": messages.iter().map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content
            })
        }).collect::<Vec<_>>(),
        "stream": true
    });

    // Use ureq for synchronous HTTP (simpler than async for this use case)
    let url = format!("{}/api/chat", base_url);

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(&body)
        .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

    // Read streaming response
    let reader = BufReader::new(response.into_reader());

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Read error: {}", e))?;

        if line.is_empty() {
            continue;
        }

        // Parse JSON response
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            // Check if done
            if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                break;
            }

            // Extract content from message
            if let Some(content) = json
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
            {
                if !content.is_empty() {
                    if event_tx.send(Event::LlmChunk(content.to_string())).is_err() {
                        break; // Channel closed
                    }
                }
            }
        }
    }

    Ok(())
}
