//! Notifications that the Axiom backend sends to any UI
//!
//! These notifications inform the UI about state changes in the backend.
//! They are serializable for web UI integration over WebSocket/gRPC.

use crate::types::{
    AgentId, AgentStatus, AgentType, AgentView, CliAgentInfo, OutputContext, ProviderInfo,
    ProviderStatus, TerminalScreen,
};
use crate::workspace::{Workspace, WorkspaceId, WorkspaceView};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Notifications that the backend sends to any UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Notification {
    /// Agent was spawned
    AgentSpawned {
        /// The agent ID
        id: AgentId,

        /// Human-readable name
        name: String,

        /// Type of agent
        agent_type: AgentType,

        /// Parent agent ID (for child agents)
        parent_id: Option<AgentId>,
    },

    /// Agent status changed
    AgentStatusChanged {
        /// The agent ID
        id: AgentId,

        /// New status
        status: AgentStatus,
    },

    /// Agent produced output (streaming)
    AgentOutput {
        /// The agent ID
        id: AgentId,

        /// Output chunk (text)
        chunk: String,
    },

    /// CLI agent PTY output (raw for terminal rendering)
    PtyOutput {
        /// The runtime agent ID
        id: AgentId,

        /// Raw PTY output data
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },

    /// CLI agent PTY screen update
    ///
    /// Sent periodically with the current terminal screen state.
    PtyScreen {
        /// The runtime agent ID
        id: AgentId,

        /// Current terminal screen
        screen: TerminalScreen,
    },

    /// CLI agent PTY exited
    PtyExited {
        /// The runtime agent ID
        id: AgentId,

        /// Exit code
        exit_code: i32,
    },

    /// File was modified (by an agent or file watcher)
    FileModified {
        /// Path to the modified file
        path: PathBuf,
    },

    /// File content loaded (in response to ReadFile command)
    FileLoaded {
        /// Path to the file
        path: PathBuf,

        /// File content
        content: String,
    },

    /// File read error
    FileError {
        /// Path to the file
        path: PathBuf,

        /// Error message
        message: String,
    },

    /// Error occurred in the backend
    Error {
        /// Error message
        message: String,
    },

    /// Informational message
    Info {
        /// Message text
        message: String,
    },

    /// Warning message
    Warning {
        /// Warning text
        message: String,
    },

    /// LLM provider status changed
    LlmStatusChanged {
        /// Provider identifier
        provider_id: String,

        /// New status
        status: ProviderStatus,
    },

    /// Available models changed for a provider
    ModelsChanged {
        /// Provider identifier
        provider_id: String,

        /// Updated list of available models
        models: Vec<String>,
    },

    /// Active LLM model changed
    ActiveModelChanged {
        /// Provider identifier
        provider_id: String,

        /// New active model
        model: String,
    },

    /// State snapshot (in response to GetSnapshot command)
    Snapshot {
        /// All agents
        agents: Vec<AgentView>,

        /// All LLM providers
        providers: Vec<ProviderInfo>,

        /// All CLI agents
        cli_agents: Vec<CliAgentInfo>,

        /// Current output context
        context: OutputContext,
    },

    /// Providers list (in response to ListProviders command)
    ProvidersList {
        /// Available providers
        providers: Vec<ProviderInfo>,
    },

    /// CLI agents list (in response to ListCliAgents command)
    CliAgentsList {
        /// Available CLI agents
        agents: Vec<CliAgentInfo>,
    },

    /// Output context changed
    ContextChanged {
        /// New context
        context: OutputContext,
    },

    /// Backend is shutting down
    ShuttingDown,

    // ========== Workspace Notifications ==========

    /// List of all workspaces (response to ListWorkspaces)
    WorkspaceList {
        /// All registered workspaces
        workspaces: Vec<WorkspaceView>,

        /// Currently active workspace ID
        active_id: Option<WorkspaceId>,
    },

    /// Workspace was created
    WorkspaceCreated {
        /// The created workspace
        workspace: Workspace,
    },

    /// Workspace was deleted
    WorkspaceDeleted {
        /// ID of deleted workspace
        workspace_id: WorkspaceId,
    },

    /// Workspace was activated (switched to)
    WorkspaceActivated {
        /// The activated workspace
        workspace: Workspace,
    },

    /// Workspace was deactivated
    WorkspaceDeactivated {
        /// ID of deactivated workspace
        workspace_id: WorkspaceId,
    },

    /// Workspace details (response to GetWorkspace)
    WorkspaceDetails {
        /// The workspace
        workspace: Workspace,
    },

    /// Workspace was updated
    WorkspaceUpdated {
        /// The updated workspace
        workspace: Workspace,
    },

    /// File list (response to ListFiles)
    FileList {
        /// Directory path
        path: PathBuf,

        /// Files and directories
        entries: Vec<FileEntry>,
    },
}

/// File entry for directory listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// File or directory name
    pub name: String,

    /// Full path
    pub path: PathBuf,

    /// Whether this is a directory
    pub is_directory: bool,

    /// File size in bytes (0 for directories)
    pub size: u64,

    /// Last modified timestamp (Unix epoch seconds)
    pub modified: i64,

    /// Whether this is a hidden file
    pub is_hidden: bool,
}

impl Notification {
    /// Create an AgentSpawned notification
    pub fn agent_spawned(
        id: AgentId,
        name: impl Into<String>,
        agent_type: AgentType,
        parent_id: Option<AgentId>,
    ) -> Self {
        Notification::AgentSpawned {
            id,
            name: name.into(),
            agent_type,
            parent_id,
        }
    }

    /// Create an AgentStatusChanged notification
    pub fn agent_status_changed(id: AgentId, status: AgentStatus) -> Self {
        Notification::AgentStatusChanged { id, status }
    }

    /// Create an AgentOutput notification
    pub fn agent_output(id: AgentId, chunk: impl Into<String>) -> Self {
        Notification::AgentOutput {
            id,
            chunk: chunk.into(),
        }
    }

    /// Create a PtyOutput notification
    pub fn pty_output(id: AgentId, data: Vec<u8>) -> Self {
        Notification::PtyOutput { id, data }
    }

    /// Create a PtyExited notification
    pub fn pty_exited(id: AgentId, exit_code: i32) -> Self {
        Notification::PtyExited { id, exit_code }
    }

    /// Create a FileModified notification
    pub fn file_modified(path: impl Into<PathBuf>) -> Self {
        Notification::FileModified { path: path.into() }
    }

    /// Create a FileLoaded notification
    pub fn file_loaded(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Notification::FileLoaded {
            path: path.into(),
            content: content.into(),
        }
    }

    /// Create an Error notification
    pub fn error(message: impl Into<String>) -> Self {
        Notification::Error {
            message: message.into(),
        }
    }

    /// Create an Info notification
    pub fn info(message: impl Into<String>) -> Self {
        Notification::Info {
            message: message.into(),
        }
    }

    /// Create a Warning notification
    pub fn warning(message: impl Into<String>) -> Self {
        Notification::Warning {
            message: message.into(),
        }
    }

    /// Create a WorkspaceList notification
    pub fn workspace_list(workspaces: Vec<WorkspaceView>, active_id: Option<WorkspaceId>) -> Self {
        Notification::WorkspaceList {
            workspaces,
            active_id,
        }
    }

    /// Create a WorkspaceCreated notification
    pub fn workspace_created(workspace: Workspace) -> Self {
        Notification::WorkspaceCreated { workspace }
    }

    /// Create a WorkspaceDeleted notification
    pub fn workspace_deleted(workspace_id: WorkspaceId) -> Self {
        Notification::WorkspaceDeleted { workspace_id }
    }

    /// Create a WorkspaceActivated notification
    pub fn workspace_activated(workspace: Workspace) -> Self {
        Notification::WorkspaceActivated { workspace }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_serialization() {
        let notif = Notification::agent_spawned(
            AgentId::new(1),
            "Test Agent",
            AgentType::Shell,
            None,
        );
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("AgentSpawned"));
        assert!(json.contains("Test Agent"));

        let parsed: Notification = serde_json::from_str(&json).unwrap();
        match parsed {
            Notification::AgentSpawned { name, .. } => assert_eq!(name, "Test Agent"),
            _ => panic!("Wrong notification type"),
        }
    }

    #[test]
    fn test_pty_output_notification() {
        let notif = Notification::pty_output(AgentId::new(42), vec![0x1b, 0x5b, 0x41]);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("PtyOutput"));
    }

    #[test]
    fn test_error_notification() {
        let notif = Notification::error("Something went wrong");
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("Error"));
        assert!(json.contains("Something went wrong"));
    }
}
