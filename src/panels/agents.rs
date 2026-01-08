//! Agents panel for displaying active/completed agents
//!
//! Shows the list of spawned agents with their status,
//! allowing users to select and view agent output.

use crate::agents::AgentRegistry;
use crate::core::Result;
use crate::events::Event;
use crate::panels::Panel;
use crate::state::{AgentId, AppState, OutputContext, PanelId};
use crossbeam_channel::Sender;
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use parking_lot::RwLock;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::cell::RefCell;
use std::sync::Arc;

/// Agents panel showing spawned agents
pub struct AgentsPanel {
    /// Reference to agent registry
    registry: Arc<RwLock<AgentRegistry>>,

    /// Event sender for context switching
    event_tx: Sender<Event>,

    /// Selected index in the list (UI state)
    selected_index: usize,

    /// Scroll offset
    scroll_offset: usize,

    /// Visible height
    visible_height: usize,

    /// List area for mouse detection
    list_area: RefCell<Rect>,

    /// Agent IDs in display order (cached for mouse click handling)
    agent_ids: RefCell<Vec<AgentId>>,
}

impl AgentsPanel {
    /// Create a new agents panel
    pub fn new(registry: Arc<RwLock<AgentRegistry>>, event_tx: Sender<Event>) -> Self {
        Self {
            registry,
            event_tx,
            selected_index: 0,
            scroll_offset: 0,
            visible_height: 20,
            list_area: RefCell::new(Rect::default()),
            agent_ids: RefCell::new(Vec::new()),
        }
    }

    /// Get the number of agents
    fn agent_count(&self) -> usize {
        self.registry.read().len()
    }

    /// Move selection up
    fn select_prev(&mut self) {
        let count = self.agent_count();
        if count == 0 {
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
        let count = self.agent_count();
        if count == 0 {
            return;
        }

        if self.selected_index < count - 1 {
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
    fn notify_selection_change(&self) {
        let ids = self.agent_ids.borrow();
        if let Some(agent_id) = ids.get(self.selected_index) {
            let _ = self.event_tx.send(Event::SwitchContext(OutputContext::Agent {
                agent_id: *agent_id,
            }));
        }
    }

    /// Select by click position (accounting for 2-line items)
    fn select_at(&mut self, row: u16) {
        let list_area = *self.list_area.borrow();
        if row < list_area.y || row >= list_area.y + list_area.height {
            return;
        }

        let clicked_row = (row - list_area.y) as usize;
        // Each agent takes 2 lines
        let new_index = self.scroll_offset + (clicked_row / 2);

        let count = self.agent_count();
        if new_index < count {
            self.selected_index = new_index;
            self.notify_selection_change();
        }
    }

    /// Format duration for display
    fn format_duration(secs: u64) -> String {
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else {
            format!("{}h", secs / 3600)
        }
    }

    /// Format token count compactly
    fn format_tokens_compact(tokens: usize) -> String {
        if tokens == 0 {
            String::new()
        } else if tokens < 1000 {
            format!("{}t", tokens)
        } else if tokens < 1_000_000 {
            format!("{:.1}Kt", tokens as f64 / 1000.0)
        } else {
            format!("{:.1}Mt", tokens as f64 / 1_000_000.0)
        }
    }

    /// Build a spinner character based on elapsed time
    fn spinner(elapsed_ms: u128) -> char {
        const FRAMES: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
        let idx = ((elapsed_ms / 100) % FRAMES.len() as u128) as usize;
        FRAMES[idx]
    }

    /// Build a mini progress bar
    fn mini_progress_bar(progress: u8, width: usize) -> String {
        let filled = (progress as usize * width) / 100;
        let empty = width.saturating_sub(filled);
        format!("[{}{}]", "▓".repeat(filled), "░".repeat(empty))
    }

    /// Build an animated bar for running agents without explicit progress
    fn animated_bar(elapsed_ms: u128, width: usize) -> String {
        // Create a sliding highlight effect
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

impl Panel for AgentsPanel {
    fn id(&self) -> PanelId {
        PanelId::AGENTS
    }

    fn name(&self) -> &str {
        "Agents"
    }

    fn handle_input(&mut self, event: &Event, _state: &mut AppState) -> Result<bool> {
        match event {
            Event::Key(key) => match key.code {
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
                    let count = self.agent_count();
                    if count > 0 {
                        self.selected_index = count - 1;
                        self.ensure_visible();
                        self.notify_selection_change();
                    }
                    Ok(true)
                }
                _ => Ok(false),
            },
            Event::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.select_at(mouse.row);
                        Ok(true)
                    }
                    MouseEventKind::ScrollUp => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(3);
                        Ok(true)
                    }
                    MouseEventKind::ScrollDown => {
                        let max_scroll = self.agent_count().saturating_sub(self.visible_height);
                        self.scroll_offset = (self.scroll_offset + 3).min(max_scroll);
                        Ok(true)
                    }
                    _ => Ok(false),
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

        let registry = self.registry.read();
        let running = registry.running_count();
        let total = registry.len();

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

        *self.list_area.borrow_mut() = inner;

        // Build list items and cache IDs
        let mut items: Vec<ListItem> = Vec::new();
        let mut ids: Vec<AgentId> = Vec::new();

        for (idx, agent) in registry.agents().enumerate() {
            ids.push(agent.id);

            let is_selected = idx == self.selected_index;
            let elapsed_ms = agent.elapsed().as_millis();

            // Spinner for running agents
            let status_indicator = if agent.status.is_running() {
                Self::spinner(elapsed_ms)
            } else {
                agent.status.indicator().chars().next().unwrap_or('?')
            };

            let status_style = match &agent.status {
                crate::agents::AgentStatus::Pending => Style::default().fg(Color::Yellow),
                crate::agents::AgentStatus::Running => {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                }
                crate::agents::AgentStatus::Completed => Style::default().fg(Color::Green),
                crate::agents::AgentStatus::Error(_) => Style::default().fg(Color::Red),
                crate::agents::AgentStatus::Cancelled => Style::default().fg(Color::DarkGray),
                crate::agents::AgentStatus::Idle => Style::default().fg(Color::Blue),
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

            // Line 2: Stats - Time, Tokens, Lines
            let duration = Self::format_duration(agent.elapsed().as_secs());
            let tokens = Self::format_tokens_compact(agent.token_count);
            let lines = if agent.line_count > 0 {
                format!("{}L", agent.line_count)
            } else {
                String::new()
            };

            // Build stats with progress indicator
            let stats_parts = if let Some(progress) = agent.progress {
                let bar = Self::mini_progress_bar(progress, 8);
                format!("  ⏱{} ◇{} {} {}", duration, tokens, lines, bar)
            } else if agent.status.is_running() {
                // Show animated running indicator for agents without explicit progress
                let bar = Self::animated_bar(elapsed_ms, 8);
                format!("  ⏱{} ◇{} {} {}", duration, tokens, lines, bar)
            } else {
                format!("  ⏱{} ◇{} {}", duration, tokens, lines)
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

            items.push(ListItem::new(vec![line1, line2]).style(item_style));
        }

        *self.agent_ids.borrow_mut() = ids;

        if items.is_empty() {
            let msg = Line::from(Span::styled(
                "No agents yet",
                Style::default().fg(Color::DarkGray),
            ));
            frame.render_widget(ratatui::widgets::Paragraph::new(msg), inner);
        } else {
            let list = List::new(items);
            frame.render_widget(list, inner);
        }
    }

    fn on_resize(&mut self, _cols: u16, rows: u16) {
        // Each agent takes 2 lines, so divide by 2
        self.visible_height = (rows.saturating_sub(2) / 2) as usize;
    }
}
