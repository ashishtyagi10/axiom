//! Layout management for panel arrangement

use crate::state::PanelId;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Panel layout areas
///
/// New 3-column agentic layout:
/// ```text
/// ┌──────────────┬─────────────────────────────┬──────────────┐
/// │              │        Output Area          │              │
/// │  File Tree   │   (file content OR agent    │    Agents    │
/// │    (20%)     │         output)             │    (20%)     │
/// │              ├─────────────────────────────┤              │
/// │              │        Input Area           │              │
/// │              │   (unified chat input)      │              │
/// └──────────────┴─────────────────────────────┴──────────────┘
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
        _ => 3,                     // Minimal: just borders + 1 line
    };

    // Vertical split in middle: output (flexible) | input (fixed height)
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),              // Output gets remaining space
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_standard_split() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = get_layout(area);

        // All areas should have non-zero dimensions
        assert!(layout.file_tree.width > 0);
        assert!(layout.output.width > 0);
        assert!(layout.input.width > 0);
        assert!(layout.agents.width > 0);
        assert!(layout.status.width > 0);
    }

    #[test]
    fn test_layout_percentage_ratios() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = get_layout(area);

        // File tree should be approximately 20%
        let file_tree_pct = (layout.file_tree.width as f64 / area.width as f64) * 100.0;
        assert!(file_tree_pct >= 15.0 && file_tree_pct <= 25.0);

        // Agents should be approximately 20%
        let agents_pct = (layout.agents.width as f64 / area.width as f64) * 100.0;
        assert!(agents_pct >= 15.0 && agents_pct <= 25.0);
    }

    #[test]
    fn test_layout_focused_input_height() {
        let area = Rect::new(0, 0, 100, 50);

        // Layout without focus
        let layout_normal = get_layout_with_focus(area, None);

        // Layout with input focused
        let layout_focused = get_layout_with_focus(area, Some(PanelId::INPUT));

        // Input should be taller when focused
        assert!(layout_focused.input.height > layout_normal.input.height);
    }

    #[test]
    fn test_layout_status_bar_at_bottom() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = get_layout(area);

        // Status bar should be at the bottom
        assert_eq!(layout.status.y + layout.status.height, area.y + area.height);
        assert_eq!(layout.status.height, 1);
    }

    #[test]
    fn test_layout_small_terminal() {
        // Test with very small terminal size
        let area = Rect::new(0, 0, 40, 15);
        let layout = get_layout(area);

        // Should still produce valid layout
        assert!(layout.file_tree.width > 0);
        assert!(layout.output.height > 0);
        assert!(layout.input.height > 0);
    }

    #[test]
    fn test_layout_areas_no_overlap() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = get_layout(area);

        // File tree and agents should not overlap
        assert!(
            layout.file_tree.x + layout.file_tree.width <= layout.output.x
            || layout.output.x + layout.output.width <= layout.file_tree.x
        );

        // Output and input should not overlap vertically in middle section
        assert!(
            layout.output.y + layout.output.height <= layout.input.y
            || layout.input.y + layout.input.height <= layout.output.y
        );
    }

    #[test]
    fn test_panel_at_file_tree() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = get_layout(area);

        // Click inside file tree area
        let x = layout.file_tree.x + 5;
        let y = layout.file_tree.y + 5;
        assert_eq!(layout.panel_at(x, y), Some(PanelId::FILE_TREE));
    }

    #[test]
    fn test_panel_at_output() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = get_layout(area);

        // Click inside output area
        let x = layout.output.x + 5;
        let y = layout.output.y + 5;
        assert_eq!(layout.panel_at(x, y), Some(PanelId::OUTPUT));
    }

    #[test]
    fn test_panel_at_input() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = get_layout(area);

        // Click inside input area
        let x = layout.input.x + 5;
        let y = layout.input.y + 1;
        assert_eq!(layout.panel_at(x, y), Some(PanelId::INPUT));
    }

    #[test]
    fn test_panel_at_agents() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = get_layout(area);

        // Click inside agents area
        let x = layout.agents.x + 5;
        let y = layout.agents.y + 5;
        assert_eq!(layout.panel_at(x, y), Some(PanelId::AGENTS));
    }

    #[test]
    fn test_panel_at_outside() {
        let area = Rect::new(10, 10, 80, 40);
        let layout = get_layout(area);

        // Click outside all panels (before the layout starts)
        assert_eq!(layout.panel_at(0, 0), None);
    }

    #[test]
    fn test_layout_minimum_sizes() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = get_layout(area);

        // Content area should have minimum height
        assert!(layout.output.height >= 5);

        // Input should have minimum height
        assert!(layout.input.height >= 3);
    }
}
