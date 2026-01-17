//! Scroll indicator utilities for panels

use crate::ui::theme::theme;
use ratatui::{
    layout::Rect,
    style::Style,
    Frame,
};

/// Scroll bar state and rendering
pub struct ScrollBar {
    /// Current scroll position (0-indexed)
    pub current: usize,
    /// Number of visible items
    pub visible: usize,
    /// Total number of items
    pub total: usize,
    /// Whether content is scrollable
    pub scrollable: bool,
}

impl ScrollBar {
    /// Create a new scroll bar
    pub fn new(current: usize, visible: usize, total: usize) -> Self {
        Self {
            current,
            visible,
            total,
            scrollable: total > visible,
        }
    }

    /// Check if at bottom
    pub fn at_bottom(&self) -> bool {
        if !self.scrollable {
            return true;
        }
        self.current + self.visible >= self.total
    }

    /// Render the scroll bar on the right edge of the given area
    /// Returns the area of the down arrow button for click detection
    pub fn render(&self, frame: &mut Frame, area: Rect, focused: bool) -> Option<Rect> {
        if !self.scrollable || area.height < 3 {
            return None;
        }

        let bar_x = area.x + area.width - 1;
        let bar_height = area.height.saturating_sub(1) as usize; // -1 for down arrow
        let arrow_y = area.y + area.height - 1;

        // Calculate thumb size and position
        let thumb_size = ((self.visible as f64 / self.total as f64) * bar_height as f64)
            .max(1.0) as usize;
        let max_scroll = self.total.saturating_sub(self.visible);
        let thumb_pos = if max_scroll > 0 {
            ((self.current as f64 / max_scroll as f64) * (bar_height - thumb_size) as f64) as usize
        } else {
            0
        };

        // Track and thumb colors
        let t = theme();
        let track_style = Style::default().fg(t.scrollbar_track);
        let thumb_style = if focused {
            Style::default().fg(t.scrollbar_thumb_focused)
        } else {
            Style::default().fg(t.scrollbar_thumb)
        };

        // Render track and thumb
        for i in 0..bar_height {
            let y = area.y + i as u16;
            let (ch, style) = if i >= thumb_pos && i < thumb_pos + thumb_size {
                ('┃', thumb_style) // Thumb
            } else {
                ('│', track_style) // Track
            };
            frame.buffer_mut().set_string(bar_x, y, &ch.to_string(), style);
        }

        // Render down arrow button
        let arrow_style = if self.at_bottom() {
            Style::default().fg(t.text_muted)
        } else if focused {
            Style::default().fg(t.accent_highlight)
        } else {
            Style::default().fg(t.text_secondary)
        };
        frame.buffer_mut().set_string(bar_x, arrow_y, "▼", arrow_style);

        // Return arrow button area for click detection
        Some(Rect::new(bar_x, arrow_y, 1, 1))
    }

    /// Check if a click is on the down arrow
    pub fn is_arrow_click(&self, x: u16, y: u16, area: Rect) -> bool {
        if !self.scrollable || area.height < 3 {
            return false;
        }
        let bar_x = area.x + area.width - 1;
        let arrow_y = area.y + area.height - 1;
        x == bar_x && y == arrow_y
    }

    /// Check if a click is on the scroll track (for page up/down)
    /// Returns Some(true) for click above thumb (page up), Some(false) for below (page down)
    pub fn track_click(&self, x: u16, y: u16, area: Rect) -> Option<bool> {
        if !self.scrollable || area.height < 3 {
            return None;
        }
        let bar_x = area.x + area.width - 1;
        let bar_height = area.height.saturating_sub(1) as usize;
        let arrow_y = area.y + area.height - 1;

        if x != bar_x || y >= arrow_y {
            return None;
        }

        let click_pos = (y - area.y) as usize;
        let thumb_size = ((self.visible as f64 / self.total as f64) * bar_height as f64)
            .max(1.0) as usize;
        let max_scroll = self.total.saturating_sub(self.visible);
        let thumb_pos = if max_scroll > 0 {
            ((self.current as f64 / max_scroll as f64) * (bar_height - thumb_size) as f64) as usize
        } else {
            0
        };

        if click_pos < thumb_pos {
            Some(true) // Above thumb - page up
        } else if click_pos >= thumb_pos + thumb_size {
            Some(false) // Below thumb - page down
        } else {
            None // On thumb - no action
        }
    }
}

/// Generate a scroll indicator string for panel titles
///
/// # Arguments
/// * `current` - Current scroll position (0-indexed line at top of view)
/// * `visible` - Number of visible lines
/// * `total` - Total number of lines
///
/// # Returns
/// A string like "[1-20/100]" or empty if no scrolling needed
pub fn scroll_indicator(current: usize, visible: usize, total: usize) -> String {
    if total <= visible {
        return String::new();
    }

    let start = current + 1; // 1-indexed for display
    let end = (current + visible).min(total);
    let percent = if total > 0 {
        ((current as f64 / (total - visible).max(1) as f64) * 100.0) as usize
    } else {
        0
    };

    format!(" [{}-{}/{}] {}%", start, end, total, percent)
}

/// Generate a compact scroll indicator (just percentage)
pub fn scroll_percent(current: usize, visible: usize, total: usize) -> String {
    if total <= visible {
        return String::new();
    }

    let percent = if total > visible {
        ((current as f64 / (total - visible) as f64) * 100.0) as usize
    } else {
        0
    };

    format!(" {}%", percent.min(100))
}

/// Check if content is scrollable
pub fn is_scrollable(visible: usize, total: usize) -> bool {
    total > visible
}

/// Render scroll bar characters for the right edge
/// Returns a vec of (row_index, char) for the scrollbar
pub fn scrollbar_chars(
    current: usize,
    visible: usize,
    total: usize,
    height: usize,
) -> Vec<(usize, char)> {
    if total <= visible || height == 0 {
        return vec![];
    }

    let mut chars = Vec::new();

    // Calculate thumb size and position
    let thumb_size = ((visible as f64 / total as f64) * height as f64).max(1.0) as usize;
    let thumb_pos = ((current as f64 / (total - visible) as f64) * (height - thumb_size) as f64) as usize;

    for i in 0..height {
        let c = if i >= thumb_pos && i < thumb_pos + thumb_size {
            '█' // Thumb
        } else {
            '░' // Track
        };
        chars.push((i, c));
    }

    chars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_indicator_no_scroll() {
        assert_eq!(scroll_indicator(0, 20, 10), "");
        assert_eq!(scroll_indicator(0, 20, 20), "");
    }

    #[test]
    fn test_scroll_indicator_at_top() {
        let result = scroll_indicator(0, 20, 100);
        assert!(result.contains("1-20"));
        assert!(result.contains("100"));
    }

    #[test]
    fn test_scroll_indicator_at_bottom() {
        let result = scroll_indicator(80, 20, 100);
        assert!(result.contains("81-100"));
        assert!(result.contains("100%"));
    }

    #[test]
    fn test_scroll_indicator_middle() {
        let result = scroll_indicator(40, 20, 100);
        assert!(result.contains("41-60"));
        assert!(result.contains("100"));
    }

    #[test]
    fn test_scroll_percent_no_scroll() {
        assert_eq!(scroll_percent(0, 20, 10), "");
        assert_eq!(scroll_percent(0, 20, 20), "");
    }

    #[test]
    fn test_scroll_percent_at_top() {
        let result = scroll_percent(0, 20, 100);
        assert!(result.contains("0%"));
    }

    #[test]
    fn test_scroll_percent_at_bottom() {
        let result = scroll_percent(80, 20, 100);
        assert!(result.contains("100%"));
    }

    #[test]
    fn test_is_scrollable() {
        assert!(!is_scrollable(20, 10));
        assert!(!is_scrollable(20, 20));
        assert!(is_scrollable(20, 100));
    }

    #[test]
    fn test_scrollbar_chars_no_scroll() {
        let chars = scrollbar_chars(0, 20, 10, 10);
        assert!(chars.is_empty());
    }

    #[test]
    fn test_scrollbar_chars_at_top() {
        let chars = scrollbar_chars(0, 10, 100, 10);
        assert!(!chars.is_empty());
        // Thumb should be at the top
        assert_eq!(chars[0].1, '█');
    }

    #[test]
    fn test_scrollbar_chars_at_bottom() {
        let chars = scrollbar_chars(90, 10, 100, 10);
        assert!(!chars.is_empty());
        // Thumb should be at the bottom
        assert_eq!(chars[chars.len() - 1].1, '█');
    }

    #[test]
    fn test_scrollbar_new() {
        let sb = ScrollBar::new(0, 20, 100);
        assert_eq!(sb.current, 0);
        assert_eq!(sb.visible, 20);
        assert_eq!(sb.total, 100);
        assert!(sb.scrollable);
    }

    #[test]
    fn test_scrollbar_not_scrollable() {
        let sb = ScrollBar::new(0, 20, 10);
        assert!(!sb.scrollable);
    }

    #[test]
    fn test_scrollbar_at_bottom() {
        // Not scrollable - always at bottom
        let sb = ScrollBar::new(0, 20, 10);
        assert!(sb.at_bottom());

        // Scrollable, at top
        let sb = ScrollBar::new(0, 20, 100);
        assert!(!sb.at_bottom());

        // Scrollable, at bottom
        let sb = ScrollBar::new(80, 20, 100);
        assert!(sb.at_bottom());
    }

    #[test]
    fn test_scrollbar_is_arrow_click() {
        let sb = ScrollBar::new(0, 20, 100);
        let area = Rect::new(0, 0, 10, 25);

        // Arrow is at bottom-right
        let bar_x = area.x + area.width - 1;
        let arrow_y = area.y + area.height - 1;

        assert!(sb.is_arrow_click(bar_x, arrow_y, area));
        assert!(!sb.is_arrow_click(0, 0, area));
    }

    #[test]
    fn test_scrollbar_is_arrow_click_not_scrollable() {
        let sb = ScrollBar::new(0, 20, 10);
        let area = Rect::new(0, 0, 10, 25);

        // Should return false when not scrollable
        assert!(!sb.is_arrow_click(9, 24, area));
    }

    #[test]
    fn test_scrollbar_track_click_not_scrollable() {
        let sb = ScrollBar::new(0, 20, 10);
        let area = Rect::new(0, 0, 10, 25);

        assert!(sb.track_click(9, 10, area).is_none());
    }

    #[test]
    fn test_scrollbar_track_click_above_thumb() {
        // Scrollbar at middle position
        let sb = ScrollBar::new(50, 20, 100);
        let area = Rect::new(0, 0, 10, 25);
        let bar_x = area.x + area.width - 1;

        // Click above thumb should return Some(true) for page up
        let result = sb.track_click(bar_x, 1, area);
        assert!(result.is_some());
    }

    #[test]
    fn test_scrollbar_track_click_wrong_column() {
        let sb = ScrollBar::new(0, 20, 100);
        let area = Rect::new(0, 0, 10, 25);

        // Click not on scroll bar column
        assert!(sb.track_click(5, 10, area).is_none());
    }

    #[test]
    fn test_scroll_indicator_with_zero_total() {
        let result = scroll_indicator(0, 20, 0);
        // Should not panic, return empty
        assert_eq!(result, "");
    }
}
