//! Terminal panel with PTY integration
//!
//! CRITICAL: Implements on_resize to propagate size changes to PTY.

use crate::core::Result;
use crate::events::Event;
use crate::state::{AppState, PanelId};
use crate::terminal::Pty;
use crossbeam_channel::Sender;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;

/// Terminal panel with embedded PTY
pub struct TerminalPanel {
    /// PTY handle
    pty: Pty,

    /// vt100 parser for terminal emulation
    parser: Arc<RwLock<vt100::Parser>>,

    /// Current panel size
    size: (u16, u16),

    /// Scroll offset from bottom
    scroll_offset: u16,
}

impl TerminalPanel {
    /// Create a new terminal panel
    ///
    /// # Arguments
    /// * `event_tx` - Channel to send PTY events
    /// * `cwd` - Working directory for the shell
    pub fn new(event_tx: Sender<Event>, cwd: &Path) -> Result<Self> {
        // Start with reasonable default size - will be resized immediately
        let cols = 80;
        let rows = 24;

        let pty = Pty::new(cols, rows, event_tx, cwd)?;
        let parser = Arc::new(RwLock::new(vt100::Parser::new(rows, cols, 1000)));

        Ok(Self {
            pty,
            parser,
            size: (cols, rows),
            scroll_offset: 0,
        })
    }

    /// Process PTY output data
    pub fn process_output(&mut self, data: &[u8]) {
        let mut parser = self.parser.write();
        parser.process(data);
    }

    /// Get terminal content as lines for rendering
    fn get_screen_lines(&self) -> Vec<Line<'static>> {
        let parser = self.parser.read();
        let screen = parser.screen();
        let (rows, cols) = screen.size();

        let mut lines = Vec::new();

        for row in 0..rows {
            let mut spans = Vec::new();

            for col in 0..cols {
                let cell = screen.cell(row, col);
                if let Some(cell) = cell {
                    let text = cell.contents();
                    if text.is_empty() {
                        spans.push(Span::raw(" "));
                    } else {
                        let mut style = Style::default();

                        // Apply foreground color
                        let fg = cell.fgcolor();
                        style = style.fg(vt100_color_to_ratatui(fg));

                        // Apply background color
                        let bg = cell.bgcolor();
                        style = style.bg(vt100_color_to_ratatui(bg));

                        // Apply attributes
                        if cell.bold() {
                            style = style.add_modifier(ratatui::style::Modifier::BOLD);
                        }
                        if cell.underline() {
                            style = style.add_modifier(ratatui::style::Modifier::UNDERLINED);
                        }
                        if cell.inverse() {
                            style = style.add_modifier(ratatui::style::Modifier::REVERSED);
                        }

                        spans.push(Span::styled(text.to_string(), style));
                    }
                } else {
                    spans.push(Span::raw(" "));
                }
            }

            lines.push(Line::from(spans));
        }

        lines
    }

    /// Write keyboard input to PTY
    fn write_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        let bytes: Vec<u8> = match code {
            KeyCode::Char(c) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    // Control characters
                    let ctrl_char = (c.to_ascii_lowercase() as u8).wrapping_sub(b'a' - 1);
                    vec![ctrl_char]
                } else {
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    s.as_bytes().to_vec()
                }
            }
            KeyCode::Enter => vec![b'\r'],
            KeyCode::Backspace => vec![0x7f],
            KeyCode::Tab => vec![b'\t'],
            KeyCode::Esc => vec![0x1b],
            KeyCode::Up => b"\x1b[A".to_vec(),
            KeyCode::Down => b"\x1b[B".to_vec(),
            KeyCode::Right => b"\x1b[C".to_vec(),
            KeyCode::Left => b"\x1b[D".to_vec(),
            KeyCode::Home => b"\x1b[H".to_vec(),
            KeyCode::End => b"\x1b[F".to_vec(),
            KeyCode::PageUp => b"\x1b[5~".to_vec(),
            KeyCode::PageDown => b"\x1b[6~".to_vec(),
            KeyCode::Delete => b"\x1b[3~".to_vec(),
            KeyCode::Insert => b"\x1b[2~".to_vec(),
            KeyCode::F(n) => {
                match n {
                    1 => b"\x1bOP".to_vec(),
                    2 => b"\x1bOQ".to_vec(),
                    3 => b"\x1bOR".to_vec(),
                    4 => b"\x1bOS".to_vec(),
                    5 => b"\x1b[15~".to_vec(),
                    6 => b"\x1b[17~".to_vec(),
                    7 => b"\x1b[18~".to_vec(),
                    8 => b"\x1b[19~".to_vec(),
                    9 => b"\x1b[20~".to_vec(),
                    10 => b"\x1b[21~".to_vec(),
                    11 => b"\x1b[23~".to_vec(),
                    12 => b"\x1b[24~".to_vec(),
                    _ => vec![],
                }
            }
            _ => vec![],
        };

        if !bytes.is_empty() {
            self.pty.write(&bytes)?;
        }

        Ok(())
    }
}

impl super::Panel for TerminalPanel {
    fn id(&self) -> PanelId {
        PanelId::TERMINAL
    }

    fn name(&self) -> &str {
        "Terminal"
    }

    fn handle_input(&mut self, event: &Event, _state: &mut AppState) -> Result<bool> {
        match event {
            Event::Key(key) => {
                self.write_key(key.code, key.modifiers)?;
                Ok(true)
            }
            Event::PtyOutput(data) => {
                self.process_output(data);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Get scrollback info from parser
        let parser = self.parser.read();
        let screen = parser.screen();
        let scrollback_len = screen.scrollback();
        let (_rows, _cols) = screen.size();
        drop(parser); // Release lock before getting lines

        // Generate scroll indicator if there's scrollback
        let scroll_info = if scrollback_len > 0 {
            format!(" [{}↑] ", scrollback_len)
        } else {
            String::new()
        };

        let title = format!(" Terminal{} ", scroll_info);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);

        // Get terminal content
        let lines = self.get_screen_lines();

        let paragraph = Paragraph::new(lines).block(block);

        frame.render_widget(paragraph, area);

        // Draw cursor if focused and cursor is visible
        // Some applications (like Claude Code) draw their own cursor and hide the terminal cursor
        if focused {
            let parser = self.parser.read();
            let screen = parser.screen();

            // Only show cursor if the application hasn't hidden it
            if !screen.hide_cursor() {
                let cursor = screen.cursor_position();
                let cursor_x = inner.x + cursor.1 as u16;
                let cursor_y = inner.y + cursor.0 as u16;

                if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                    frame.set_cursor_position((cursor_x, cursor_y));
                }
            }
        }
    }

    fn on_resize(&mut self, cols: u16, rows: u16) {
        // CRITICAL: Resize PTY when panel size changes
        // Account for borders on all sides (top/bottom and left/right)
        let inner_rows = rows.saturating_sub(2); // Top and bottom borders
        let inner_cols = cols.saturating_sub(2); // Left and right borders

        // Ensure minimum size to prevent issues
        let inner_cols = inner_cols.max(1);
        let inner_rows = inner_rows.max(1);

        if let Err(e) = self.pty.resize(inner_cols, inner_rows) {
            eprintln!("PTY resize failed: {}", e);
        }

        // Also resize the parser to match PTY size
        let mut parser = self.parser.write();
        parser.set_size(inner_rows, inner_cols);

        self.size = (inner_cols, inner_rows);
    }

    fn on_focus(&mut self) {
        self.scroll_offset = 0;
    }
}

/// Convert vt100 color to ratatui color
fn vt100_color_to_ratatui(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(0) => Color::Black,
        vt100::Color::Idx(1) => Color::Red,
        vt100::Color::Idx(2) => Color::Green,
        vt100::Color::Idx(3) => Color::Yellow,
        vt100::Color::Idx(4) => Color::Blue,
        vt100::Color::Idx(5) => Color::Magenta,
        vt100::Color::Idx(6) => Color::Cyan,
        vt100::Color::Idx(7) => Color::Gray,
        vt100::Color::Idx(8) => Color::DarkGray,
        vt100::Color::Idx(9) => Color::LightRed,
        vt100::Color::Idx(10) => Color::LightGreen,
        vt100::Color::Idx(11) => Color::LightYellow,
        vt100::Color::Idx(12) => Color::LightBlue,
        vt100::Color::Idx(13) => Color::LightMagenta,
        vt100::Color::Idx(14) => Color::LightCyan,
        vt100::Color::Idx(15) => Color::White,
        vt100::Color::Idx(n) => Color::Indexed(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
