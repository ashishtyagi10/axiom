//! Layout management for panel arrangement

use crate::state::PanelId;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Panel layout areas
pub struct AppLayout {
    pub file_tree: Rect,
    pub editor: Rect,
    pub terminal: Rect,
    pub chat: Rect,
    pub status: Rect,
}

/// Calculate layout areas for all panels
///
/// Layout (normal):
/// ```text
/// ┌─────────┬─────────────────────────┬─────────┐
/// │         │         Editor          │         │
/// │  Files  ├─────────────────────────┤  Chat   │
/// │  (20%)  │        Terminal         │  (20%)  │
/// │         │                         │         │
/// ├─────────┴─────────────────────────┴─────────┤
/// │                    Status                    │
/// └─────────────────────────────────────────────┘
/// ```
///
/// Layout (chat focused - expanded):
/// ```text
/// ┌─────┬───────────────────┬───────────────────┐
/// │     │      Editor       │                   │
/// │Files├───────────────────┤       Chat        │
/// │(15%)│     Terminal      │       (40%)       │
/// │     │                   │                   │
/// ├─────┴───────────────────┴───────────────────┤
/// │                    Status                    │
/// └─────────────────────────────────────────────┘
/// ```
pub fn get_layout(area: Rect) -> AppLayout {
    get_layout_with_focus(area, None)
}

/// Calculate layout with focus-aware sizing
pub fn get_layout_with_focus(area: Rect, focused: Option<PanelId>) -> AppLayout {
    // Main vertical split: content + status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),    // Content area
            Constraint::Length(1),  // Status bar
        ])
        .split(area);

    let content_area = main_chunks[0];
    let status = main_chunks[1];

    // Determine layout percentages based on focus
    let (file_tree_pct, middle_pct, chat_pct) = match focused {
        Some(PanelId::CHAT) => (15, 45, 40),      // Chat expanded
        Some(PanelId::FILE_TREE) => (30, 50, 20), // File tree expanded
        _ => (20, 60, 20),                         // Default
    };

    // Horizontal split: file tree | middle | chat
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(file_tree_pct),
            Constraint::Percentage(middle_pct),
            Constraint::Percentage(chat_pct),
        ])
        .split(content_area);

    let file_tree = h_chunks[0];
    let middle = h_chunks[1];
    let chat = h_chunks[2];

    // Vertical split in middle: editor | terminal
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60), // Editor
            Constraint::Percentage(40), // Terminal
        ])
        .split(middle);

    let editor = v_chunks[0];
    let terminal = v_chunks[1];

    AppLayout {
        file_tree,
        editor,
        terminal,
        chat,
        status,
    }
}
