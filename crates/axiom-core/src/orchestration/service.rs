//! Orchestration Service
//!
//! Main service for coordinating agents and LLM interactions.

use super::{
    developer::{build_developer_messages, get_file_tree, parse_developer_response},
    orchestrator::{build_orchestrator_messages, parse_orchestrator_response},
    types::{
        AgentMapping, AgentOperation, AgentRole, ChatMessage, DeveloperResponse, LlmSettings,
        OrchestratorDecision, ProviderConfig,
    },
};
use crate::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Orchestration service for managing agent interactions
pub struct OrchestrationService {
    /// LLM settings
    settings: RwLock<LlmSettings>,
    /// Workspace path
    workspace_path: PathBuf,
    /// HTTP client for API calls
    client: ureq::Agent,
}

impl OrchestrationService {
    /// Create a new orchestration service
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            settings: RwLock::new(LlmSettings::default()),
            workspace_path,
            client: ureq::Agent::new(),
        }
    }

    /// Create with custom settings
    pub fn with_settings(workspace_path: PathBuf, settings: LlmSettings) -> Self {
        Self {
            settings: RwLock::new(settings),
            workspace_path,
            client: ureq::Agent::new(),
        }
    }

    /// Get current LLM settings
    pub fn settings(&self) -> LlmSettings {
        self.settings.read().clone()
    }

    /// Update provider configuration
    pub fn update_provider(&self, provider_id: &str, updates: ProviderConfigUpdate) {
        let mut settings = self.settings.write();
        if let Some(provider) = settings.providers.iter_mut().find(|p| p.id == provider_id) {
            if let Some(api_key) = updates.api_key {
                provider.api_key = api_key;
            }
            if let Some(base_url) = updates.base_url {
                provider.base_url = Some(base_url);
            }
            if let Some(default_model) = updates.default_model {
                provider.default_model = default_model;
            }
            if let Some(enabled) = updates.enabled {
                provider.enabled = enabled;
            }
        }
    }

    /// Update agent mapping
    pub fn update_agent_mapping(&self, agent_id: AgentRole, provider_id: &str, model_id: &str) {
        let mut settings = self.settings.write();
        if let Some(mapping) = settings.agent_mappings.iter_mut().find(|m| m.agent_id == agent_id) {
            mapping.provider_id = provider_id.to_string();
            mapping.model_id = model_id.to_string();
        } else {
            settings.agent_mappings.push(AgentMapping {
                agent_id,
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
            });
        }
    }

    /// Run the orchestrator to decide next action
    pub fn orchestrate(&self, messages: &[ChatMessage]) -> Result<OrchestratorDecision> {
        let llm_messages = build_orchestrator_messages(messages);
        let response = self.call_llm(AgentRole::Orchestrator, &llm_messages)?;
        parse_orchestrator_response(&response)
    }

    /// Run the developer agent
    pub fn run_developer(&self, task: &str) -> Result<DeveloperResponse> {
        let file_list = get_file_tree(&self.workspace_path, 3);
        let llm_messages = build_developer_messages(task, &self.workspace_path, &file_list);
        let response = self.call_llm(AgentRole::Developer, &llm_messages)?;
        parse_developer_response(&response)
    }

    /// Execute developer operations
    pub async fn execute_operations(&self, operations: &[AgentOperation]) -> Vec<OperationResult> {
        let mut results = Vec::new();

        for op in operations {
            let result = match op {
                AgentOperation::Write { path, content } => {
                    self.execute_write(path, content).await
                }
                AgentOperation::Delete { path } => {
                    self.execute_delete(path).await
                }
                AgentOperation::Execute { command } => {
                    self.execute_command(command).await
                }
            };
            results.push(result);
        }

        results
    }

    /// Call LLM for a specific agent
    fn call_llm(&self, agent: AgentRole, messages: &[ChatMessage]) -> Result<String> {
        let settings = self.settings.read();

        // Find mapping for this agent
        let mapping = settings
            .agent_mappings
            .iter()
            .find(|m| m.agent_id == agent)
            .ok_or_else(|| crate::AxiomError::Config(format!("No mapping for agent {:?}", agent)))?;

        // Find provider
        let provider = settings
            .providers
            .iter()
            .find(|p| p.id == mapping.provider_id && p.enabled)
            .ok_or_else(|| {
                crate::AxiomError::Config(format!(
                    "Provider {} not found or not enabled",
                    mapping.provider_id
                ))
            })?;

        // Make API call based on provider
        match provider.id.as_str() {
            "openai" => self.call_openai(provider, &mapping.model_id, messages),
            "anthropic" => self.call_anthropic(provider, &mapping.model_id, messages),
            "gemini" => self.call_gemini(provider, &mapping.model_id, messages),
            "ollama" => self.call_ollama(provider, &mapping.model_id, messages),
            _ => Err(crate::AxiomError::Config(format!(
                "Unsupported provider: {}",
                provider.id
            ))),
        }
    }

    fn call_openai(
        &self,
        provider: &ProviderConfig,
        model: &str,
        messages: &[ChatMessage],
    ) -> Result<String> {
        let base_url = provider
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");

        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        super::types::MessageRole::System => "system",
                        super::types::MessageRole::User => "user",
                        super::types::MessageRole::Assistant => "assistant",
                    },
                    "content": m.content
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": api_messages,
            "temperature": 0.7
        });

        let response = self
            .client
            .post(&format!("{}/chat/completions", base_url))
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", provider.api_key))
            .send_json(&body)
            .map_err(|e| crate::AxiomError::Llm(e.to_string()))?;

        let json: serde_json::Value = response
            .into_json()
            .map_err(|e| crate::AxiomError::Llm(e.to_string()))?;

        json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| crate::AxiomError::Llm("Invalid OpenAI response".to_string()))
    }

    fn call_anthropic(
        &self,
        provider: &ProviderConfig,
        model: &str,
        messages: &[ChatMessage],
    ) -> Result<String> {
        let mut system_content = String::new();
        let mut api_messages: Vec<serde_json::Value> = Vec::new();

        for msg in messages {
            match msg.role {
                super::types::MessageRole::System => {
                    system_content.push_str(&msg.content);
                    system_content.push('\n');
                }
                super::types::MessageRole::User => {
                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": msg.content
                    }));
                }
                super::types::MessageRole::Assistant => {
                    api_messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": msg.content
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": 8192,
            "messages": api_messages
        });

        if !system_content.is_empty() {
            body["system"] = serde_json::Value::String(system_content.trim().to_string());
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .set("Content-Type", "application/json")
            .set("x-api-key", &provider.api_key)
            .set("anthropic-version", "2023-06-01")
            .send_json(&body)
            .map_err(|e| crate::AxiomError::Llm(e.to_string()))?;

        let json: serde_json::Value = response
            .into_json()
            .map_err(|e| crate::AxiomError::Llm(e.to_string()))?;

        json.get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| crate::AxiomError::Llm("Invalid Anthropic response".to_string()))
    }

    fn call_gemini(
        &self,
        provider: &ProviderConfig,
        model: &str,
        messages: &[ChatMessage],
    ) -> Result<String> {
        let contents: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        super::types::MessageRole::System | super::types::MessageRole::User => "user",
                        super::types::MessageRole::Assistant => "model",
                    },
                    "parts": [{"text": m.content}]
                })
            })
            .collect();

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, provider.api_key
        );

        let response = self
            .client
            .post(&url)
            .set("Content-Type", "application/json")
            .send_json(&serde_json::json!({ "contents": contents }))
            .map_err(|e| crate::AxiomError::Llm(e.to_string()))?;

        let json: serde_json::Value = response
            .into_json()
            .map_err(|e| crate::AxiomError::Llm(e.to_string()))?;

        json.get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| crate::AxiomError::Llm("Invalid Gemini response".to_string()))
    }

    fn call_ollama(
        &self,
        provider: &ProviderConfig,
        model: &str,
        messages: &[ChatMessage],
    ) -> Result<String> {
        let base_url = provider
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434");

        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        super::types::MessageRole::System => "system",
                        super::types::MessageRole::User => "user",
                        super::types::MessageRole::Assistant => "assistant",
                    },
                    "content": m.content
                })
            })
            .collect();

        let response = self
            .client
            .post(&format!("{}/api/chat", base_url))
            .set("Content-Type", "application/json")
            .send_json(&serde_json::json!({
                "model": model,
                "messages": api_messages,
                "stream": false
            }))
            .map_err(|e| crate::AxiomError::Llm(e.to_string()))?;

        let json: serde_json::Value = response
            .into_json()
            .map_err(|e| crate::AxiomError::Llm(e.to_string()))?;

        json.get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| crate::AxiomError::Llm("Invalid Ollama response".to_string()))
    }

    async fn execute_write(&self, path: &PathBuf, content: &str) -> OperationResult {
        let full_path = if path.is_absolute() {
            path.clone()
        } else {
            self.workspace_path.join(path)
        };

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return OperationResult {
                    success: false,
                    message: format!("Failed to create directory: {}", e),
                };
            }
        }

        match tokio::fs::write(&full_path, content).await {
            Ok(_) => OperationResult {
                success: true,
                message: format!("Wrote {}", full_path.display()),
            },
            Err(e) => OperationResult {
                success: false,
                message: format!("Failed to write file: {}", e),
            },
        }
    }

    async fn execute_delete(&self, path: &PathBuf) -> OperationResult {
        let full_path = if path.is_absolute() {
            path.clone()
        } else {
            self.workspace_path.join(path)
        };

        match tokio::fs::remove_file(&full_path).await {
            Ok(_) => OperationResult {
                success: true,
                message: format!("Deleted {}", full_path.display()),
            },
            Err(e) => OperationResult {
                success: false,
                message: format!("Failed to delete file: {}", e),
            },
        }
    }

    async fn execute_command(&self, command: &str) -> OperationResult {
        use tokio::process::Command;

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.workspace_path)
            .output()
            .await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(1);

                OperationResult {
                    success: exit_code == 0,
                    message: if exit_code == 0 {
                        format!("Command succeeded:\n{}", stdout)
                    } else {
                        format!("Command failed (exit {}):\n{}\n{}", exit_code, stdout, stderr)
                    },
                }
            }
            Err(e) => OperationResult {
                success: false,
                message: format!("Failed to execute command: {}", e),
            },
        }
    }
}

/// Updates for provider configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfigUpdate {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    pub enabled: Option<bool>,
}

/// Result of executing an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub success: bool,
    pub message: String,
}
