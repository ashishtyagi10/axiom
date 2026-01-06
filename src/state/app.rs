//! Central application state container
//!
//! Uses composition instead of a god object with 40+ fields.

use super::{FocusState, InputMode};

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
        }
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
}
