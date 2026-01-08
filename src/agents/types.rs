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
    fn test_agent_type_label() {
        assert_eq!(AgentType::Shell.label(), "Shell");
        assert_eq!(AgentType::Custom("MyTool".to_string()).label(), "MyTool");
    }
}
