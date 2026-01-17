//! Agent type definitions
//!
//! Defines the status and type enums for agents in the system.

/// Agent execution status
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    /// Agent is waiting to be executed
    Pending,

    /// Agent is currently running
    Running,

    /// Agent completed successfully
    Completed,

    /// Agent encountered an error
    Error(String),

    /// Agent was cancelled by user
    Cancelled,

    /// Agent is idle, waiting for next input (used by Conductor)
    Idle,
}

impl AgentStatus {
    /// Check if the agent is in a terminal state (completed, error, or cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentStatus::Completed | AgentStatus::Error(_) | AgentStatus::Cancelled
        )
    }

    /// Check if the agent is currently running
    pub fn is_running(&self) -> bool {
        matches!(self, AgentStatus::Running)
    }

    /// Get a short status indicator for display
    pub fn indicator(&self) -> &'static str {
        match self {
            AgentStatus::Pending => "⏳",
            AgentStatus::Running => "⚡",
            AgentStatus::Completed => "✓",
            AgentStatus::Error(_) => "✗",
            AgentStatus::Cancelled => "⊘",
            AgentStatus::Idle => "💤",
        }
    }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Pending => write!(f, "Pending"),
            AgentStatus::Running => write!(f, "Running"),
            AgentStatus::Completed => write!(f, "Completed"),
            AgentStatus::Error(e) => write!(f, "Error: {}", e),
            AgentStatus::Cancelled => write!(f, "Cancelled"),
            AgentStatus::Idle => write!(f, "Idle"),
        }
    }
}

/// Type of agent (what tool/capability it represents)
#[derive(Debug, Clone, PartialEq)]
pub enum AgentType {
    /// The conductor agent that routes user input
    Conductor,

    /// Code generation/modification agent
    Coder,

    /// Shell command execution agent
    Shell,

    /// File system operations agent (read, write, search)
    FileOps,

    /// Search/retrieval agent
    Search,

    /// Custom tool agent with a specified name
    Custom(String),

    /// External CLI agent (Claude Code, Gemini CLI, etc.)
    CliAgent {
        /// The agent config ID (e.g., "claude", "gemini")
        config_id: String,
    },
}

impl AgentType {
    /// Get a short label for the agent type
    pub fn label(&self) -> &str {
        match self {
            AgentType::Conductor => "Conductor",
            AgentType::Coder => "Coder",
            AgentType::Shell => "Shell",
            AgentType::FileOps => "FileOps",
            AgentType::Search => "Search",
            AgentType::Custom(name) => name,
            AgentType::CliAgent { config_id } => config_id,
        }
    }

    /// Get an icon for the agent type
    pub fn icon(&self) -> &'static str {
        match self {
            AgentType::Conductor => "🎯",
            AgentType::Coder => "💻",
            AgentType::Shell => "🖥️",
            AgentType::FileOps => "📁",
            AgentType::Search => "🔍",
            AgentType::Custom(_) => "🔧",
            AgentType::CliAgent { .. } => "🤖",
        }
    }

    /// Check if this is a CLI agent
    pub fn is_cli_agent(&self) -> bool {
        matches!(self, AgentType::CliAgent { .. })
    }

    /// Get the CLI agent config ID if this is a CLI agent
    pub fn cli_agent_config_id(&self) -> Option<&str> {
        match self {
            AgentType::CliAgent { config_id } => Some(config_id),
            _ => None,
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_terminal() {
        assert!(!AgentStatus::Pending.is_terminal());
        assert!(!AgentStatus::Running.is_terminal());
        assert!(AgentStatus::Completed.is_terminal());
        assert!(AgentStatus::Error("test".to_string()).is_terminal());
        assert!(AgentStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_agent_status_is_running() {
        assert!(!AgentStatus::Pending.is_running());
        assert!(AgentStatus::Running.is_running());
        assert!(!AgentStatus::Completed.is_running());
        assert!(!AgentStatus::Error("err".to_string()).is_running());
        assert!(!AgentStatus::Cancelled.is_running());
        assert!(!AgentStatus::Idle.is_running());
    }

    #[test]
    fn test_agent_status_indicator() {
        assert_eq!(AgentStatus::Pending.indicator(), "⏳");
        assert_eq!(AgentStatus::Running.indicator(), "⚡");
        assert_eq!(AgentStatus::Completed.indicator(), "✓");
        assert_eq!(AgentStatus::Error("err".to_string()).indicator(), "✗");
        assert_eq!(AgentStatus::Cancelled.indicator(), "⊘");
        assert_eq!(AgentStatus::Idle.indicator(), "💤");
    }

    #[test]
    fn test_agent_status_display() {
        assert_eq!(format!("{}", AgentStatus::Pending), "Pending");
        assert_eq!(format!("{}", AgentStatus::Running), "Running");
        assert_eq!(format!("{}", AgentStatus::Completed), "Completed");
        assert_eq!(format!("{}", AgentStatus::Error("oops".to_string())), "Error: oops");
        assert_eq!(format!("{}", AgentStatus::Cancelled), "Cancelled");
        assert_eq!(format!("{}", AgentStatus::Idle), "Idle");
    }

    #[test]
    fn test_agent_status_equality() {
        assert_eq!(AgentStatus::Pending, AgentStatus::Pending);
        assert_ne!(AgentStatus::Pending, AgentStatus::Running);
        assert_eq!(
            AgentStatus::Error("a".to_string()),
            AgentStatus::Error("a".to_string())
        );
        assert_ne!(
            AgentStatus::Error("a".to_string()),
            AgentStatus::Error("b".to_string())
        );
    }

    #[test]
    fn test_agent_type_label() {
        assert_eq!(AgentType::Shell.label(), "Shell");
        assert_eq!(AgentType::Custom("MyTool".to_string()).label(), "MyTool");
        assert_eq!(AgentType::Conductor.label(), "Conductor");
        assert_eq!(AgentType::Coder.label(), "Coder");
        assert_eq!(AgentType::FileOps.label(), "FileOps");
        assert_eq!(AgentType::Search.label(), "Search");
        assert_eq!(
            AgentType::CliAgent { config_id: "claude".to_string() }.label(),
            "claude"
        );
    }

    #[test]
    fn test_agent_type_icon() {
        assert_eq!(AgentType::Conductor.icon(), "🎯");
        assert_eq!(AgentType::Coder.icon(), "💻");
        assert_eq!(AgentType::Shell.icon(), "🖥️");
        assert_eq!(AgentType::FileOps.icon(), "📁");
        assert_eq!(AgentType::Search.icon(), "🔍");
        assert_eq!(AgentType::Custom("any".to_string()).icon(), "🔧");
        assert_eq!(AgentType::CliAgent { config_id: "test".to_string() }.icon(), "🤖");
    }

    #[test]
    fn test_agent_type_is_cli_agent() {
        assert!(!AgentType::Conductor.is_cli_agent());
        assert!(!AgentType::Shell.is_cli_agent());
        assert!(!AgentType::Coder.is_cli_agent());
        assert!(!AgentType::Search.is_cli_agent());
        assert!(!AgentType::FileOps.is_cli_agent());
        assert!(!AgentType::Custom("test".to_string()).is_cli_agent());
        assert!(AgentType::CliAgent { config_id: "claude".to_string() }.is_cli_agent());
    }

    #[test]
    fn test_agent_type_cli_config_id() {
        assert!(AgentType::Shell.cli_agent_config_id().is_none());
        assert!(AgentType::Conductor.cli_agent_config_id().is_none());

        let cli = AgentType::CliAgent { config_id: "gemini".to_string() };
        assert_eq!(cli.cli_agent_config_id(), Some("gemini"));
    }

    #[test]
    fn test_agent_type_display() {
        assert_eq!(format!("{}", AgentType::Shell), "Shell");
        assert_eq!(format!("{}", AgentType::Custom("MyAgent".to_string())), "MyAgent");
        assert_eq!(
            format!("{}", AgentType::CliAgent { config_id: "claude".to_string() }),
            "claude"
        );
    }

    #[test]
    fn test_agent_type_equality() {
        assert_eq!(AgentType::Shell, AgentType::Shell);
        assert_ne!(AgentType::Shell, AgentType::Coder);
        assert_eq!(
            AgentType::Custom("a".to_string()),
            AgentType::Custom("a".to_string())
        );
        assert_ne!(
            AgentType::Custom("a".to_string()),
            AgentType::Custom("b".to_string())
        );
        assert_eq!(
            AgentType::CliAgent { config_id: "x".to_string() },
            AgentType::CliAgent { config_id: "x".to_string() }
        );
    }

    #[test]
    fn test_agent_type_clone() {
        let original = AgentType::Custom("test".to_string());
        let cloned = original.clone();
        assert_eq!(original, cloned);

        let cli = AgentType::CliAgent { config_id: "claude".to_string() };
        let cli_cloned = cli.clone();
        assert_eq!(cli, cli_cloned);
    }
}
