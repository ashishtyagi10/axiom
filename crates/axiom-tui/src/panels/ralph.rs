//! Ralph Loop panel for displaying iteration progress

use super::Panel;
use crate::events::TuiEvent;
use crate::state::{AppState, PanelId};
use axiom_core::{
    AxiomService, CompletionReason, IterationStatus, RalphState, RalphStatus, Result,
};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

/// Ralph Loop panel showing iteration progress
pub struct RalphPanel {
    /// Cached Ralph state
    state: Option<RalphState>,

    /// Animation tick counter for spinners
    tick_counter: u64,

    /// Scroll offset for iteration history
    scroll_offset: usize,

    /// Selected index in history
    selected_index: usize,
}

impl RalphPanel {
    pub fn new() -> Self {
        Self {
            state: None,
            tick_counter: 0,
            scroll_offset: 0,
            selected_index: 0,
        }
    }

    /// Update from service
    pub fn update_from_service(&mut self, service: &AxiomService) {
        self.state = service.ralph_state();
    }

    /// Tick the animation counter
    pub fn tick(&mut self) {
        self.tick_counter = self.tick_counter.wrapping_add(1);
    }

    /// Get spinner character
    fn spinner(&self) -> char {
        const FRAMES: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
        let idx = (self.tick_counter / 2) as usize % FRAMES.len();
        FRAMES[idx]
    }

    /// Format duration
    fn format_duration(secs: f64) -> String {
        let secs = secs as u64;
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Get status color
    fn status_color(status: &RalphStatus) -> Color {
        match status {
            RalphStatus::Idle => Color::DarkGray,
            RalphStatus::Running => Color::Cyan,
            RalphStatus::Paused => Color::Yellow,
            RalphStatus::Complete => Color::Green,
            RalphStatus::Error => Color::Red,
        }
    }

    /// Get iteration status icon
    fn iteration_icon(status: &IterationStatus) -> &'static str {
        match status {
            IterationStatus::Success => "✓",
            IterationStatus::Failed => "✗",
            IterationStatus::Timeout => "⏱",
            IterationStatus::Cancelled => "⊘",
        }
    }

    /// Get iteration status color
    fn iteration_color(status: &IterationStatus) -> Color {
        match status {
            IterationStatus::Success => Color::Green,
            IterationStatus::Failed => Color::Red,
            IterationStatus::Timeout => Color::Yellow,
            IterationStatus::Cancelled => Color::DarkGray,
        }
    }

    /// Render when no Ralph Loop is active
    fn render_idle(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .title(" Ralph Loop ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No Ralph Loop active",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Start with: #ralph <task>",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        frame.render_widget(Paragraph::new(lines), inner);
    }

    /// Render active Ralph Loop
    fn render_active(&mut self, frame: &mut Frame, area: Rect, focused: bool, state: &RalphState) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let status_indicator = if state.status == RalphStatus::Running {
            format!("{} ", self.spinner())
        } else {
            String::new()
        };

        let title = format!(" {}Ralph Loop ", status_indicator);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into sections
        let chunks = Layout::vertical([
            Constraint::Length(3), // Status header
            Constraint::Length(2), // Progress
            Constraint::Length(2), // Task
            Constraint::Min(4),    // Iteration history
        ])
        .split(inner);

        // Status header
        self.render_status_header(frame, chunks[0], state);

        // Progress bar
        self.render_progress(frame, chunks[1], state);

        // Task description
        self.render_task(frame, chunks[2], state);

        // Iteration history
        self.render_history(frame, chunks[3], state, focused);
    }

    fn render_status_header(&self, frame: &mut Frame, area: Rect, state: &RalphState) {
        let status_style = Style::default()
            .fg(Self::status_color(&state.status))
            .add_modifier(Modifier::BOLD);

        let elapsed = Self::format_duration(state.elapsed_secs() as f64);
        let success_rate = format!("{:.0}%", state.success_rate());

        let line = Line::from(vec![
            Span::styled(format!("{:?}", state.status), status_style),
            Span::raw(" | "),
            Span::styled(format!("⏱ {}", elapsed), Style::default().fg(Color::White)),
            Span::raw(" | "),
            Span::styled(
                format!("Success: {}", success_rate),
                Style::default().fg(Color::Green),
            ),
        ]);

        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_progress(&self, frame: &mut Frame, area: Rect, state: &RalphState) {
        let current = state.current_iteration;
        let max = state.config.max_iterations;

        let ratio = if max > 0 {
            (current as f64 / max as f64).min(1.0)
        } else {
            0.0
        };

        let label = format!("Iteration {}/{}", current, max);

        let gauge = Gauge::default()
            .ratio(ratio)
            .label(label)
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray));

        frame.render_widget(gauge, area);
    }

    fn render_task(&self, frame: &mut Frame, area: Rect, state: &RalphState) {
        let task = if state.task.len() > area.width as usize - 2 {
            format!("{}...", &state.task[..area.width as usize - 5])
        } else {
            state.task.clone()
        };

        let line = Line::from(vec![
            Span::styled("Task: ", Style::default().fg(Color::Yellow)),
            Span::styled(task, Style::default().fg(Color::White)),
        ]);

        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_history(&mut self, frame: &mut Frame, area: Rect, state: &RalphState, focused: bool) {
        let history = &state.iteration_history;

        if history.is_empty() {
            let line = Line::from(Span::styled(
                "No iterations yet",
                Style::default().fg(Color::DarkGray),
            ));
            frame.render_widget(Paragraph::new(line), area);
            return;
        }

        // Build list items (most recent first)
        let items: Vec<ListItem> = history
            .iter()
            .rev()
            .enumerate()
            .skip(self.scroll_offset)
            .take(area.height as usize)
            .map(|(idx, record)| {
                let is_selected = idx == self.selected_index;
                let icon = Self::iteration_icon(&record.status);
                let color = Self::iteration_color(&record.status);
                let duration = Self::format_duration(record.duration_secs);

                let summary = if record.summary.len() > 30 {
                    format!("{}...", &record.summary[..27])
                } else {
                    record.summary.clone()
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", icon),
                        Style::default().fg(color),
                    ),
                    Span::styled(
                        format!("#{} ", record.iteration),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(summary, Style::default().fg(Color::White)),
                    Span::raw(" "),
                    Span::styled(
                        format!("({})", duration),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);

                let style = if is_selected && focused {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, area);
    }

    /// Render completion message
    fn render_complete(&self, frame: &mut Frame, area: Rect, focused: bool, state: &RalphState) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let title = " Ralph Loop - Complete ";

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let reason_text = match &state.completion_reason {
            Some(CompletionReason::TaskComplete) => "✓ Task completed successfully",
            Some(CompletionReason::MaxIterations) => "⚠ Max iterations reached",
            Some(CompletionReason::UserStopped) => "⊘ Stopped by user",
            Some(CompletionReason::CircuitBreaker) => "⚡ Circuit breaker triggered",
            Some(CompletionReason::Error { message }) => {
                &format!("✗ Error: {}", message)
            }
            None => "Complete",
        };

        let reason_color = match &state.completion_reason {
            Some(CompletionReason::TaskComplete) => Color::Green,
            Some(CompletionReason::MaxIterations) => Color::Yellow,
            Some(CompletionReason::UserStopped) => Color::DarkGray,
            Some(CompletionReason::CircuitBreaker) => Color::Red,
            Some(CompletionReason::Error { .. }) => Color::Red,
            None => Color::White,
        };

        let lines = vec![
            Line::from(Span::styled(
                reason_text,
                Style::default().fg(reason_color).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Total iterations: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}", state.completed_iterations()),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Success rate: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.0}%", state.success_rate()),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled("Total time: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    Self::format_duration(state.elapsed_secs() as f64),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

impl Default for RalphPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for RalphPanel {
    fn id(&self) -> PanelId {
        PanelId::RALPH
    }

    fn name(&self) -> &str {
        "Ralph Loop"
    }

    fn handle_input(&mut self, event: &TuiEvent, _state: &mut AppState) -> Result<bool> {
        match event {
            TuiEvent::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                    Ok(true)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ref state) = self.state {
                        let max_idx = state.iteration_history.len().saturating_sub(1);
                        if self.selected_index < max_idx {
                            self.selected_index += 1;
                        }
                    }
                    Ok(true)
                }
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        match &self.state {
            None => self.render_idle(frame, area, focused),
            Some(state) => {
                if state.status == RalphStatus::Complete || state.status == RalphStatus::Error {
                    self.render_complete(frame, area, focused, state);
                } else {
                    // Clone state for the render call
                    let state_clone = state.clone();
                    self.render_active(frame, area, focused, &state_clone);
                }
            }
        }
    }

    fn on_focus(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }
}
