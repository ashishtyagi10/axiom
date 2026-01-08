//! Output context system for tracking what's displayed in the output area
//!
//! The OutputContext determines whether the output panel shows file content
//! or agent output based on user selection.

use std::path::PathBuf;

/// Unique identifier for spawned agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Represents what is currently displayed in the output area
#[derive(Debug, Clone, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_context_file() {
        let ctx = OutputContext::File {
            path: PathBuf::from("/test/file.rs"),
        };
        assert!(ctx.is_file());
        assert!(!ctx.is_agent());
        assert!(!ctx.is_empty());
        assert_eq!(ctx.file_path(), Some(&PathBuf::from("/test/file.rs")));
    }

    #[test]
    fn test_output_context_agent() {
        let ctx = OutputContext::Agent {
            agent_id: AgentId::new(42),
        };
        assert!(!ctx.is_file());
        assert!(ctx.is_agent());
        assert!(!ctx.is_empty());
        assert_eq!(ctx.agent_id(), Some(AgentId::new(42)));
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::new(123);
        assert_eq!(format!("{}", id), "agent-123");
    }
}
