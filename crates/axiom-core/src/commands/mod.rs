//! Commands that any UI can send to the Axiom backend
//!
//! These commands represent all actions a UI can request from the backend.
//! They are serializable for web UI integration over WebSocket/gRPC.

pub mod slash;

use crate::ralph::RalphConfig;
use crate::types::AgentId;
use crate::workspace::WorkspaceId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Commands that any UI can send to the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    /// Process user input through the conductor
    ///
    /// The conductor will analyze the input and spawn appropriate agents.
    ProcessInput {
        /// The user's input text
        text: String,
    },

    /// Execute a shell command directly
    ///
    /// Bypasses the conductor and executes the command in a shell.
    ExecuteShell {
        /// The shell command to execute
        command: String,
    },

    /// Invoke a CLI agent (e.g., Claude Code, Gemini CLI)
    ///
    /// Starts a PTY session with the specified CLI agent.
    InvokeCliAgent {
        /// The agent config ID (e.g., "claude", "gemini")
        agent_id: String,

        /// The user's prompt to pass to the agent
        prompt: String,
    },

    /// Send input to a running CLI agent's PTY
    ///
    /// Sends raw bytes to the PTY for keyboard input, etc.
    SendPtyInput {
        /// The runtime agent ID
        agent_id: AgentId,

        /// Raw input data to send
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },

    /// Resize a PTY session
    ///
    /// Updates the terminal dimensions for proper rendering.
    ResizePty {
        /// The runtime agent ID
        agent_id: AgentId,

        /// New terminal width in columns
        cols: u16,

        /// New terminal height in rows
        rows: u16,
    },

    /// Read a file
    ///
    /// Loads a file and sends its content via notification.
    ReadFile {
        /// Path to the file to read
        path: PathBuf,
    },

    /// Write a file
    ///
    /// Writes content to a file.
    WriteFile {
        /// Path to the file to write
        path: PathBuf,

        /// Content to write
        content: String,
    },

    /// Change the active LLM provider/model
    ///
    /// Switches which LLM is used for conductor operations.
    SetLlmModel {
        /// Provider identifier (e.g., "ollama", "claude")
        provider_id: String,

        /// Model name within the provider
        model: String,
    },

    /// Cancel a running agent
    ///
    /// Attempts to stop the specified agent.
    CancelAgent {
        /// The agent ID to cancel
        agent_id: AgentId,
    },

    /// List available LLM providers and their models
    ListProviders,

    /// List available CLI agents
    ListCliAgents,

    /// Get the current state snapshot
    ///
    /// Returns a snapshot of all agents, providers, etc.
    GetSnapshot,

    /// Select an output context (what to display)
    SelectContext {
        /// The context to display
        context: crate::types::OutputContext,
    },

    /// Shutdown the backend
    ///
    /// Gracefully shuts down all agents and services.
    Shutdown,

    // ========== Slash Commands ==========

    /// Execute a slash command
    ///
    /// Slash commands are user-facing commands that start with "/" (e.g., /help, /exit).
    SlashCommand {
        /// The parsed slash command
        command: slash::SlashCommand,
    },

    // ========== Workspace Commands ==========

    /// List all registered workspaces
    ListWorkspaces,

    /// Create a new workspace
    CreateWorkspace {
        /// Human-readable name
        name: String,

        /// Root path of the workspace
        path: PathBuf,
    },

    /// Delete a workspace (does not delete files)
    DeleteWorkspace {
        /// Workspace ID to delete
        workspace_id: WorkspaceId,
    },

    /// Activate a workspace (switch to it)
    ActivateWorkspace {
        /// Workspace ID to activate
        workspace_id: WorkspaceId,
    },

    /// Deactivate the current workspace
    DeactivateWorkspace,

    /// Get workspace details
    GetWorkspace {
        /// Workspace ID to get
        workspace_id: WorkspaceId,
    },

    /// Update workspace metadata
    UpdateWorkspace {
        /// Workspace ID to update
        workspace_id: WorkspaceId,

        /// New name (optional)
        name: Option<String>,
    },

    /// List files in a workspace directory
    ListFiles {
        /// Path relative to workspace root
        path: PathBuf,

        /// Include hidden files
        #[serde(default)]
        include_hidden: bool,
    },

    // ========== Ralph Loop Commands ==========

    /// Start a Ralph Loop for autonomous iterative development
    ///
    /// Begins an autonomous development loop that runs iterations until
    /// the task is complete or limits are reached.
    StartRalphLoop {
        /// The task description for the loop
        task: String,

        /// Configuration for the loop (optional, uses defaults if not provided)
        #[serde(default)]
        config: RalphConfig,
    },

    /// Stop the current Ralph Loop
    ///
    /// Gracefully terminates the active loop after the current iteration.
    StopRalphLoop,

    /// Get the status of the current Ralph Loop
    ///
    /// Returns the current state via a RalphStatusUpdate notification.
    GetRalphStatus,

    /// Provide feedback for the next Ralph Loop iteration
    ///
    /// This feedback will be included in the prompt for the next iteration.
    UpdateRalphFeedback {
        /// Feedback text for the next iteration
        feedback: String,
    },
}

impl Command {
    /// Create a ProcessInput command
    pub fn process_input(text: impl Into<String>) -> Self {
        Command::ProcessInput { text: text.into() }
    }

    /// Create an ExecuteShell command
    pub fn execute_shell(command: impl Into<String>) -> Self {
        Command::ExecuteShell {
            command: command.into(),
        }
    }

    /// Create an InvokeCliAgent command
    pub fn invoke_cli_agent(agent_id: impl Into<String>, prompt: impl Into<String>) -> Self {
        Command::InvokeCliAgent {
            agent_id: agent_id.into(),
            prompt: prompt.into(),
        }
    }

    /// Create a SendPtyInput command
    pub fn send_pty_input(agent_id: AgentId, data: Vec<u8>) -> Self {
        Command::SendPtyInput { agent_id, data }
    }

    /// Create a ResizePty command
    pub fn resize_pty(agent_id: AgentId, cols: u16, rows: u16) -> Self {
        Command::ResizePty {
            agent_id,
            cols,
            rows,
        }
    }

    /// Create a ReadFile command
    pub fn read_file(path: impl Into<PathBuf>) -> Self {
        Command::ReadFile { path: path.into() }
    }

    /// Create a WriteFile command
    pub fn write_file(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Command::WriteFile {
            path: path.into(),
            content: content.into(),
        }
    }

    /// Create a SetLlmModel command
    pub fn set_llm_model(provider_id: impl Into<String>, model: impl Into<String>) -> Self {
        Command::SetLlmModel {
            provider_id: provider_id.into(),
            model: model.into(),
        }
    }

    /// Create a CancelAgent command
    pub fn cancel_agent(agent_id: AgentId) -> Self {
        Command::CancelAgent { agent_id }
    }

    /// Create a CreateWorkspace command
    pub fn create_workspace(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Command::CreateWorkspace {
            name: name.into(),
            path: path.into(),
        }
    }

    /// Create a DeleteWorkspace command
    pub fn delete_workspace(workspace_id: WorkspaceId) -> Self {
        Command::DeleteWorkspace { workspace_id }
    }

    /// Create an ActivateWorkspace command
    pub fn activate_workspace(workspace_id: WorkspaceId) -> Self {
        Command::ActivateWorkspace { workspace_id }
    }

    /// Create a ListFiles command
    pub fn list_files(path: impl Into<PathBuf>) -> Self {
        Command::ListFiles {
            path: path.into(),
            include_hidden: false,
        }
    }

    /// Create a SlashCommand command
    pub fn slash_command(command: slash::SlashCommand) -> Self {
        Command::SlashCommand { command }
    }

    /// Create a StartRalphLoop command with default config
    pub fn start_ralph_loop(task: impl Into<String>) -> Self {
        Command::StartRalphLoop {
            task: task.into(),
            config: RalphConfig::default(),
        }
    }

    /// Create a StartRalphLoop command with custom config
    pub fn start_ralph_loop_with_config(task: impl Into<String>, config: RalphConfig) -> Self {
        Command::StartRalphLoop {
            task: task.into(),
            config,
        }
    }

    /// Create an UpdateRalphFeedback command
    pub fn update_ralph_feedback(feedback: impl Into<String>) -> Self {
        Command::UpdateRalphFeedback {
            feedback: feedback.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_serialization() {
        let cmd = Command::process_input("hello world");
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("ProcessInput"));
        assert!(json.contains("hello world"));

        let parsed: Command = serde_json::from_str(&json).unwrap();
        match parsed {
            Command::ProcessInput { text } => assert_eq!(text, "hello world"),
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_cli_agent_command() {
        let cmd = Command::invoke_cli_agent("claude", "explain this code");
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("InvokeCliAgent"));
        assert!(json.contains("claude"));
        assert!(json.contains("explain this code"));
    }

    #[test]
    fn test_pty_input_command() {
        let cmd = Command::send_pty_input(AgentId::new(42), vec![0x1b, 0x5b, 0x41]); // ESC [ A
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("SendPtyInput"));
    }

    #[test]
    fn test_start_ralph_loop_command() {
        let cmd = Command::start_ralph_loop("implement feature X");
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("StartRalphLoop"));
        assert!(json.contains("implement feature X"));

        let parsed: Command = serde_json::from_str(&json).unwrap();
        match parsed {
            Command::StartRalphLoop { task, config } => {
                assert_eq!(task, "implement feature X");
                assert_eq!(config.max_iterations, 20); // default
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_start_ralph_loop_with_config() {
        let config = RalphConfig::default().with_max_iterations(10);
        let cmd = Command::start_ralph_loop_with_config("fix bugs", config);
        let json = serde_json::to_string(&cmd).unwrap();

        let parsed: Command = serde_json::from_str(&json).unwrap();
        match parsed {
            Command::StartRalphLoop { task, config } => {
                assert_eq!(task, "fix bugs");
                assert_eq!(config.max_iterations, 10);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_stop_ralph_loop_command() {
        let cmd = Command::StopRalphLoop;
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("StopRalphLoop"));

        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Command::StopRalphLoop));
    }

    #[test]
    fn test_get_ralph_status_command() {
        let cmd = Command::GetRalphStatus;
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("GetRalphStatus"));
    }

    #[test]
    fn test_update_ralph_feedback_command() {
        let cmd = Command::update_ralph_feedback("focus on error handling");
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("UpdateRalphFeedback"));
        assert!(json.contains("focus on error handling"));

        let parsed: Command = serde_json::from_str(&json).unwrap();
        match parsed {
            Command::UpdateRalphFeedback { feedback } => {
                assert_eq!(feedback, "focus on error handling");
            }
            _ => panic!("Wrong command type"),
        }
    }
}
