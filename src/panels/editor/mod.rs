//! Editor panel for text editing with syntax highlighting
//!
//! Features:
//! - Syntax highlighting via syntect
//! - Git-style diff tracking for LLM modifications
//! - Vim-style cursor movement
//! - Multi-file tabs support
//! - Text selection with Shift+Arrow keys
//! - Clipboard operations (Ctrl+C/X/V)
//! - Undo/redo (Ctrl+Z/Y)

mod diff;
mod highlight;
mod selection;
mod undo;

pub use diff::DiffTracker;
pub use highlight::Highlighter;
pub use selection::{Position, Selection};
pub use undo::{EditOp, UndoStack};

use crate::core::Result;
use crate::events::Event;
use crate::state::{AppState, PanelId};
use crate::ui::ScrollBar;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
use std::cell::RefCell;
use std::path::PathBuf;

/// Single file tab state
pub struct FileTab {
    /// File path (if saved)
    pub file_path: Option<PathBuf>,
    /// Text content as lines
    pub lines: Vec<String>,
    /// Cursor position (line, column)
    pub cursor: (usize, usize),
    /// Scroll offset (line, column)
    pub scroll: (usize, usize),
    /// File has unsaved changes
    pub modified: bool,
    /// Cached highlighted lines
    pub highlighted_lines: Vec<Vec<(String, Style)>>,
    /// Diff tracker for LLM modifications
    pub diff_tracker: DiffTracker,
    /// Whether highlight cache is dirty
    pub highlight_dirty: bool,
    /// Text selection state
    pub selection: Selection,
    /// Undo history
    pub undo_stack: UndoStack,
}

impl FileTab {
    /// Create a new empty tab
    fn new() -> Self {
        Self {
            file_path: None,
            lines: vec![String::new()],
            cursor: (0, 0),
            scroll: (0, 0),
            modified: false,
            highlighted_lines: Vec::new(),
            diff_tracker: DiffTracker::new(),
            highlight_dirty: true,
            selection: Selection::new(),
            undo_stack: UndoStack::new(),
        }
    }

    /// Create a tab for a new file (not yet saved)
    fn new_file(path: PathBuf) -> Self {
        Self {
            file_path: Some(path),
            lines: vec![String::new()],
            cursor: (0, 0),
            scroll: (0, 0),
            modified: true,
            highlighted_lines: Vec::new(),
            diff_tracker: DiffTracker::new(),
            highlight_dirty: true,
            selection: Selection::new(),
            undo_stack: UndoStack::new(),
        }
    }

    /// Get display name for tab
    fn display_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[New]".to_string())
    }

    /// Get current line content
    fn current_line(&self) -> &str {
        self.lines.get(self.cursor.0).map(|s| s.as_str()).unwrap_or("")
    }
}

/// Editor panel with multi-file tabs
pub struct EditorPanel {
    /// Open file tabs
    tabs: Vec<FileTab>,
    /// Active tab index
    active_tab: usize,
    /// Syntax highlighter (shared across tabs)
    highlighter: Highlighter,
    /// Visible height (updated on render)
    visible_height: usize,
    /// Tab bar area for mouse click detection (interior mutability for render)
    tab_bar_area: RefCell<Option<Rect>>,
    /// Calculated tab boundaries (start_x, end_x) for each tab
    tab_boundaries: RefCell<Vec<(u16, u16)>>,
    /// Content area for scroll bar click detection
    content_area: RefCell<Rect>,
}

impl Default for EditorPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorPanel {
    /// Create new empty editor
    pub fn new() -> Self {
        Self {
            tabs: vec![FileTab::new()],
            active_tab: 0,
            highlighter: Highlighter::new(),
            visible_height: 20,
            tab_bar_area: RefCell::new(None),
            tab_boundaries: RefCell::new(Vec::new()),
            content_area: RefCell::new(Rect::default()),
        }
    }

    // ==================== Tab Access ====================

    /// Get active tab reference
    fn active_tab(&self) -> &FileTab {
        &self.tabs[self.active_tab]
    }

    /// Get active tab mutable reference
    fn active_tab_mut(&mut self) -> &mut FileTab {
        &mut self.tabs[self.active_tab]
    }

    /// Check if editor has any tabs
    pub fn has_tabs(&self) -> bool {
        !self.tabs.is_empty()
    }

    /// Get tab count
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    // ==================== Tab Navigation ====================

    /// Find tab index by path
    pub fn find_tab_by_path(&self, path: &std::path::Path) -> Option<usize> {
        self.tabs.iter().position(|tab| {
            tab.file_path.as_deref() == Some(path)
        })
    }

    /// Switch to tab by index
    pub fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
        }
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    /// Switch to previous tab
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }

    /// Close tab by index
    pub fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 {
            // Keep at least one tab (empty)
            self.tabs[0] = FileTab::new();
            return;
        }

        if index < self.tabs.len() {
            self.tabs.remove(index);
            // Adjust active tab if needed
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            } else if self.active_tab > index {
                self.active_tab -= 1;
            }
        }
    }

    /// Close current tab
    pub fn close_current_tab(&mut self) {
        self.close_tab(self.active_tab);
    }

    // ==================== File Operations ====================

    /// Open a file in editor (creates new tab or switches to existing)
    pub fn open(&mut self, path: &std::path::Path) -> Result<()> {
        // Check if file is already open
        if let Some(idx) = self.find_tab_by_path(path) {
            self.active_tab = idx;
            return Ok(());
        }

        // Read file content
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::core::AxiomError::Io(e))?;

        // Create new tab
        let mut tab = FileTab::new();
        tab.lines = content.lines().map(String::from).collect();
        if tab.lines.is_empty() {
            tab.lines.push(String::new());
        }
        tab.file_path = Some(path.to_path_buf());
        tab.modified = false;
        tab.highlight_dirty = true;

        // Refresh highlighting for new tab
        tab.highlighted_lines = self.highlighter.highlight_all(&tab.lines, Some(path));

        // Replace empty initial tab or add new tab
        if self.tabs.len() == 1 && self.tabs[0].file_path.is_none() && !self.tabs[0].modified {
            self.tabs[0] = tab;
        } else {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }

        Ok(())
    }

    /// Set up editor for a new file (not yet saved)
    pub fn set_new_file(&mut self, path: &std::path::Path) {
        // Check if already open
        if let Some(idx) = self.find_tab_by_path(path) {
            self.active_tab = idx;
            return;
        }

        let tab = FileTab::new_file(path.to_path_buf());

        // Replace empty initial tab or add new tab
        if self.tabs.len() == 1 && self.tabs[0].file_path.is_none() && !self.tabs[0].modified {
            self.tabs[0] = tab;
        } else {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }
    }

    /// Get current file path
    pub fn current_file(&self) -> Option<&std::path::Path> {
        self.active_tab().file_path.as_deref()
    }

    /// Check if a specific file is open (in any tab)
    pub fn has_file_open(&self, path: &std::path::Path) -> bool {
        self.find_tab_by_path(path).is_some()
    }

    // ==================== Diff Tracking ====================

    /// Start tracking changes for LLM diff display
    pub fn start_diff_tracking(&mut self) {
        let tab = self.active_tab_mut();
        tab.diff_tracker.start_tracking(&tab.lines);
    }

    /// Stop diff tracking
    pub fn stop_diff_tracking(&mut self) {
        self.active_tab_mut().diff_tracker.stop_tracking();
    }

    /// Apply content from LLM with diff tracking
    pub fn apply_llm_modification(&mut self, content: &str) {
        self.start_diff_tracking();

        let tab = self.active_tab_mut();
        tab.lines = content.lines().map(String::from).collect();
        if tab.lines.is_empty() {
            tab.lines.push(String::new());
        }

        tab.diff_tracker.update_diff(&tab.lines);
        tab.modified = true;
        tab.highlight_dirty = true;

        // Refresh highlighting
        let path = tab.file_path.clone();
        let lines = tab.lines.clone();
        self.active_tab_mut().highlighted_lines =
            self.highlighter.highlight_all(&lines, path.as_deref());
    }

    /// Apply modification to specific tab by path
    pub fn apply_modification_to_path(&mut self, path: &std::path::Path, content: &str) {
        // Find or create tab
        if let Some(idx) = self.find_tab_by_path(path) {
            self.active_tab = idx;
        } else if path.exists() {
            let _ = self.open(path);
        } else {
            self.set_new_file(path);
        }
        self.apply_llm_modification(content);
    }

    // ==================== Highlighting ====================

    /// Refresh syntax highlighting for active tab
    fn refresh_highlighting(&mut self) {
        let path = self.tabs[self.active_tab].file_path.clone();
        let lines = self.tabs[self.active_tab].lines.clone();
        let highlighted = self.highlighter.highlight_all(&lines, path.as_deref());
        self.tabs[self.active_tab].highlighted_lines = highlighted;
        self.tabs[self.active_tab].highlight_dirty = false;
    }

    /// Mark active tab as needing re-highlight
    fn mark_line_dirty(&mut self, _line: usize) {
        self.active_tab_mut().highlight_dirty = true;
    }

    /// Update diff after content changes
    fn update_diff(&mut self) {
        let tab = self.active_tab_mut();
        if tab.diff_tracker.is_tracking() {
            let lines = tab.lines.clone();
            tab.diff_tracker.update_diff(&lines);
        }
    }

    // ==================== Selection ====================

    /// Get cursor as Position
    fn cursor_pos(&self) -> Position {
        let tab = self.active_tab();
        Position::new(tab.cursor.0, tab.cursor.1)
    }

    /// Start or extend selection based on shift key
    fn handle_selection(&mut self, shift_held: bool) {
        let tab = self.active_tab_mut();
        let cursor_pos = Position::new(tab.cursor.0, tab.cursor.1);

        if shift_held {
            if !tab.selection.is_active() {
                tab.selection.start(cursor_pos);
            }
        } else {
            tab.selection.clear();
        }
    }

    /// Get selected text
    fn get_selected_text(&self) -> Option<String> {
        let tab = self.active_tab();
        let cursor_pos = Position::new(tab.cursor.0, tab.cursor.1);
        tab.selection.get_text(cursor_pos, &tab.lines)
    }

    /// Delete text in range and return deleted text
    fn delete_range(&mut self, start: Position, end: Position) {
        let tab = self.active_tab_mut();

        if start.line == end.line {
            // Single line deletion
            let line = &mut tab.lines[start.line];
            let start_byte: usize = line.chars().take(start.column).map(|c| c.len_utf8()).sum();
            let end_byte: usize = line.chars().take(end.column).map(|c| c.len_utf8()).sum();
            line.drain(start_byte..end_byte);
        } else {
            // Multi-line deletion
            // Keep part of first line before start
            let first_line = &tab.lines[start.line];
            let start_byte: usize = first_line.chars().take(start.column).map(|c| c.len_utf8()).sum();
            let first_part = first_line[..start_byte].to_string();

            // Keep part of last line after end
            let last_line = &tab.lines[end.line];
            let end_byte: usize = last_line.chars().take(end.column).map(|c| c.len_utf8()).sum();
            let last_part = last_line[end_byte..].to_string();

            // Remove lines from end.line down to start.line+1
            for _ in (start.line + 1)..=end.line {
                if start.line + 1 < tab.lines.len() {
                    tab.lines.remove(start.line + 1);
                }
            }

            // Combine first and last parts
            tab.lines[start.line] = first_part + &last_part;
        }

        tab.modified = true;
        tab.highlight_dirty = true;
    }

    /// Delete selection and return deleted text
    fn delete_selection(&mut self) -> Option<String> {
        let cursor_pos = self.cursor_pos();
        let tab = self.active_tab();

        if let Some((start, end)) = tab.selection.range(cursor_pos) {
            let deleted = tab.selection.get_text(cursor_pos, &tab.lines)?;

            // Record for undo
            let tab = self.active_tab_mut();
            tab.undo_stack.push(EditOp::Delete {
                start,
                end,
                deleted_text: deleted.clone(),
            });

            // Perform deletion
            self.delete_range(start, end);

            // Clear selection and move cursor to start
            let tab = self.active_tab_mut();
            tab.selection.clear();
            tab.cursor = (start.line, start.column);

            self.update_diff();
            Some(deleted)
        } else {
            None
        }
    }

    /// Insert text at cursor position
    fn insert_text(&mut self, text: &str) {
        for c in text.chars() {
            if c == '\n' {
                self.newline();
            } else {
                self.insert_char(c);
            }
        }
    }

    // ==================== Clipboard Operations ====================

    /// Copy selection to clipboard
    fn copy_selection(&mut self) -> bool {
        if let Some(text) = self.get_selected_text() {
            crate::clipboard::copy(&text).is_ok()
        } else {
            false
        }
    }

    /// Cut selection to clipboard
    fn cut_selection(&mut self) -> bool {
        if let Some(text) = self.get_selected_text() {
            if crate::clipboard::copy(&text).is_ok() {
                self.delete_selection();
                return true;
            }
        }
        false
    }

    /// Paste from clipboard
    fn paste(&mut self) -> bool {
        if let Ok(text) = crate::clipboard::paste() {
            // Delete selection first if any
            self.delete_selection();

            let start_pos = self.cursor_pos();

            // Insert text
            self.insert_text(&text);

            // Record for undo
            let tab = self.active_tab_mut();
            tab.undo_stack.push(EditOp::Insert {
                pos: start_pos,
                text,
            });

            return true;
        }
        false
    }

    // ==================== Undo/Redo ====================

    /// Undo last operation
    fn undo(&mut self) -> bool {
        let op = {
            let tab = self.active_tab_mut();
            tab.undo_stack.pop_undo()
        };

        if let Some(op) = op {
            match op.clone() {
                EditOp::Insert { pos, text } => {
                    // Undo insert = delete the text
                    let end = self.calculate_end_position(pos, &text);
                    self.delete_range(pos, end);
                    let tab = self.active_tab_mut();
                    tab.cursor = (pos.line, pos.column);
                    tab.undo_stack.push_redo(op);
                }
                EditOp::Delete { start, deleted_text, .. } => {
                    // Undo delete = insert the text
                    {
                        let tab = self.active_tab_mut();
                        tab.cursor = (start.line, start.column);
                    }
                    self.insert_text(&deleted_text);
                    let tab = self.active_tab_mut();
                    tab.undo_stack.push_redo(op);
                }
            }
            self.active_tab_mut().highlight_dirty = true;
            self.update_diff();
            return true;
        }
        false
    }

    /// Redo last undone operation
    fn redo(&mut self) -> bool {
        let op = {
            let tab = self.active_tab_mut();
            tab.undo_stack.pop_redo()
        };

        if let Some(op) = op {
            match op.clone() {
                EditOp::Insert { pos, text } => {
                    // Redo insert = insert the text again
                    {
                        let tab = self.active_tab_mut();
                        tab.cursor = (pos.line, pos.column);
                    }
                    self.insert_text(&text);
                    let tab = self.active_tab_mut();
                    tab.undo_stack.push(op);
                }
                EditOp::Delete { start, end, .. } => {
                    // Redo delete = delete again
                    self.delete_range(start, end);
                    let tab = self.active_tab_mut();
                    tab.cursor = (start.line, start.column);
                    tab.undo_stack.push(op);
                }
            }
            self.active_tab_mut().highlight_dirty = true;
            self.update_diff();
            return true;
        }
        false
    }

    /// Calculate end position after inserting text
    fn calculate_end_position(&self, start: Position, text: &str) -> Position {
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return start;
        }

        if lines.len() == 1 {
            Position::new(start.line, start.column + lines[0].chars().count())
        } else {
            Position::new(
                start.line + lines.len() - 1,
                lines.last().map(|l| l.chars().count()).unwrap_or(0),
            )
        }
    }

    // ==================== Editing Operations ====================

    /// Insert character at cursor
    fn insert_char(&mut self, c: char) {
        let tab = self.active_tab_mut();
        let row = tab.cursor.0;
        let col = tab.cursor.1;
        let line = &mut tab.lines[row];
        let col = col.min(line.chars().count());

        let byte_pos: usize = line.chars().take(col).map(|ch| ch.len_utf8()).sum();
        line.insert(byte_pos, c);

        tab.cursor.1 = col + 1;
        tab.modified = true;
        self.mark_line_dirty(row);
        self.update_diff();
    }

    /// Delete character before cursor (backspace)
    fn backspace(&mut self) {
        let tab = self.active_tab_mut();
        let row = tab.cursor.0;
        let col = tab.cursor.1;

        if col > 0 {
            let line = &mut tab.lines[row];
            let col = col.min(line.chars().count());

            if col > 0 {
                let byte_pos: usize = line.chars().take(col).map(|ch| ch.len_utf8()).sum();
                let prev_char_len = line.chars().nth(col - 1).map(|ch| ch.len_utf8()).unwrap_or(1);
                line.remove(byte_pos - prev_char_len);
                tab.cursor.1 = col - 1;
                tab.modified = true;
                tab.highlight_dirty = true;
            }
        } else if row > 0 {
            let current = tab.lines.remove(row);
            tab.cursor.0 = row - 1;
            tab.cursor.1 = tab.lines[tab.cursor.0].chars().count();
            tab.lines[tab.cursor.0].push_str(&current);
            tab.modified = true;
            tab.highlight_dirty = true;
        }
        self.update_diff();
    }

    /// Delete character at cursor
    fn delete(&mut self) {
        let tab = self.active_tab_mut();
        let row = tab.cursor.0;
        let col = tab.cursor.1;
        let line_len = tab.lines[row].chars().count();

        if col < line_len {
            let line = &mut tab.lines[row];
            let byte_pos: usize = line.chars().take(col).map(|ch| ch.len_utf8()).sum();
            line.remove(byte_pos);
            tab.modified = true;
            tab.highlight_dirty = true;
        } else if row + 1 < tab.lines.len() {
            let next = tab.lines.remove(row + 1);
            tab.lines[row].push_str(&next);
            tab.modified = true;
            tab.highlight_dirty = true;
        }
        self.update_diff();
    }

    /// Insert newline at cursor
    fn newline(&mut self) {
        let tab = self.active_tab_mut();
        let row = tab.cursor.0;
        let col = tab.cursor.1;
        let line = &mut tab.lines[row];
        let byte_pos: usize = line.chars().take(col).map(|ch| ch.len_utf8()).sum();

        let rest = line.split_off(byte_pos);
        tab.cursor.0 = row + 1;
        tab.cursor.1 = 0;
        tab.lines.insert(tab.cursor.0, rest);
        tab.modified = true;
        tab.highlight_dirty = true;
        self.update_diff();
    }

    // ==================== Cursor Movement ====================

    /// Move cursor and adjust scroll
    fn move_cursor(&mut self, dir: Direction) {
        let tab = self.active_tab_mut();
        match dir {
            Direction::Up => {
                if tab.cursor.0 > 0 {
                    tab.cursor.0 -= 1;
                    let line_len = tab.current_line().chars().count();
                    tab.cursor.1 = tab.cursor.1.min(line_len);
                }
            }
            Direction::Down => {
                if tab.cursor.0 + 1 < tab.lines.len() {
                    tab.cursor.0 += 1;
                    let line_len = tab.current_line().chars().count();
                    tab.cursor.1 = tab.cursor.1.min(line_len);
                }
            }
            Direction::Left => {
                if tab.cursor.1 > 0 {
                    tab.cursor.1 -= 1;
                } else if tab.cursor.0 > 0 {
                    tab.cursor.0 -= 1;
                    tab.cursor.1 = tab.current_line().chars().count();
                }
            }
            Direction::Right => {
                let line_len = tab.current_line().chars().count();
                if tab.cursor.1 < line_len {
                    tab.cursor.1 += 1;
                } else if tab.cursor.0 + 1 < tab.lines.len() {
                    tab.cursor.0 += 1;
                    tab.cursor.1 = 0;
                }
            }
        }
        self.ensure_cursor_visible();
    }

    // ==================== Scrolling ====================

    fn scroll_up(&mut self, n: usize) {
        self.active_tab_mut().scroll.0 = self.active_tab().scroll.0.saturating_sub(n);
    }

    fn scroll_down(&mut self, n: usize) {
        let tab = self.active_tab_mut();
        let max_scroll = tab.lines.len().saturating_sub(1);
        tab.scroll.0 = (tab.scroll.0 + n).min(max_scroll);
    }

    fn scroll_half_page_up(&mut self) {
        let half = self.visible_height / 2;
        self.scroll_up(half.max(1));
    }

    fn scroll_half_page_down(&mut self) {
        let half = self.visible_height / 2;
        self.scroll_down(half.max(1));
    }

    fn scroll_page_up(&mut self) {
        self.scroll_up(self.visible_height.saturating_sub(2));
    }

    fn scroll_page_down(&mut self) {
        self.scroll_down(self.visible_height.saturating_sub(2));
    }

    fn scroll_to_bottom(&mut self) {
        let tab = self.active_tab_mut();
        let max_scroll = tab.lines.len().saturating_sub(1);
        tab.scroll.0 = max_scroll;
        tab.cursor.0 = tab.lines.len().saturating_sub(1);
        tab.cursor.1 = 0;
    }

    fn ensure_cursor_visible(&mut self) {
        let visible_height = self.visible_height;
        let tab = &mut self.tabs[self.active_tab];
        if tab.cursor.0 < tab.scroll.0 {
            tab.scroll.0 = tab.cursor.0;
        } else if tab.cursor.0 >= tab.scroll.0 + visible_height {
            tab.scroll.0 = tab.cursor.0 - visible_height + 1;
        }
    }

    // ==================== Display ====================

    /// Get title for display
    fn title(&self) -> String {
        let tab = self.active_tab();
        let name = tab.display_name();

        let syntax = self.highlighter.detect_syntax(tab.file_path.as_deref());

        let diff_indicator = if tab.diff_tracker.is_tracking() {
            " [DIFF]"
        } else {
            ""
        };

        let scroll_info = crate::ui::scroll::scroll_indicator(
            tab.scroll.0,
            self.visible_height,
            tab.lines.len(),
        );

        let modified_marker = if tab.modified { " *" } else { "" };

        format!(" {}{} ({}){}{} ", name, modified_marker, syntax, diff_indicator, scroll_info)
    }
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl super::Panel for EditorPanel {
    fn id(&self) -> PanelId {
        PanelId::EDITOR
    }

    fn name(&self) -> &str {
        "Editor"
    }

    fn handle_input(&mut self, event: &Event, state: &mut AppState) -> Result<bool> {
        if let Event::Key(key) = event {
            // Tab navigation keys (work in all modes)
            // Alt+] on Windows/Linux, Cmd+] on Mac (SUPER modifier)
            match (key.code, key.modifiers) {
                // Alt+] or Cmd+]: next tab
                (KeyCode::Char(']'), m)
                    if m.contains(KeyModifiers::ALT) || m.contains(KeyModifiers::SUPER) =>
                {
                    self.next_tab();
                    return Ok(true);
                }
                // Alt+[ or Cmd+[: previous tab
                (KeyCode::Char('['), m)
                    if m.contains(KeyModifiers::ALT) || m.contains(KeyModifiers::SUPER) =>
                {
                    self.prev_tab();
                    return Ok(true);
                }
                // Ctrl+W: close current tab
                (KeyCode::Char('w'), m) if m.contains(KeyModifiers::CONTROL) => {
                    self.close_current_tab();
                    return Ok(true);
                }
                // Alt+1-9: switch to tab by number
                (KeyCode::Char(c), m) if m.contains(KeyModifiers::ALT) && c.is_ascii_digit() => {
                    let idx = c.to_digit(10).unwrap_or(0) as usize;
                    if idx > 0 && idx <= self.tabs.len() {
                        self.switch_tab(idx - 1);
                    }
                    return Ok(true);
                }
                // Ctrl+C: Copy selection
                (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                    if self.copy_selection() {
                        state.info("Copied to clipboard".to_string());
                    }
                    return Ok(true);
                }
                // Ctrl+X: Cut selection
                (KeyCode::Char('x'), m) if m.contains(KeyModifiers::CONTROL) => {
                    if self.cut_selection() {
                        self.refresh_highlighting();
                        state.info("Cut to clipboard".to_string());
                    }
                    return Ok(true);
                }
                // Ctrl+V: Paste
                (KeyCode::Char('v'), m) if m.contains(KeyModifiers::CONTROL) => {
                    if self.paste() {
                        self.refresh_highlighting();
                    }
                    return Ok(true);
                }
                // Ctrl+Z: Undo
                (KeyCode::Char('z'), m) if m.contains(KeyModifiers::CONTROL) => {
                    if self.undo() {
                        self.refresh_highlighting();
                    }
                    return Ok(true);
                }
                // Ctrl+Y: Redo
                (KeyCode::Char('y'), m) if m.contains(KeyModifiers::CONTROL) => {
                    if self.redo() {
                        self.refresh_highlighting();
                    }
                    return Ok(true);
                }
                _ => {}
            }

            // Handle PageUp/PageDown in both modes
            match key.code {
                KeyCode::PageUp => {
                    self.scroll_page_up();
                    let visible_height = self.visible_height;
                    let tab = &mut self.tabs[self.active_tab];
                    if tab.cursor.0 >= tab.scroll.0 + visible_height {
                        tab.cursor.0 = tab.scroll.0 + visible_height - 1;
                    }
                    return Ok(true);
                }
                KeyCode::PageDown => {
                    self.scroll_page_down();
                    let tab = &mut self.tabs[self.active_tab];
                    if tab.cursor.0 < tab.scroll.0 {
                        tab.cursor.0 = tab.scroll.0;
                    }
                    return Ok(true);
                }
                _ => {}
            }

            // Normal mode: vim-style navigation + arrow key scrolling
            if !state.input_mode.is_editing() {
                return match key.code {
                    KeyCode::Up => {
                        self.scroll_up(1);
                        Ok(true)
                    }
                    KeyCode::Down => {
                        self.scroll_down(1);
                        Ok(true)
                    }
                    KeyCode::Left => {
                        self.active_tab_mut().scroll.1 = self.active_tab().scroll.1.saturating_sub(1);
                        Ok(true)
                    }
                    KeyCode::Right => {
                        self.active_tab_mut().scroll.1 += 1;
                        Ok(true)
                    }
                    KeyCode::Char('j') => {
                        self.move_cursor(Direction::Down);
                        Ok(true)
                    }
                    KeyCode::Char('k') => {
                        self.move_cursor(Direction::Up);
                        Ok(true)
                    }
                    KeyCode::Char('h') => {
                        self.move_cursor(Direction::Left);
                        Ok(true)
                    }
                    KeyCode::Char('l') => {
                        self.move_cursor(Direction::Right);
                        Ok(true)
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.scroll_half_page_up();
                        Ok(true)
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.scroll_half_page_down();
                        Ok(true)
                    }
                    KeyCode::Char('g') => {
                        // Go to beginning of file
                        let tab = self.active_tab_mut();
                        tab.cursor = (0, 0);
                        tab.scroll.0 = 0;
                        Ok(true)
                    }
                    KeyCode::Char('G') => {
                        let tab = self.active_tab_mut();
                        tab.cursor.0 = tab.lines.len().saturating_sub(1);
                        tab.cursor.1 = 0;
                        self.ensure_cursor_visible();
                        Ok(true)
                    }
                    KeyCode::Home => {
                        let tab = self.active_tab_mut();
                        tab.cursor = (0, 0);
                        tab.scroll.0 = 0;
                        Ok(true)
                    }
                    KeyCode::End => {
                        let tab = self.active_tab_mut();
                        tab.cursor.0 = tab.lines.len().saturating_sub(1);
                        tab.cursor.1 = 0;
                        self.ensure_cursor_visible();
                        Ok(true)
                    }
                    _ => Ok(false),
                };
            }

            // Insert mode: editing
            let shift = key.modifiers.contains(KeyModifiers::SHIFT);

            match key.code {
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Clear selection and delete if typing over selection
                    if self.active_tab().selection.is_active() {
                        self.delete_selection();
                    }
                    self.insert_char(c);
                    Ok(true)
                }
                KeyCode::Backspace => {
                    // Delete selection if active, otherwise normal backspace
                    if self.active_tab().selection.is_active() {
                        self.delete_selection();
                        self.refresh_highlighting();
                    } else {
                        self.backspace();
                    }
                    Ok(true)
                }
                KeyCode::Delete => {
                    // Delete selection if active, otherwise normal delete
                    if self.active_tab().selection.is_active() {
                        self.delete_selection();
                        self.refresh_highlighting();
                    } else {
                        self.delete();
                    }
                    Ok(true)
                }
                KeyCode::Enter => {
                    // Clear selection if active
                    if self.active_tab().selection.is_active() {
                        self.delete_selection();
                    }
                    self.newline();
                    Ok(true)
                }
                KeyCode::Up => {
                    self.handle_selection(shift);
                    self.move_cursor(Direction::Up);
                    Ok(true)
                }
                KeyCode::Down => {
                    self.handle_selection(shift);
                    self.move_cursor(Direction::Down);
                    Ok(true)
                }
                KeyCode::Left => {
                    self.handle_selection(shift);
                    self.move_cursor(Direction::Left);
                    Ok(true)
                }
                KeyCode::Right => {
                    self.handle_selection(shift);
                    self.move_cursor(Direction::Right);
                    Ok(true)
                }
                KeyCode::Home => {
                    self.handle_selection(shift);
                    self.active_tab_mut().cursor.1 = 0;
                    Ok(true)
                }
                KeyCode::End => {
                    self.handle_selection(shift);
                    let tab = self.active_tab_mut();
                    tab.cursor.1 = tab.current_line().chars().count();
                    Ok(true)
                }
                KeyCode::Esc => {
                    // Escape clears selection
                    self.active_tab_mut().selection.clear();
                    Ok(false) // Let Escape propagate for mode change
                }
                _ => Ok(false),
            }
        } else if let Event::Mouse(mouse) = event {
            match mouse.kind {
                // Handle mouse click on tabs or scroll bar
                crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                    let x = mouse.column;
                    let y = mouse.row;

                    // Check for scroll bar clicks first
                    let content = *self.content_area.borrow();
                    let tab = self.active_tab();
                    let scrollbar = ScrollBar::new(tab.scroll.0, self.visible_height, tab.lines.len());

                    if scrollbar.is_arrow_click(x, y, content) {
                        self.scroll_to_bottom();
                        return Ok(true);
                    }
                    if let Some(page_up) = scrollbar.track_click(x, y, content) {
                        if page_up {
                            self.scroll_page_up();
                        } else {
                            self.scroll_page_down();
                        }
                        return Ok(true);
                    }

                    // Check if click is in tab bar area (copy values to avoid borrow conflicts)
                    let tab_area = *self.tab_bar_area.borrow();
                    let clicked_tab = if let Some(area) = tab_area {
                        if y == area.y && x >= area.x && x < area.x + area.width {
                            // Find which tab was clicked
                            let boundaries = self.tab_boundaries.borrow();
                            boundaries.iter().enumerate()
                                .find(|(_, &(start_x, end_x))| x >= start_x && x < end_x)
                                .map(|(idx, _)| idx)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(idx) = clicked_tab {
                        self.switch_tab(idx);
                        return Ok(true);
                    }
                    Ok(false)
                }
                // Handle mouse scroll
                crossterm::event::MouseEventKind::ScrollUp => {
                    self.scroll_up(3);
                    Ok(true)
                }
                crossterm::event::MouseEventKind::ScrollDown => {
                    self.scroll_down(3);
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Create outer block with title
        let block = Block::default()
            .title(self.title())
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);

        // Split inner area: tab bar (1 line) + content
        let chunks = Layout::default()
            .direction(LayoutDirection::Vertical)
            .constraints([
                Constraint::Length(1), // Tab bar
                Constraint::Min(1),    // Content
            ])
            .split(inner);

        let tab_bar_area = chunks[0];
        let content_area = chunks[1];
        let visible_height = content_area.height as usize;

        // Store content area for scroll bar click detection
        *self.content_area.borrow_mut() = content_area;

        // Store tab bar area for mouse click detection
        *self.tab_bar_area.borrow_mut() = Some(tab_bar_area);

        // Calculate and store tab boundaries for click detection
        {
            let mut boundaries = self.tab_boundaries.borrow_mut();
            boundaries.clear();
            let mut current_x = tab_bar_area.x;
            for tab in &self.tabs {
                let name = tab.display_name();
                let modified = if tab.modified { "*" } else { "" };
                let title = format!(" {}{} ", name, modified);
                let tab_width = title.len() as u16 + 1; // +1 for divider
                boundaries.push((current_x, current_x + tab_width));
                current_x += tab_width;
            }
        }

        // Render outer block
        frame.render_widget(block, area);

        // Render tab bar
        let tab_titles: Vec<Line> = self.tabs.iter().enumerate().map(|(idx, tab)| {
            let name = tab.display_name();
            let modified = if tab.modified { "*" } else { "" };
            let title = format!(" {}{} ", name, modified);

            if idx == self.active_tab {
                Line::from(Span::styled(
                    title,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(
                    title,
                    Style::default().fg(Color::DarkGray),
                ))
            }
        }).collect();

        let tabs_widget = Tabs::new(tab_titles)
            .style(Style::default().fg(Color::DarkGray))
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .select(self.active_tab)
            .divider(Span::raw("|"));

        frame.render_widget(tabs_widget, tab_bar_area);

        // Get active tab data for rendering content
        let tab = self.active_tab();

        // Calculate line number width
        let line_count = tab.lines.len();
        let gutter_width = format!("{}", line_count).len() + 2;

        let scroll_y = tab.scroll.0;

        // Get selection range for highlighting
        let cursor_pos = Position::new(tab.cursor.0, tab.cursor.1);
        let selection_bg = Color::Rgb(60, 80, 120); // Selection background color

        // Build lines with syntax highlighting
        let lines: Vec<Line> = tab
            .lines
            .iter()
            .enumerate()
            .skip(scroll_y)
            .take(visible_height)
            .map(|(idx, line)| {
                let change = tab.diff_tracker.get_line_change(idx);

                let diff_marker = Span::styled(
                    format!("{}", change.gutter_char()),
                    change.gutter_style(),
                );

                let line_num = format!("{:>width$} ", idx + 1, width = gutter_width - 2);
                let line_num_style = Style::default().fg(Color::DarkGray);

                // Get selection range on this line if any
                let line_char_count = line.chars().count();
                let selection_range = tab.selection.line_range(cursor_pos, idx, line_char_count);

                let content_spans: Vec<Span> = if idx < tab.highlighted_lines.len() {
                    // Apply selection highlighting to syntax-highlighted spans
                    let mut result_spans = Vec::new();
                    let mut char_pos = 0;

                    for (text, style) in &tab.highlighted_lines[idx] {
                        let text_char_count = text.chars().count();
                        let span_start = char_pos;
                        let span_end = char_pos + text_char_count;

                        if let Some((sel_start, sel_end)) = selection_range {
                            // Check if this span overlaps with selection
                            if span_end > sel_start && span_start < sel_end {
                                // Split span into parts: before, selected, after
                                let text_chars: Vec<char> = text.chars().collect();

                                // Part before selection
                                if span_start < sel_start {
                                    let before_end = (sel_start - span_start).min(text_chars.len());
                                    let before: String = text_chars[..before_end].iter().collect();
                                    let mut s = *style;
                                    if let Some(bg_style) = change.line_bg_style() {
                                        s = s.bg(bg_style.bg.unwrap_or(Color::Reset));
                                    }
                                    result_spans.push(Span::styled(before, s));
                                }

                                // Selected part
                                let sel_local_start = sel_start.saturating_sub(span_start);
                                let sel_local_end = (sel_end - span_start).min(text_chars.len());
                                if sel_local_start < text_chars.len() && sel_local_end > sel_local_start {
                                    let selected: String = text_chars[sel_local_start..sel_local_end].iter().collect();
                                    let s = style.bg(selection_bg);
                                    result_spans.push(Span::styled(selected, s));
                                }

                                // Part after selection
                                if span_end > sel_end {
                                    let after_start = sel_end.saturating_sub(span_start);
                                    if after_start < text_chars.len() {
                                        let after: String = text_chars[after_start..].iter().collect();
                                        let mut s = *style;
                                        if let Some(bg_style) = change.line_bg_style() {
                                            s = s.bg(bg_style.bg.unwrap_or(Color::Reset));
                                        }
                                        result_spans.push(Span::styled(after, s));
                                    }
                                }
                            } else {
                                // No overlap with selection
                                let mut s = *style;
                                if let Some(bg_style) = change.line_bg_style() {
                                    s = s.bg(bg_style.bg.unwrap_or(Color::Reset));
                                }
                                result_spans.push(Span::styled(text.clone(), s));
                            }
                        } else {
                            // No selection - normal rendering
                            let mut s = *style;
                            if let Some(bg_style) = change.line_bg_style() {
                                s = s.bg(bg_style.bg.unwrap_or(Color::Reset));
                            }
                            result_spans.push(Span::styled(text.clone(), s));
                        }

                        char_pos = span_end;
                    }
                    result_spans
                } else {
                    vec![Span::raw(line.clone())]
                };

                let mut spans = vec![
                    diff_marker,
                    Span::styled(line_num, line_num_style),
                ];
                spans.extend(content_spans);

                Line::from(spans)
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, content_area);

        // Render scroll bar
        let scrollbar = ScrollBar::new(scroll_y, visible_height, tab.lines.len());
        scrollbar.render(frame, content_area, focused);

        // Show cursor if focused and in view
        if focused {
            if tab.cursor.0 >= scroll_y && tab.cursor.0 < scroll_y + visible_height {
                let cursor_x = content_area.x + gutter_width as u16 + tab.cursor.1 as u16;
                let cursor_y = content_area.y + (tab.cursor.0 - scroll_y) as u16;

                if cursor_x < content_area.x + content_area.width
                    && cursor_y < content_area.y + content_area.height
                {
                    frame.set_cursor_position((cursor_x, cursor_y));
                }
            }
        }
    }

    fn on_resize(&mut self, _cols: u16, rows: u16) {
        // Update visible height (account for borders + tab bar)
        self.visible_height = rows.saturating_sub(3) as usize;
    }

    fn on_blur(&mut self) {
        // Clear selection when losing focus
        self.active_tab_mut().selection.clear();
    }
}
