//! Gemini (Google AI) LLM provider
//!
//! Connects to Google's Gemini API for chat completions.

use super::{ChatMessage, LlmError, LlmProvider, ProviderCapabilities, ProviderStatus};
use crate::events::Event;
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use std::io::{BufRead, BufReader};

/// Gemini provider for Google AI
pub struct GeminiProvider {
    /// API key
    api_key: String,

    /// Base URL for the API
    base_url: String,

    /// Current model
    model: RwLock<String>,

    /// Available models
    models: Vec<String>,
}

impl GeminiProvider {
    /// Create a new Gemini provider
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            model: RwLock::new(model.to_string()),
            models: vec![
                "gemini-2.0-flash".to_string(),
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-1.5-flash-8b".to_string(),
            ],
        }
    }

    /// Create with a specific base URL
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }
}

impl LlmProvider for GeminiProvider {
    fn id(&self) -> &str {
        "gemini"
    }

    fn name(&self) -> &str {
        "Gemini"
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
            max_context: 1_000_000, // Gemini 1.5 Pro has 1M context
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
            if let Err(e) = send_gemini_request(&base_url, &api_key, &model, messages, &event_tx) {
                let _ = event_tx.send(Event::LlmError(e.to_string()));
            }
            let _ = event_tx.send(Event::LlmDone);
        });
    }
}

/// Send request to Gemini API and stream response
fn send_gemini_request(
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    event_tx: &Sender<Event>,
) -> Result<(), LlmError> {
    // Convert messages to Gemini format
    let mut contents: Vec<serde_json::Value> = Vec::new();
    let mut system_instruction: Option<String> = None;

    for msg in messages {
        let role = match msg.role_str() {
            "system" => {
                // Gemini uses system_instruction instead of a system role in contents
                system_instruction = Some(msg.text());
                continue;
            }
            "assistant" => "model",
            _ => "user",
        };

        contents.push(serde_json::json!({
            "role": role,
            "parts": [{
                "text": msg.text()
            }]
        }));
    }

    // Build request body
    let mut body = serde_json::json!({
        "contents": contents,
        "generationConfig": {
            "maxOutputTokens": 8192,
            "temperature": 0.7
        }
    });

    // Add system instruction if present
    if let Some(instruction) = system_instruction {
        body["system_instruction"] = serde_json::json!({
            "parts": [{
                "text": instruction
            }]
        });
    }

    // Use streamGenerateContent endpoint
    let url = format!(
        "{}/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
        base_url, model, api_key
    );

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(&body)?;

    // Read SSE streaming response
    let reader = BufReader::new(response.into_reader());

    for line in reader.lines() {
        let line = line?;

        if line.is_empty() || !line.starts_with("data: ") {
            continue;
        }

        let data = &line[6..]; // Skip "data: " prefix

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
            // Check for errors
            if let Some(error) = json.get("error") {
                let message = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                return Err(LlmError::Api {
                    status: error.get("code").and_then(|c| c.as_u64()).unwrap_or(500) as u16,
                    message: message.to_string(),
                });
            }

            // Extract text from candidates
            if let Some(candidates) = json.get("candidates").and_then(|c| c.as_array()) {
                for candidate in candidates {
                    if let Some(content) = candidate.get("content") {
                        if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                            for part in parts {
                                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                    if !text.is_empty() {
                                        if event_tx.send(Event::LlmChunk(text.to_string())).is_err()
                                        {
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
