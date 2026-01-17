//! Layout management for panel arrangement

use crate::state::PanelId;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Panel layout areas
///
/// New 3-column agentic layout:
/// ```text
/// +---------------+-----------------------------+---------------+
/// |               |        Output Area          |               |
/// |  File Tree    |   (file content OR agent    |    Agents     |
/// |    (20%)      |         output)             |    (20%)      |
/// |               +-----------------------------+               |
/// |               |        Input Area           |               |
/// |               |   (unified chat input)      |               |
/// +---------------+-----------------------------+---------------+
/// ```
pub struct AppLayout {
    /// Left panel: file tree navigation (20%)
    pub file_tree: Rect,

    /// Center-top: output area for file/agent content (60% of center)
    pub output: Rect,

    /// Center-bottom: input area for commands (40% of center)
    pub input: Rect,

    /// Right panel: agents list (20%)
    pub agents: Rect,

    /// Bottom: status bar
    pub status: Rect,
}

/// Calculate layout areas for all panels
pub fn get_layout(area: Rect) -> AppLayout {
    get_layout_with_focus(area, None)
}

/// Calculate layout with focus-aware sizing
///
/// Layout adjusts based on which panel is focused:
/// - Agents focused: right panel expands (similar to old chat expansion)
/// - Input focused: input area expands vertically
/// - Default: 20% | 60% | 20% horizontal split
pub fn get_layout_with_focus(area: Rect, focused: Option<PanelId>) -> AppLayout {
    // Main vertical split: content + status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Content area
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    let content_area = main_chunks[0];
    let status = main_chunks[1];

    // Fixed horizontal layout: 20% | 60% | 20%
    let (file_tree_pct, middle_pct, agents_pct) = (20, 60, 20);

    // Horizontal split: file tree | middle | agents
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(file_tree_pct),
            Constraint::Percentage(middle_pct),
            Constraint::Percentage(agents_pct),
        ])
        .split(content_area);

    let file_tree = h_chunks[0];
    let middle = h_chunks[1];
    let agents = h_chunks[2];

    // Input panel has minimal height (3 lines for border + 1 line content)
    // It can expand when focused or when content requires more space
    let input_height = match focused {
        Some(PanelId::INPUT) => 6, // Slightly larger when focused
        _ => 3,                    // Minimal: just borders + 1 line
    };

    // Vertical split in middle: output (flexible) | input (fixed height)
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),               // Output gets remaining space
            Constraint::Length(input_height), // Input has fixed small height
        ])
        .split(middle);

    let output = v_chunks[0];
    let input = v_chunks[1];

    AppLayout {
        file_tree,
        output,
        input,
        agents,
        status,
    }
}

impl AppLayout {
    /// Determine which panel contains the given coordinates
    pub fn panel_at(&self, x: u16, y: u16) -> Option<PanelId> {
        let pos = (x, y).into();
        if self.file_tree.contains(pos) {
            Some(PanelId::FILE_TREE)
        } else if self.output.contains(pos) {
            Some(PanelId::OUTPUT)
        } else if self.input.contains(pos) {
            Some(PanelId::INPUT)
        } else if self.agents.contains(pos) {
            Some(PanelId::AGENTS)
        } else {
            None
        }
    }
}
