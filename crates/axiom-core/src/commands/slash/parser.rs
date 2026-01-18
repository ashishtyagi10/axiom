//! Slash command parser
//!
//! Parses user input strings starting with "/" into structured commands.

use super::{ModelSubcommand, SlashCommand, ThemeSubcommand, WorkspaceSubcommand};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during slash command parsing
#[derive(Debug, Clone, Error)]
pub enum ParseError {
    /// Empty command (just "/")
    #[error("Empty command")]
    Empty,

    /// Unknown command
    #[error("Unknown command: {0}")]
    UnknownCommand(String),

    /// Missing required argument
    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    /// Invalid argument format
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

/// Parser for slash commands
pub struct SlashCommandParser;

impl SlashCommandParser {
    /// Parse a string into a slash command
    ///
    /// Returns `None` if the input doesn't start with "/"
    /// Returns `Some(Ok(...))` if parsing succeeds
    /// Returns `Some(Err(...))` if parsing fails
    pub fn parse(input: &str) -> Option<Result<SlashCommand, ParseError>> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        Some(Self::parse_command(&trimmed[1..]))
    }

    /// Parse the command portion (after the "/" prefix)
    fn parse_command(input: &str) -> Result<SlashCommand, ParseError> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        let (cmd, args) = parts.split_first().ok_or(ParseError::Empty)?;

        match cmd.to_lowercase().as_str() {
            // Help commands
            "help" | "h" | "?" => Ok(SlashCommand::Help {
                command: args.first().map(|s| s.to_string()),
            }),

            // Clear output
            "clear" | "cls" => Ok(SlashCommand::Clear),

            // Settings
            "settings" | "config" => Ok(SlashCommand::Settings),

            // Exit/Quit
            "exit" | "quit" | "q" => Ok(SlashCommand::Exit),

            // Version
            "version" | "v" => Ok(SlashCommand::Version),

            // Initialize workspace
            "init" => Ok(SlashCommand::Init {
                path: args.first().map(PathBuf::from),
            }),

            // Workspace commands
            "workspace" | "ws" => Self::parse_workspace(args),

            // Model commands
            "model" | "m" => Self::parse_model(args),

            // Theme commands
            "theme" => Self::parse_theme(args),

            // Unknown command - treat as custom
            other => Ok(SlashCommand::Custom {
                name: other.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
            }),
        }
    }

    /// Parse workspace subcommand
    fn parse_workspace(args: &[&str]) -> Result<SlashCommand, ParseError> {
        let subcommand = args.first().map(|s| s.to_lowercase());

        match subcommand.as_deref() {
            Some("list") | Some("ls") | None => {
                Ok(SlashCommand::Workspace(WorkspaceSubcommand::List))
            }
            Some("switch") | Some("sw") => {
                let id = args.get(1).map(|s| s.to_string());
                Ok(SlashCommand::Workspace(WorkspaceSubcommand::Switch { id }))
            }
            Some("create") | Some("new") => {
                let name = args
                    .get(1)
                    .ok_or_else(|| ParseError::MissingArgument("workspace name".to_string()))?
                    .to_string();
                let path = args
                    .get(2)
                    .map(PathBuf::from)
                    .ok_or_else(|| ParseError::MissingArgument("workspace path".to_string()))?;
                Ok(SlashCommand::Workspace(WorkspaceSubcommand::Create {
                    name,
                    path,
                }))
            }
            Some(other) => Err(ParseError::UnknownCommand(format!("workspace {}", other))),
        }
    }

    /// Parse model subcommand
    fn parse_model(args: &[&str]) -> Result<SlashCommand, ParseError> {
        let subcommand = args.first().map(|s| s.to_lowercase());

        match subcommand.as_deref() {
            Some("list") | Some("ls") | None => {
                Ok(SlashCommand::Model(ModelSubcommand::List))
            }
            Some("set") | Some("use") => {
                let model = args
                    .get(1)
                    .ok_or_else(|| ParseError::MissingArgument("model name".to_string()))?
                    .to_string();
                Ok(SlashCommand::Model(ModelSubcommand::Set { model }))
            }
            Some("current") | Some("show") => {
                Ok(SlashCommand::Model(ModelSubcommand::Current))
            }
            Some(other) => Err(ParseError::UnknownCommand(format!("model {}", other))),
        }
    }

    /// Parse theme subcommand
    fn parse_theme(args: &[&str]) -> Result<SlashCommand, ParseError> {
        let subcommand = args.first().map(|s| s.to_lowercase());

        match subcommand.as_deref() {
            Some("toggle") | None => Ok(SlashCommand::Theme(ThemeSubcommand::Toggle)),
            Some("set") => {
                let variant = args
                    .get(1)
                    .ok_or_else(|| ParseError::MissingArgument("theme variant".to_string()))?
                    .to_string();
                Ok(SlashCommand::Theme(ThemeSubcommand::Set { variant }))
            }
            Some("dark") => Ok(SlashCommand::Theme(ThemeSubcommand::Set {
                variant: "dark".to_string(),
            })),
            Some("light") => Ok(SlashCommand::Theme(ThemeSubcommand::Set {
                variant: "light".to_string(),
            })),
            Some(other) => Err(ParseError::UnknownCommand(format!("theme {}", other))),
        }
    }

    /// Get help for all commands
    pub fn get_all_commands_help() -> Vec<super::types::CommandHelp> {
        use super::types::CommandHelp;

        vec![
            CommandHelp {
                name: "help".to_string(),
                aliases: vec!["h".to_string(), "?".to_string()],
                description: "Show help information".to_string(),
                usage: "/help [command]".to_string(),
                examples: vec!["/help".to_string(), "/help workspace".to_string()],
            },
            CommandHelp {
                name: "clear".to_string(),
                aliases: vec!["cls".to_string()],
                description: "Clear the output panel".to_string(),
                usage: "/clear".to_string(),
                examples: vec!["/clear".to_string()],
            },
            CommandHelp {
                name: "settings".to_string(),
                aliases: vec!["config".to_string()],
                description: "Open settings panel".to_string(),
                usage: "/settings".to_string(),
                examples: vec!["/settings".to_string()],
            },
            CommandHelp {
                name: "exit".to_string(),
                aliases: vec!["quit".to_string(), "q".to_string()],
                description: "Exit the application".to_string(),
                usage: "/exit".to_string(),
                examples: vec!["/exit".to_string(), "/q".to_string()],
            },
            CommandHelp {
                name: "version".to_string(),
                aliases: vec!["v".to_string()],
                description: "Show version information".to_string(),
                usage: "/version".to_string(),
                examples: vec!["/version".to_string()],
            },
            CommandHelp {
                name: "init".to_string(),
                aliases: vec![],
                description: "Initialize a new workspace".to_string(),
                usage: "/init [path]".to_string(),
                examples: vec!["/init".to_string(), "/init ./myproject".to_string()],
            },
            CommandHelp {
                name: "workspace".to_string(),
                aliases: vec!["ws".to_string()],
                description: "Workspace management".to_string(),
                usage: "/workspace <subcommand>".to_string(),
                examples: vec![
                    "/workspace list".to_string(),
                    "/workspace switch myproject".to_string(),
                    "/workspace create myproject /path/to/project".to_string(),
                ],
            },
            CommandHelp {
                name: "model".to_string(),
                aliases: vec!["m".to_string()],
                description: "Model selection".to_string(),
                usage: "/model <subcommand>".to_string(),
                examples: vec![
                    "/model list".to_string(),
                    "/model set claude-sonnet".to_string(),
                    "/model current".to_string(),
                ],
            },
            CommandHelp {
                name: "theme".to_string(),
                aliases: vec![],
                description: "Theme management".to_string(),
                usage: "/theme <subcommand>".to_string(),
                examples: vec![
                    "/theme toggle".to_string(),
                    "/theme dark".to_string(),
                    "/theme light".to_string(),
                ],
            },
        ]
    }

    /// Get help for a specific command
    pub fn get_command_help(name: &str) -> Option<super::types::CommandHelp> {
        Self::get_all_commands_help()
            .into_iter()
            .find(|h| h.name == name || h.aliases.contains(&name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Basic Parsing ====================

    #[test]
    fn test_non_slash_returns_none() {
        assert!(SlashCommandParser::parse("hello").is_none());
        assert!(SlashCommandParser::parse("").is_none());
        assert!(SlashCommandParser::parse("# agent").is_none());
    }

    #[test]
    fn test_empty_slash() {
        let result = SlashCommandParser::parse("/");
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), Err(ParseError::Empty)));
    }

    #[test]
    fn test_slash_only_spaces() {
        let result = SlashCommandParser::parse("/   ");
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), Err(ParseError::Empty)));
    }

    // ==================== Help Command ====================

    #[test]
    fn test_help_command() {
        let result = SlashCommandParser::parse("/help").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Help { command: None }));
    }

    #[test]
    fn test_help_with_topic() {
        let result = SlashCommandParser::parse("/help workspace").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Help { command: Some(cmd) } if cmd == "workspace"
        ));
    }

    #[test]
    fn test_help_aliases() {
        let h = SlashCommandParser::parse("/h").unwrap().unwrap();
        let q = SlashCommandParser::parse("/?").unwrap().unwrap();
        assert!(matches!(h, SlashCommand::Help { .. }));
        assert!(matches!(q, SlashCommand::Help { .. }));
    }

    // ==================== Clear Command ====================

    #[test]
    fn test_clear_command() {
        let result = SlashCommandParser::parse("/clear").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Clear));
    }

    #[test]
    fn test_cls_alias() {
        let result = SlashCommandParser::parse("/cls").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Clear));
    }

    // ==================== Settings Command ====================

    #[test]
    fn test_settings_command() {
        let result = SlashCommandParser::parse("/settings").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Settings));
    }

    #[test]
    fn test_config_alias() {
        let result = SlashCommandParser::parse("/config").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Settings));
    }

    // ==================== Exit Command ====================

    #[test]
    fn test_exit_command() {
        let result = SlashCommandParser::parse("/exit").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Exit));
    }

    #[test]
    fn test_quit_alias() {
        let result = SlashCommandParser::parse("/quit").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Exit));
    }

    #[test]
    fn test_q_alias() {
        let result = SlashCommandParser::parse("/q").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Exit));
    }

    // ==================== Version Command ====================

    #[test]
    fn test_version_command() {
        let result = SlashCommandParser::parse("/version").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Version));
    }

    #[test]
    fn test_v_alias() {
        let result = SlashCommandParser::parse("/v").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Version));
    }

    // ==================== Init Command ====================

    #[test]
    fn test_init_no_path() {
        let result = SlashCommandParser::parse("/init").unwrap().unwrap();
        assert!(matches!(result, SlashCommand::Init { path: None }));
    }

    #[test]
    fn test_init_with_path() {
        let result = SlashCommandParser::parse("/init ./myproject").unwrap().unwrap();
        if let SlashCommand::Init { path: Some(p) } = result {
            assert_eq!(p, PathBuf::from("./myproject"));
        } else {
            panic!("Expected Init with path");
        }
    }

    // ==================== Workspace Commands ====================

    #[test]
    fn test_workspace_list() {
        let result = SlashCommandParser::parse("/workspace list").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Workspace(WorkspaceSubcommand::List)
        ));
    }

    #[test]
    fn test_workspace_default_list() {
        let result = SlashCommandParser::parse("/workspace").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Workspace(WorkspaceSubcommand::List)
        ));
    }

    #[test]
    fn test_workspace_switch() {
        let result = SlashCommandParser::parse("/workspace switch myproj")
            .unwrap()
            .unwrap();
        assert!(matches!(
            result,
            SlashCommand::Workspace(WorkspaceSubcommand::Switch { id: Some(i) }) if i == "myproj"
        ));
    }

    #[test]
    fn test_workspace_switch_no_id() {
        let result = SlashCommandParser::parse("/workspace switch").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Workspace(WorkspaceSubcommand::Switch { id: None })
        ));
    }

    #[test]
    fn test_workspace_create() {
        let result = SlashCommandParser::parse("/workspace create myproj /path/to/proj")
            .unwrap()
            .unwrap();
        if let SlashCommand::Workspace(WorkspaceSubcommand::Create { name, path }) = result {
            assert_eq!(name, "myproj");
            assert_eq!(path, PathBuf::from("/path/to/proj"));
        } else {
            panic!("Expected workspace create");
        }
    }

    #[test]
    fn test_workspace_create_missing_path() {
        let result = SlashCommandParser::parse("/workspace create myproj").unwrap();
        assert!(matches!(result, Err(ParseError::MissingArgument(_))));
    }

    #[test]
    fn test_workspace_create_missing_name() {
        let result = SlashCommandParser::parse("/workspace create").unwrap();
        assert!(matches!(result, Err(ParseError::MissingArgument(_))));
    }

    #[test]
    fn test_ws_alias() {
        let result = SlashCommandParser::parse("/ws list").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Workspace(WorkspaceSubcommand::List)
        ));
    }

    // ==================== Model Commands ====================

    #[test]
    fn test_model_list() {
        let result = SlashCommandParser::parse("/model list").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Model(ModelSubcommand::List)
        ));
    }

    #[test]
    fn test_model_default_list() {
        let result = SlashCommandParser::parse("/model").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Model(ModelSubcommand::List)
        ));
    }

    #[test]
    fn test_model_set() {
        let result = SlashCommandParser::parse("/model set gpt-4").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Model(ModelSubcommand::Set { model }) if model == "gpt-4"
        ));
    }

    #[test]
    fn test_model_set_missing() {
        let result = SlashCommandParser::parse("/model set").unwrap();
        assert!(matches!(result, Err(ParseError::MissingArgument(_))));
    }

    #[test]
    fn test_model_current() {
        let result = SlashCommandParser::parse("/model current").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Model(ModelSubcommand::Current)
        ));
    }

    #[test]
    fn test_m_alias() {
        let result = SlashCommandParser::parse("/m list").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Model(ModelSubcommand::List)
        ));
    }

    // ==================== Theme Commands ====================

    #[test]
    fn test_theme_toggle() {
        let result = SlashCommandParser::parse("/theme toggle").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Theme(ThemeSubcommand::Toggle)
        ));
    }

    #[test]
    fn test_theme_default_toggle() {
        let result = SlashCommandParser::parse("/theme").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Theme(ThemeSubcommand::Toggle)
        ));
    }

    #[test]
    fn test_theme_set() {
        let result = SlashCommandParser::parse("/theme set monokai").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Theme(ThemeSubcommand::Set { variant }) if variant == "monokai"
        ));
    }

    #[test]
    fn test_theme_dark() {
        let result = SlashCommandParser::parse("/theme dark").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Theme(ThemeSubcommand::Set { variant }) if variant == "dark"
        ));
    }

    #[test]
    fn test_theme_light() {
        let result = SlashCommandParser::parse("/theme light").unwrap().unwrap();
        assert!(matches!(
            result,
            SlashCommand::Theme(ThemeSubcommand::Set { variant }) if variant == "light"
        ));
    }

    // ==================== Custom Commands ====================

    #[test]
    fn test_unknown_command_becomes_custom() {
        let result = SlashCommandParser::parse("/mycustom arg1 arg2").unwrap().unwrap();
        if let SlashCommand::Custom { name, args } = result {
            assert_eq!(name, "mycustom");
            assert_eq!(args, vec!["arg1", "arg2"]);
        } else {
            panic!("Expected custom command");
        }
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_case_insensitive() {
        let lower = SlashCommandParser::parse("/help").unwrap().unwrap();
        let upper = SlashCommandParser::parse("/HELP").unwrap().unwrap();
        let mixed = SlashCommandParser::parse("/HeLp").unwrap().unwrap();

        assert!(matches!(lower, SlashCommand::Help { .. }));
        assert!(matches!(upper, SlashCommand::Help { .. }));
        assert!(matches!(mixed, SlashCommand::Help { .. }));
    }

    // ==================== Help System ====================

    #[test]
    fn test_get_all_commands_help() {
        let help = SlashCommandParser::get_all_commands_help();
        assert!(!help.is_empty());
        assert!(help.iter().any(|h| h.name == "help"));
        assert!(help.iter().any(|h| h.name == "exit"));
    }

    #[test]
    fn test_get_command_help() {
        let help = SlashCommandParser::get_command_help("help");
        assert!(help.is_some());
        assert_eq!(help.unwrap().name, "help");
    }

    #[test]
    fn test_get_command_help_by_alias() {
        let help = SlashCommandParser::get_command_help("q");
        assert!(help.is_some());
        assert_eq!(help.unwrap().name, "exit");
    }
}
