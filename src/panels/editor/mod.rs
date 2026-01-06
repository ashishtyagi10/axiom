//! Editor panel for text editing with syntax highlighting
//!
//! Features:
//! - Syntax highlighting via syntect
//! - Git-style diff tracking for LLM modifications
//! - Vim-style cursor movement

mod diff;
mod highlight;

pub use diff::DiffTracker;
pub use highlight::Highlighter;

use crate::core::Result;
use crate::events::Event;
use crate::state::{AppState, PanelId};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::PathBuf;

/// Editor panel state
pub struct EditorPanel {
    /// Current file path (if any)
    file_path: Option<PathBuf>,

    /// Text content as lines
    lines: Vec<String>,

    /// Cursor position (line, column)
    cursor: (usize, usize),

    /// Scroll offset (line, column)
    scroll: (usize, usize),

    /// File has unsaved changes
    modified: bool,

    /// Syntax highlighter
    highlighter: Highlighter,

    /// Cached highlighted lines
    highlighted_lines: Vec<Vec<(String, Style)>>,

    /// Diff tracker for LLM modifications
    diff_tracker: DiffTracker,

    /// Whether highlight cache is dirty
    highlight_dirty: bool,

    /// Visible height (updated on render)
    visible_height: usize,
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
            file_path: None,
            lines: vec![String::new()],
            cursor: (0, 0),
            scroll: (0, 0),
            modified: false,
            highlighter: Highlighter::new(),
            highlighted_lines: Vec::new(),
            diff_tracker: DiffTracker::new(),
            highlight_dirty: true,
            visible_height: 20, // Default, updated on render
        }
    }

    /// Open a file in the editor
    pub fn open(&mut self, path: &std::path::Path) -> Result<()> {
        let content =
            std::fs::read_to_string(path).map_err(|e| crate::core::AxiomError::Io(e))?;

        self.lines = content.lines().map(String::from).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }

        self.file_path = Some(path.to_path_buf());
        self.cursor = (0, 0);
        self.scroll = (0, 0);
        self.modified = false;
        self.highlight_dirty = true;

        // Stop any existing diff tracking
        self.diff_tracker.stop_tracking();

        // Refresh syntax highlighting
        self.refresh_highlighting();

        Ok(())
    }

    /// Refresh syntax highlighting for all lines
    fn refresh_highlighting(&mut self) {
        self.highlighted_lines = self
            .highlighter
            .highlight_all(&self.lines, self.file_path.as_deref());
        self.highlight_dirty = false;
    }

    /// Mark a single line as needing re-highlight
    fn mark_line_dirty(&mut self, _line: usize) {
        // For simplicity, mark entire file dirty
        // A more sophisticated implementation could track per-line dirty state
        self.highlight_dirty = true;
    }

    /// Start tracking changes for LLM diff display
    pub fn start_diff_tracking(&mut self) {
        self.diff_tracker.start_tracking(&self.lines);
    }

    /// Stop diff tracking
    pub fn stop_diff_tracking(&mut self) {
        self.diff_tracker.stop_tracking();
    }

    /// Update diff after content changes
    fn update_diff(&mut self) {
        if self.diff_tracker.is_tracking() {
            self.diff_tracker.update_diff(&self.lines);
        }
    }

    /// Get current line
    fn current_line(&self) -> &str {
        self.lines
            .get(self.cursor.0)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Insert character at cursor
    fn insert_char(&mut self, c: char) {
        let row = self.cursor.0;
        let col = self.cursor.1;
        let line = &mut self.lines[row];
        let col = col.min(line.chars().count());

        // Insert at byte position
        let byte_pos: usize = line.chars().take(col).map(|ch| ch.len_utf8()).sum();
        line.insert(byte_pos, c);

        self.cursor.1 = col + 1;
        self.modified = true;
        self.mark_line_dirty(row);
        self.update_diff();
    }

    /// Delete character before cursor (backspace)
    fn backspace(&mut self) {
        let row = self.cursor.0;
        let col = self.cursor.1;

        if col > 0 {
            let line = &mut self.lines[row];
            let col = col.min(line.chars().count());

            if col > 0 {
                // Find byte position of previous char
                let byte_pos: usize = line.chars().take(col).map(|ch| ch.len_utf8()).sum();
                let prev_char_len = line
                    .chars()
                    .nth(col - 1)
                    .map(|ch| ch.len_utf8())
                    .unwrap_or(1);
                line.remove(byte_pos - prev_char_len);
                self.cursor.1 = col - 1;
                self.modified = true;
                self.mark_line_dirty(row);
            }
        } else if row > 0 {
            // Join with previous line
            let current = self.lines.remove(row);
            self.cursor.0 = row - 1;
            self.cursor.1 = self.lines[self.cursor.0].chars().count();
            self.lines[self.cursor.0].push_str(&current);
            self.modified = true;
            self.highlight_dirty = true;
        }
        self.update_diff();
    }

    /// Delete character at cursor
    fn delete(&mut self) {
        let row = self.cursor.0;
        let col = self.cursor.1;
        let line_len = self.lines[row].chars().count();

        if col < line_len {
            let line = &mut self.lines[row];
            let byte_pos: usize = line.chars().take(col).map(|ch| ch.len_utf8()).sum();
            line.remove(byte_pos);
            self.modified = true;
            self.mark_line_dirty(row);
        } else if row + 1 < self.lines.len() {
            // Join with next line
            let next = self.lines.remove(row + 1);
            self.lines[row].push_str(&next);
            self.modified = true;
            self.highlight_dirty = true;
        }
        self.update_diff();
    }

    /// Insert newline at cursor
    fn newline(&mut self) {
        let row = self.cursor.0;
        let col = self.cursor.1;
        let line = &mut self.lines[row];
        let byte_pos: usize = line.chars().take(col).map(|ch| ch.len_utf8()).sum();

        let rest = line.split_off(byte_pos);
        self.cursor.0 = row + 1;
        self.cursor.1 = 0;
        self.lines.insert(self.cursor.0, rest);
        self.modified = true;
        self.highlight_dirty = true;
        self.update_diff();
    }

    /// Move cursor and adjust scroll to keep cursor visible
    fn move_cursor(&mut self, dir: Direction) {
        match dir {
            Direction::Up => {
                if self.cursor.0 > 0 {
                    self.cursor.0 -= 1;
                    let line_len = self.current_line().chars().count();
                    self.cursor.1 = self.cursor.1.min(line_len);
                }
            }
            Direction::Down => {
                if self.cursor.0 + 1 < self.lines.len() {
                    self.cursor.0 += 1;
                    let line_len = self.current_line().chars().count();
                    self.cursor.1 = self.cursor.1.min(line_len);
                }
            }
            Direction::Left => {
                if self.cursor.1 > 0 {
                    self.cursor.1 -= 1;
                } else if self.cursor.0 > 0 {
                    self.cursor.0 -= 1;
                    self.cursor.1 = self.current_line().chars().count();
                }
            }
            Direction::Right => {
                let line_len = self.current_line().chars().count();
                if self.cursor.1 < line_len {
                    self.cursor.1 += 1;
                } else if self.cursor.0 + 1 < self.lines.len() {
                    self.cursor.0 += 1;
                    self.cursor.1 = 0;
                }
            }
        }
        self.ensure_cursor_visible();
    }

    /// Scroll up by n lines
    fn scroll_up(&mut self, n: usize) {
        self.scroll.0 = self.scroll.0.saturating_sub(n);
    }

    /// Scroll down by n lines
    fn scroll_down(&mut self, n: usize) {
        let max_scroll = self.lines.len().saturating_sub(1);
        self.scroll.0 = (self.scroll.0 + n).min(max_scroll);
    }

    /// Scroll up by half page
    fn scroll_half_page_up(&mut self) {
        let half = self.visible_height / 2;
        self.scroll_up(half.max(1));
    }

    /// Scroll down by half page
    fn scroll_half_page_down(&mut self) {
        let half = self.visible_height / 2;
        self.scroll_down(half.max(1));
    }

    /// Scroll up by full page
    fn scroll_page_up(&mut self) {
        self.scroll_up(self.visible_height.saturating_sub(2));
    }

    /// Scroll down by full page
    fn scroll_page_down(&mut self) {
        self.scroll_down(self.visible_height.saturating_sub(2));
    }

    /// Ensure cursor is visible within the viewport
    fn ensure_cursor_visible(&mut self) {
        if self.cursor.0 < self.scroll.0 {
            self.scroll.0 = self.cursor.0;
        } else if self.cursor.0 >= self.scroll.0 + self.visible_height {
            self.scroll.0 = self.cursor.0 - self.visible_height + 1;
        }
    }

    /// Get title for display
    fn title(&self) -> String {
        let name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[New]".to_string());

        let syntax = self
            .highlighter
            .detect_syntax(self.file_path.as_deref());

        let diff_indicator = if self.diff_tracker.is_tracking() {
            " [DIFF]"
        } else {
            ""
        };

        // Add scroll indicator
        let scroll_info = crate::ui::scroll::scroll_indicator(
            self.scroll.0,
            self.visible_height,
            self.lines.len(),
        );

        let modified_marker = if self.modified { " •" } else { "" };

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
            // Handle PageUp/PageDown in both modes
            match key.code {
                KeyCode::PageUp => {
                    self.scroll_page_up();
                    // Move cursor to stay in view
                    if self.cursor.0 >= self.scroll.0 + self.visible_height {
                        self.cursor.0 = self.scroll.0 + self.visible_height - 1;
                    }
                    return Ok(true);
                }
                KeyCode::PageDown => {
                    self.scroll_page_down();
                    // Move cursor to stay in view
                    if self.cursor.0 < self.scroll.0 {
                        self.cursor.0 = self.scroll.0;
                    }
                    return Ok(true);
                }
                _ => {}
            }

            // Normal mode: vim-style navigation + arrow key scrolling
            if !state.input_mode.is_editing() {
                return match key.code {
                    // Arrow keys scroll the view
                    KeyCode::Up => {
                        self.scroll_up(1);
                        Ok(true)
                    }
                    KeyCode::Down => {
                        self.scroll_down(1);
                        Ok(true)
                    }
                    KeyCode::Left => {
                        self.scroll.1 = self.scroll.1.saturating_sub(1);
                        Ok(true)
                    }
                    KeyCode::Right => {
                        self.scroll.1 += 1;
                        Ok(true)
                    }
                    // Vim keys move cursor
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
                    // Ctrl+U/D for half-page scroll
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.scroll_half_page_up();
                        Ok(true)
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.scroll_half_page_down();
                        Ok(true)
                    }
                    // g/G for top/bottom
                    KeyCode::Char('g') => {
                        self.cursor = (0, 0);
                        self.scroll.0 = 0;
                        Ok(true)
                    }
                    KeyCode::Char('G') => {
                        self.cursor.0 = self.lines.len().saturating_sub(1);
                        self.cursor.1 = 0;
                        self.ensure_cursor_visible();
                        Ok(true)
                    }
                    KeyCode::Home => {
                        self.cursor = (0, 0);
                        self.scroll.0 = 0;
                        Ok(true)
                    }
                    KeyCode::End => {
                        self.cursor.0 = self.lines.len().saturating_sub(1);
                        self.cursor.1 = 0;
                        self.ensure_cursor_visible();
                        Ok(true)
                    }
                    _ => Ok(false),
                };
            }

            // Insert mode: editing
            match key.code {
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.insert_char(c);
                    Ok(true)
                }
                KeyCode::Backspace => {
                    self.backspace();
                    Ok(true)
                }
                KeyCode::Delete => {
                    self.delete();
                    Ok(true)
                }
                KeyCode::Enter => {
                    self.newline();
                    Ok(true)
                }
                KeyCode::Up => {
                    self.move_cursor(Direction::Up);
                    Ok(true)
                }
                KeyCode::Down => {
                    self.move_cursor(Direction::Down);
                    Ok(true)
                }
                KeyCode::Left => {
                    self.move_cursor(Direction::Left);
                    Ok(true)
                }
                KeyCode::Right => {
                    self.move_cursor(Direction::Right);
                    Ok(true)
                }
                KeyCode::Home => {
                    self.cursor.1 = 0;
                    Ok(true)
                }
                KeyCode::End => {
                    self.cursor.1 = self.current_line().chars().count();
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .title(self.title())
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        let visible_height = inner.height as usize;

        // Calculate line number width (+ 1 for diff gutter)
        let line_count = self.lines.len();
        let gutter_width = format!("{}", line_count).len() + 2; // +2 for diff marker and space

        // Use stored scroll position
        let scroll_y = self.scroll.0;

        // Build lines with syntax highlighting
        let lines: Vec<Line> = self
            .lines
            .iter()
            .enumerate()
            .skip(scroll_y)
            .take(visible_height)
            .map(|(idx, line)| {
                // Get diff change type
                let change = self.diff_tracker.get_line_change(idx);

                // Diff gutter marker
                let diff_marker = Span::styled(
                    format!("{}", change.gutter_char()),
                    change.gutter_style(),
                );

                // Line number
                let line_num = format!("{:>width$} ", idx + 1, width = gutter_width - 2);
                let line_num_style = Style::default().fg(Color::DarkGray);

                // Get highlighted content or use cached
                let content_spans: Vec<Span> = if idx < self.highlighted_lines.len() {
                    self.highlighted_lines[idx]
                        .iter()
                        .map(|(text, style)| {
                            // Apply line background for diffs
                            let mut s = *style;
                            if let Some(bg_style) = change.line_bg_style() {
                                s = s.bg(bg_style.bg.unwrap_or(Color::Reset));
                            }
                            Span::styled(text.clone(), s)
                        })
                        .collect()
                } else {
                    // Fallback: plain text
                    vec![Span::raw(line.clone())]
                };

                // Build line with gutter
                let mut spans = vec![
                    diff_marker,
                    Span::styled(line_num, line_num_style),
                ];
                spans.extend(content_spans);

                Line::from(spans)
            })
            .collect();

        // Show scroll position indicator in title if scrolled
        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);

        // Show cursor if focused and cursor is in view
        if focused {
            // Check if cursor is within visible range
            if self.cursor.0 >= scroll_y && self.cursor.0 < scroll_y + visible_height {
                let cursor_x = inner.x + gutter_width as u16 + self.cursor.1 as u16;
                let cursor_y = inner.y + (self.cursor.0 - scroll_y) as u16;

                if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                    frame.set_cursor_position((cursor_x, cursor_y));
                }
            }
        }
    }

    fn on_resize(&mut self, _cols: u16, rows: u16) {
        // Update visible height (account for borders)
        self.visible_height = rows.saturating_sub(2) as usize;
    }
}
