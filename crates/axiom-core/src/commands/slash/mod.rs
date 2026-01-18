//! Slash command support
//!
//! This module provides slash command parsing and execution for Axiom.
//! Slash commands start with "/" and provide quick access to common actions.
//!
//! # Example
//!
//! ```ignore
//! use axiom_core::commands::slash::{SlashCommand, SlashCommandParser};
//!
//! // Parse a slash command
//! if let Some(Ok(cmd)) = SlashCommandParser::parse("/help") {
//!     match cmd {
//!         SlashCommand::Help { command } => println!("Show help for: {:?}", command),
//!         _ => {}
//!     }
//! }
//! ```

mod parser;
mod types;

pub use parser::{ParseError, SlashCommandParser};
pub use types::*;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Slash command enumeration
///
/// Represents all supported slash commands that can be parsed from user input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "args")]
pub enum SlashCommand {
    /// Show help information
    ///
    /// - `/help` - show all commands
    /// - `/help <command>` - show help for specific command
    Help {
        /// Optional command to get help for
        command: Option<String>,
    },

    /// Clear the output panel
    ///
    /// Aliases: `/cls`
    Clear,

    /// Open settings panel
    ///
    /// Aliases: `/config`
    Settings,

    /// Exit the application
    ///
    /// Aliases: `/quit`, `/q`
    Exit,

    /// Show version information
    ///
    /// Aliases: `/v`
    Version,

    /// Initialize a new workspace
    ///
    /// - `/init` - initialize in current directory
    /// - `/init <path>` - initialize at specified path
    Init {
        /// Optional path to initialize
        path: Option<PathBuf>,
    },

    /// Workspace management commands
    Workspace(WorkspaceSubcommand),

    /// Model selection commands
    Model(ModelSubcommand),

    /// Theme commands
    Theme(ThemeSubcommand),

    /// Custom/extension command
    ///
    /// For commands not built-in, allows extensions to handle them
    Custom {
        /// Command name (without /)
        name: String,
        /// Arguments passed to the command
        args: Vec<String>,
    },
}

/// Workspace management subcommands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subcommand")]
pub enum WorkspaceSubcommand {
    /// List all workspaces
    ///
    /// `/workspace list` or `/workspace`
    List,

    /// Switch to a workspace
    ///
    /// - `/workspace switch` - open workspace selector
    /// - `/workspace switch <id>` - switch to specific workspace
    Switch {
        /// Optional workspace ID to switch to
        id: Option<String>,
    },

    /// Create a new workspace
    ///
    /// `/workspace create <name> <path>`
    Create {
        /// Workspace name
        name: String,
        /// Workspace root path
        path: PathBuf,
    },
}

/// Model management subcommands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subcommand")]
pub enum ModelSubcommand {
    /// List available models
    ///
    /// `/model list` or `/model`
    List,

    /// Set the active model
    ///
    /// `/model set <model_name>`
    Set {
        /// Model name to activate
        model: String,
    },

    /// Show currently active model
    ///
    /// `/model current`
    Current,
}

/// Theme management subcommands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subcommand")]
pub enum ThemeSubcommand {
    /// Toggle between light and dark themes
    ///
    /// `/theme toggle` or `/theme`
    Toggle,

    /// Set a specific theme variant
    ///
    /// `/theme set <variant>` or `/theme dark` or `/theme light`
    Set {
        /// Theme variant name
        variant: String,
    },
}

impl SlashCommand {
    /// Get the command name (for display purposes)
    pub fn name(&self) -> &'static str {
        match self {
            SlashCommand::Help { .. } => "help",
            SlashCommand::Clear => "clear",
            SlashCommand::Settings => "settings",
            SlashCommand::Exit => "exit",
            SlashCommand::Version => "version",
            SlashCommand::Init { .. } => "init",
            SlashCommand::Workspace(_) => "workspace",
            SlashCommand::Model(_) => "model",
            SlashCommand::Theme(_) => "theme",
            SlashCommand::Custom { .. } => "custom",
        }
    }

    /// Check if this command should exit the application
    pub fn is_exit(&self) -> bool {
        matches!(self, SlashCommand::Exit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_command_name() {
        assert_eq!(SlashCommand::Help { command: None }.name(), "help");
        assert_eq!(SlashCommand::Clear.name(), "clear");
        assert_eq!(SlashCommand::Exit.name(), "exit");
    }

    #[test]
    fn test_is_exit() {
        assert!(SlashCommand::Exit.is_exit());
        assert!(!SlashCommand::Clear.is_exit());
        assert!(!SlashCommand::Help { command: None }.is_exit());
    }

    #[test]
    fn test_slash_command_serialize() {
        let cmd = SlashCommand::Help {
            command: Some("workspace".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("Help"));
        assert!(json.contains("workspace"));
    }

    #[test]
    fn test_slash_command_deserialize() {
        let json = r#"{"type":"Help","args":{"command":"workspace"}}"#;
        let cmd: SlashCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(
            cmd,
            SlashCommand::Help { command: Some(c) } if c == "workspace"
        ));
    }

    #[test]
    fn test_workspace_subcommand_serialize() {
        let sub = WorkspaceSubcommand::Switch {
            id: Some("proj1".to_string()),
        };
        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("Switch"));
        assert!(json.contains("proj1"));
    }

    #[test]
    fn test_model_subcommand_serialize() {
        let sub = ModelSubcommand::Set {
            model: "gpt-4".to_string(),
        };
        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("Set"));
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_theme_subcommand_serialize() {
        let sub = ThemeSubcommand::Set {
            variant: "dark".to_string(),
        };
        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("Set"));
        assert!(json.contains("dark"));
    }
}
