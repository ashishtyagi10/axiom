//! Orchestration types for agent coordination
//!
//! Defines the types used by the agent orchestration system.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Agent roles in the orchestration system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    /// Orchestrates and delegates tasks to other agents
    Orchestrator,
    /// Product Owner - defines requirements and user stories
    Po,
    /// Architect - designs technical solutions
    Architect,
    /// Developer - writes and modifies code
    Developer,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Orchestrator => "orchestrator",
            AgentRole::Po => "po",
            AgentRole::Architect => "architect",
            AgentRole::Developer => "developer",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AgentRole::Orchestrator => "Orchestrator",
            AgentRole::Po => "Product Owner",
            AgentRole::Architect => "Architect",
            AgentRole::Developer => "Developer",
        }
    }
}

impl std::str::FromStr for AgentRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "orchestrator" => Ok(AgentRole::Orchestrator),
            "po" | "product_owner" => Ok(AgentRole::Po),
            "architect" => Ok(AgentRole::Architect),
            "developer" => Ok(AgentRole::Developer),
            _ => Err(format!("Unknown agent role: {}", s)),
        }
    }
}

/// Agent state for tracking status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub id: AgentRole,
    pub name: String,
    pub status: AgentStateStatus,
    pub current_task: Option<String>,
    pub last_message: Option<String>,
}

/// Status of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStateStatus {
    Idle,
    Thinking,
    Working,
    Waiting,
}

/// Decision made by the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorDecision {
    /// Next agent to handle the task, or "user" to return to user
    pub next_agent: NextAgent,
    /// Reasoning for the decision
    pub reasoning: String,
    /// Task or message for the next agent/user
    pub task: Option<String>,
}

/// Who should handle the next step
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NextAgent {
    Po,
    Architect,
    Developer,
    User,
}

impl std::str::FromStr for NextAgent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "po" | "product_owner" => Ok(NextAgent::Po),
            "architect" => Ok(NextAgent::Architect),
            "developer" => Ok(NextAgent::Developer),
            "user" => Ok(NextAgent::User),
            _ => Err(format!("Unknown next agent: {}", s)),
        }
    }
}

/// Operation type for the developer agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AgentOperation {
    /// Write or overwrite a file
    Write {
        path: PathBuf,
        content: String,
    },
    /// Delete a file
    Delete {
        path: PathBuf,
    },
    /// Execute a shell command
    Execute {
        command: String,
    },
}

/// Response from the developer agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperResponse {
    /// Reasoning/explanation of the approach
    pub reasoning: String,
    /// Operations to perform
    pub operations: Vec<AgentOperation>,
    /// Summary message for the user
    pub message: String,
}

/// Chat message for LLM interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// Configuration for agent-provider mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMapping {
    pub agent_id: AgentRole,
    pub provider_id: String,
    pub model_id: String,
}

/// LLM settings for the orchestration system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSettings {
    pub providers: Vec<ProviderConfig>,
    pub agent_mappings: Vec<AgentMapping>,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub default_model: String,
    pub enabled: bool,
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            providers: vec![
                ProviderConfig {
                    id: "openai".to_string(),
                    name: "OpenAI".to_string(),
                    api_key: String::new(),
                    base_url: None,
                    default_model: "gpt-4o".to_string(),
                    enabled: false,
                },
                ProviderConfig {
                    id: "anthropic".to_string(),
                    name: "Anthropic".to_string(),
                    api_key: String::new(),
                    base_url: None,
                    default_model: "claude-sonnet-4-20250514".to_string(),
                    enabled: false,
                },
                ProviderConfig {
                    id: "gemini".to_string(),
                    name: "Google Gemini".to_string(),
                    api_key: String::new(),
                    base_url: None,
                    default_model: "gemini-1.5-pro".to_string(),
                    enabled: false,
                },
                ProviderConfig {
                    id: "ollama".to_string(),
                    name: "Ollama (Local)".to_string(),
                    api_key: "na".to_string(),
                    base_url: Some("http://localhost:11434".to_string()),
                    default_model: "llama3".to_string(),
                    enabled: false,
                },
            ],
            agent_mappings: vec![
                AgentMapping {
                    agent_id: AgentRole::Orchestrator,
                    provider_id: "openai".to_string(),
                    model_id: "gpt-4o".to_string(),
                },
                AgentMapping {
                    agent_id: AgentRole::Po,
                    provider_id: "openai".to_string(),
                    model_id: "gpt-4o".to_string(),
                },
                AgentMapping {
                    agent_id: AgentRole::Architect,
                    provider_id: "openai".to_string(),
                    model_id: "gpt-4o".to_string(),
                },
                AgentMapping {
                    agent_id: AgentRole::Developer,
                    provider_id: "openai".to_string(),
                    model_id: "gpt-4o".to_string(),
                },
            ],
        }
    }
}
