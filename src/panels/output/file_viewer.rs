//! File viewer component for the output panel
//!
//! Displays file content with syntax highlighting in read-only mode.

use crate::panels::editor::Highlighter;
use crate::ui::theme::theme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::path::Path;

/// Read-only file viewer with syntax highlighting
pub struct FileViewer {
    /// File content as lines
    lines: Vec<String>,

    /// Cached highlighted lines
    highlighted: Vec<Vec<(String, Style)>>,

    /// Scroll offset (first visible line)
    scroll_offset: usize,

    /// Visible height in lines
    visible_height: usize,

    /// Syntax highlighter
    highlighter: Highlighter,

    /// Currently loaded file path
    current_path: Option<std::path::PathBuf>,
}

impl FileViewer {
    /// Create a new file viewer
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            highlighted: Vec::new(),
            scroll_offset: 0,
            visible_height: 20,
            highlighter: Highlighter::new(),
            current_path: None,
        }
    }

    /// Load file content
    pub fn load_file(&mut self, path: &Path) {
        // Don't reload if same file
        if self.current_path.as_deref() == Some(path) {
            return;
        }

        self.current_path = Some(path.to_path_buf());
        self.scroll_offset = 0;

        match std::fs::read_to_string(path) {
            Ok(content) => {
                self.lines = content.lines().map(String::from).collect();
                if self.lines.is_empty() {
                    self.lines.push(String::new());
                }
                // Re-highlight
                self.highlighted = self.highlighter.highlight_all(&self.lines, Some(path));
            }
            Err(e) => {
                self.lines = vec![format!("Error reading file: {}", e)];
                self.highlighted = vec![vec![(
                    self.lines[0].clone(),
                    Style::default().fg(Color::Red),
                )]];
            }
        }
    }

    /// Reload current file
    pub fn reload(&mut self) {
        if let Some(path) = self.current_path.clone() {
            self.current_path = None; // Force reload
            self.load_file(&path);
        }
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset.min(self.max_scroll());
    }

    /// Get visible height
    pub fn set_visible_height(&mut self, height: usize) {
        self.visible_height = height;
    }

    /// Maximum scroll offset
    fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(self.visible_height)
    }

    /// Scroll up by lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll down by lines
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(self.max_scroll());
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Render the file content
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let t = theme();

        if self.lines.is_empty() {
            let msg = Paragraph::new(Span::styled(
                "Empty file",
                Style::default().fg(t.text_muted),
            ));
            frame.render_widget(msg, area);
            return;
        }

        // Calculate line number width
        let line_num_width = self.lines.len().to_string().len();
        let visible_end = (self.scroll_offset + area.height as usize).min(self.lines.len());

        let display_lines: Vec<Line> = (self.scroll_offset..visible_end)
            .map(|i| {
                let line_num = format!("{:>width$} ", i + 1, width = line_num_width);

                let mut spans = vec![Span::styled(
                    line_num,
                    Style::default().fg(t.text_secondary),
                )];

                // Add highlighted content
                if i < self.highlighted.len() {
                    for (text, style) in &self.highlighted[i] {
                        spans.push(Span::styled(text.clone(), *style));
                    }
                } else if i < self.lines.len() {
                    spans.push(Span::raw(self.lines[i].clone()));
                }

                Line::from(spans)
            })
            .collect();

        let paragraph = Paragraph::new(display_lines);
        frame.render_widget(paragraph, area);
    }
}

impl Default for FileViewer {
    fn default() -> Self {
        Self::new()
    }
}
