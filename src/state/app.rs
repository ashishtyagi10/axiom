//! Central application state container
//!
//! Uses composition instead of a god object with 40+ fields.

use super::{FocusState, InputMode};
use axiom_core::{Workspace, WorkspaceId, WorkspaceManager};
use std::sync::Arc;

/// Central application state
///
/// This is intentionally minimal - panel-specific state lives in panels.
pub struct AppState {
    /// Current input mode
    pub input_mode: InputMode,

    /// Focus management
    pub focus: FocusState,

    /// Application should quit
    pub should_quit: bool,

    /// Status bar message (if any)
    pub status_message: Option<StatusMessage>,

    /// Current working directory
    pub cwd: std::path::PathBuf,

    /// Workspace manager (optional, initialized on demand)
    pub workspace_manager: Option<Arc<WorkspaceManager>>,

    /// Active workspace ID (if any)
    pub active_workspace_id: Option<WorkspaceId>,
}

/// Status bar message with optional timeout
pub struct StatusMessage {
    pub text: String,
    pub level: MessageLevel,
}

/// Message severity level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| {
            dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"))
        });

        Self {
            input_mode: InputMode::default(),
            focus: FocusState::new(),
            should_quit: false,
            status_message: None,
            cwd,
            workspace_manager: None,
            active_workspace_id: None,
        }
    }

    /// Create with a specific working directory
    pub fn with_cwd(cwd: std::path::PathBuf) -> Self {
        Self {
            input_mode: InputMode::default(),
            focus: FocusState::new(),
            should_quit: false,
            status_message: None,
            cwd,
            workspace_manager: None,
            active_workspace_id: None,
        }
    }

    /// Initialize workspace manager with global config
    pub fn init_workspace_manager(&mut self, config: axiom_core::AxiomConfig) -> crate::core::Result<()> {
        match WorkspaceManager::new(config) {
            Ok(manager) => {
                let manager = Arc::new(manager);

                // Check if there's an existing workspace for the current directory
                if let Some(workspace) = manager.find_by_path(&self.cwd) {
                    self.active_workspace_id = Some(workspace.id);
                }

                self.workspace_manager = Some(manager);
                Ok(())
            }
            Err(e) => Err(crate::core::AxiomError::Config(format!(
                "Failed to initialize workspace manager: {}",
                e
            ))),
        }
    }

    /// Get the active workspace (if any)
    pub fn active_workspace(&self) -> Option<Workspace> {
        let manager = self.workspace_manager.as_ref()?;
        self.active_workspace_id.and_then(|id| manager.get_workspace(id))
    }

    /// Get the workspace name for display (or directory name if no workspace)
    pub fn workspace_name(&self) -> String {
        self.active_workspace()
            .map(|ws| ws.name.clone())
            .unwrap_or_else(|| {
                self.cwd
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("axiom")
                    .to_string()
            })
    }

    /// Switch to a different workspace
    pub fn switch_workspace(&mut self, id: WorkspaceId) -> crate::core::Result<std::path::PathBuf> {
        let manager = self.workspace_manager.as_ref().ok_or_else(|| {
            crate::core::AxiomError::Config("Workspace manager not initialized".into())
        })?;

        // Get the workspace to validate it exists
        let workspace = manager.get_workspace(id).ok_or_else(|| {
            crate::core::AxiomError::Config(format!("Workspace not found: {}", id))
        })?;

        // Validate path exists
        if !workspace.path.exists() {
            return Err(crate::core::AxiomError::Config(format!(
                "Workspace path does not exist: {}",
                workspace.path.display()
            )));
        }

        // Update state
        let new_path = workspace.path.clone();
        self.cwd = new_path.clone();
        self.active_workspace_id = Some(id);

        // Activate in manager (updates last_accessed, etc.)
        let _ = manager.activate_workspace(id);

        Ok(new_path)
    }

    /// Request application quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Set status message
    pub fn set_status(&mut self, text: impl Into<String>, level: MessageLevel) {
        self.status_message = Some(StatusMessage {
            text: text.into(),
            level,
        });
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Set info status
    pub fn info(&mut self, text: impl Into<String>) {
        self.set_status(text, MessageLevel::Info);
    }

    /// Set error status
    pub fn error(&mut self, text: impl Into<String>) {
        self.set_status(text, MessageLevel::Error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = AppState::new();
        assert!(!state.should_quit);
        assert_eq!(state.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_default_state() {
        let state = AppState::default();
        assert!(!state.should_quit);
        assert_eq!(state.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_quit() {
        let mut state = AppState::new();
        state.quit();
        assert!(state.should_quit);
    }

    #[test]
    fn test_status_message() {
        let mut state = AppState::new();

        state.info("Hello");
        assert!(state.status_message.is_some());

        state.clear_status();
        assert!(state.status_message.is_none());
    }

    #[test]
    fn test_status_info() {
        let mut state = AppState::new();
        state.info("Info message");

        let msg = state.status_message.as_ref().unwrap();
        assert_eq!(msg.text, "Info message");
        assert_eq!(msg.level, MessageLevel::Info);
    }

    #[test]
    fn test_status_error() {
        let mut state = AppState::new();
        state.error("Error message");

        let msg = state.status_message.as_ref().unwrap();
        assert_eq!(msg.text, "Error message");
        assert_eq!(msg.level, MessageLevel::Error);
    }

    #[test]
    fn test_set_status_warning() {
        let mut state = AppState::new();
        state.set_status("Warning message", MessageLevel::Warning);

        let msg = state.status_message.as_ref().unwrap();
        assert_eq!(msg.text, "Warning message");
        assert_eq!(msg.level, MessageLevel::Warning);
    }

    #[test]
    fn test_status_overwrite() {
        let mut state = AppState::new();

        state.info("First");
        state.info("Second");

        let msg = state.status_message.as_ref().unwrap();
        assert_eq!(msg.text, "Second");
    }

    #[test]
    fn test_cwd_exists() {
        let state = AppState::new();
        // CWD should be set to something
        assert!(!state.cwd.as_os_str().is_empty());
    }

    #[test]
    fn test_message_level_equality() {
        assert_eq!(MessageLevel::Info, MessageLevel::Info);
        assert_eq!(MessageLevel::Warning, MessageLevel::Warning);
        assert_eq!(MessageLevel::Error, MessageLevel::Error);
        assert_ne!(MessageLevel::Info, MessageLevel::Error);
    }

    #[test]
    fn test_message_level_debug() {
        let level = MessageLevel::Info;
        let debug = format!("{:?}", level);
        assert!(debug.contains("Info"));
    }

    #[test]
    fn test_message_level_clone() {
        let level = MessageLevel::Warning;
        let cloned = level.clone();
        assert_eq!(level, cloned);
    }

    #[test]
    fn test_app_state_focus_accessible() {
        let state = AppState::new();
        // Should be able to access focus state
        let _ = state.focus.current();
    }

    #[test]
    fn test_app_state_input_mode_accessible() {
        let mut state = AppState::new();
        state.input_mode = InputMode::Insert;
        assert_eq!(state.input_mode, InputMode::Insert);
    }
}
