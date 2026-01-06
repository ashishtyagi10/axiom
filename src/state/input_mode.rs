//! Input mode state machine
//!
//! Explicit state machine for input handling modes.
//! Prevents scattered if-else chains and mode confusion.

/// Input mode state machine
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    /// Normal navigation mode - arrow keys move between panels
    Normal,

    /// Text insertion mode - typing goes to active text field
    Insert,

    /// Command palette is open
    Command { query: String },

    /// Search mode with direction
    Search { query: String, forward: bool },

    /// Modal dialog is open (settings, confirm, etc.)
    Modal { name: String },
}

impl Default for InputMode {
    fn default() -> Self {
        Self::Normal
    }
}

impl InputMode {
    /// Check if currently in a text editing mode
    pub fn is_editing(&self) -> bool {
        matches!(self, InputMode::Insert | InputMode::Command { .. } | InputMode::Search { .. })
    }

    /// Check if a modal/overlay is open
    pub fn is_modal(&self) -> bool {
        matches!(self, InputMode::Command { .. } | InputMode::Search { .. } | InputMode::Modal { .. })
    }

    /// Get the current query if in search/command mode
    pub fn query(&self) -> Option<&str> {
        match self {
            InputMode::Command { query } => Some(query),
            InputMode::Search { query, .. } => Some(query),
            _ => None,
        }
    }

    /// Transition to normal mode
    pub fn to_normal(&mut self) {
        *self = InputMode::Normal;
    }

    /// Transition to insert mode
    pub fn to_insert(&mut self) {
        *self = InputMode::Insert;
    }

    /// Open command palette
    pub fn open_command(&mut self) {
        *self = InputMode::Command { query: String::new() };
    }

    /// Open search
    pub fn open_search(&mut self, forward: bool) {
        *self = InputMode::Search { query: String::new(), forward };
    }

    /// Open a named modal
    pub fn open_modal(&mut self, name: impl Into<String>) {
        *self = InputMode::Modal { name: name.into() };
    }

    /// Check if a specific modal is open
    pub fn is_modal_open(&self, modal_name: &str) -> bool {
        matches!(self, InputMode::Modal { name } if name == modal_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_normal() {
        let mode = InputMode::default();
        assert_eq!(mode, InputMode::Normal);
    }

    #[test]
    fn test_is_editing() {
        assert!(!InputMode::Normal.is_editing());
        assert!(InputMode::Insert.is_editing());
        assert!(InputMode::Command { query: String::new() }.is_editing());
    }

    #[test]
    fn test_transitions() {
        let mut mode = InputMode::Normal;

        mode.to_insert();
        assert_eq!(mode, InputMode::Insert);

        mode.to_normal();
        assert_eq!(mode, InputMode::Normal);

        mode.open_command();
        assert!(matches!(mode, InputMode::Command { .. }));
    }
}
