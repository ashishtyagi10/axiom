//! Core types for Axiom backend
//!
//! These types are serializable for web UI integration.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for spawned agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u64);

impl AgentId {
    /// Create a new AgentId from a raw value
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "agent-{}", self.0)
    }
}

/// Agent execution status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
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
    Custom { name: String },

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
            AgentType::Custom { name } => name,
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
            AgentType::Custom { .. } => "🔧",
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

/// Read-only view of an agent for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentView {
    /// Unique agent identifier
    pub id: AgentId,

    /// Human-readable name
    pub name: String,

    /// Type of agent
    pub agent_type: AgentType,

    /// Current status
    pub status: AgentStatus,

    /// Number of output lines
    pub line_count: usize,

    /// Elapsed time in seconds
    pub elapsed_secs: f64,

    /// Parent agent ID (for child agents)
    pub parent_id: Option<AgentId>,

    /// Token count (for LLM agents)
    pub token_count: usize,
}

/// Represents what is currently displayed in the output area
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputContext {
    /// Viewing a file from the file tree
    File { path: PathBuf },

    /// Viewing output from a specific agent
    Agent { agent_id: AgentId },

    /// Empty state (no context selected)
    Empty,
}

impl Default for OutputContext {
    fn default() -> Self {
        Self::Empty
    }
}

impl OutputContext {
    /// Check if currently showing a file
    pub fn is_file(&self) -> bool {
        matches!(self, OutputContext::File { .. })
    }

    /// Check if currently showing an agent
    pub fn is_agent(&self) -> bool {
        matches!(self, OutputContext::Agent { .. })
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        matches!(self, OutputContext::Empty)
    }

    /// Get the file path if this is a file context
    pub fn file_path(&self) -> Option<&PathBuf> {
        match self {
            OutputContext::File { path } => Some(path),
            _ => None,
        }
    }

    /// Get the agent ID if this is an agent context
    pub fn agent_id(&self) -> Option<AgentId> {
        match self {
            OutputContext::Agent { agent_id } => Some(*agent_id),
            _ => None,
        }
    }
}

/// Terminal screen in UI-agnostic format (for PTY rendering)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalScreen {
    /// Lines of terminal content
    pub lines: Vec<TerminalLine>,

    /// Cursor position (col, row)
    pub cursor: Option<(u16, u16)>,

    /// Terminal dimensions
    pub cols: u16,
    pub rows: u16,
}

/// A single line of terminal content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalLine {
    /// Cells in this line
    pub cells: Vec<TerminalCell>,
}

/// A single cell in the terminal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalCell {
    /// Character in this cell
    pub char: char,

    /// Foreground color
    pub fg: TerminalColor,

    /// Background color
    pub bg: TerminalColor,

    /// Bold style
    pub bold: bool,

    /// Underline style
    pub underline: bool,

    /// Italic style
    pub italic: bool,

    /// Inverse/reverse video style
    pub inverse: bool,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            char: ' ',
            fg: TerminalColor::Default,
            bg: TerminalColor::Default,
            bold: false,
            underline: false,
            italic: false,
            inverse: false,
        }
    }
}

/// UI-agnostic color representation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TerminalColor {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Rgb { r: u8, g: u8, b: u8 },
    Indexed { index: u8 },
}

/// Information about an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider identifier (e.g., "ollama", "claude")
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Current status
    pub status: ProviderStatus,

    /// Currently selected model
    pub current_model: Option<String>,

    /// Available models
    pub models: Vec<String>,
}

/// Status of an LLM provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProviderStatus {
    /// Provider is available and ready
    Available,

    /// Provider is connecting/initializing
    Connecting,

    /// Provider is unavailable
    Unavailable,

    /// Provider encountered an error
    Error(String),
}

/// Information about a CLI agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliAgentInfo {
    /// Agent identifier (e.g., "claude", "gemini")
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Icon for display
    pub icon: String,

    /// Whether this agent is enabled
    pub enabled: bool,
}

/// Request to spawn a new agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnRequest {
    /// Type of agent to spawn
    pub agent_type: AgentType,

    /// Human-readable name
    pub name: String,

    /// Description of the task
    pub description: String,

    /// Task-specific parameters (JSON-serializable)
    pub parameters: Option<String>,

    /// Parent agent that is spawning this agent
    pub parent_id: Option<AgentId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_serialization() {
        let id = AgentId::new(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");

        let parsed: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_agent_status_serialization() {
        let status = AgentStatus::Error("test error".to_string());
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Error"));
        assert!(json.contains("test error"));
    }

    #[test]
    fn test_agent_type_serialization() {
        let agent_type = AgentType::CliAgent {
            config_id: "claude".to_string(),
        };
        let json = serde_json::to_string(&agent_type).unwrap();
        assert!(json.contains("CliAgent"));
        assert!(json.contains("claude"));
    }

    #[test]
    fn test_output_context_serialization() {
        let ctx = OutputContext::File {
            path: PathBuf::from("/test/file.rs"),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("File"));
        assert!(json.contains("/test/file.rs"));
    }
}
