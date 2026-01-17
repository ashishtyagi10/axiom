//! Main render function

use super::layout::get_layout_with_focus;
use super::theme::theme;
use crate::panels::{Panel, PanelRegistry};
use crate::state::{AppState, PanelId};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Render the entire application
pub fn render(frame: &mut Frame, state: &AppState, panels: &mut PanelRegistry) {
    let area = frame.area();

    // Workspace selection phase: full-screen selector only
    if state.active_workspace_id.is_none() {
        panels.workspace_selector.render(frame, area);
        return; // Don't render main UI until workspace is selected
    }

    let focused = state.focus.current();

    // Get layout with focus-aware sizing
    let layout = get_layout_with_focus(area, Some(focused));

    // Render all panels
    panels
        .file_tree
        .render(frame, layout.file_tree, focused == PanelId::FILE_TREE);
    panels
        .output
        .render(frame, layout.output, focused == PanelId::OUTPUT);
    panels
        .input
        .render(frame, layout.input, focused == PanelId::INPUT);
    panels
        .agents
        .render(frame, layout.agents, focused == PanelId::AGENTS);

    // Render status bar and get model badge area
    let model_badge_area = render_status_bar(frame, layout.status, state, panels);
    panels.model_badge_area = Some(model_badge_area);

    // Render model selector modal if open
    if state.input_mode.is_modal_open("model_selector") {
        panels.model_selector.render(frame, area);
    }

    // Render settings modal if open
    if state.input_mode.is_modal_open("settings") {
        panels.settings.render(frame, area);
    }

    // Render workspace selector modal if open
    if state.input_mode.is_modal_open("workspace_selector") {
        panels.workspace_selector.render(frame, area);
    }
}

/// Render the status bar, returns the model badge area for click detection
fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    panels: &PanelRegistry,
) -> Rect {
    let focused = state.focus.current();
    let mode = format!(" {} ", state.input_mode_name());
    let focus = format!(" {} ", panel_name(focused));

    // Workspace name badge
    let workspace_name = state.workspace_name();
    let workspace_badge = format!(" 📁 {} ", workspace_name);

    // Get running agents count
    let registry = panels.agent_registry.read();
    let running = registry.running_count();
    let total = registry.len();
    drop(registry);

    let agents_badge = if running > 0 {
        format!(" ⚡ {}/{} ", running, total)
    } else if total > 0 {
        format!(" ✓ {} ", total)
    } else {
        " 🤖 ".to_string()
    };

    let status_text = if let Some(msg) = &state.status_message {
        msg.text.clone()
    } else {
        String::new()
    };

    // Calculate model badge position for click detection
    let prefix_len = mode.len() + 1 + focus.len() + 1 + workspace_badge.len() + 1;
    let model_badge_area = Rect::new(
        area.x + prefix_len as u16,
        area.y,
        agents_badge.len() as u16,
        1,
    );

    // Build status line with theme colors
    let t = theme();
    let spans = vec![
        Span::styled(mode, Style::default().bg(t.statusbar_mode_bg).fg(t.statusbar_mode_fg)),
        Span::raw(" "),
        Span::styled(
            focus,
            Style::default().bg(t.statusbar_focus_bg).fg(t.statusbar_focus_fg),
        ),
        Span::raw(" "),
        Span::styled(
            workspace_badge,
            Style::default().bg(t.statusbar_workspace_bg).fg(t.statusbar_workspace_fg),
        ),
        Span::raw(" "),
        Span::styled(
            agents_badge,
            Style::default().bg(t.statusbar_agents_bg).fg(t.statusbar_agents_fg),
        ),
        Span::raw(" "),
        Span::styled(status_text, Style::default().fg(t.text_secondary)),
        Span::raw("  "),
        Span::styled(
            " Ctrl+T: Theme  Ctrl+W: Workspaces  q: Quit ",
            Style::default().fg(t.text_muted),
        ),
    ];

    let status = Paragraph::new(Line::from(spans)).style(Style::default().bg(t.statusbar_bg));

    frame.render_widget(status, area);

    model_badge_area
}

/// Get panel display name
fn panel_name(id: PanelId) -> &'static str {
    match id {
        PanelId::FILE_TREE => "Files",
        PanelId::OUTPUT => "Output",
        PanelId::INPUT => "Input",
        PanelId::AGENTS => "Agents",
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
