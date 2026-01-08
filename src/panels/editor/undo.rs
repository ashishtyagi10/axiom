//! Undo/redo system for the editor
//!
//! Tracks edit operations and allows reverting changes.

use super::selection::Position;

/// Maximum undo history size
const MAX_UNDO_HISTORY: usize = 100;

/// An edit operation that can be undone
#[derive(Debug, Clone)]
pub enum EditOp {
    /// Insert text at position
    Insert {
        pos: Position,
        text: String,
    },
    /// Delete text from range
    Delete {
        start: Position,
        end: Position,
        deleted_text: String,
    },
}

/// Undo stack for tracking operations
#[derive(Debug, Default)]
pub struct UndoStack {
    /// Undo stack
    undo: Vec<EditOp>,
    /// Redo stack
    redo: Vec<EditOp>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Push an operation onto the undo stack
    pub fn push(&mut self, op: EditOp) {
        self.undo.push(op);
        self.redo.clear(); // Clear redo stack on new edit

        // Limit history size
        if self.undo.len() > MAX_UNDO_HISTORY {
            self.undo.remove(0);
        }
    }

    /// Pop the last operation for undo
    pub fn pop_undo(&mut self) -> Option<EditOp> {
        self.undo.pop()
    }

    /// Push an operation onto the redo stack
    pub fn push_redo(&mut self, op: EditOp) {
        self.redo.push(op);
    }

    /// Pop the last operation for redo
    pub fn pop_redo(&mut self) -> Option<EditOp> {
        self.redo.pop()
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_pop() {
        let mut stack = UndoStack::new();

        stack.push(EditOp::Insert {
            pos: Position::new(0, 0),
            text: "hello".to_string(),
        });

        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        let op = stack.pop_undo().unwrap();
        matches!(op, EditOp::Insert { .. });

        assert!(!stack.can_undo());
    }

    #[test]
    fn test_redo_cleared_on_new_edit() {
        let mut stack = UndoStack::new();

        stack.push(EditOp::Insert {
            pos: Position::new(0, 0),
            text: "a".to_string(),
        });

        let op = stack.pop_undo().unwrap();
        stack.push_redo(op);

        assert!(stack.can_redo());

        // New edit should clear redo
        stack.push(EditOp::Insert {
            pos: Position::new(0, 0),
            text: "b".to_string(),
        });

        assert!(!stack.can_redo());
    }
}
