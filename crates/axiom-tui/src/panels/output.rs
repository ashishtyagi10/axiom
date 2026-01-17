//! Output panel for displaying file content or agent output

use super::Panel;
use crate::events::TuiEvent;
use crate::state::{AppState, OutputContext, PanelId};
use axiom_core::{AgentView, AxiomService, Result, TerminalScreen};
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::path::PathBuf;

/// Output panel for file/agent content
pub struct OutputPanel {
    /// Current output context
    context: OutputContext,

    /// Scroll offset
    scroll: usize,

    /// Cached content lines
    content: Vec<String>,

    /// Inner area for scroll
    inner_area: Rect,
}

impl OutputPanel {
    pub fn new() -> Self {
        Self {
            context: OutputContext::Empty,
            scroll: 0,
            content: Vec::new(),
            inner_area: Rect::default(),
        }
    }

    /// Set the output context
    pub fn set_context(&mut self, context: OutputContext) {
        self.context = context;
        self.scroll = 0;
        self.content.clear();
    }

    /// Get current context
    pub fn context(&self) -> &OutputContext {
        &self.context
    }

    /// Update content from service
    pub fn update_from_service(&mut self, service: &AxiomService) {
        self.content.clear();

        match &self.context {
            OutputContext::File { path } => {
                // Read file content
                if let Ok(content) = std::fs::read_to_string(path) {
                    self.content = content.lines().map(|s| s.to_string()).collect();
                } else {
                    self.content = vec!["(Failed to read file)".to_string()];
                }
            }
            OutputContext::Agent { agent_id } => {
                // Get agent output from service
                if let Some(screen) = service.pty_screen(*agent_id) {
                    // PTY agent - render terminal screen
                    self.content = screen
                        .lines
                        .iter()
                        .map(|line| line.cells.iter().map(|c| c.char).collect())
                        .collect();
                } else if let Some(output) = service.agent_output(*agent_id) {
                    // Regular agent output
                    self.content = output.lines().map(|s| s.to_string()).collect();
                }
            }
            OutputContext::Empty => {
                self.content = vec!["Select a file or agent to view".to_string()];
            }
        }
    }

    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        let max_scroll = self.content.len().saturating_sub(1);
        if self.scroll < max_scroll {
            self.scroll += 1;
        }
    }

    fn page_up(&mut self, height: usize) {
        self.scroll = self.scroll.saturating_sub(height);
    }

    fn page_down(&mut self, height: usize) {
        let max_scroll = self.content.len().saturating_sub(height);
        self.scroll = (self.scroll + height).min(max_scroll);
    }
}

impl Default for OutputPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for OutputPanel {
    fn id(&self) -> PanelId {
        PanelId::OUTPUT
    }

    fn name(&self) -> &str {
        "Output"
    }

    fn handle_input(&mut self, event: &TuiEvent, _state: &mut AppState) -> Result<bool> {
        match event {
            TuiEvent::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_up();
                    Ok(true)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_down();
                    Ok(true)
                }
                KeyCode::PageUp => {
                    self.page_up(self.inner_area.height as usize);
                    Ok(true)
                }
                KeyCode::PageDown => {
                    self.page_down(self.inner_area.height as usize);
                    Ok(true)
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.scroll = 0;
                    Ok(true)
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.scroll = self.content.len().saturating_sub(1);
                    Ok(true)
                }
                _ => Ok(false),
            },
            TuiEvent::Mouse(mouse) => {
                let inner = self.inner_area;
                if mouse.column >= inner.x
                    && mouse.column < inner.x + inner.width
                    && mouse.row >= inner.y
                    && mouse.row < inner.y + inner.height
                {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            self.scroll_up();
                            Ok(true)
                        }
                        MouseEventKind::ScrollDown => {
                            self.scroll_down();
                            Ok(true)
                        }
                        _ => Ok(false),
                    }
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        let title = match &self.context {
            OutputContext::File { path } => {
                format!(
                    " {} ",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )
            }
            OutputContext::Agent { agent_id } => {
                format!(" Agent {} ", agent_id.value())
            }
            OutputContext::Empty => " Output ".to_string(),
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let inner = block.inner(area);
        self.inner_area = inner;

        let visible = inner.height as usize;
        let lines: Vec<Line> = self
            .content
            .iter()
            .skip(self.scroll)
            .take(visible)
            .map(|s| Line::from(s.as_str()))
            .collect();

        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }
}
