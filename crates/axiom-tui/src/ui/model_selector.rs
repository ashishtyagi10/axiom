//! Model selector modal for choosing LLM models

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Model selector state
pub struct ModelSelector {
    /// Available models
    pub models: Vec<String>,

    /// Currently selected index
    pub selected: usize,

    /// Currently active model (the one being used)
    pub current_model: String,

    /// Loading state
    pub loading: bool,

    /// Error message if any
    pub error: Option<String>,

    /// Cached modal area for hit testing
    pub modal_area: Option<Rect>,

    /// Cached list area for hit testing
    pub list_area: Option<Rect>,
}

impl ModelSelector {
    /// Create a new model selector
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            selected: 0,
            current_model: String::new(),
            loading: true,
            error: None,
            modal_area: None,
            list_area: None,
        }
    }

    /// Set available models
    pub fn set_models(&mut self, models: Vec<String>, current: &str) {
        self.loading = false;
        self.error = None;
        self.current_model = current.to_string();

        // Find the current model in the list
        self.selected = models
            .iter()
            .position(|m| m == current)
            .unwrap_or(0);

        self.models = models;
    }

    /// Set error state
    pub fn set_error(&mut self, error: String) {
        self.loading = false;
        self.error = Some(error);
    }

    /// Move selection up
    pub fn up(&mut self) {
        if !self.models.is_empty() && self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn down(&mut self) {
        if !self.models.is_empty() && self.selected < self.models.len() - 1 {
            self.selected += 1;
        }
    }

    /// Get the selected model
    pub fn selected_model(&self) -> Option<&str> {
        self.models.get(self.selected).map(|s| s.as_str())
    }

    /// Check if a point is inside the modal
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.modal_area
            .map(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
            .unwrap_or(false)
    }

    /// Handle mouse click at position, returns true if an item was clicked
    pub fn handle_click(&mut self, x: u16, y: u16) -> bool {
        if let Some(list_area) = self.list_area {
            if x >= list_area.x && x < list_area.x + list_area.width
                && y >= list_area.y && y < list_area.y + list_area.height
            {
                let clicked_index = (y - list_area.y) as usize;
                if clicked_index < self.models.len() {
                    self.selected = clicked_index;
                    return true;
                }
            }
        }
        false
    }

    /// Handle mouse scroll
    pub fn handle_scroll(&mut self, down: bool) {
        if down {
            self.down();
        } else {
            self.up();
        }
    }

    /// Render the model selector modal
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Calculate centered modal area (40% width, 50% height)
        let modal_width = (area.width as f32 * 0.4).max(30.0) as u16;
        let modal_height = (area.height as f32 * 0.5).max(10.0) as u16;

        let x = (area.width.saturating_sub(modal_width)) / 2;
        let y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(x, y, modal_width, modal_height);

        // Cache for hit testing
        self.modal_area = Some(modal_area);

        // Clear the background
        frame.render_widget(Clear, modal_area);

        // Modal block
        let block = Block::default()
            .title(" Select Model (↑↓ Enter Esc) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Rgb(30, 30, 40)));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        // Content based on state
        if self.loading {
            let loading = Paragraph::new("Loading models...")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(loading, inner);
            return;
        }

        if let Some(ref error) = self.error {
            let error_text = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(error_text, inner);
            return;
        }

        if self.models.is_empty() {
            let empty = Paragraph::new("No models available")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
            return;
        }

        // Split into list and help text
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(2)])
            .split(inner);

        // Cache list area for click detection
        self.list_area = Some(chunks[0]);

        // Create list items
        let items: Vec<ListItem> = self
            .models
            .iter()
            .enumerate()
            .map(|(i, model)| {
                let is_current = model == &self.current_model;
                let prefix = if is_current { "● " } else { "  " };

                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(Span::styled(
                    format!("{}{}", prefix, model),
                    style,
                )))
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected));

        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black));

        frame.render_stateful_widget(list, chunks[0], &mut list_state);

        // Help text
        let help = Paragraph::new("● = current model")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[1]);
    }
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new()
    }
}
