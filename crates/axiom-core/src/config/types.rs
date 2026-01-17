//! Configuration types for Axiom
//!
//! Defines the structure of `.axiom.toml` configuration.

use super::cli_agents::CliAgentsConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AxiomConfig {
    /// LLM configuration
    #[serde(default)]
    pub llm: LlmConfig,

    /// CLI agent configurations
    #[serde(default)]
    pub cli_agents: CliAgentsConfig,
}

/// LLM configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default provider to use (claude, gemini, ollama, openai)
    #[serde(default = "default_provider")]
    pub default_provider: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Maximum retries for failed requests
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Provider configurations
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

fn default_provider() -> String {
    "ollama".to_string()
}

fn default_timeout() -> u64 {
    120
}

fn default_max_retries() -> u32 {
    3
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            default_provider: default_provider(),
            timeout: default_timeout(),
            max_retries: default_max_retries(),
            providers: default_providers(),
        }
    }
}

/// Individual provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Whether this provider is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// API key (supports ${ENV_VAR} syntax)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Base URL for the API
    #[serde(default)]
    pub base_url: Option<String>,

    /// Default model for this provider
    #[serde(default)]
    pub default_model: Option<String>,

    /// Available models (if empty, will be fetched from API)
    #[serde(default)]
    pub models: Vec<String>,
}

fn default_enabled() -> bool {
    true
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key: None,
            base_url: None,
            default_model: None,
            models: Vec::new(),
        }
    }
}

/// Create default provider configurations
fn default_providers() -> HashMap<String, ProviderConfig> {
    let mut providers = HashMap::new();

    // Ollama - local, no API key needed
    providers.insert(
        "ollama".to_string(),
        ProviderConfig {
            enabled: true,
            api_key: None,
            base_url: Some("http://localhost:11434".to_string()),
            default_model: Some("gemma3:4b".to_string()),
            models: Vec::new(),
        },
    );

    // Claude - requires API key
    providers.insert(
        "claude".to_string(),
        ProviderConfig {
            enabled: false, // Disabled by default until API key is set
            api_key: None,
            base_url: Some("https://api.anthropic.com".to_string()),
            default_model: Some("claude-sonnet-4-20250514".to_string()),
            models: vec![
                "claude-sonnet-4-20250514".to_string(),
                "claude-opus-4-20250514".to_string(),
            ],
        },
    );

    // Gemini - requires API key
    providers.insert(
        "gemini".to_string(),
        ProviderConfig {
            enabled: false, // Disabled by default until API key is set
            api_key: None,
            base_url: Some("https://generativelanguage.googleapis.com".to_string()),
            default_model: Some("gemini-2.0-flash".to_string()),
            models: vec![
                "gemini-2.0-flash".to_string(),
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
            ],
        },
    );

    // OpenAI - requires API key
    providers.insert(
        "openai".to_string(),
        ProviderConfig {
            enabled: false,
            api_key: None,
            base_url: Some("https://api.openai.com".to_string()),
            default_model: Some("gpt-4o".to_string()),
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
            ],
        },
    );

    providers
}

impl AxiomConfig {
    /// Get a provider config by name
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.llm.providers.get(name)
    }

    /// Get the default provider config
    pub fn default_provider_config(&self) -> Option<&ProviderConfig> {
        self.get_provider(&self.llm.default_provider)
    }

    /// Get all enabled providers
    pub fn enabled_providers(&self) -> Vec<(&String, &ProviderConfig)> {
        self.llm
            .providers
            .iter()
            .filter(|(_, config)| config.enabled && config.api_key.is_some())
            .collect()
    }
}
