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
    /// Returns the default input mode (Normal).
    fn default() -> Self {
        Self::Normal
    }
}

impl InputMode {
    /// Returns true if the current mode allows for text editing
    pub fn is_editing(&self) -> bool {
        matches!(
            self,
            InputMode::Insert | InputMode::Command { .. } | InputMode::Search { .. }
        )
    }

    /// Returns true if an overlay or modal is currently active
    pub fn is_modal(&self) -> bool {
        matches!(
            self,
            InputMode::Command { .. } | InputMode::Search { .. } | InputMode::Modal { .. }
        )
    }

    /// Returns the active query string if in Search or Command mode
    pub fn query(&self) -> Option<&str> {
        match self {
            InputMode::Command { query } => Some(query),
            InputMode::Search { query, .. } => Some(query),
            _ => None,
        }
    }

    /// Transitions the state to Normal navigation mode
    pub fn to_normal(&mut self) {
        *self = InputMode::Normal;
    }

    /// Transitions the state to Insert mode for text input
    pub fn to_insert(&mut self) {
        *self = InputMode::Insert;
    }

    /// Transitions to Command mode, initializing an empty query string
    pub fn open_command(&mut self) {
        *self = InputMode::Command {
            query: String::new(),
        };
    }

    /// Transitions to Search mode with the specified direction
    pub fn open_search(&mut self, forward: bool) {
        *self = InputMode::Search {
            query: String::new(),
            forward,
        };
    }

    /// Transitions to Modal mode with the given name
    pub fn open_modal(&mut self, name: impl Into<String>) {
        *self = InputMode::Modal { name: name.into() };
    }

    /// Returns true if a modal with the specified name is currently open
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
        assert!(InputMode::Command {
            query: String::new()
        }
        .is_editing());
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
