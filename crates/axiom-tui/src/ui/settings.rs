//! Settings modal for configuring API keys and providers

use axiom_core::{AxiomConfig, LlmConfig, ProviderConfig};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Available providers in order
const PROVIDERS: [&str; 4] = ["ollama", "claude", "gemini", "openai"];

/// Settings row enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsRow {
    DefaultProvider,
    ClaudeApiKey,
    GeminiApiKey,
    OpenAiApiKey,
    OllamaUrl,
    CancelButton,
    SaveButton,
}

impl SettingsRow {
    fn all() -> &'static [SettingsRow] {
        &[
            SettingsRow::DefaultProvider,
            SettingsRow::ClaudeApiKey,
            SettingsRow::GeminiApiKey,
            SettingsRow::OpenAiApiKey,
            SettingsRow::OllamaUrl,
            SettingsRow::CancelButton,
            SettingsRow::SaveButton,
        ]
    }

    fn from_index(idx: usize) -> Option<SettingsRow> {
        Self::all().get(idx).copied()
    }

    fn is_text_field(&self) -> bool {
        matches!(
            self,
            SettingsRow::ClaudeApiKey
                | SettingsRow::GeminiApiKey
                | SettingsRow::OpenAiApiKey
                | SettingsRow::OllamaUrl
        )
    }

    fn is_button(&self) -> bool {
        matches!(self, SettingsRow::CancelButton | SettingsRow::SaveButton)
    }

    fn label(&self) -> &'static str {
        match self {
            SettingsRow::DefaultProvider => "Default Provider",
            SettingsRow::ClaudeApiKey => "Anthropic API Key",
            SettingsRow::GeminiApiKey => "Google API Key",
            SettingsRow::OpenAiApiKey => "OpenAI API Key",
            SettingsRow::OllamaUrl => "Ollama Base URL",
            SettingsRow::CancelButton => "Cancel",
            SettingsRow::SaveButton => "Save",
        }
    }
}

/// Action returned from settings interactions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsAction {
    Save,
    Cancel,
    StartEdit,
    None,
}

/// Settings modal state
pub struct SettingsModal {
    // Settings values
    pub default_provider: String,
    pub api_keys: HashMap<String, String>,
    pub ollama_url: String,

    // Original values (for change detection)
    original_provider: String,
    original_keys: HashMap<String, String>,
    original_url: String,

    // UI state
    pub selected_row: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub cursor_pos: usize,

    // Hit testing
    pub modal_area: Option<Rect>,
    pub row_areas: Vec<Rect>,
}

impl SettingsModal {
    /// Create from current configuration
    pub fn new(config: &AxiomConfig) -> Self {
        let mut api_keys = HashMap::new();

        // Extract API keys from providers
        for (name, provider) in &config.llm.providers {
            if let Some(ref key) = provider.api_key {
                api_keys.insert(name.clone(), key.clone());
            }
        }

        let ollama_url = config
            .llm
            .providers
            .get("ollama")
            .and_then(|p| p.base_url.clone())
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        Self {
            default_provider: config.llm.default_provider.clone(),
            original_provider: config.llm.default_provider.clone(),
            api_keys: api_keys.clone(),
            original_keys: api_keys,
            ollama_url: ollama_url.clone(),
            original_url: ollama_url,
            selected_row: 0,
            editing: false,
            edit_buffer: String::new(),
            cursor_pos: 0,
            modal_area: None,
            row_areas: Vec::new(),
        }
    }

    /// Check if there are unsaved changes
    pub fn has_changes(&self) -> bool {
        self.default_provider != self.original_provider
            || self.api_keys != self.original_keys
            || self.ollama_url != self.original_url
    }

    /// Navigate up
    pub fn up(&mut self) {
        if !self.editing && self.selected_row > 0 {
            self.selected_row -= 1;
        }
    }

    /// Navigate down
    pub fn down(&mut self) {
        if !self.editing && self.selected_row < SettingsRow::all().len() - 1 {
            self.selected_row += 1;
        }
    }

    /// Cycle dropdown left
    pub fn left(&mut self) {
        if !self.editing {
            if let Some(SettingsRow::DefaultProvider) = SettingsRow::from_index(self.selected_row) {
                let current_idx = PROVIDERS
                    .iter()
                    .position(|p| *p == self.default_provider)
                    .unwrap_or(0);
                let new_idx = if current_idx == 0 {
                    PROVIDERS.len() - 1
                } else {
                    current_idx - 1
                };
                self.default_provider = PROVIDERS[new_idx].to_string();
            }
        }
    }

    /// Cycle dropdown right
    pub fn right(&mut self) {
        if !self.editing {
            if let Some(SettingsRow::DefaultProvider) = SettingsRow::from_index(self.selected_row) {
                let current_idx = PROVIDERS
                    .iter()
                    .position(|p| *p == self.default_provider)
                    .unwrap_or(0);
                let new_idx = (current_idx + 1) % PROVIDERS.len();
                self.default_provider = PROVIDERS[new_idx].to_string();
            }
        }
    }

    /// Handle Enter key
    pub fn enter(&mut self) -> SettingsAction {
        let row = SettingsRow::from_index(self.selected_row);

        if self.editing {
            // Finish editing
            self.finish_edit();
            return SettingsAction::None;
        }

        match row {
            Some(r) if r.is_text_field() => {
                self.start_edit();
                SettingsAction::StartEdit
            }
            Some(SettingsRow::SaveButton) => SettingsAction::Save,
            Some(SettingsRow::CancelButton) => SettingsAction::Cancel,
            _ => SettingsAction::None,
        }
    }

    /// Start editing current text field
    fn start_edit(&mut self) {
        let row = SettingsRow::from_index(self.selected_row);
        self.edit_buffer = match row {
            Some(SettingsRow::ClaudeApiKey) => {
                self.api_keys.get("claude").cloned().unwrap_or_default()
            }
            Some(SettingsRow::GeminiApiKey) => {
                self.api_keys.get("gemini").cloned().unwrap_or_default()
            }
            Some(SettingsRow::OpenAiApiKey) => {
                self.api_keys.get("openai").cloned().unwrap_or_default()
            }
            Some(SettingsRow::OllamaUrl) => self.ollama_url.clone(),
            _ => String::new(),
        };
        self.cursor_pos = self.edit_buffer.len();
        self.editing = true;
    }

    /// Finish editing and apply changes
    fn finish_edit(&mut self) {
        let row = SettingsRow::from_index(self.selected_row);
        match row {
            Some(SettingsRow::ClaudeApiKey) => {
                if self.edit_buffer.is_empty() {
                    self.api_keys.remove("claude");
                } else {
                    self.api_keys
                        .insert("claude".to_string(), self.edit_buffer.clone());
                }
            }
            Some(SettingsRow::GeminiApiKey) => {
                if self.edit_buffer.is_empty() {
                    self.api_keys.remove("gemini");
                } else {
                    self.api_keys
                        .insert("gemini".to_string(), self.edit_buffer.clone());
                }
            }
            Some(SettingsRow::OpenAiApiKey) => {
                if self.edit_buffer.is_empty() {
                    self.api_keys.remove("openai");
                } else {
                    self.api_keys
                        .insert("openai".to_string(), self.edit_buffer.clone());
                }
            }
            Some(SettingsRow::OllamaUrl) => {
                self.ollama_url = self.edit_buffer.clone();
            }
            _ => {}
        }
        self.editing = false;
        self.edit_buffer.clear();
    }

    /// Cancel editing
    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    /// Insert character into edit buffer
    pub fn insert_char(&mut self, c: char) {
        self.edit_buffer.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let mut new_cursor = self.cursor_pos - 1;
            while new_cursor > 0 && !self.edit_buffer.is_char_boundary(new_cursor) {
                new_cursor -= 1;
            }
            self.edit_buffer.remove(new_cursor);
            self.cursor_pos = new_cursor;
        }
    }

    /// Delete character after cursor
    pub fn delete(&mut self) {
        if self.cursor_pos < self.edit_buffer.len() {
            self.edit_buffer.remove(self.cursor_pos);
        }
    }

    /// Convert settings back to config
    pub fn to_config(&self) -> AxiomConfig {
        let mut providers = HashMap::new();

        // Ollama
        providers.insert(
            "ollama".to_string(),
            ProviderConfig {
                enabled: true,
                api_key: None,
                base_url: Some(self.ollama_url.clone()),
                default_model: Some("gemma3:4b".to_string()),
                models: Vec::new(),
            },
        );

        // Claude
        let claude_key = self.api_keys.get("claude").cloned();
        providers.insert(
            "claude".to_string(),
            ProviderConfig {
                enabled: claude_key.is_some(),
                api_key: claude_key,
                base_url: Some("https://api.anthropic.com".to_string()),
                default_model: Some("claude-sonnet-4-20250514".to_string()),
                models: vec![
                    "claude-sonnet-4-20250514".to_string(),
                    "claude-opus-4-20250514".to_string(),
                ],
            },
        );

        // Gemini
        let gemini_key = self.api_keys.get("gemini").cloned();
        providers.insert(
            "gemini".to_string(),
            ProviderConfig {
                enabled: gemini_key.is_some(),
                api_key: gemini_key,
                base_url: Some("https://generativelanguage.googleapis.com".to_string()),
                default_model: Some("gemini-2.0-flash".to_string()),
                models: vec![
                    "gemini-2.0-flash".to_string(),
                    "gemini-1.5-pro".to_string(),
                    "gemini-1.5-flash".to_string(),
                ],
            },
        );

        // OpenAI
        let openai_key = self.api_keys.get("openai").cloned();
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                enabled: openai_key.is_some(),
                api_key: openai_key,
                base_url: Some("https://api.openai.com".to_string()),
                default_model: Some("gpt-4o".to_string()),
                models: vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "gpt-4-turbo".to_string(),
                ],
            },
        );

        AxiomConfig {
            llm: LlmConfig {
                default_provider: self.default_provider.clone(),
                timeout: 120,
                max_retries: 3,
                providers,
            },
            cli_agents: Default::default(),
        }
    }

    /// Check if point is inside modal
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.modal_area
            .map(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
            .unwrap_or(false)
    }

    /// Handle mouse click
    pub fn handle_click(&mut self, x: u16, y: u16) -> SettingsAction {
        // Check if click is on a row
        for (idx, area) in self.row_areas.iter().enumerate() {
            if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
                self.selected_row = idx;

                // If clicking on a button, activate it
                if let Some(row) = SettingsRow::from_index(idx) {
                    if row == SettingsRow::SaveButton {
                        return SettingsAction::Save;
                    } else if row == SettingsRow::CancelButton {
                        return SettingsAction::Cancel;
                    }
                }
                return SettingsAction::None;
            }
        }
        SettingsAction::None
    }

    /// Mask API key for display
    fn mask_key(key: &str) -> String {
        if key.is_empty() {
            return String::new();
        }
        if key.len() <= 8 {
            return "*".repeat(key.len());
        }
        // Show first 4 and last 4 chars
        let prefix: String = key.chars().take(4).collect();
        let suffix: String = key.chars().skip(key.len() - 4).collect();
        let middle = "*".repeat(key.len().saturating_sub(8).min(16));
        format!("{}{}{}", prefix, middle, suffix)
    }

    /// Render the settings modal
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Calculate modal size (60% width, 50% height)
        let modal_width = (area.width as f32 * 0.6).max(50.0).min(70.0) as u16;
        let modal_height = (area.height as f32 * 0.5).max(15.0).min(20.0) as u16;

        let x = (area.width.saturating_sub(modal_width)) / 2;
        let y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);

        // Clear background
        frame.render_widget(Clear, modal_area);

        // Modal block
        let title = if self.has_changes() {
            " Settings * (unsaved) "
        } else {
            " Settings "
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Rgb(30, 30, 40)));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        // Layout rows
        self.row_areas.clear();

        let row_height = 2u16;
        let mut current_y = inner.y;

        for (idx, row) in SettingsRow::all().iter().enumerate() {
            let is_selected = idx == self.selected_row;
            let row_area = Rect::new(inner.x, current_y, inner.width, row_height);

            if current_y + row_height > inner.y + inner.height {
                break;
            }

            self.row_areas.push(row_area);
            self.render_row(frame, row_area, *row, is_selected);

            current_y += row_height;
        }

        // Show cursor if editing
        if self.editing {
            let row_area = self.row_areas.get(self.selected_row);
            if let Some(area) = row_area {
                // Calculate cursor position (after label + value prefix)
                let label_width = 18;
                let cursor_x = area.x + label_width + self.cursor_pos as u16 + 1;
                let cursor_y = area.y;
                if cursor_x < area.x + area.width {
                    frame.set_cursor_position((cursor_x, cursor_y));
                }
            }
        }
    }

    /// Render a single settings row
    fn render_row(&self, frame: &mut Frame, area: Rect, row: SettingsRow, selected: bool) {
        let label_style = if selected && !row.is_button() {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let is_editing_this =
            self.editing && SettingsRow::from_index(self.selected_row) == Some(row);

        match row {
            SettingsRow::DefaultProvider => {
                let value_style = if selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Gray)
                };

                let provider_display = format!("< {} >", self.default_provider);
                let line = Line::from(vec![
                    Span::styled(format!("{:>16}: ", row.label()), label_style),
                    Span::styled(provider_display, value_style),
                ]);

                frame.render_widget(Paragraph::new(line), area);
            }

            SettingsRow::ClaudeApiKey
            | SettingsRow::GeminiApiKey
            | SettingsRow::OpenAiApiKey => {
                let provider = match row {
                    SettingsRow::ClaudeApiKey => "claude",
                    SettingsRow::GeminiApiKey => "gemini",
                    SettingsRow::OpenAiApiKey => "openai",
                    _ => "",
                };

                let value = if is_editing_this {
                    self.edit_buffer.clone()
                } else {
                    self.api_keys.get(provider).cloned().unwrap_or_default()
                };

                let display_value = if is_editing_this {
                    value.clone()
                } else {
                    Self::mask_key(&value)
                };

                let status = if value.is_empty() {
                    Span::styled(" o", Style::default().fg(Color::DarkGray))
                } else {
                    Span::styled(" +", Style::default().fg(Color::Green))
                };

                let value_style = if is_editing_this {
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Rgb(50, 50, 60))
                } else if selected {
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let line = Line::from(vec![
                    Span::styled(format!("{:>16}: ", row.label()), label_style),
                    Span::styled(
                        format!(
                            "[{:<24}]",
                            display_value.chars().take(24).collect::<String>()
                        ),
                        value_style,
                    ),
                    status,
                ]);

                frame.render_widget(Paragraph::new(line), area);
            }

            SettingsRow::OllamaUrl => {
                let value = if is_editing_this {
                    self.edit_buffer.clone()
                } else {
                    self.ollama_url.clone()
                };

                let value_style = if is_editing_this {
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Rgb(50, 50, 60))
                } else if selected {
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let line = Line::from(vec![
                    Span::styled(format!("{:>16}: ", row.label()), label_style),
                    Span::styled(
                        format!("[{:<24}]", value.chars().take(24).collect::<String>()),
                        value_style,
                    ),
                ]);

                frame.render_widget(Paragraph::new(line), area);
            }

            SettingsRow::CancelButton | SettingsRow::SaveButton => {
                let button_style = if selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };

                let label = format!(" {} ", row.label());
                let padding = (area.width as usize).saturating_sub(label.len()) / 2;
                let padded = format!("{:>width$}", label, width = padding + label.len());

                frame.render_widget(
                    Paragraph::new(padded)
                        .style(button_style)
                        .alignment(Alignment::Center),
                    area,
                );
            }
        }
    }
}

impl Default for SettingsModal {
    fn default() -> Self {
        Self::new(&AxiomConfig::default())
    }
}
