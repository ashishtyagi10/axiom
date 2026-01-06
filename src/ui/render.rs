//! Main render function

use super::layout::get_layout_with_focus;
use crate::panels::{Panel, PanelRegistry};
use crate::state::{AppState, PanelId};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Render the entire application
pub fn render(frame: &mut Frame, state: &AppState, panels: &mut PanelRegistry) {
    let area = frame.area();
    let focused = state.focus.current();

    // Get layout with focus-aware sizing (chat expands when focused)
    let layout = get_layout_with_focus(area, Some(focused));

    panels.file_tree.render(frame, layout.file_tree, focused == PanelId::FILE_TREE);
    panels.editor.render(frame, layout.editor, focused == PanelId::EDITOR);
    panels.terminal.render(frame, layout.terminal, focused == PanelId::TERMINAL);
    panels.chat.render(frame, layout.chat, focused == PanelId::CHAT);

    // Render status bar and get model badge area
    let model_badge_area = render_status_bar(frame, layout.status, state, panels);
    panels.model_badge_area = Some(model_badge_area);

    // Render model selector modal if open
    if state.input_mode.is_modal_open("model_selector") {
        panels.model_selector.render(frame, area);
    }
}

/// Render the status bar, returns the model badge area for click detection
fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState, panels: &PanelRegistry) -> Rect {
    let focused = state.focus.current();
    let mode = format!(" {} ", state.input_mode_name());
    let focus = format!(" {} ", panel_name(focused));
    let model = format!(" 🤖 {} ", panels.chat.current_model());

    let status_text = if let Some(msg) = &state.status_message {
        msg.text.clone()
    } else {
        String::new()
    };

    // Calculate model badge position for click detection
    // mode + " " + focus + " " = prefix before model badge
    let prefix_len = mode.len() + 1 + focus.len() + 1;
    let model_badge_area = Rect::new(
        area.x + prefix_len as u16,
        area.y,
        model.len() as u16,
        1,
    );

    // Build status line
    let spans = vec![
        Span::styled(mode, Style::default().bg(Color::Blue).fg(Color::White)),
        Span::raw(" "),
        Span::styled(focus, Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" "),
        Span::styled(model, Style::default().bg(Color::Magenta).fg(Color::White)),
        Span::raw(" "),
        Span::styled(status_text, Style::default().fg(Color::Gray)),
        Span::raw("  "),
        Span::styled(
            " Ctrl+M: Model  q: Quit  Tab: Switch ",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let status = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(status, area);

    model_badge_area
}

/// Get panel display name
fn panel_name(id: PanelId) -> &'static str {
    match id {
        PanelId::FILE_TREE => "Files",
        PanelId::EDITOR => "Editor",
        PanelId::TERMINAL => "Terminal",
        PanelId::CHAT => "Chat",
        _ => "Unknown",
    }
}

// Add helper to AppState for display
impl AppState {
    pub fn input_mode_name(&self) -> &'static str {
        match &self.input_mode {
            crate::state::InputMode::Normal => "NORMAL",
            crate::state::InputMode::Insert => "INSERT",
            crate::state::InputMode::Command { .. } => "COMMAND",
            crate::state::InputMode::Search { .. } => "SEARCH",
            crate::state::InputMode::Modal { .. } => "MODAL",
        }
    }
}
