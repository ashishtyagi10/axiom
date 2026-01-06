//! Panel system with trait-based composition
//!
//! Each panel implements the Panel trait for consistent behavior.

mod editor;
mod file_tree;
mod terminal;
mod chat;

pub use editor::EditorPanel;
pub use file_tree::FileTreePanel;
pub use terminal::TerminalPanel;
pub use chat::ChatPanel;

use crate::core::Result;
use crate::events::Event;
use crate::state::{AppState, PanelId};
use crate::ui::ModelSelector;
use ratatui::layout::Rect;
use ratatui::Frame;

/// Panel trait - defines the interface for all panels
///
/// Each panel manages its own state and rendering.
/// The main app just calls these methods without knowing panel internals.
pub trait Panel: Send {
    /// Get the panel's unique identifier
    fn id(&self) -> PanelId;

    /// Get the panel's display name
    fn name(&self) -> &str;

    /// Handle an input event
    ///
    /// Returns Ok(true) if the event was consumed, Ok(false) to propagate.
    fn handle_input(&mut self, event: &Event, state: &mut AppState) -> Result<bool>;

    /// Render the panel to the frame
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool);

    /// Called when this panel gains focus
    fn on_focus(&mut self) {}

    /// Called when this panel loses focus
    fn on_blur(&mut self) {}

    /// Called when the panel area is resized
    ///
    /// For terminal panel, this triggers PTY resize.
    fn on_resize(&mut self, _cols: u16, _rows: u16) {}

    /// Check if the panel can be closed
    fn can_close(&self) -> bool {
        true
    }

    /// Cleanup when panel is being destroyed
    fn destroy(&mut self) {}
}

/// Container for all panels
pub struct PanelRegistry {
    pub file_tree: FileTreePanel,
    pub editor: EditorPanel,
    pub terminal: TerminalPanel,
    pub chat: ChatPanel,
    pub model_selector: ModelSelector,
    /// Cached model badge area for click detection
    pub model_badge_area: Option<Rect>,
}

impl PanelRegistry {
    /// Create panel registry with all default panels
    pub fn new(event_tx: crossbeam_channel::Sender<Event>, cwd: &std::path::Path) -> Result<Self> {
        Ok(Self {
            file_tree: FileTreePanel::new(cwd),
            editor: EditorPanel::new(),
            terminal: TerminalPanel::new(event_tx.clone(), cwd)?,
            chat: ChatPanel::new(event_tx),
            model_selector: ModelSelector::new(),
            model_badge_area: None,
        })
    }

    /// Check if click is on model badge
    pub fn is_model_badge_click(&self, x: u16, y: u16) -> bool {
        self.model_badge_area
            .map(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
            .unwrap_or(false)
    }

    /// Open the model selector modal
    pub fn open_model_selector(&mut self) {
        let current = self.chat.current_model();
        match self.chat.list_models() {
            Ok(models) => {
                self.model_selector.set_models(models, &current);
            }
            Err(e) => {
                self.model_selector.set_error(e);
            }
        }
    }

    /// Apply the selected model
    pub fn apply_selected_model(&mut self) -> Option<String> {
        if let Some(model) = self.model_selector.selected_model() {
            let model = model.to_string();
            self.chat.set_model(&model);
            Some(model)
        } else {
            None
        }
    }

    /// Get panel by ID
    pub fn get(&self, id: PanelId) -> &dyn Panel {
        match id {
            PanelId::FILE_TREE => &self.file_tree,
            PanelId::EDITOR => &self.editor,
            PanelId::TERMINAL => &self.terminal,
            PanelId::CHAT => &self.chat,
            _ => &self.editor, // fallback
        }
    }

    /// Get mutable panel by ID
    pub fn get_mut(&mut self, id: PanelId) -> &mut dyn Panel {
        match id {
            PanelId::FILE_TREE => &mut self.file_tree,
            PanelId::EDITOR => &mut self.editor,
            PanelId::TERMINAL => &mut self.terminal,
            PanelId::CHAT => &mut self.chat,
            _ => &mut self.editor, // fallback
        }
    }

    /// Notify all panels of resize
    pub fn notify_resize(&mut self, terminal_area: Rect) {
        // Terminal panel handles border calculations internally
        self.terminal.on_resize(terminal_area.width, terminal_area.height);
    }

    /// Notify all panels of resize with full layout
    pub fn notify_resize_all(&mut self, layout: &crate::ui::AppLayout) {
        self.file_tree.on_resize(layout.file_tree.width, layout.file_tree.height);
        self.editor.on_resize(layout.editor.width, layout.editor.height);
        // Terminal panel handles border calculations internally
        self.terminal.on_resize(layout.terminal.width, layout.terminal.height);
        self.chat.on_resize(layout.chat.width, layout.chat.height);
    }

    /// Handle focus change - notify panels and update layout
    pub fn handle_focus_change(&mut self, new_focus: PanelId, area: Rect) {
        // Notify panels of focus/blur
        for id in [PanelId::FILE_TREE, PanelId::EDITOR, PanelId::TERMINAL, PanelId::CHAT] {
            if id == new_focus {
                self.get_mut(id).on_focus();
            } else {
                self.get_mut(id).on_blur();
            }
        }

        // Recompute layout with new focus and notify panels of size changes
        let layout = crate::ui::get_layout_with_focus(area, Some(new_focus));
        self.notify_resize_all(&layout);
    }
}
