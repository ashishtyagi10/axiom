//! Scroll indicator utilities for panels

use ratatui::{
    layout::Rect,
    style::{Color, Style},
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
        let track_style = Style::default().fg(Color::DarkGray);
        let thumb_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
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
            Style::default().fg(Color::DarkGray)
        } else if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
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
}
