//! CLI Agent configuration types
//!
//! Defines configuration for external CLI coding agents like Claude Code,
//! Gemini CLI, GitHub Copilot, etc. that can be invoked from Axiom.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a single CLI agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliAgentConfig {
    /// Human-readable name for display
    #[serde(default)]
    pub name: String,

    /// Command to execute (e.g., "claude", "gemini", "gh")
    pub command: String,

    /// Default arguments to pass before the user prompt
    #[serde(default)]
    pub default_args: Vec<String>,

    /// Whether to run in the current working directory
    #[serde(default = "default_true")]
    pub use_cwd: bool,

    /// Additional environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Whether this agent is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Icon for display in UI
    #[serde(default = "default_icon")]
    pub icon: String,
}

fn default_true() -> bool {
    true
}

fn default_icon() -> String {
    "⚙️".to_string()
}

impl Default for CliAgentConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            default_args: Vec::new(),
            use_cwd: true,
            env: HashMap::new(),
            enabled: true,
            icon: default_icon(),
        }
    }
}

/// Collection of CLI agent configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliAgentsConfig {
    /// Map of agent ID to configuration
    #[serde(default = "default_cli_agents")]
    pub agents: HashMap<String, CliAgentConfig>,
}

impl Default for CliAgentsConfig {
    fn default() -> Self {
        Self {
            agents: default_cli_agents(),
        }
    }
}

impl CliAgentsConfig {
    /// Get a CLI agent config by ID
    pub fn get(&self, id: &str) -> Option<&CliAgentConfig> {
        self.agents.get(id)
    }

    /// Get all enabled agents
    pub fn enabled_agents(&self) -> impl Iterator<Item = (&String, &CliAgentConfig)> {
        self.agents.iter().filter(|(_, config)| config.enabled)
    }

    /// Check if an agent ID exists and is enabled
    pub fn is_available(&self, id: &str) -> bool {
        self.agents.get(id).map(|c| c.enabled).unwrap_or(false)
    }
}

/// Create default CLI agent configurations
pub fn default_cli_agents() -> HashMap<String, CliAgentConfig> {
    let mut agents = HashMap::new();

    // Claude Code
    agents.insert(
        "claude".to_string(),
        CliAgentConfig {
            name: "Claude Code".to_string(),
            command: "claude".to_string(),
            default_args: vec![],
            use_cwd: true,
            env: HashMap::new(),
            enabled: true,
            icon: "🤖".to_string(),
        },
    );

    // Gemini CLI
    agents.insert(
        "gemini".to_string(),
        CliAgentConfig {
            name: "Gemini CLI".to_string(),
            command: "gemini".to_string(),
            default_args: vec![],
            use_cwd: true,
            env: HashMap::new(),
            enabled: true,
            icon: "💎".to_string(),
        },
    );

    // GitHub Copilot
    agents.insert(
        "copilot".to_string(),
        CliAgentConfig {
            name: "GitHub Copilot".to_string(),
            command: "gh".to_string(),
            default_args: vec!["copilot".to_string(), "suggest".to_string()],
            use_cwd: true,
            env: HashMap::new(),
            enabled: true,
            icon: "🐙".to_string(),
        },
    );

    // OpenCode
    agents.insert(
        "opencode".to_string(),
        CliAgentConfig {
            name: "OpenCode".to_string(),
            command: "opencode".to_string(),
            default_args: vec![],
            use_cwd: true,
            env: HashMap::new(),
            enabled: true,
            icon: "🔓".to_string(),
        },
    );

    // Aider
    agents.insert(
        "aider".to_string(),
        CliAgentConfig {
            name: "Aider".to_string(),
            command: "aider".to_string(),
            default_args: vec![],
            use_cwd: true,
            env: HashMap::new(),
            enabled: true,
            icon: "🔧".to_string(),
        },
    );

    agents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cli_agents() {
        let agents = default_cli_agents();
        assert!(agents.contains_key("claude"));
        assert!(agents.contains_key("gemini"));
        assert!(agents.contains_key("copilot"));

        let claude = agents.get("claude").unwrap();
        assert_eq!(claude.command, "claude");
        assert!(claude.enabled);
    }

    #[test]
    fn test_cli_agents_config() {
        let config = CliAgentsConfig::default();
        assert!(config.is_available("claude"));
        assert!(!config.is_available("nonexistent"));
    }

    #[test]
    fn test_enabled_agents() {
        let mut config = CliAgentsConfig::default();
        config.agents.get_mut("gemini").unwrap().enabled = false;

        let enabled: Vec<_> = config.enabled_agents().collect();
        assert!(enabled.iter().any(|(id, _)| *id == "claude"));
        assert!(!enabled.iter().any(|(id, _)| *id == "gemini"));
    }
}
