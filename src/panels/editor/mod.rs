//! Editor panel for text editing with syntax highlighting
//!
//! Features:
//! - Syntax highlighting via syntect
//! - Git-style diff tracking for LLM modifications
//! - Vim-style cursor movement
//! - Multi-file tabs support

mod diff;
mod highlight;

pub use diff::DiffTracker;
pub use highlight::Highlighter;

use crate::core::Result;
use crate::events::Event;
use crate::state::{AppState, PanelId};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
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
            match (key.code, key.modifiers) {
                // Ctrl+Tab: next tab
                (KeyCode::Tab, m) if m.contains(KeyModifiers::CONTROL) => {
                    if m.contains(KeyModifiers::SHIFT) {
                        self.prev_tab();
                    } else {
                        self.next_tab();
                    }
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
                    self.active_tab_mut().cursor.1 = 0;
                    Ok(true)
                }
                KeyCode::End => {
                    let tab = self.active_tab_mut();
                    tab.cursor.1 = tab.current_line().chars().count();
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

                let content_spans: Vec<Span> = if idx < tab.highlighted_lines.len() {
                    tab.highlighted_lines[idx]
                        .iter()
                        .map(|(text, style)| {
                            let mut s = *style;
                            if let Some(bg_style) = change.line_bg_style() {
                                s = s.bg(bg_style.bg.unwrap_or(Color::Reset));
                            }
                            Span::styled(text.clone(), s)
                        })
                        .collect()
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
}
