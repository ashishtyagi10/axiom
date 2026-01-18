//! Diff tracking for LLM code modifications
//!
//! Tracks original content vs modified content and provides
//! git-style diff visualization in the editor.

use crate::ui::theme::theme;
use ratatui::style::Style;
use std::collections::HashMap;

/// Type of change for a line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineChange {
    /// Line was added (green background)
    Added,
    /// Line was removed (red background, shown as ghost line)
    Removed,
    /// Line was modified (yellow gutter marker)
    Modified,
    /// No change
    Unchanged,
}

impl LineChange {
    /// Get the gutter indicator character
    pub fn gutter_char(&self) -> char {
        match self {
            LineChange::Added => '+',
            LineChange::Removed => '-',
            LineChange::Modified => '~',
            LineChange::Unchanged => ' ',
        }
    }

    /// Get the gutter style
    pub fn gutter_style(&self) -> Style {
        let t = theme();
        match self {
            LineChange::Added => Style::default().fg(t.diff_added_fg),
            LineChange::Removed => Style::default().fg(t.diff_removed_fg),
            LineChange::Modified => Style::default().fg(t.diff_modified_fg),
            LineChange::Unchanged => Style::default().fg(t.text_muted),
        }
    }

    /// Get the line background style
    pub fn line_bg_style(&self) -> Option<Style> {
        let t = theme();
        match self {
            LineChange::Added => Some(Style::default().bg(t.diff_added_bg)),
            LineChange::Removed => Some(Style::default().bg(t.diff_removed_bg)),
            LineChange::Modified => Some(Style::default().bg(t.diff_modified_bg)),
            LineChange::Unchanged => None,
        }
    }
}

/// A line that was removed (ghost line)
#[derive(Debug, Clone)]
pub struct GhostLine {
    /// Original line number (0-indexed)
    pub original_line: usize,
    /// Content of the removed line
    pub content: String,
    /// Position to display (after which current line)
    pub display_after: usize,
}

/// Diff tracker for editor content
pub struct DiffTracker {
    /// Original content (snapshot when LLM starts editing)
    original_lines: Vec<String>,

    /// Whether tracking is active
    tracking: bool,

    /// Change status for each line in current content
    line_changes: HashMap<usize, LineChange>,

    /// Removed lines (ghost lines to show)
    ghost_lines: Vec<GhostLine>,
}

impl Default for DiffTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffTracker {
    /// Create a new diff tracker
    pub fn new() -> Self {
        Self {
            original_lines: Vec::new(),
            tracking: false,
            line_changes: HashMap::new(),
            ghost_lines: Vec::new(),
        }
    }

    /// Start tracking changes from current content
    pub fn start_tracking(&mut self, lines: &[String]) {
        self.original_lines = lines.to_vec();
        self.tracking = true;
        self.line_changes.clear();
        self.ghost_lines.clear();
    }

    /// Stop tracking and clear diff state
    pub fn stop_tracking(&mut self) {
        self.tracking = false;
        self.original_lines.clear();
        self.line_changes.clear();
        self.ghost_lines.clear();
    }

    /// Check if tracking is active
    pub fn is_tracking(&self) -> bool {
        self.tracking
    }

    /// Get change type for a line
    pub fn get_line_change(&self, line_num: usize) -> LineChange {
        if !self.tracking {
            return LineChange::Unchanged;
        }
        self.line_changes.get(&line_num).copied().unwrap_or(LineChange::Unchanged)
    }

    /// Get ghost lines to display after a given line
    pub fn get_ghost_lines_after(&self, line_num: usize) -> Vec<&GhostLine> {
        if !self.tracking {
            return Vec::new();
        }
        self.ghost_lines
            .iter()
            .filter(|g| g.display_after == line_num)
            .collect()
    }

    /// Update diff by comparing current content with original
    pub fn update_diff(&mut self, current_lines: &[String]) {
        if !self.tracking {
            return;
        }

        self.line_changes.clear();
        self.ghost_lines.clear();

        // Use simple LCS-based diff algorithm
        let diff = compute_diff(&self.original_lines, current_lines);

        for change in diff {
            match change {
                DiffOp::Equal(_, _) => {
                    // Line unchanged
                }
                DiffOp::Insert(new_idx) => {
                    self.line_changes.insert(new_idx, LineChange::Added);
                }
                DiffOp::Delete(old_idx, display_after) => {
                    self.ghost_lines.push(GhostLine {
                        original_line: old_idx,
                        content: self.original_lines[old_idx].clone(),
                        display_after,
                    });
                }
                DiffOp::Replace(_old_idx, new_idx) => {
                    self.line_changes.insert(new_idx, LineChange::Modified);
                }
            }
        }
    }

    /// Accept all changes (make current content the new original)
    pub fn accept_changes(&mut self, current_lines: &[String]) {
        if self.tracking {
            self.original_lines = current_lines.to_vec();
            self.line_changes.clear();
            self.ghost_lines.clear();
        }
    }

    /// Revert to original content
    pub fn get_original(&self) -> Option<Vec<String>> {
        if self.tracking {
            Some(self.original_lines.clone())
        } else {
            None
        }
    }
}

/// Diff operation type
#[derive(Debug)]
enum DiffOp {
    /// Lines are equal (old_idx, new_idx)
    Equal(usize, usize),
    /// Line was inserted at new_idx
    Insert(usize),
    /// Line was deleted from old_idx, display after new_idx
    Delete(usize, usize),
    /// Line was replaced (old_idx, new_idx)
    Replace(usize, usize),
}

/// Simple diff algorithm using longest common subsequence
fn compute_diff(old: &[String], new: &[String]) -> Vec<DiffOp> {
    let mut result = Vec::new();

    // Build LCS table
    let m = old.len();
    let n = new.len();

    if m == 0 && n == 0 {
        return result;
    }

    // For empty original, all lines are added
    if m == 0 {
        for i in 0..n {
            result.push(DiffOp::Insert(i));
        }
        return result;
    }

    // For empty new, all lines are deleted
    if n == 0 {
        for i in 0..m {
            result.push(DiffOp::Delete(i, 0));
        }
        return result;
    }

    // LCS dynamic programming
    let mut lcs = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if old[i - 1] == new[j - 1] {
                lcs[i][j] = lcs[i - 1][j - 1] + 1;
            } else {
                lcs[i][j] = lcs[i - 1][j].max(lcs[i][j - 1]);
            }
        }
    }

    // Backtrack to find diff
    let mut i = m;
    let mut j = n;
    let mut changes = Vec::new();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            changes.push(DiffOp::Equal(i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || lcs[i][j - 1] >= lcs[i - 1][j]) {
            changes.push(DiffOp::Insert(j - 1));
            j -= 1;
        } else if i > 0 {
            let display_after = if j > 0 { j - 1 } else { 0 };
            changes.push(DiffOp::Delete(i - 1, display_after));
            i -= 1;
        }
    }

    // Reverse to get correct order
    changes.reverse();

    // Merge adjacent insert/delete into replace
    let mut merged = Vec::new();
    let mut skip_next = false;

    for (idx, change) in changes.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        match change {
            DiffOp::Delete(old_idx, _) => {
                // Check if next is an insert at same position
                if let Some(DiffOp::Insert(new_idx)) = changes.get(idx + 1) {
                    merged.push(DiffOp::Replace(*old_idx, *new_idx));
                    skip_next = true;
                } else {
                    merged.push(change.clone());
                }
            }
            _ => merged.push(change.clone()),
        }
    }

    merged
}

impl Clone for DiffOp {
    fn clone(&self) -> Self {
        match self {
            DiffOp::Equal(a, b) => DiffOp::Equal(*a, *b),
            DiffOp::Insert(a) => DiffOp::Insert(*a),
            DiffOp::Delete(a, b) => DiffOp::Delete(*a, *b),
            DiffOp::Replace(a, b) => DiffOp::Replace(*a, *b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_diff() {
        let mut tracker = DiffTracker::new();
        let original = vec!["line 1".to_string(), "line 2".to_string(), "line 3".to_string()];
        tracker.start_tracking(&original);

        let modified = vec!["line 1".to_string(), "line 2 modified".to_string(), "line 3".to_string()];
        tracker.update_diff(&modified);

        assert_eq!(tracker.get_line_change(0), LineChange::Unchanged);
        assert_eq!(tracker.get_line_change(1), LineChange::Modified);
        assert_eq!(tracker.get_line_change(2), LineChange::Unchanged);
    }

    #[test]
    fn test_added_lines() {
        let mut tracker = DiffTracker::new();
        let original = vec!["line 1".to_string(), "line 2".to_string()];
        tracker.start_tracking(&original);

        let modified = vec!["line 1".to_string(), "new line".to_string(), "line 2".to_string()];
        tracker.update_diff(&modified);

        assert_eq!(tracker.get_line_change(1), LineChange::Added);
    }
}
