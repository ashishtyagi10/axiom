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

use crate::config::AxiomConfig;
use crate::core::Result;
use crate::events::Event;
use crate::llm::ProviderRegistry;
use crate::state::{AppState, PanelId};
use crate::ui::{ModelSelector, SettingsModal};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::sync::Arc;
use parking_lot::RwLock;

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
    /// Settings modal
    pub settings: SettingsModal,
    /// Cached model badge area for click detection
    pub model_badge_area: Option<Rect>,
    /// LLM provider registry for multi-provider support
    pub llm_registry: Arc<RwLock<ProviderRegistry>>,
}

impl PanelRegistry {
    /// Create panel registry with all default panels and LLM providers
    pub fn new(
        event_tx: crossbeam_channel::Sender<Event>,
        cwd: &std::path::Path,
        llm_registry: ProviderRegistry,
        config: &AxiomConfig,
    ) -> Result<Self> {
        let registry = Arc::new(RwLock::new(llm_registry));

        // Get the active provider for the chat panel
        let provider = registry
            .read()
            .active()
            .expect("No active LLM provider configured");

        Ok(Self {
            file_tree: FileTreePanel::new(cwd),
            editor: EditorPanel::new(),
            terminal: TerminalPanel::new(event_tx.clone(), cwd)?,
            chat: ChatPanel::new(event_tx, provider),
            model_selector: ModelSelector::new(),
            settings: SettingsModal::new(config),
            model_badge_area: None,
            llm_registry: registry,
        })
    }

    /// Open the settings modal with current configuration
    pub fn open_settings(&mut self, config: &AxiomConfig) {
        self.settings = SettingsModal::new(config);
    }

    /// Apply settings and return updated config if there are changes
    pub fn apply_settings(&self) -> Option<AxiomConfig> {
        if self.settings.has_changes() {
            Some(self.settings.to_config())
        } else {
            None
        }
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
        let provider_id = self.chat.provider_id();

        // Get all models from all providers
        let registry = self.llm_registry.read();
        let mut all_models: Vec<String> = Vec::new();

        // Add models from all available providers with provider prefix
        for info in registry.provider_info() {
            if let Some(provider) = registry.get(&info.id) {
                if let Ok(models) = provider.list_models() {
                    for model in models {
                        // Format: "provider:model" for display
                        all_models.push(format!("{}:{}", info.id, model));
                    }
                }
            }
        }

        let current_full = format!("{}:{}", provider_id, current);
        self.model_selector.set_models(all_models, &current_full);
    }

    /// Apply the selected model
    pub fn apply_selected_model(&mut self) -> Option<String> {
        if let Some(selection) = self.model_selector.selected_model() {
            let selection = selection.to_string();

            // Parse "provider:model" format
            if let Some(colon_idx) = selection.find(':') {
                let provider_id = &selection[..colon_idx];
                let model = &selection[colon_idx + 1..];

                // Switch provider if different
                if provider_id != self.chat.provider_id() {
                    if let Some(provider) = self.llm_registry.read().get(provider_id) {
                        self.chat.set_provider(provider);
                    }
                }

                // Set the model
                self.chat.set_model(model);
                return Some(selection);
            }

            // Fallback: just set model on current provider
            self.chat.set_model(&selection);
            Some(selection)
        } else {
            None
        }
    }

    /// Switch to a specific provider
    pub fn switch_provider(&mut self, provider_id: &str) -> bool {
        if let Some(provider) = self.llm_registry.read().get(provider_id) {
            self.chat.set_provider(provider);
            true
        } else {
            false
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
