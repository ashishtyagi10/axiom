//! Chat panel for AI interaction with markdown support

use crate::core::Result;
use crate::events::Event;
use crate::llm::SharedProvider;
use crate::state::{AppState, PanelId};
use crate::ui::{render_markdown, ScrollBar};
use crossbeam_channel::Sender;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::cell::Cell;

/// File modification parsed from LLM response
#[derive(Debug, Clone)]
pub struct FileModification {
    pub path: String,
    pub content: String,
}

/// Parse code blocks with file paths from LLM response
/// Format: ```lang:path/to/file.ext or ```path/to/file.ext
fn parse_file_modifications(content: &str) -> Vec<FileModification> {
    let mut modifications = Vec::new();
    let mut remaining = content;

    while let Some(start_idx) = remaining.find("```") {
        let after_backticks = &remaining[start_idx + 3..];

        // Find end of first line (the info string)
        if let Some(newline_idx) = after_backticks.find('\n') {
            let info_string = &after_backticks[..newline_idx];

            // Find closing ```
            let content_start = &after_backticks[newline_idx + 1..];
            if let Some(end_idx) = content_start.find("```") {
                let code_content = &content_start[..end_idx];

                // Parse info string for file path
                // Format: lang:path or just path
                if let Some(path) = extract_file_path(info_string) {
                    modifications.push(FileModification {
                        path,
                        content: code_content.to_string(),
                    });
                }

                // Move past this code block
                remaining = &content_start[end_idx + 3..];
                continue;
            }
        }

        // Couldn't parse this block, move past the ```
        remaining = &remaining[start_idx + 3..];
    }

    modifications
}

/// Extract file path from code block info string
/// Supports: "rust:src/main.rs", "src/main.rs", "python:utils/helper.py"
fn extract_file_path(info_string: &str) -> Option<String> {
    let trimmed = info_string.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Check for lang:path format
    if let Some(colon_idx) = trimmed.find(':') {
        let path = trimmed[colon_idx + 1..].trim();
        if looks_like_file_path(path) {
            return Some(path.to_string());
        }
    }

    // Check if the whole info string is a path
    if looks_like_file_path(trimmed) {
        return Some(trimmed.to_string());
    }

    None
}

/// Check if a string looks like a file path
fn looks_like_file_path(s: &str) -> bool {
    // Must contain a dot (extension) or path separator
    (s.contains('.') || s.contains('/') || s.contains('\\'))
        // Must not contain spaces (likely natural language)
        && !s.contains(' ')
        // Must not be too long
        && s.len() < 256
}

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

    /// LLM provider (supports multiple backends)
    llm: SharedProvider,

    /// Event sender for LLM responses
    event_tx: Option<Sender<Event>>,

    /// Scroll offset for chat history (0 = bottom)
    scroll_offset: usize,

    /// History area for scroll bar click detection (updated during render)
    history_area: Cell<Rect>,

    /// Total history lines count (updated during render)
    history_line_count: Cell<usize>,
}

impl ChatPanel {
    /// Create new chat panel with the given LLM provider
    pub fn new(event_tx: Sender<Event>, llm: SharedProvider) -> Self {
        Self {
            messages: vec![ChatMessage {
                role: Role::System,
                content: "Welcome to Axiom! Type a message and press Enter to chat.".to_string(),
            }],
            input: String::new(),
            cursor: 0,
            is_generating: false,
            streaming_buffer: String::new(),
            llm,
            event_tx: Some(event_tx),
            scroll_offset: 0,
            history_area: Cell::new(Rect::default()),
            history_line_count: Cell::new(0),
        }
    }

    /// Switch to a different LLM provider
    pub fn set_provider(&mut self, provider: SharedProvider) {
        self.llm = provider;
    }

    /// Get the current provider name
    pub fn provider_name(&self) -> &str {
        self.llm.name()
    }

    /// Get the current provider ID
    pub fn provider_id(&self) -> &str {
        self.llm.id()
    }

    /// Scroll to bottom of chat history
    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
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

        // Reset scroll to bottom when user sends message
        self.scroll_offset = 0;

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
        // Reset scroll to bottom when new content arrives
        self.scroll_offset = 0;
    }

    /// Complete streaming response
    pub fn complete_response(&mut self) {
        if !self.streaming_buffer.is_empty() {
            let content = std::mem::take(&mut self.streaming_buffer);

            // Parse for file modifications and send events
            let modifications = parse_file_modifications(&content);
            if let Some(ref event_tx) = self.event_tx {
                for modification in modifications {
                    let _ = event_tx.send(Event::FileModification {
                        path: modification.path,
                        content: modification.content,
                    });
                }
            }

            self.messages.push(ChatMessage {
                role: Role::Assistant,
                content,
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
        self.llm.list_models().map_err(|e| e.to_string())
    }

    /// Set the model to use
    pub fn set_model(&self, model: &str) {
        let _ = self.llm.set_model(model);
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
            Event::Mouse(mouse) => {
                match mouse.kind {
                    crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                        let x = mouse.column;
                        let y = mouse.row;
                        let area = self.history_area.get();
                        let total = self.history_line_count.get();
                        let visible = area.height as usize;

                        // Chat uses inverted scroll (scroll_offset 0 = at bottom)
                        // Convert to normal scroll position for ScrollBar
                        let max_scroll = total.saturating_sub(visible);
                        let normal_scroll = max_scroll.saturating_sub(self.scroll_offset);

                        let scrollbar = ScrollBar::new(normal_scroll, visible, total);

                        if scrollbar.is_arrow_click(x, y, area) {
                            // Down arrow = scroll to bottom (newest)
                            self.scroll_to_bottom();
                            return Ok(true);
                        }
                        if let Some(page_up) = scrollbar.track_click(x, y, area) {
                            let page_size = visible;
                            if page_up {
                                // Page up = see older messages = increase offset
                                self.scroll_offset = self.scroll_offset.saturating_add(page_size);
                            } else {
                                // Page down = see newer messages = decrease offset
                                self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
                            }
                            return Ok(true);
                        }
                        Ok(false)
                    }
                    crossterm::event::MouseEventKind::ScrollUp => {
                        // Scroll up (increase offset to see older messages)
                        self.scroll_offset = self.scroll_offset.saturating_add(3);
                        Ok(true)
                    }
                    crossterm::event::MouseEventKind::ScrollDown => {
                        // Scroll down (decrease offset toward newest messages)
                        self.scroll_offset = self.scroll_offset.saturating_sub(3);
                        Ok(true)
                    }
                    _ => Ok(false),
                }
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

        // Store for scroll bar click detection
        self.history_area.set(history_inner);
        self.history_line_count.set(history_lines.len());

        // Calculate scroll position with manual offset support
        let visible_height = history_inner.height as usize;
        let history_len = history_lines.len();
        let max_scroll = if history_len > visible_height {
            history_len - visible_height
        } else {
            0
        };
        // scroll_offset is from bottom (0 = at bottom, N = N lines up from bottom)
        let scroll = max_scroll.saturating_sub(self.scroll_offset).min(max_scroll);

        // Generate scroll indicator and model info
        let scroll_info = crate::ui::scroll::scroll_indicator(scroll, visible_height, history_len);
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

        // Render scroll bar for history area
        let scrollbar = ScrollBar::new(scroll, visible_height, history_len);
        scrollbar.render(frame, history_inner, focused);

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
                    // Lines before the last: each takes ceil(len/width) rows, minimum 1 for empty lines
                    let line_rows = if line.is_empty() {
                        1
                    } else {
                        (line.len() + inner_width - 1) / inner_width
                    };
                    row += line_rows as u16;
                }
            }

            // Last segment determines column and any additional wrapped rows
            let last_segment = lines.last().unwrap_or(&"");
            let col = if last_segment.is_empty() {
                0
            } else {
                (last_segment.len() % inner_width) as u16
            };
            row += if last_segment.is_empty() {
                0
            } else {
                ((last_segment.len() - 1) / inner_width) as u16
            };

            let cursor_x = input_inner.x + col;
            let cursor_y = input_inner.y + row;

            if cursor_x < input_inner.x + input_inner.width && cursor_y < input_inner.y + input_inner.height {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_modifications_with_lang_colon_path() {
        let content = r#"Here's the updated code:

```rust:src/main.rs
fn main() {
    println!("Hello!");
}
```

Hope that helps!"#;

        let mods = parse_file_modifications(content);
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].path, "src/main.rs");
        assert!(mods[0].content.contains("fn main()"));
    }

    #[test]
    fn test_parse_file_modifications_path_only() {
        let content = r#"```src/lib.rs
pub fn hello() {}
```"#;

        let mods = parse_file_modifications(content);
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].path, "src/lib.rs");
    }

    #[test]
    fn test_parse_file_modifications_multiple() {
        let content = r#"```rust:src/a.rs
fn a() {}
```

```python:utils/b.py
def b():
    pass
```"#;

        let mods = parse_file_modifications(content);
        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].path, "src/a.rs");
        assert_eq!(mods[1].path, "utils/b.py");
    }

    #[test]
    fn test_parse_file_modifications_ignores_plain_code_blocks() {
        let content = r#"```rust
fn example() {}
```"#;

        let mods = parse_file_modifications(content);
        assert_eq!(mods.len(), 0); // No path, so ignored
    }

    #[test]
    fn test_looks_like_file_path() {
        assert!(looks_like_file_path("src/main.rs"));
        assert!(looks_like_file_path("file.txt"));
        assert!(looks_like_file_path("path/to/file"));
        assert!(!looks_like_file_path("rust")); // language name, not path
        assert!(!looks_like_file_path("some text with spaces"));
    }
}
