//! Scroll indicator utilities for panels

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
