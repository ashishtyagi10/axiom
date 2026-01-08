//! Markdown rendering for terminal display
//!
//! Converts markdown to styled ratatui Lines.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Render markdown text to styled lines
pub fn render_markdown(text: &str, base_style: Style) -> Vec<Line<'static>> {
    let renderer = MarkdownRenderer::new(base_style);
    renderer.render(text)
}

/// Markdown renderer state
struct MarkdownRenderer {
    base_style: Style,
    lines: Vec<Line<'static>>,
    current_line: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    in_code_block: bool,
    code_block_lang: Option<String>,
    code_block_content: String,
    list_depth: usize,
    ordered_list_num: Option<u64>,
}

impl MarkdownRenderer {
    fn new(base_style: Style) -> Self {
        Self {
            base_style,
            lines: Vec::new(),
            current_line: Vec::new(),
            style_stack: vec![base_style],
            in_code_block: false,
            code_block_lang: None,
            code_block_content: String::new(),
            list_depth: 0,
            ordered_list_num: None,
        }
    }

    fn current_style(&self) -> Style {
        *self.style_stack.last().unwrap_or(&self.base_style)
    }

    fn push_style(&mut self, style: Style) {
        self.style_stack.push(style);
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    fn flush_line(&mut self) {
        if !self.current_line.is_empty() {
            self.lines.push(Line::from(std::mem::take(&mut self.current_line)));
        } else {
            self.lines.push(Line::from(""));
        }
    }

    fn add_text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_block_content.push_str(text);
            return;
        }

        let style = self.current_style();

        // Handle newlines in text
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.flush_line();
            }
            if !part.is_empty() {
                self.current_line.push(Span::styled(part.to_string(), style));
            }
        }
    }

    fn render_code_block(&mut self) {
        let content = std::mem::take(&mut self.code_block_content);
        let lang = self.code_block_lang.take();

        // Calculate box width based on content
        let content_lines: Vec<&str> = content.lines().collect();
        let max_line_len = content_lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let inner_width = max_line_len.max(40).min(76); // Min 40, max 76 chars for code

        // Total width between corners = inner_width + 2 (for " " padding on each side)
        let total_inner = inner_width + 2;

        // Style definitions
        let border_style = Style::default()
            .fg(Color::Rgb(80, 80, 90))
            .bg(Color::Rgb(30, 34, 42));
        let header_bg_style = Style::default()
            .fg(Color::Rgb(180, 180, 80))
            .bg(Color::Rgb(30, 34, 42))
            .add_modifier(Modifier::BOLD);
        let code_style = Style::default()
            .fg(Color::Rgb(212, 212, 212))
            .bg(Color::Rgb(30, 34, 42));

        let lang_display = lang.as_deref().unwrap_or("code");
        let lang_with_spaces = format!(" {} ", lang_display);
        let lang_len = lang_with_spaces.len();

        // Build header: ┌─── python ────────────────────────────────┐
        // Total dashes = total_inner - lang_len
        let total_dashes = total_inner.saturating_sub(lang_len);
        let left_dashes = 3.min(total_dashes);
        let right_dashes = total_dashes.saturating_sub(left_dashes);

        self.lines.push(Line::from(vec![
            Span::styled("┌", border_style),
            Span::styled("─".repeat(left_dashes), border_style),
            Span::styled(lang_with_spaces, header_bg_style),
            Span::styled("─".repeat(right_dashes), border_style),
            Span::styled("┐", border_style),
        ]));

        // Code content - each line padded to fill the box
        for line in content_lines.iter() {
            let padding = inner_width.saturating_sub(line.len());
            self.lines.push(Line::from(vec![
                Span::styled("│ ", border_style),
                Span::styled(line.to_string(), code_style),
                Span::styled(" ".repeat(padding), code_style),
                Span::styled(" │", border_style),
            ]));
        }

        // If content was empty, add one empty line
        if content_lines.is_empty() {
            self.lines.push(Line::from(vec![
                Span::styled("│ ", border_style),
                Span::styled(" ".repeat(inner_width), code_style),
                Span::styled(" │", border_style),
            ]));
        }

        // Footer: └────────────────────────────────────────────────┘
        self.lines.push(Line::from(vec![
            Span::styled("└", border_style),
            Span::styled("─".repeat(total_inner), border_style),
            Span::styled("┘", border_style),
        ]));
    }

    fn render(mut self, text: &str) -> Vec<Line<'static>> {
        // Process text in segments, handling >>>user...<<< and >>>axiom...<<< blocks
        let mut remaining = text;

        while !remaining.is_empty() {
            // Look for the nearest special block
            let user_pos = remaining.find(">>>user\n");
            let axiom_pos = remaining.find(">>>axiom\n");

            match (user_pos, axiom_pos) {
                (Some(u), Some(a)) if u < a => {
                    // User block comes first
                    if u > 0 {
                        self.render_markdown_segment(&remaining[..u]);
                    }
                    remaining = &remaining[u..];
                    remaining = self.process_user_block(remaining);
                }
                (Some(u), Some(a)) if a < u => {
                    // Axiom block comes first
                    if a > 0 {
                        self.render_markdown_segment(&remaining[..a]);
                    }
                    remaining = &remaining[a..];
                    remaining = self.process_axiom_block(remaining);
                }
                (Some(u), None) => {
                    // Only user block
                    if u > 0 {
                        self.render_markdown_segment(&remaining[..u]);
                    }
                    remaining = &remaining[u..];
                    remaining = self.process_user_block(remaining);
                }
                (None, Some(a)) => {
                    // Only axiom block
                    if a > 0 {
                        self.render_markdown_segment(&remaining[..a]);
                    }
                    remaining = &remaining[a..];
                    remaining = self.process_axiom_block(remaining);
                }
                (None, None) => {
                    // No special blocks, render as markdown
                    self.render_markdown_segment(remaining);
                    break;
                }
                _ => {
                    self.render_markdown_segment(remaining);
                    break;
                }
            }
        }

        // Flush any remaining content
        if !self.current_line.is_empty() {
            self.flush_line();
        }

        self.lines
    }

    fn process_user_block<'a>(&mut self, text: &'a str) -> &'a str {
        let block_start = 8; // ">>>user\n".len()
        if let Some(end) = text[block_start..].find("\n<<<") {
            let user_text = &text[block_start..block_start + end];
            self.render_user_message_box(user_text);
            &text[block_start + end + 4..] // skip "\n<<<"
        } else {
            // No closing tag, render rest as markdown
            self.render_markdown_segment(text);
            ""
        }
    }

    fn process_axiom_block<'a>(&mut self, text: &'a str) -> &'a str {
        let block_start = 9; // ">>>axiom\n".len()
        if let Some(end) = text[block_start..].find("\n<<<") {
            let axiom_text = &text[block_start..block_start + end];
            self.render_axiom_message_box(axiom_text);
            &text[block_start + end + 4..] // skip "\n<<<"
        } else {
            // No closing tag yet (streaming), render what we have
            self.render_axiom_message_box(&text[block_start..]);
            ""
        }
    }

    /// Render a user message as a right-aligned box
    fn render_user_message_box(&mut self, text: &str) {
        let text_len = text.lines().map(|l| l.len()).max().unwrap_or(0);
        let box_width: usize = 50.min(text_len + 4).max(20);
        let inner_width = box_width - 4; // Account for "│ " and " │"

        // Styles
        let border_style = Style::default()
            .fg(Color::Rgb(100, 140, 180))
            .bg(Color::Rgb(35, 45, 55));
        let text_style = Style::default()
            .fg(Color::Rgb(220, 230, 240))
            .bg(Color::Rgb(35, 45, 55));

        // Calculate right-alignment padding (assume ~80 char width)
        let display_width: usize = 78;
        let padding = display_width.saturating_sub(box_width);
        let pad_str = " ".repeat(padding);

        // Top border
        self.lines.push(Line::from(vec![
            Span::raw(pad_str.clone()),
            Span::styled("┌", border_style),
            Span::styled("─".repeat(box_width - 2), border_style),
            Span::styled("┐", border_style),
        ]));

        // Content lines - wrap text if needed
        for line in text.lines() {
            // Simple word wrapping
            let mut remaining = line;
            while !remaining.is_empty() {
                let chunk_len = remaining.len().min(inner_width);
                let chunk = &remaining[..chunk_len];
                let text_padding = inner_width.saturating_sub(chunk.len());

                self.lines.push(Line::from(vec![
                    Span::raw(pad_str.clone()),
                    Span::styled("│ ", border_style),
                    Span::styled(chunk.to_string(), text_style),
                    Span::styled(" ".repeat(text_padding), text_style),
                    Span::styled(" │", border_style),
                ]));

                remaining = &remaining[chunk_len..];
            }
        }

        // If text was empty, add one empty line
        if text.is_empty() || text.lines().count() == 0 {
            self.lines.push(Line::from(vec![
                Span::raw(pad_str.clone()),
                Span::styled("│ ", border_style),
                Span::styled(" ".repeat(inner_width), text_style),
                Span::styled(" │", border_style),
            ]));
        }

        // Bottom border
        self.lines.push(Line::from(vec![
            Span::raw(pad_str),
            Span::styled("└", border_style),
            Span::styled("─".repeat(box_width - 2), border_style),
            Span::styled("┘", border_style),
        ]));

        // Add empty line after
        self.lines.push(Line::from(""));
    }

    /// Render an Axiom response as a left-aligned box
    fn render_axiom_message_box(&mut self, text: &str) {
        // Styles - teal/cyan theme for Axiom
        let header_style = Style::default()
            .fg(Color::Rgb(80, 200, 180))
            .add_modifier(Modifier::BOLD);
        let border_style = Style::default()
            .fg(Color::Rgb(60, 80, 70));
        let text_style = Style::default()
            .fg(Color::Rgb(220, 220, 220));

        // Header with Axiom label
        self.lines.push(Line::from(vec![
            Span::styled("◆ ", header_style),
            Span::styled("Axiom", header_style),
        ]));

        // Render content as markdown (supports code blocks, lists, etc.)
        if !text.is_empty() {
            // Add left border indicator for each line
            let rendered = MarkdownRenderer::new(text_style).render_markdown_only(text);
            for line in rendered {
                let mut new_spans = vec![Span::styled("│ ", border_style)];
                new_spans.extend(line.spans);
                self.lines.push(Line::from(new_spans));
            }
        }

        // Add empty line after
        self.lines.push(Line::from(""));
    }

    /// Render markdown without special block processing (for nested rendering)
    fn render_markdown_only(mut self, text: &str) -> Vec<Line<'static>> {
        self.render_markdown_segment(text);
        if !self.current_line.is_empty() {
            self.flush_line();
        }
        self.lines
    }

    /// Render a segment of markdown
    fn render_markdown_segment(&mut self, text: &str) {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);

        let parser = Parser::new_ext(text, options);

        for event in parser {
            match event {
                Event::Start(tag) => self.handle_start_tag(tag),
                Event::End(tag) => self.handle_end_tag(tag),
                Event::Text(text) => self.add_text(&text),
                Event::Code(code) => {
                    // Inline code
                    let style = Style::default()
                        .fg(Color::Rgb(230, 180, 80))
                        .bg(Color::Rgb(50, 50, 50));
                    self.current_line.push(Span::styled(format!("`{}`", code), style));
                }
                Event::SoftBreak => {
                    self.current_line.push(Span::raw(" "));
                }
                Event::HardBreak => {
                    self.flush_line();
                }
                Event::Rule => {
                    self.flush_line();
                    self.lines.push(Line::from(Span::styled(
                        "─".repeat(40),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                _ => {}
            }
        }
    }

    fn handle_start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush_line();
                let style = match level {
                    pulldown_cmark::HeadingLevel::H1 => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    pulldown_cmark::HeadingLevel::H2 => Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                };
                self.push_style(style);

                // Add heading prefix
                let prefix = match level {
                    pulldown_cmark::HeadingLevel::H1 => "# ",
                    pulldown_cmark::HeadingLevel::H2 => "## ",
                    pulldown_cmark::HeadingLevel::H3 => "### ",
                    _ => "#### ",
                };
                self.current_line.push(Span::styled(
                    prefix.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Tag::Paragraph => {
                if !self.lines.is_empty() && !self.current_line.is_empty() {
                    self.flush_line();
                }
            }
            Tag::BlockQuote => {
                self.flush_line();
                self.push_style(Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC));
                self.current_line.push(Span::styled(
                    "│ ".to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Tag::CodeBlock(kind) => {
                self.flush_line();
                self.in_code_block = true;
                self.code_block_content.clear();
                self.code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let lang = lang.to_string();
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang)
                        }
                    }
                    CodeBlockKind::Indented => None,
                };
            }
            Tag::List(start) => {
                self.flush_line();
                self.list_depth += 1;
                self.ordered_list_num = start;
            }
            Tag::Item => {
                self.flush_line();
                let indent = "  ".repeat(self.list_depth.saturating_sub(1));
                let bullet = if let Some(num) = self.ordered_list_num {
                    self.ordered_list_num = Some(num + 1);
                    format!("{}. ", num)
                } else {
                    "• ".to_string()
                };
                self.current_line.push(Span::styled(
                    format!("{}{}", indent, bullet),
                    Style::default().fg(Color::Yellow),
                ));
            }
            Tag::Emphasis => {
                self.push_style(self.current_style().add_modifier(Modifier::ITALIC));
            }
            Tag::Strong => {
                self.push_style(self.current_style().add_modifier(Modifier::BOLD));
            }
            Tag::Strikethrough => {
                self.push_style(self.current_style().add_modifier(Modifier::CROSSED_OUT));
            }
            Tag::Link { dest_url, .. } => {
                self.push_style(Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED));
                // Store the URL to show after link text
            }
            _ => {}
        }
    }

    fn handle_end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                self.pop_style();
                self.flush_line();
                self.lines.push(Line::from("")); // Add spacing after heading
            }
            TagEnd::Paragraph => {
                self.flush_line();
            }
            TagEnd::BlockQuote => {
                self.pop_style();
                self.flush_line();
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                self.render_code_block();
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
                if self.list_depth == 0 {
                    self.ordered_list_num = None;
                }
            }
            TagEnd::Item => {
                self.flush_line();
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                self.pop_style();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_markdown() {
        let lines = render_markdown("Hello **world**", Style::default());
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let lines = render_markdown(md, Style::default());
        assert!(lines.len() >= 3); // header + code + footer
    }
}
