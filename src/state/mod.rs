//! Application state management
//!
//! Contains the central state container and input mode state machine.

mod app;
mod focus;
mod input_mode;

pub use app::AppState;
pub use focus::{FocusState, PanelId};
pub use input_mode::InputMode;
