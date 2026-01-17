//! Panel system with trait-based composition
//!
//! Each panel implements the Panel trait for consistent behavior.
//! Panels use AxiomService for backend operations via Commands.

mod file_tree;
mod input;
mod output;
mod agents;

pub use file_tree::FileTreePanel;
pub use input::InputPanel;
pub use output::OutputPanel;
pub use agents::AgentsPanel;

use crate::events::TuiEvent;
use crate::state::{AppState, PanelId};
use axiom_core::Result;
use ratatui::layout::Rect;
use ratatui::Frame;

/// Panel trait - defines the interface for all panels
pub trait Panel: Send {
    /// Get the panel's unique identifier
    fn id(&self) -> PanelId;

    /// Get the panel's display name
    fn name(&self) -> &str;

    /// Handle an input event
    fn handle_input(&mut self, event: &TuiEvent, state: &mut AppState) -> Result<bool>;

    /// Render the panel to the frame
    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool);

    /// Called when this panel gains focus
    fn on_focus(&mut self) {}

    /// Called when this panel loses focus
    fn on_blur(&mut self) {}

    /// Called when the panel area is resized
    fn on_resize(&mut self, _cols: u16, _rows: u16) {}
}
