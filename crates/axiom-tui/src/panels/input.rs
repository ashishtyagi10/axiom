//! Input panel for command entry

use super::Panel;
use crate::events::TuiEvent;
use crate::state::{AppState, PanelId};
use axiom_core::{Command, Result};
use crossbeam_channel::Sender;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Input panel for unified command entry
pub struct InputPanel {
    /// Input buffer
    buffer: String,

    /// Cursor position
    cursor: usize,

    /// Command sender (to AxiomService)
    command_tx: Option<Sender<Command>>,

    /// Pending command to send
    pub pending_command: Option<Command>,
}

impl InputPanel {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            command_tx: None,
            pending_command: None,
        }
    }

    /// Set the command sender
    pub fn set_command_sender(&mut self, tx: Sender<Command>) {
        self.command_tx = Some(tx);
    }

    /// Take pending command
    pub fn take_pending_command(&mut self) -> Option<Command> {
        self.pending_command.take()
    }

    fn submit(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let text = std::mem::take(&mut self.buffer);
        self.cursor = 0;

        // Parse command type
        let command = if text.starts_with('!') {
            // Shell command
            Command::ExecuteShell {
                command: text[1..].to_string(),
            }
        } else if text.starts_with('#') {
            // CLI agent invocation
            let parts: Vec<&str> = text[1..].splitn(2, ' ').collect();
            if !parts.is_empty() {
                Command::InvokeCliAgent {
                    agent_id: parts[0].to_string(),
                    prompt: parts.get(1).unwrap_or(&"").to_string(),
                }
            } else {
                return;
            }
        } else {
            // Regular text to conductor
            Command::ProcessInput { text }
        };

        self.pending_command = Some(command);
    }
}

impl Default for InputPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for InputPanel {
    fn id(&self) -> PanelId {
        PanelId::INPUT
    }

    fn name(&self) -> &str {
        "Input"
    }

    fn handle_input(&mut self, event: &TuiEvent, state: &mut AppState) -> Result<bool> {
        // Auto-enter insert mode when focused
        if !state.input_mode.is_editing() {
            state.input_mode.to_insert();
        }

        match event {
            TuiEvent::Key(key) => match key.code {
                KeyCode::Enter => {
                    self.submit();
                    Ok(true)
                }
                KeyCode::Char(c) => {
                    self.buffer.insert(self.cursor, c);
                    self.cursor += c.len_utf8();
                    Ok(true)
                }
                KeyCode::Backspace => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                        self.buffer.remove(self.cursor);
                    }
                    Ok(true)
                }
                KeyCode::Delete => {
                    if self.cursor < self.buffer.len() {
                        self.buffer.remove(self.cursor);
                    }
                    Ok(true)
                }
                KeyCode::Left => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                    Ok(true)
                }
                KeyCode::Right => {
                    if self.cursor < self.buffer.len() {
                        self.cursor += 1;
                    }
                    Ok(true)
                }
                KeyCode::Home => {
                    self.cursor = 0;
                    Ok(true)
                }
                KeyCode::End => {
                    self.cursor = self.buffer.len();
                    Ok(true)
                }
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        let block = Block::default()
            .title(" Input (# agent, ! shell) ")
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let inner = block.inner(area);

        let input_text = if self.buffer.is_empty() {
            Line::from(Span::styled(
                "Type a message...",
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            Line::from(vec![
                Span::raw("> "),
                Span::raw(&self.buffer),
            ])
        };

        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(input_text), inner);

        // Show cursor
        if focused {
            let cursor_x = inner.x + 2 + self.cursor as u16;
            let cursor_y = inner.y;
            if cursor_x < inner.x + inner.width {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}
