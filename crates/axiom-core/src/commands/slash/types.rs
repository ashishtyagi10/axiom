//! Slash command result types
//!
//! These types represent the results of slash command execution.

use serde::{Deserialize, Serialize};

/// Result of executing a slash command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SlashCommandResult {
    /// Command succeeded with optional message
    Success {
        /// Optional status message to display
        message: Option<String>,
    },

    /// Command requires a UI action
    UiAction(UiAction),

    /// Command returned data to display
    Data(SlashCommandData),

    /// Command failed with error message
    Error {
        /// Error description
        message: String,
    },

    /// Command requests application exit
    Exit,
}

impl SlashCommandResult {
    /// Create a success result with a message
    pub fn success(message: impl Into<String>) -> Self {
        SlashCommandResult::Success {
            message: Some(message.into()),
        }
    }

    /// Create a success result without a message
    pub fn ok() -> Self {
        SlashCommandResult::Success { message: None }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        SlashCommandResult::Error {
            message: message.into(),
        }
    }

    /// Create an exit result
    pub fn exit() -> Self {
        SlashCommandResult::Exit
    }

    /// Create a UI action result
    pub fn action(action: UiAction) -> Self {
        SlashCommandResult::UiAction(action)
    }

    /// Create a data result
    pub fn data(data: SlashCommandData) -> Self {
        SlashCommandResult::Data(data)
    }
}

/// UI action to be performed by the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum UiAction {
    /// Open the settings modal/panel
    OpenSettings,

    /// Open the model selector
    OpenModelSelector,

    /// Open the workspace selector
    OpenWorkspaceSelector,

    /// Clear the output panel
    ClearOutput,

    /// Toggle between light/dark theme
    ToggleTheme,

    /// Set a specific theme variant
    SetTheme {
        /// Theme variant name
        variant: String,
    },

    /// Focus a specific panel
    FocusPanel {
        /// Panel identifier
        panel: String,
    },
}

/// Data returned by slash commands
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "data_type", content = "value")]
pub enum SlashCommandData {
    /// Help information for commands
    Help {
        /// List of available commands
        commands: Vec<CommandHelp>,
    },

    /// Version information
    Version {
        /// Version string
        version: String,
        /// Git commit hash (optional)
        commit: Option<String>,
    },

    /// List of workspaces
    WorkspaceList(Vec<WorkspaceInfo>),

    /// List of available models
    ModelList {
        /// Provider ID
        provider: String,
        /// List of model names
        models: Vec<String>,
        /// Currently active model
        active: Option<String>,
    },

    /// Generic text output
    Text(String),
}

/// Help information for a single command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHelp {
    /// Command name (without /)
    pub name: String,
    /// Aliases for the command
    pub aliases: Vec<String>,
    /// Brief description
    pub description: String,
    /// Usage syntax
    pub usage: String,
    /// Example usages
    pub examples: Vec<String>,
}

/// Workspace information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// Workspace identifier
    pub id: String,
    /// Workspace name
    pub name: String,
    /// Workspace path
    pub path: String,
    /// Whether this is the active workspace
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_success() {
        let result = SlashCommandResult::success("Done!");
        assert!(matches!(
            result,
            SlashCommandResult::Success {
                message: Some(msg)
            } if msg == "Done!"
        ));
    }

    #[test]
    fn test_result_ok() {
        let result = SlashCommandResult::ok();
        assert!(matches!(
            result,
            SlashCommandResult::Success { message: None }
        ));
    }

    #[test]
    fn test_result_error() {
        let result = SlashCommandResult::error("Failed");
        assert!(matches!(
            result,
            SlashCommandResult::Error { message } if message == "Failed"
        ));
    }

    #[test]
    fn test_result_serialization() {
        let result = SlashCommandResult::success("Test");
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Success"));
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_ui_action_serialization() {
        let action = UiAction::OpenSettings;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("OpenSettings"));
    }

    #[test]
    fn test_data_serialization() {
        let data = SlashCommandData::Version {
            version: "1.0.0".to_string(),
            commit: Some("abc123".to_string()),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("Version"));
        assert!(json.contains("1.0.0"));
    }
}
