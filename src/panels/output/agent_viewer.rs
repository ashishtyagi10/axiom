//! Agent viewer component for the output panel
//!
//! Displays agent output with full markdown support, status, token count, and progress.
//! For Conductor agents, also displays aggregated output from child agents.

use crate::agents::{Agent, AgentType};
use crate::ui::markdown::render_markdown;
use crate::ui::theme::theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

/// Agent output viewer with markdown rendering
pub struct AgentViewer {
    /// Scroll offset (first visible line)
    scroll_offset: usize,

    /// Visible height in lines
    visible_height: usize,

    /// Cached rendered lines (owned strings for 'static lifetime)
    cached_lines: Vec<String>,

    /// Cached line count
    cached_line_count: usize,

    /// Last output length (for cache invalidation)
    last_output_len: usize,

    /// Last agent ID for cache invalidation
    last_agent_id: Option<u64>,

    /// Last children output length (for cache invalidation)
    last_children_len: usize,
}

impl AgentViewer {
    /// Create a new agent viewer
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            visible_height: 20,
            cached_lines: Vec::new(),
            cached_line_count: 0,
            last_output_len: 0,
            last_agent_id: None,
            last_children_len: 0,
        }
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset
    pub fn set_scroll_offset(&mut self, offset: usize) {
        let max = self.max_scroll();
        self.scroll_offset = offset.min(max);
    }

    /// Set visible height
    pub fn set_visible_height(&mut self, height: usize) {
        self.visible_height = height;
    }

    /// Maximum scroll offset
    fn max_scroll(&self) -> usize {
        self.cached_line_count.saturating_sub(self.visible_height)
    }

    /// Scroll up by lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll down by lines
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(self.max_scroll());
    }

    /// Clear the viewer (reset state)
    pub fn clear(&mut self) {
        self.scroll_offset = 0;
        self.cached_lines.clear();
        self.cached_line_count = 0;
        self.last_output_len = 0;
        self.last_agent_id = None;
        self.last_children_len = 0;
    }

    /// Build a spinner character based on elapsed time
    fn spinner(elapsed_ms: u128) -> char {
        const FRAMES: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
        let idx = ((elapsed_ms / 100) % FRAMES.len() as u128) as usize;
        FRAMES[idx]
    }

    /// Update cache if needed
    fn update_cache(&mut self, agent: &Agent, children: &[Agent]) {
        let agent_id = agent.id.value();
        let output_len = agent.output.len();
        let children_len: usize = children.iter().map(|c| c.output.len()).sum();

        // Check if we need to update the cache
        if self.last_agent_id == Some(agent_id)
            && self.last_output_len == output_len
            && self.last_children_len == children_len
        {
            return;
        }

        self.last_agent_id = Some(agent_id);
        self.last_output_len = output_len;
        self.last_children_len = children_len;

        // Build combined output with children
        self.cached_lines.clear();

        // Add agent's own output
        if !agent.output.is_empty() {
            for line in agent.output.lines() {
                self.cached_lines.push(line.to_string());
            }
        }

        // Add children output with separators (only for Conductor)
        if agent.agent_type == AgentType::Conductor && !children.is_empty() {
            for child in children {
                // Add separator header for each child
                self.cached_lines.push(String::new());
                self.cached_lines.push(format!(
                    "┌─── {} {} ({}) ───",
                    child.agent_type.icon(),
                    child.name,
                    child.status
                ));

                // Add child's output
                if !child.output.is_empty() {
                    for line in child.output.lines() {
                        self.cached_lines.push(format!("│ {}", line));
                    }
                } else {
                    self.cached_lines.push("│ (no output)".to_string());
                }

                self.cached_lines.push("└───────────────────────────────".to_string());
            }
        }

        self.cached_line_count = self.cached_lines.len();

        // Auto-scroll to bottom for streaming content (any running agent)
        let any_running = agent.status.is_running()
            || children.iter().any(|c| c.status.is_running());
        if any_running {
            // Always scroll to bottom when streaming
            self.scroll_offset = self.max_scroll();
        }
    }

    /// Render output with markdown
    fn render_output(&self, area_width: usize) -> Vec<Line<'static>> {
        if self.cached_lines.is_empty() {
            return Vec::new();
        }

        // Join cached lines and render as markdown with themed base style
        let t = theme();
        let content = self.cached_lines.join("\n");
        render_markdown(&content, Style::default().fg(t.text_primary))
    }

    /// Render the agent output with markdown
    ///
    /// For Conductor agents, also renders aggregated output from children.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, agent: &Agent, children: &[Agent]) {
        // Update cache if needed
        self.update_cache(agent, children);

        let elapsed_ms = agent.elapsed().as_millis();
        let t = theme();

        // Build display lines - just the markdown content, no header
        let mut display_lines: Vec<Line> = Vec::new();

        if self.cached_lines.is_empty() {
            display_lines.push(Line::from(""));
            let waiting_msg = if agent.status.is_running() {
                format!("{} Waiting for output...", Self::spinner(elapsed_ms))
            } else {
                "No output".to_string()
            };
            display_lines.push(Line::from(Span::styled(
                waiting_msg,
                Style::default().fg(t.text_muted),
            )));
        } else {
            // Render markdown content
            let markdown_lines = self.render_output(area.width as usize);
            let total_lines = markdown_lines.len();

            // Add markdown-rendered content with scrolling
            let max_visible = (area.height as usize).saturating_sub(1);
            let visible_end = (self.scroll_offset + max_visible).min(total_lines);

            for line in markdown_lines.into_iter()
                .skip(self.scroll_offset)
                .take(visible_end.saturating_sub(self.scroll_offset))
            {
                display_lines.push(line);
            }

            // Scroll indicator if content is scrollable
            if total_lines > max_visible {
                let pct = if total_lines == 0 {
                    100
                } else {
                    (visible_end * 100) / total_lines
                };
                display_lines.push(Line::from(Span::styled(
                    format!("──── {}% ({}/{}) ────", pct, visible_end, total_lines),
                    Style::default().fg(t.text_muted),
                )));
            }
        }

        let paragraph = Paragraph::new(display_lines)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }
}

impl Default for AgentViewer {
    fn default() -> Self {
        Self::new()
    }
}
