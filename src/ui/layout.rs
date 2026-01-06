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
///
/// Layout (terminal focused - expanded):
/// ```text
/// ┌─────────┬─────────────────────────┬─────────┐
/// │         │      Editor (30%)       │         │
/// │  Files  ├─────────────────────────┤  Chat   │
/// │  (20%)  │     Terminal (70%)      │  (20%)  │
/// │         │                         │         │
/// ├─────────┴─────────────────────────┴─────────┤
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
    // File tree stays fixed width, only chat expands horizontally
    let (file_tree_pct, middle_pct, chat_pct) = match focused {
        Some(PanelId::CHAT) => (15, 45, 40),      // Chat expanded
        _ => (20, 60, 20),                         // Default (file tree stays 20%)
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

    // Determine editor/terminal split based on focus
    // Editor is the main area - gets more space when focused
    let (editor_pct, terminal_pct) = match focused {
        Some(PanelId::TERMINAL) => (50, 50), // Terminal gets 50% when focused
        Some(PanelId::EDITOR) => (70, 30),   // Editor gets 70% when focused (main area)
        _ => (60, 40),                        // Default
    };

    // Vertical split in middle: editor | terminal
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(editor_pct),
            Constraint::Percentage(terminal_pct),
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

impl AppLayout {
    /// Determine which panel contains the given coordinates
    pub fn panel_at(&self, x: u16, y: u16) -> Option<PanelId> {
        if self.file_tree.contains((x, y).into()) {
            Some(PanelId::FILE_TREE)
        } else if self.editor.contains((x, y).into()) {
            Some(PanelId::EDITOR)
        } else if self.terminal.contains((x, y).into()) {
            Some(PanelId::TERMINAL)
        } else if self.chat.contains((x, y).into()) {
            Some(PanelId::CHAT)
        } else {
            None
        }
    }
}
