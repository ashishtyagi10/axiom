//! Theme system for light/dark mode support

use parking_lot::RwLock;
use ratatui::style::Color;
use std::sync::LazyLock;

/// All semantic colors used throughout the UI
#[derive(Debug, Clone)]
pub struct Theme {
    // Backgrounds
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_modal: Color,
    pub bg_selection: Color,
    pub bg_hover: Color,

    // Text
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_inverse: Color,

    // Borders
    pub border_focused: Color,
    pub border_unfocused: Color,

    // Accents
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub accent_highlight: Color,

    // Status
    pub status_success: Color,
    pub status_error: Color,
    pub status_warning: Color,
    pub status_info: Color,

    // Diff
    pub diff_added_bg: Color,
    pub diff_added_fg: Color,
    pub diff_removed_bg: Color,
    pub diff_removed_fg: Color,
    pub diff_modified_bg: Color,
    pub diff_modified_fg: Color,

    // Code blocks
    pub code_bg: Color,
    pub code_border: Color,
    pub code_text: Color,

    // Scrollbar
    pub scrollbar_track: Color,
    pub scrollbar_thumb: Color,
    pub scrollbar_thumb_focused: Color,

    // Status bar
    pub statusbar_bg: Color,
    pub statusbar_mode_bg: Color,
    pub statusbar_mode_fg: Color,
    pub statusbar_focus_bg: Color,
    pub statusbar_focus_fg: Color,
    pub statusbar_workspace_bg: Color,
    pub statusbar_workspace_fg: Color,
    pub statusbar_agents_bg: Color,
    pub statusbar_agents_fg: Color,

    // File tree
    pub file_tree_directory: Color,
    pub file_tree_file: Color,
    pub file_tree_symlink: Color,

    // Agent status
    pub agent_running: Color,
    pub agent_completed: Color,
    pub agent_failed: Color,
    pub agent_pending: Color,
}

/// Theme variant selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeVariant {
    #[default]
    Dark,
    Light,
    System,
}

impl ThemeVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeVariant::Dark => "dark",
            ThemeVariant::Light => "light",
            ThemeVariant::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" => ThemeVariant::Light,
            "system" => ThemeVariant::System,
            _ => ThemeVariant::Dark,
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            ThemeVariant::Dark => ThemeVariant::Light,
            ThemeVariant::Light => ThemeVariant::System,
            ThemeVariant::System => ThemeVariant::Dark,
        }
    }
}

impl Theme {
    /// Dark theme - optimized for dark terminal backgrounds
    pub fn dark() -> Self {
        Self {
            // Backgrounds
            bg_primary: Color::Reset,
            bg_secondary: Color::Rgb(30, 30, 40),
            bg_modal: Color::Rgb(25, 25, 35),
            bg_selection: Color::Rgb(50, 50, 70),
            bg_hover: Color::Rgb(40, 40, 55),

            // Text
            text_primary: Color::White,
            text_secondary: Color::Gray,
            text_muted: Color::DarkGray,
            text_inverse: Color::Black,

            // Borders
            border_focused: Color::Cyan,
            border_unfocused: Color::DarkGray,

            // Accents
            accent_primary: Color::Cyan,
            accent_secondary: Color::Magenta,
            accent_highlight: Color::Yellow,

            // Status
            status_success: Color::Green,
            status_error: Color::Red,
            status_warning: Color::Yellow,
            status_info: Color::Cyan,

            // Diff
            diff_added_bg: Color::Rgb(30, 50, 30),
            diff_added_fg: Color::Green,
            diff_removed_bg: Color::Rgb(50, 30, 30),
            diff_removed_fg: Color::Red,
            diff_modified_bg: Color::Rgb(50, 50, 30),
            diff_modified_fg: Color::Yellow,

            // Code blocks
            code_bg: Color::Rgb(35, 35, 45),
            code_border: Color::Rgb(60, 60, 80),
            code_text: Color::Rgb(200, 200, 210),

            // Scrollbar
            scrollbar_track: Color::Rgb(40, 40, 50),
            scrollbar_thumb: Color::Rgb(80, 80, 100),
            scrollbar_thumb_focused: Color::Rgb(100, 100, 130),

            // Status bar
            statusbar_bg: Color::Black,
            statusbar_mode_bg: Color::Blue,
            statusbar_mode_fg: Color::White,
            statusbar_focus_bg: Color::DarkGray,
            statusbar_focus_fg: Color::White,
            statusbar_workspace_bg: Color::Rgb(60, 60, 80),
            statusbar_workspace_fg: Color::Cyan,
            statusbar_agents_bg: Color::Magenta,
            statusbar_agents_fg: Color::White,

            // File tree
            file_tree_directory: Color::Cyan,
            file_tree_file: Color::White,
            file_tree_symlink: Color::Magenta,

            // Agent status
            agent_running: Color::Yellow,
            agent_completed: Color::Green,
            agent_failed: Color::Red,
            agent_pending: Color::DarkGray,
        }
    }

    /// Light theme - optimized for light terminal backgrounds
    pub fn light() -> Self {
        Self {
            // Backgrounds
            bg_primary: Color::Reset,
            bg_secondary: Color::Rgb(240, 240, 245),
            bg_modal: Color::Rgb(248, 248, 252),
            bg_selection: Color::Rgb(180, 200, 230),
            bg_hover: Color::Rgb(210, 215, 225),

            // Text - darker for better readability on light backgrounds
            text_primary: Color::Rgb(20, 20, 30),
            text_secondary: Color::Rgb(50, 50, 70),
            text_muted: Color::Rgb(90, 90, 110),
            text_inverse: Color::White,

            // Borders - darker for visibility
            border_focused: Color::Rgb(30, 90, 160),
            border_unfocused: Color::Rgb(140, 140, 160),

            // Accents - saturated and dark enough to read
            accent_primary: Color::Rgb(20, 80, 150),
            accent_secondary: Color::Rgb(120, 40, 120),
            accent_highlight: Color::Rgb(160, 100, 0),

            // Status - darker, more saturated
            status_success: Color::Rgb(20, 120, 20),
            status_error: Color::Rgb(180, 30, 30),
            status_warning: Color::Rgb(160, 100, 0),
            status_info: Color::Rgb(20, 80, 150),

            // Diff
            diff_added_bg: Color::Rgb(200, 240, 200),
            diff_added_fg: Color::Rgb(20, 100, 20),
            diff_removed_bg: Color::Rgb(250, 210, 210),
            diff_removed_fg: Color::Rgb(160, 30, 30),
            diff_modified_bg: Color::Rgb(250, 240, 180),
            diff_modified_fg: Color::Rgb(120, 80, 0),

            // Code blocks
            code_bg: Color::Rgb(235, 235, 242),
            code_border: Color::Rgb(160, 160, 180),
            code_text: Color::Rgb(30, 30, 45),

            // Scrollbar
            scrollbar_track: Color::Rgb(220, 220, 230),
            scrollbar_thumb: Color::Rgb(150, 150, 170),
            scrollbar_thumb_focused: Color::Rgb(110, 110, 130),

            // Status bar
            statusbar_bg: Color::Rgb(225, 225, 235),
            statusbar_mode_bg: Color::Rgb(30, 90, 160),
            statusbar_mode_fg: Color::White,
            statusbar_focus_bg: Color::Rgb(150, 150, 170),
            statusbar_focus_fg: Color::Rgb(20, 20, 30),
            statusbar_workspace_bg: Color::Rgb(180, 195, 220),
            statusbar_workspace_fg: Color::Rgb(20, 60, 120),
            statusbar_agents_bg: Color::Rgb(120, 40, 120),
            statusbar_agents_fg: Color::White,

            // File tree - darker colors for readability
            file_tree_directory: Color::Rgb(20, 80, 150),
            file_tree_file: Color::Rgb(20, 20, 30),
            file_tree_symlink: Color::Rgb(120, 40, 120),

            // Agent status - darker for visibility
            agent_running: Color::Rgb(160, 100, 0),
            agent_completed: Color::Rgb(20, 120, 20),
            agent_failed: Color::Rgb(180, 30, 30),
            agent_pending: Color::Rgb(90, 90, 110),
        }
    }
}

// Global theme state
struct ThemeState {
    theme: Theme,
    variant: ThemeVariant,
}

static THEME_STATE: LazyLock<RwLock<ThemeState>> = LazyLock::new(|| {
    RwLock::new(ThemeState {
        theme: Theme::dark(),
        variant: ThemeVariant::Dark,
    })
});

/// Get a read guard to the current theme
pub fn theme() -> parking_lot::MappedRwLockReadGuard<'static, Theme> {
    parking_lot::RwLockReadGuard::map(THEME_STATE.read(), |state| &state.theme)
}

/// Get the current theme variant
pub fn current_variant() -> ThemeVariant {
    THEME_STATE.read().variant
}

/// Set the theme to a specific variant
pub fn set_theme(variant: ThemeVariant) {
    let mut state = THEME_STATE.write();
    state.variant = variant;
    state.theme = match variant {
        ThemeVariant::Dark => Theme::dark(),
        ThemeVariant::Light => Theme::light(),
        ThemeVariant::System => {
            // For now, default to dark. Could detect terminal background in future.
            Theme::dark()
        }
    };
}

/// Toggle between light and dark themes
pub fn toggle_theme() {
    let current = current_variant();
    let next = match current {
        ThemeVariant::Dark => ThemeVariant::Light,
        ThemeVariant::Light => ThemeVariant::Dark,
        ThemeVariant::System => ThemeVariant::Light,
    };
    set_theme(next);
}

/// Cycle through all theme variants
pub fn cycle_theme() {
    let current = current_variant();
    set_theme(current.cycle());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_toggle() {
        set_theme(ThemeVariant::Dark);
        assert_eq!(current_variant(), ThemeVariant::Dark);

        toggle_theme();
        assert_eq!(current_variant(), ThemeVariant::Light);

        toggle_theme();
        assert_eq!(current_variant(), ThemeVariant::Dark);
    }

    #[test]
    fn test_theme_cycle() {
        set_theme(ThemeVariant::Dark);

        cycle_theme();
        assert_eq!(current_variant(), ThemeVariant::Light);

        cycle_theme();
        assert_eq!(current_variant(), ThemeVariant::System);

        cycle_theme();
        assert_eq!(current_variant(), ThemeVariant::Dark);
    }

    #[test]
    fn test_variant_from_str() {
        assert_eq!(ThemeVariant::from_str("dark"), ThemeVariant::Dark);
        assert_eq!(ThemeVariant::from_str("light"), ThemeVariant::Light);
        assert_eq!(ThemeVariant::from_str("system"), ThemeVariant::System);
        assert_eq!(ThemeVariant::from_str("LIGHT"), ThemeVariant::Light);
        assert_eq!(ThemeVariant::from_str("unknown"), ThemeVariant::Dark);
    }
}
