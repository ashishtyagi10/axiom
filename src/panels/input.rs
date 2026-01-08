//! Input panel for unified command entry
//!
//! Handles user input with prefix-based routing:
//! - Plain text → LLM conductor
//! - `!command` or `:command` → Shell execution

use crate::clipboard;
use crate::core::Result;
use crate::events::Event;
use crate::panels::Panel;
use crate::state::{AppState, PanelId};
use crossbeam_channel::Sender;
use crossterm::event::{KeyCode, KeyModifiers, MouseEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::cell::RefCell;

/// Parsed input command
#[derive(Debug, Clone)]
pub enum InputCommand {
    /// Chat message to conductor
    Chat(String),

    /// Shell command (prefixed with ! or :)
    Shell(String),

    /// Empty input
    Empty,
}

impl InputCommand {
    /// Parse input text into a command
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return InputCommand::Empty;
        }

        if trimmed.starts_with('!') || trimmed.starts_with(':') {
            InputCommand::Shell(trimmed[1..].to_string())
        } else {
            InputCommand::Chat(trimmed.to_string())
        }
    }
}

/// Input panel for command entry
pub struct InputPanel {
    /// Current input buffer
    input: String,

    /// Cursor position in input
    cursor: usize,

    /// Selection anchor (where selection started)
    selection_anchor: Option<usize>,

    /// Command history
    history: Vec<String>,

    /// Current position in history (None = new input)
    history_index: Option<usize>,

    /// Saved input when browsing history
    saved_input: String,

    /// Event sender
    event_tx: Sender<Event>,

    /// Input area for mouse detection
    input_area: RefCell<Rect>,

    /// Whether input is currently processing
    is_processing: bool,
}

impl InputPanel {
    /// Create a new input panel
    pub fn new(event_tx: Sender<Event>) -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            selection_anchor: None,
            history: Vec::new(),
            history_index: None,
            saved_input: String::new(),
            event_tx,
            input_area: RefCell::new(Rect::default()),
            is_processing: false,
        }
    }

    /// Set processing state
    pub fn set_processing(&mut self, processing: bool) {
        self.is_processing = processing;
    }

    /// Check if processing
    pub fn is_processing(&self) -> bool {
        self.is_processing
    }

    // ==================== Selection ====================

    /// Get selected text range (start, end)
    fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            let start = anchor.min(self.cursor);
            let end = anchor.max(self.cursor);
            (start, end)
        })
    }

    /// Get selected text
    fn selected_text(&self) -> Option<String> {
        self.selection_range().map(|(start, end)| {
            self.input.chars().skip(start).take(end - start).collect()
        })
    }

    /// Delete selected text
    fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            let before: String = self.input.chars().take(start).collect();
            let after: String = self.input.chars().skip(end).collect();
            self.input = format!("{}{}", before, after);
            self.cursor = start;
            self.selection_anchor = None;
        }
    }

    /// Clear selection
    fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    // ==================== Cursor Movement ====================

    /// Move cursor left
    fn move_left(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        } else if !extend_selection {
            self.clear_selection();
        }

        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right
    fn move_right(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        } else if !extend_selection {
            self.clear_selection();
        }

        let len = self.input.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    /// Move to start of line
    fn move_home(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        } else if !extend_selection {
            self.clear_selection();
        }
        self.cursor = 0;
    }

    /// Move to end of line
    fn move_end(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        } else if !extend_selection {
            self.clear_selection();
        }
        self.cursor = self.input.chars().count();
    }

    // ==================== Text Editing ====================

    /// Insert character at cursor
    fn insert_char(&mut self, c: char) {
        // Delete selection first if any
        if self.selection_anchor.is_some() {
            self.delete_selection();
        }

        let before: String = self.input.chars().take(self.cursor).collect();
        let after: String = self.input.chars().skip(self.cursor).collect();
        self.input = format!("{}{}{}", before, c, after);
        self.cursor += 1;
    }

    /// Insert string at cursor
    fn insert_str(&mut self, s: &str) {
        // Delete selection first if any
        if self.selection_anchor.is_some() {
            self.delete_selection();
        }

        let before: String = self.input.chars().take(self.cursor).collect();
        let after: String = self.input.chars().skip(self.cursor).collect();
        self.input = format!("{}{}{}", before, s, after);
        self.cursor += s.chars().count();
    }

    /// Delete character before cursor (backspace)
    fn delete_back(&mut self) {
        if self.selection_anchor.is_some() {
            self.delete_selection();
            return;
        }

        if self.cursor > 0 {
            let before: String = self.input.chars().take(self.cursor - 1).collect();
            let after: String = self.input.chars().skip(self.cursor).collect();
            self.input = format!("{}{}", before, after);
            self.cursor -= 1;
        }
    }

    /// Delete character at cursor (delete)
    fn delete_forward(&mut self) {
        if self.selection_anchor.is_some() {
            self.delete_selection();
            return;
        }

        let len = self.input.chars().count();
        if self.cursor < len {
            let before: String = self.input.chars().take(self.cursor).collect();
            let after: String = self.input.chars().skip(self.cursor + 1).collect();
            self.input = format!("{}{}", before, after);
        }
    }

    // ==================== History Navigation ====================

    /// Navigate to previous history entry
    fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // Save current input and go to most recent
                self.saved_input = self.input.clone();
                self.history_index = Some(self.history.len() - 1);
                self.input = self.history[self.history.len() - 1].clone();
            }
            Some(idx) if idx > 0 => {
                self.history_index = Some(idx - 1);
                self.input = self.history[idx - 1].clone();
            }
            _ => {}
        }

        self.cursor = self.input.chars().count();
        self.clear_selection();
    }

    /// Navigate to next history entry
    fn history_next(&mut self) {
        match self.history_index {
            Some(idx) => {
                if idx < self.history.len() - 1 {
                    self.history_index = Some(idx + 1);
                    self.input = self.history[idx + 1].clone();
                } else {
                    // Back to saved input
                    self.history_index = None;
                    self.input = self.saved_input.clone();
                }
            }
            None => {}
        }

        self.cursor = self.input.chars().count();
        self.clear_selection();
    }

    // ==================== Submit ====================

    /// Submit current input
    fn submit(&mut self) {
        if self.input.trim().is_empty() || self.is_processing {
            return;
        }

        let command = InputCommand::parse(&self.input);

        // Add to history
        self.history.push(self.input.clone());
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        // Send appropriate event
        match command {
            InputCommand::Chat(text) => {
                let _ = self.event_tx.send(Event::ConductorRequest(text));
            }
            InputCommand::Shell(cmd) => {
                let _ = self.event_tx.send(Event::ShellExecute(cmd));
            }
            InputCommand::Empty => {}
        }

        // Clear input
        self.input.clear();
        self.cursor = 0;
        self.history_index = None;
        self.clear_selection();
    }

    // ==================== Clipboard ====================

    /// Copy selection to clipboard
    fn copy_to_clipboard(&self) {
        if let Some(text) = self.selected_text() {
            let _ = clipboard::copy(&text);
        }
    }

    /// Cut selection to clipboard
    fn cut_to_clipboard(&mut self) {
        if let Some(text) = self.selected_text() {
            let _ = clipboard::copy(&text);
            self.delete_selection();
        }
    }

    /// Paste from clipboard
    fn paste_from_clipboard(&mut self) {
        if let Ok(text) = clipboard::paste() {
            // Remove newlines for single-line input
            let clean_text: String = text.replace('\n', " ").replace('\r', "");
            self.insert_str(&clean_text);
        }
    }

    /// Select all text
    fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.input.chars().count();
    }

    // ==================== Prompt ====================

    /// Get the prompt prefix based on input
    fn prompt(&self) -> (&'static str, Style) {
        let trimmed = self.input.trim_start();
        if trimmed.starts_with('!') || trimmed.starts_with(':') {
            ("$ ", Style::default().fg(Color::Yellow))
        } else {
            ("> ", Style::default().fg(Color::Cyan))
        }
    }
}

impl Panel for InputPanel {
    fn id(&self) -> PanelId {
        PanelId::INPUT
    }

    fn name(&self) -> &str {
        "Input"
    }

    fn handle_input(&mut self, event: &Event, state: &mut AppState) -> Result<bool> {
        match event {
            Event::Key(key) => {
                let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                match key.code {
                    // Submit
                    KeyCode::Enter if !shift => {
                        self.submit();
                        Ok(true)
                    }

                    // Insert newline (for multi-line input in future)
                    KeyCode::Enter if shift => {
                        self.insert_char('\n');
                        Ok(true)
                    }

                    // Navigation
                    KeyCode::Left => {
                        self.move_left(shift);
                        Ok(true)
                    }
                    KeyCode::Right => {
                        self.move_right(shift);
                        Ok(true)
                    }
                    KeyCode::Home => {
                        self.move_home(shift);
                        Ok(true)
                    }
                    KeyCode::End => {
                        self.move_end(shift);
                        Ok(true)
                    }

                    // History navigation
                    KeyCode::Up => {
                        self.history_prev();
                        Ok(true)
                    }
                    KeyCode::Down => {
                        self.history_next();
                        Ok(true)
                    }

                    // Editing
                    KeyCode::Backspace => {
                        self.delete_back();
                        Ok(true)
                    }
                    KeyCode::Delete => {
                        self.delete_forward();
                        Ok(true)
                    }

                    // Clipboard
                    KeyCode::Char('c') if ctrl => {
                        self.copy_to_clipboard();
                        Ok(true)
                    }
                    KeyCode::Char('x') if ctrl => {
                        self.cut_to_clipboard();
                        Ok(true)
                    }
                    KeyCode::Char('v') if ctrl => {
                        self.paste_from_clipboard();
                        Ok(true)
                    }
                    KeyCode::Char('a') if ctrl => {
                        self.select_all();
                        Ok(true)
                    }

                    // Clear input
                    KeyCode::Char('u') if ctrl => {
                        self.input.clear();
                        self.cursor = 0;
                        self.clear_selection();
                        Ok(true)
                    }

                    // Character input
                    KeyCode::Char(c) => {
                        if !ctrl {
                            self.insert_char(c);
                        }
                        Ok(true)
                    }

                    // Escape clears selection
                    KeyCode::Esc => {
                        if self.selection_anchor.is_some() {
                            self.clear_selection();
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    }

                    _ => Ok(false),
                }
            }
            Event::Mouse(mouse) => {
                // Handle click to position cursor
                if let MouseEventKind::Down(_) = mouse.kind {
                    let area = *self.input_area.borrow();
                    if mouse.row >= area.y
                        && mouse.row < area.y + area.height
                        && mouse.column >= area.x
                        && mouse.column < area.x + area.width
                    {
                        // Calculate cursor position from click
                        let click_col = (mouse.column - area.x) as usize;
                        // Account for prompt
                        let (prompt, _) = self.prompt();
                        let prompt_len = prompt.len();
                        if click_col > prompt_len {
                            let text_pos = click_col - prompt_len;
                            self.cursor = text_pos.min(self.input.chars().count());
                            self.clear_selection();
                        }
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        *self.input_area.borrow_mut() = area;

        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Compact title - show processing state or just prompt indicator
        let title = if self.is_processing {
            " ... "
        } else if focused {
            " Input "
        } else {
            " > "
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Build input line with prompt
        let (prompt, prompt_style) = self.prompt();
        let mut spans = vec![Span::styled(prompt, prompt_style)];

        // Render input with selection highlighting
        let chars: Vec<char> = self.input.chars().collect();
        let selection = self.selection_range();

        for (i, c) in chars.iter().enumerate() {
            let is_selected = selection
                .map(|(start, end)| i >= start && i < end)
                .unwrap_or(false);

            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };

            spans.push(Span::styled(c.to_string(), style));
        }

        // Show cursor
        if focused {
            let cursor_char = if self.cursor < chars.len() {
                chars[self.cursor].to_string()
            } else {
                " ".to_string()
            };

            // If cursor is at the position we already rendered, we need to modify it
            // For simplicity, we'll render cursor indicator after input
            if self.cursor == chars.len() {
                spans.push(Span::styled(
                    "_",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::SLOW_BLINK),
                ));
            }
        }

        let input_line = Line::from(spans);
        let paragraph = Paragraph::new(input_line);
        frame.render_widget(paragraph, inner);
    }
}
