//! Syntax highlighting with syntect
//!
//! Provides efficient syntax highlighting with per-line caching.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

/// Syntax highlighter with caching
pub struct Highlighter {
    /// Syntax definitions
    syntax_set: SyntaxSet,

    /// Color themes
    theme_set: ThemeSet,

    /// Current theme name
    theme_name: String,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter {
    /// Create a new highlighter
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    /// Get syntax name for a file path
    pub fn detect_syntax(&self, path: Option<&Path>) -> &str {
        path.and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .and_then(|ext| {
                self.syntax_set
                    .find_syntax_by_extension(ext)
                    .map(|s| s.name.as_str())
            })
            .unwrap_or("Plain Text")
    }

    /// Highlight all lines (for initial load or refresh)
    pub fn highlight_all(
        &mut self,
        lines: &[String],
        file_path: Option<&Path>,
    ) -> Vec<Vec<(String, Style)>> {
        let syntax = file_path
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .and_then(|ext| self.syntax_set.find_syntax_by_extension(ext))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self
            .theme_set
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(|| self.theme_set.themes.values().next().unwrap());

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut result = Vec::with_capacity(lines.len());

        for line in lines {
            // Add newline for syntect's line-based parsing
            let line_with_newline = format!("{}\n", line);
            let highlighted = highlighter
                .highlight_line(&line_with_newline, &self.syntax_set)
                .unwrap_or_default();

            let styled: Vec<(String, Style)> = highlighted
                .into_iter()
                .map(|(style, text)| {
                    // Remove trailing newline from last segment
                    let text = text.trim_end_matches('\n').to_string();
                    (text, syntect_style_to_ratatui(style))
                })
                .filter(|(text, _)| !text.is_empty())
                .collect();

            result.push(styled);
        }

        result
    }

    /// Convert highlighted spans to ratatui Line
    pub fn to_line(spans: &[(String, Style)]) -> Line<'static> {
        let ratatui_spans: Vec<Span> = spans
            .iter()
            .map(|(text, style)| Span::styled(text.clone(), *style))
            .collect();
        Line::from(ratatui_spans)
    }
}

/// Convert syntect style to ratatui style
fn syntect_style_to_ratatui(style: syntect::highlighting::Style) -> Style {
    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);

    let mut ratatui_style = Style::default().fg(fg);

    // Apply font styles
    if style.font_style.contains(FontStyle::BOLD) {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
    }

    ratatui_style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_syntax() {
        let highlighter = Highlighter::new();
        assert_eq!(highlighter.detect_syntax(Some(Path::new("test.rs"))), "Rust");
        assert_eq!(highlighter.detect_syntax(Some(Path::new("test.py"))), "Python");
        assert_eq!(highlighter.detect_syntax(Some(Path::new("test.js"))), "JavaScript");
    }
}
