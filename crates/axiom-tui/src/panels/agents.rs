//! Agents panel for displaying active/completed agents

use super::Panel;
use crate::events::TuiEvent;
use crate::state::{AppState, OutputContext, PanelId};
use axiom_core::{AgentId, AgentStatus, AgentType, AgentView, AxiomService, Result};
use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Agents panel showing spawned agents
pub struct AgentsPanel {
    /// Selected index in the list
    selected_index: usize,

    /// Scroll offset
    scroll_offset: usize,

    /// Visible height (in items, each item is 2 lines)
    visible_height: usize,

    /// List area for mouse detection
    list_area: Rect,

    /// Cached agent views
    agents: Vec<AgentView>,

    /// Pending context switch
    pub pending_context: Option<OutputContext>,
}

impl AgentsPanel {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            scroll_offset: 0,
            visible_height: 10,
            list_area: Rect::default(),
            agents: Vec::new(),
            pending_context: None,
        }
    }

    /// Take pending context switch
    pub fn take_pending_context(&mut self) -> Option<OutputContext> {
        self.pending_context.take()
    }

    /// Update agents from service
    pub fn update_from_service(&mut self, service: &AxiomService) {
        self.agents = service.agents();
        // Clamp selection if agents were removed
        if !self.agents.is_empty() && self.selected_index >= self.agents.len() {
            self.selected_index = self.agents.len() - 1;
        }
    }

    /// Move selection up
    fn select_prev(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
        self.ensure_visible();
        self.notify_selection_change();
    }

    /// Move selection down
    fn select_next(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        if self.selected_index < self.agents.len() - 1 {
            self.selected_index += 1;
        }
        self.ensure_visible();
        self.notify_selection_change();
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self) {
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.visible_height {
            self.scroll_offset = self.selected_index - self.visible_height + 1;
        }
    }

    /// Notify about selection change
    fn notify_selection_change(&mut self) {
        if let Some(agent) = self.agents.get(self.selected_index) {
            self.pending_context = Some(OutputContext::Agent {
                agent_id: agent.id,
            });
        }
    }

    /// Select by click position (accounting for 2-line items)
    fn select_at(&mut self, row: u16) {
        if row < self.list_area.y || row >= self.list_area.y + self.list_area.height {
            return;
        }

        let clicked_row = (row - self.list_area.y) as usize;
        // Each agent takes 2 lines
        let new_index = self.scroll_offset + (clicked_row / 2);

        if new_index < self.agents.len() {
            self.selected_index = new_index;
            self.notify_selection_change();
        }
    }

    /// Format duration for display
    fn format_duration(secs: f64) -> String {
        let secs = secs as u64;
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else {
            format!("{}h", secs / 3600)
        }
    }

    /// Build a spinner character based on elapsed time
    fn spinner(elapsed_ms: u128) -> char {
        const FRAMES: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
        let idx = ((elapsed_ms / 100) % FRAMES.len() as u128) as usize;
        FRAMES[idx]
    }

    /// Build an animated bar for running agents
    fn animated_bar(elapsed_ms: u128, width: usize) -> String {
        let cycle = (elapsed_ms / 150) as usize % (width * 2);
        let pos = if cycle < width { cycle } else { width * 2 - cycle - 1 };

        let mut bar = String::with_capacity(width + 2);
        bar.push('[');
        for i in 0..width {
            if i == pos {
                bar.push('▓');
            } else if i == pos.saturating_sub(1) || i == pos + 1 {
                bar.push('▒');
            } else {
                bar.push('░');
            }
        }
        bar.push(']');
        bar
    }
}

impl Default for AgentsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for AgentsPanel {
    fn id(&self) -> PanelId {
        PanelId::AGENTS
    }

    fn name(&self) -> &str {
        "Agents"
    }

    fn handle_input(&mut self, event: &TuiEvent, _state: &mut AppState) -> Result<bool> {
        match event {
            TuiEvent::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_prev();
                    Ok(true)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_next();
                    Ok(true)
                }
                KeyCode::Enter => {
                    self.notify_selection_change();
                    Ok(true)
                }
                KeyCode::Home => {
                    self.selected_index = 0;
                    self.scroll_offset = 0;
                    self.notify_selection_change();
                    Ok(true)
                }
                KeyCode::End => {
                    if !self.agents.is_empty() {
                        self.selected_index = self.agents.len() - 1;
                        self.ensure_visible();
                        self.notify_selection_change();
                    }
                    Ok(true)
                }
                _ => Ok(false),
            },
            TuiEvent::Mouse(mouse) => {
                let in_area = mouse.column >= self.list_area.x
                    && mouse.column < self.list_area.x + self.list_area.width
                    && mouse.row >= self.list_area.y
                    && mouse.row < self.list_area.y + self.list_area.height;

                if in_area {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            self.select_at(mouse.row);
                            Ok(true)
                        }
                        MouseEventKind::ScrollUp => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(1);
                            Ok(true)
                        }
                        MouseEventKind::ScrollDown => {
                            let max_scroll = self.agents.len().saturating_sub(self.visible_height);
                            self.scroll_offset = (self.scroll_offset + 1).min(max_scroll);
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
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let running = self.agents.iter().filter(|a| a.status.is_running()).count();
        let total = self.agents.len();

        let title = if running > 0 {
            format!(" Agents ({}/{}) ", running, total)
        } else {
            format!(" Agents ({}) ", total)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);
        self.list_area = inner;

        // Update visible height
        self.visible_height = (inner.height as usize / 2).max(1);

        if self.agents.is_empty() {
            let msg = Line::from(Span::styled(
                "No agents yet",
                Style::default().fg(Color::DarkGray),
            ));
            frame.render_widget(Paragraph::new(msg), inner);
            return;
        }

        // Build list items
        let items: Vec<ListItem> = self
            .agents
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(self.visible_height)
            .map(|(idx, agent)| {
                let is_selected = idx == self.selected_index;
                let elapsed_ms = (agent.elapsed_secs * 1000.0) as u128;

                // Spinner for running agents
                let status_indicator = if agent.status.is_running() {
                    Self::spinner(elapsed_ms)
                } else {
                    agent.status.indicator().chars().next().unwrap_or('?')
                };

                let status_style = match &agent.status {
                    AgentStatus::Pending => Style::default().fg(Color::Yellow),
                    AgentStatus::Running => {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    }
                    AgentStatus::Completed => Style::default().fg(Color::Green),
                    AgentStatus::Error(_) => Style::default().fg(Color::Red),
                    AgentStatus::Cancelled => Style::default().fg(Color::DarkGray),
                    AgentStatus::Idle => Style::default().fg(Color::Blue),
                };

                // Line 1: Status + Type icon + Name
                let name_style = if is_selected && focused {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let line1 = Line::from(vec![
                    Span::styled(format!("{} ", status_indicator), status_style),
                    Span::styled(
                        format!("{} {}", agent.agent_type.icon(), agent.name),
                        name_style,
                    ),
                ]);

                // Line 2: Stats - Time, Lines
                let duration = Self::format_duration(agent.elapsed_secs);
                let lines = if agent.line_count > 0 {
                    format!("{}L", agent.line_count)
                } else {
                    String::new()
                };

                let stats_parts = if agent.status.is_running() {
                    let bar = Self::animated_bar(elapsed_ms, 8);
                    format!("  ⏱{} {} {}", duration, lines, bar)
                } else {
                    format!("  ⏱{} {}", duration, lines)
                };

                let line2 = Line::from(Span::styled(
                    stats_parts,
                    Style::default().fg(Color::DarkGray),
                ));

                let item_style = if is_selected {
                    Style::default().bg(if focused {
                        Color::DarkGray
                    } else {
                        Color::Rgb(40, 40, 40)
                    })
                } else {
                    Style::default()
                };

                ListItem::new(vec![line1, line2]).style(item_style)
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner);
    }

    fn on_resize(&mut self, _cols: u16, rows: u16) {
        // Each agent takes 2 lines, so divide by 2
        self.visible_height = (rows.saturating_sub(2) / 2) as usize;
    }
}
