//! UI rendering module

mod layout;
pub mod markdown;
pub mod model_selector;
mod render;
pub mod scroll;
pub mod settings;
pub mod theme;
pub mod workspace_selector;

pub use layout::{get_layout, get_layout_with_focus, AppLayout};
pub use markdown::render_markdown;
pub use model_selector::ModelSelector;
pub use render::render;
pub use scroll::ScrollBar;
pub use settings::SettingsModal;
pub use theme::{theme, set_theme, toggle_theme, cycle_theme, current_variant, Theme, ThemeVariant};
pub use workspace_selector::{WorkspaceSelectorModal, WorkspaceSelectorAction, SelectorMode};
