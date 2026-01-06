//! Chat panel for AI interaction with markdown support

use crate::core::Result;
use crate::events::Event;
use crate::llm::{LlmProvider, OllamaProvider};
use crate::state::{AppState, PanelId};
use crate::ui::render_markdown;
use crossbeam_channel::Sender;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::sync::Arc;

/// Chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Role {
    User,
    Assistant,
    System,
}

/// Chat panel state
pub struct ChatPanel {
    /// Chat history
    messages: Vec<ChatMessage>,

    /// Current input buffer
    input: String,

    /// Input cursor position
    cursor: usize,

    /// Whether AI is currently generating
    is_generating: bool,

    /// Accumulated streaming response
    streaming_buffer: String,

    /// LLM provider (Ollama)
    llm: Arc<OllamaProvider>,

    /// Event sender for LLM responses
    event_tx: Option<Sender<Event>>,
}

impl ChatPanel {
    /// Create new chat panel
    pub fn new(event_tx: Sender<Event>) -> Self {
        Self {
            messages: vec![ChatMessage {
                role: Role::System,
                content: "Welcome to Axiom! Type a message and press Enter to chat.".to_string(),
            }],
            input: String::new(),
            cursor: 0,
            is_generating: false,
            streaming_buffer: String::new(),
            llm: Arc::new(OllamaProvider::default()),
            event_tx: Some(event_tx),
        }
    }

    /// Add a user message and trigger AI response
    fn send_message(&mut self) {
        if self.input.trim().is_empty() || self.is_generating {
            return;
        }

        let content = std::mem::take(&mut self.input);
        self.cursor = 0;

        self.messages.push(ChatMessage {
            role: Role::User,
            content: content.clone(),
        });

        // Start generating
        self.is_generating = true;
        self.streaming_buffer.clear();

        // Build message history for LLM
        let llm_messages: Vec<crate::llm::ChatMessage> = self
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| match m.role {
                Role::User => crate::llm::ChatMessage::user(&m.content),
                Role::Assistant => crate::llm::ChatMessage::assistant(&m.content),
                Role::System => crate::llm::ChatMessage::system(&m.content),
            })
            .collect();

        // Send to LLM
        if let Some(ref event_tx) = self.event_tx {
            self.llm.send_message(llm_messages, event_tx.clone());
        }
    }

    /// Append streaming chunk
    pub fn append_chunk(&mut self, chunk: &str) {
        self.streaming_buffer.push_str(chunk);
    }

    /// Complete streaming response
    pub fn complete_response(&mut self) {
        if !self.streaming_buffer.is_empty() {
            self.messages.push(ChatMessage {
                role: Role::Assistant,
                content: std::mem::take(&mut self.streaming_buffer),
            });
        }
        self.is_generating = false;
    }

    /// Insert character at cursor
    fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Delete character before cursor
    fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find previous char boundary
            let mut new_cursor = self.cursor - 1;
            while !self.input.is_char_boundary(new_cursor) && new_cursor > 0 {
                new_cursor -= 1;
            }
            self.input.remove(new_cursor);
            self.cursor = new_cursor;
        }
    }

    /// Move cursor up or down by lines
    fn move_cursor_vertically(&mut self, delta: i32) {
        let lines: Vec<&str> = self.input.split('\n').collect();
        if lines.is_empty() {
            return;
        }

        // Find current line and column
        let mut current_line = 0;
        let mut current_col = 0;
        let mut pos = 0;

        for (i, line) in lines.iter().enumerate() {
            if pos + line.len() >= self.cursor {
                current_line = i;
                current_col = self.cursor - pos;
                break;
            }
            pos += line.len() + 1; // +1 for newline
        }

        // Calculate new line
        let new_line = (current_line as i32 + delta).clamp(0, lines.len() as i32 - 1) as usize;

        if new_line == current_line {
            return;
        }

        // Calculate new cursor position
        let mut new_cursor = 0;
        for (i, line) in lines.iter().enumerate() {
            if i == new_line {
                new_cursor += current_col.min(line.len());
                break;
            }
            new_cursor += line.len() + 1;
        }

        self.cursor = new_cursor.min(self.input.len());
    }

    /// Format messages for display with markdown support
    fn format_history(&self, _width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for msg in &self.messages {
            match msg.role {
                Role::User => {
                    // User messages: simple display with green prefix
                    lines.push(Line::from(Span::styled(
                        "You:".to_string(),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    )));
                    for content_line in msg.content.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", content_line),
                            Style::default().fg(Color::Green),
                        )));
                    }
                }
                Role::Assistant => {
                    // Assistant messages: render as markdown
                    lines.push(Line::from(Span::styled(
                        "AI:".to_string(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )));
                    let md_lines = render_markdown(&msg.content, Style::default().fg(Color::White));
                    for line in md_lines {
                        // Indent markdown content
                        let mut indented = vec![Span::raw("  ")];
                        indented.extend(line.spans);
                        lines.push(Line::from(indented));
                    }
                }
                Role::System => {
                    // System messages: italic gray
                    for content_line in msg.content.lines() {
                        lines.push(Line::from(Span::styled(
                            content_line.to_string(),
                            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                        )));
                    }
                }
            }
            lines.push(Line::from("")); // Spacing between messages
        }

        // Add streaming buffer if generating (render as markdown)
        if self.is_generating {
            lines.push(Line::from(Span::styled(
                "AI:".to_string(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));

            if !self.streaming_buffer.is_empty() {
                let md_lines = render_markdown(&self.streaming_buffer, Style::default().fg(Color::White));
                for line in md_lines {
                    let mut indented = vec![Span::raw("  ")];
                    indented.extend(line.spans);
                    lines.push(Line::from(indented));
                }
            }

            // Blinking cursor
            lines.push(Line::from(Span::styled(
                "  ▌".to_string(),
                Style::default().fg(Color::Cyan),
            )));
        }

        lines
    }

    /// Calculate input height based on content
    fn input_height(&self, width: u16) -> u16 {
        if width == 0 {
            return 3;
        }
        let inner_width = width.saturating_sub(2) as usize; // Account for borders
        if inner_width == 0 {
            return 3;
        }

        // Count lines including wrapped lines
        let mut line_count = 0;
        for line in self.input.split('\n') {
            let wrapped_lines = (line.len() / inner_width) + 1;
            line_count += wrapped_lines;
        }

        // Minimum 3, maximum 10
        (line_count as u16 + 2).clamp(3, 10)
    }

    /// Get the current model name
    pub fn current_model(&self) -> String {
        self.llm.model()
    }

    /// List available models
    pub fn list_models(&self) -> std::result::Result<Vec<String>, String> {
        self.llm.list_models()
    }

    /// Set the model to use
    pub fn set_model(&self, model: &str) {
        self.llm.set_model(model);
    }
}

impl super::Panel for ChatPanel {
    fn id(&self) -> PanelId {
        PanelId::CHAT
    }

    fn name(&self) -> &str {
        "Chat"
    }

    fn handle_input(&mut self, event: &Event, _state: &mut AppState) -> Result<bool> {
        match event {
            Event::Key(key) => {
                match key.code {
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.insert_char(c);
                        Ok(true)
                    }
                    KeyCode::Backspace => {
                        self.backspace();
                        Ok(true)
                    }
                    // Shift+Enter or Alt+Enter: insert newline
                    KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT)
                        || key.modifiers.contains(KeyModifiers::ALT) => {
                        self.insert_char('\n');
                        Ok(true)
                    }
                    // Plain Enter: send message
                    KeyCode::Enter => {
                        self.send_message();
                        Ok(true)
                    }
                    KeyCode::Left => {
                        if self.cursor > 0 {
                            self.cursor -= 1;
                            while self.cursor > 0 && !self.input.is_char_boundary(self.cursor) {
                                self.cursor -= 1;
                            }
                        }
                        Ok(true)
                    }
                    KeyCode::Right => {
                        if self.cursor < self.input.len() {
                            self.cursor += 1;
                            while self.cursor < self.input.len() && !self.input.is_char_boundary(self.cursor) {
                                self.cursor += 1;
                            }
                        }
                        Ok(true)
                    }
                    KeyCode::Up => {
                        // Move cursor up a line
                        self.move_cursor_vertically(-1);
                        Ok(true)
                    }
                    KeyCode::Down => {
                        // Move cursor down a line
                        self.move_cursor_vertically(1);
                        Ok(true)
                    }
                    KeyCode::Home => {
                        // Move to start of current line
                        self.cursor = self.input[..self.cursor]
                            .rfind('\n')
                            .map(|p| p + 1)
                            .unwrap_or(0);
                        Ok(true)
                    }
                    KeyCode::End => {
                        // Move to end of current line
                        self.cursor = self.input[self.cursor..]
                            .find('\n')
                            .map(|p| self.cursor + p)
                            .unwrap_or(self.input.len());
                        Ok(true)
                    }
                    _ => Ok(false),
                }
            }
            Event::LlmChunk(chunk) => {
                self.append_chunk(chunk);
                Ok(true)
            }
            Event::LlmDone => {
                self.complete_response();
                Ok(true)
            }
            Event::LlmError(err) => {
                self.messages.push(ChatMessage {
                    role: Role::System,
                    content: format!("Error: {}", err),
                });
                self.is_generating = false;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Split into history and input areas with dynamic input height
        let input_height = self.input_height(area.width);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(input_height)])
            .split(area);

        // Calculate history area dimensions
        let temp_block = Block::default().borders(Borders::ALL);
        let history_inner = temp_block.inner(chunks[0]);
        let history_lines = self.format_history(history_inner.width);

        // Auto-scroll to bottom
        let visible_height = history_inner.height as usize;
        let scroll = if history_lines.len() > visible_height {
            history_lines.len() - visible_height
        } else {
            0
        };

        // Generate scroll indicator and model info
        let scroll_info = crate::ui::scroll::scroll_indicator(scroll, visible_height, history_lines.len());
        let model_name = self.llm.model();
        let status = if self.is_generating { " ⏳" } else { "" };
        let title = format!(" Chat ({}){}{}  ", model_name, status, scroll_info);

        // History area
        let history_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let history = Paragraph::new(history_lines)
            .block(history_block)
            .wrap(Wrap { trim: false })
            .scroll((scroll as u16, 0));

        frame.render_widget(history, chunks[0]);

        // Input area with text wrapping
        let input_block = Block::default()
            .title(" Message (Shift+Enter: newline) ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let input = Paragraph::new(self.input.as_str())
            .block(input_block)
            .wrap(Wrap { trim: false });

        frame.render_widget(input, chunks[1]);

        // Cursor in input area (handle multi-line)
        if focused {
            let input_inner = Block::default().borders(Borders::ALL).inner(chunks[1]);
            let inner_width = input_inner.width.max(1) as usize;

            // Calculate cursor position accounting for newlines and wrapping
            let text_before_cursor = &self.input[..self.cursor];
            let mut row = 0u16;

            // Process each line before cursor
            let lines: Vec<&str> = text_before_cursor.split('\n').collect();
            for (i, line) in lines.iter().enumerate() {
                if i < lines.len() - 1 {
                    // Not the last segment - add wrapped lines + 1 for newline
                    row += (line.len() / inner_width) as u16 + 1;
                }
            }

            // Last segment determines column and any additional wrapped rows
            let last_segment = lines.last().unwrap_or(&"");
            row += (last_segment.len() / inner_width) as u16;
            let col = (last_segment.len() % inner_width) as u16;

            let cursor_x = input_inner.x + col;
            let cursor_y = input_inner.y + row;

            if cursor_x < input_inner.x + input_inner.width && cursor_y < input_inner.y + input_inner.height {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}
