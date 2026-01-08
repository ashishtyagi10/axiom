//! Text selection for the editor
//!
//! Tracks selection anchor and provides utilities for selection operations.

use std::cmp::Ordering;

/// A position in the document
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.line.cmp(&other.line) {
            Ordering::Equal => self.column.cmp(&other.column),
            ord => ord,
        }
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Selection state for the editor
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Anchor point (where selection started)
    anchor: Option<Position>,
}

impl Selection {
    pub fn new() -> Self {
        Self { anchor: None }
    }

    /// Start selection at position
    pub fn start(&mut self, pos: Position) {
        self.anchor = Some(pos);
    }

    /// Clear selection
    pub fn clear(&mut self) {
        self.anchor = None;
    }

    /// Check if selection is active
    pub fn is_active(&self) -> bool {
        self.anchor.is_some()
    }

    /// Get normalized selection range (start, end) where start <= end
    pub fn range(&self, cursor: Position) -> Option<(Position, Position)> {
        self.anchor.map(|anchor| {
            if anchor <= cursor {
                (anchor, cursor)
            } else {
                (cursor, anchor)
            }
        })
    }

    /// Extract selected text from lines
    pub fn get_text(&self, cursor: Position, lines: &[String]) -> Option<String> {
        let (start, end) = self.range(cursor)?;

        if start.line == end.line {
            // Single line selection
            let line = lines.get(start.line)?;
            let start_byte = char_to_byte_pos(line, start.column);
            let end_byte = char_to_byte_pos(line, end.column);
            Some(line[start_byte..end_byte].to_string())
        } else {
            // Multi-line selection
            let mut result = String::new();

            // First line (from start column to end)
            if let Some(line) = lines.get(start.line) {
                let start_byte = char_to_byte_pos(line, start.column);
                result.push_str(&line[start_byte..]);
                result.push('\n');
            }

            // Middle lines (complete)
            for line_idx in (start.line + 1)..end.line {
                if let Some(line) = lines.get(line_idx) {
                    result.push_str(line);
                    result.push('\n');
                }
            }

            // Last line (from start to end column)
            if let Some(line) = lines.get(end.line) {
                let end_byte = char_to_byte_pos(line, end.column);
                result.push_str(&line[..end_byte]);
            }

            Some(result)
        }
    }

    /// Check if a position is within the selection
    pub fn contains(&self, cursor: Position, pos: Position) -> bool {
        if let Some((start, end)) = self.range(cursor) {
            pos >= start && pos < end
        } else {
            false
        }
    }

    /// Get selection range on a specific line (returns column range)
    pub fn line_range(&self, cursor: Position, line_idx: usize, line_len: usize) -> Option<(usize, usize)> {
        let (start, end) = self.range(cursor)?;

        if line_idx < start.line || line_idx > end.line {
            return None;
        }

        let sel_start = if line_idx == start.line {
            start.column
        } else {
            0
        };

        let sel_end = if line_idx == end.line {
            end.column
        } else {
            line_len
        };

        Some((sel_start, sel_end))
    }
}

/// Convert character position to byte position in a string
fn char_to_byte_pos(s: &str, char_pos: usize) -> usize {
    s.chars().take(char_pos).map(|c| c.len_utf8()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_ordering() {
        let a = Position::new(0, 5);
        let b = Position::new(1, 2);
        let c = Position::new(0, 3);

        assert!(a < b);
        assert!(c < a);
        assert!(a > c);
    }

    #[test]
    fn test_selection_range() {
        let mut sel = Selection::new();
        sel.start(Position::new(0, 5));

        // Forward selection
        let (start, end) = sel.range(Position::new(0, 10)).unwrap();
        assert_eq!(start.column, 5);
        assert_eq!(end.column, 10);

        // Backward selection
        let (start, end) = sel.range(Position::new(0, 2)).unwrap();
        assert_eq!(start.column, 2);
        assert_eq!(end.column, 5);
    }

    #[test]
    fn test_get_text_single_line() {
        let lines = vec!["Hello, World!".to_string()];
        let mut sel = Selection::new();
        sel.start(Position::new(0, 0));

        let text = sel.get_text(Position::new(0, 5), &lines).unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_get_text_multi_line() {
        let lines = vec![
            "First line".to_string(),
            "Second line".to_string(),
            "Third line".to_string(),
        ];
        let mut sel = Selection::new();
        sel.start(Position::new(0, 6));

        let text = sel.get_text(Position::new(2, 5), &lines).unwrap();
        assert_eq!(text, "line\nSecond line\nThird");
    }
}
