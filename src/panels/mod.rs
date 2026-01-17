//! Panel system with trait-based composition
//!
//! Each panel implements the Panel trait for consistent behavior.
//!
//! New agentic layout:
//! - FileTree (left): File navigation
//! - Output (center-top): File content or agent output
//! - Input (center-bottom): Unified command input
//! - Agents (right): Spawned agents list

mod agents;
mod editor;
mod file_tree;
mod input;
mod output;

// Legacy panels (kept for potential reuse of components)
mod chat;
mod terminal;

pub use agents::AgentsPanel;
pub use editor::EditorPanel;
pub use file_tree::FileTreePanel;
pub use input::InputPanel;
pub use output::OutputPanel;

// Re-export editor components for file viewer
pub use editor::{DiffTracker, Highlighter, Position, Selection};

use crate::agents::AgentRegistry;
use crate::config::{AxiomConfig, CliAgentsConfig};
use crate::core::Result;
use crate::events::Event;
use crate::llm::ProviderRegistry;
use crate::state::{AppState, OutputContext, PanelId, WorkspaceId, WorkspaceView};
use crate::ui::{ModelSelector, SettingsModal, WorkspaceSelectorModal};
use parking_lot::RwLock;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::sync::Arc;

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
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool);

    /// Called when this panel gains focus
    fn on_focus(&mut self) {}

    /// Called when this panel loses focus
    fn on_blur(&mut self) {}

    /// Called when the panel area is resized
    fn on_resize(&mut self, _cols: u16, _rows: u16) {}

    /// Check if the panel can be closed
    fn can_close(&self) -> bool {
        true
    }

    /// Cleanup when panel is being destroyed
    fn destroy(&mut self) {}
}

/// Container for all panels in the new agentic layout
pub struct PanelRegistry {
    /// Left panel: file tree navigation
    pub file_tree: FileTreePanel,

    /// Center-top: output area (file content or agent output)
    pub output: OutputPanel,

    /// Center-bottom: unified input
    pub input: InputPanel,

    /// Right panel: agents list
    pub agents: AgentsPanel,

    /// Agent registry (shared with output and agents panels)
    pub agent_registry: Arc<RwLock<AgentRegistry>>,

    /// Model selector modal
    pub model_selector: ModelSelector,

    /// Settings modal
    pub settings: SettingsModal,

    /// Workspace selector modal
    pub workspace_selector: WorkspaceSelectorModal,

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
        let llm_registry = Arc::new(RwLock::new(llm_registry));
        let agent_registry = Arc::new(RwLock::new(AgentRegistry::new()));
        let cli_agents = Arc::new(config.cli_agents.clone());

        Ok(Self {
            file_tree: FileTreePanel::new(cwd),
            output: OutputPanel::new(agent_registry.clone()),
            input: InputPanel::new(event_tx.clone(), cli_agents),
            agents: AgentsPanel::new(agent_registry.clone(), event_tx),
            agent_registry,
            model_selector: ModelSelector::new(),
            settings: SettingsModal::new(config),
            workspace_selector: WorkspaceSelectorModal::new(),
            model_badge_area: None,
            llm_registry,
        })
    }

    /// Open the workspace selector modal with workspace list
    pub fn open_workspace_selector(&mut self, workspaces: Vec<WorkspaceView>, active_id: Option<WorkspaceId>) {
        self.workspace_selector.set_workspaces(workspaces, active_id);
    }

    /// Get the agent registry
    pub fn agent_registry(&self) -> Arc<RwLock<AgentRegistry>> {
        self.agent_registry.clone()
    }

    /// Set the output context (what's displayed in output panel)
    pub fn set_output_context(&mut self, context: OutputContext) {
        self.output.set_context(context);
    }

    /// Get the current output context
    pub fn output_context(&self) -> &OutputContext {
        self.output.context()
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
        // Get all models from all providers
        let registry = self.llm_registry.read();
        let mut all_models: Vec<String> = Vec::new();

        // Add models from all available providers with provider prefix
        for info in registry.provider_info() {
            if let Some(provider) = registry.get(&info.id) {
                if let Ok(models) = provider.list_models() {
                    for model in models {
                        all_models.push(format!("{}:{}", info.id, model));
                    }
                }
            }
        }

        // Get current model (from first provider for now)
        let current = registry
            .provider_info()
            .first()
            .map(|info| format!("{}:default", info.id))
            .unwrap_or_default();

        self.model_selector.set_models(all_models, &current);
    }

    /// Apply the selected model
    pub fn apply_selected_model(&mut self) -> Option<String> {
        self.model_selector
            .selected_model()
            .map(|s| s.to_string())
    }

    /// Get panel by ID
    pub fn get(&self, id: PanelId) -> &dyn Panel {
        match id {
            PanelId::FILE_TREE => &self.file_tree,
            PanelId::OUTPUT => &self.output,
            PanelId::INPUT => &self.input,
            PanelId::AGENTS => &self.agents,
            _ => &self.output, // fallback
        }
    }

    /// Get mutable panel by ID
    pub fn get_mut(&mut self, id: PanelId) -> &mut dyn Panel {
        match id {
            PanelId::FILE_TREE => &mut self.file_tree,
            PanelId::OUTPUT => &mut self.output,
            PanelId::INPUT => &mut self.input,
            PanelId::AGENTS => &mut self.agents,
            _ => &mut self.output, // fallback
        }
    }

    /// Notify all panels of resize with full layout
    pub fn notify_resize_all(&mut self, layout: &crate::ui::AppLayout) {
        self.file_tree
            .on_resize(layout.file_tree.width, layout.file_tree.height);
        self.output
            .on_resize(layout.output.width, layout.output.height);
        self.input
            .on_resize(layout.input.width, layout.input.height);
        self.agents
            .on_resize(layout.agents.width, layout.agents.height);
    }

    /// Handle focus change - notify panels and update layout
    pub fn handle_focus_change(&mut self, new_focus: PanelId, area: Rect) {
        // Notify panels of focus/blur
        for id in [
            PanelId::FILE_TREE,
            PanelId::OUTPUT,
            PanelId::INPUT,
            PanelId::AGENTS,
        ] {
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

    /// Handle workspace switch - update panels to reflect new workspace
    pub fn handle_workspace_switch(&mut self, workspace_path: &std::path::Path) {
        // Update file tree to show new workspace root
        self.file_tree.set_root(workspace_path);

        // Clear output context (no file selected in new workspace)
        self.set_output_context(OutputContext::Empty);
    }

    /// Update the CLI agents configuration (used when switching workspaces with different configs)
    pub fn update_cli_agents(&mut self, cli_agents: CliAgentsConfig) {
        self.input.set_cli_agents(Arc::new(cli_agents));
    }
}
