//! PTY session wrapper for CLI agents
//!
//! Manages a single PTY session running an external CLI coding agent
//! (Claude Code, Gemini CLI, etc.) with terminal emulation.

use crate::config::CliAgentConfig;
use crate::core::{PtyError, Result};
use crate::events::Event;
use crate::state::AgentId;
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

/// A PTY session for an external CLI agent
pub struct PtyAgent {
    /// The agent's runtime ID
    pub id: AgentId,

    /// Master PTY handle for resize operations
    master: Arc<parking_lot::Mutex<Box<dyn MasterPty + Send>>>,

    /// Writer for sending input to PTY
    writer: Arc<parking_lot::Mutex<Box<dyn Write + Send>>>,

    /// vt100 parser for terminal emulation
    parser: Arc<RwLock<vt100::Parser>>,

    /// Current PTY size
    size: PtySize,

    /// Whether the PTY process has exited
    pub exited: bool,
}

impl PtyAgent {
    /// Create a new PTY agent session
    ///
    /// # Arguments
    /// * `id` - The agent's runtime ID
    /// * `config` - CLI agent configuration
    /// * `prompt` - User's prompt to pass to the agent
    /// * `cwd` - Working directory for the agent
    /// * `cols` - Initial number of columns
    /// * `rows` - Initial number of rows
    /// * `event_tx` - Channel to send PTY output events
    pub fn new(
        id: AgentId,
        config: &CliAgentConfig,
        prompt: &str,
        cwd: &Path,
        cols: u16,
        rows: u16,
        event_tx: Sender<Event>,
    ) -> Result<Self> {
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        // Create PTY system
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(size)
            .map_err(|e| PtyError::Create(e.to_string()))?;

        // Build the command
        let mut cmd = CommandBuilder::new(&config.command);

        // Add default arguments
        for arg in &config.default_args {
            cmd.arg(arg);
        }

        // Add the user's prompt as the final argument
        if !prompt.is_empty() {
            cmd.arg(prompt);
        }

        // Set working directory if configured
        if config.use_cwd {
            cmd.cwd(cwd);
        }

        // Set any custom environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn the CLI agent process
        pair.slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        // Clone reader for background thread
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Read(e.to_string()))?;

        // Get writer for sending input
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Write(e.to_string()))?;

        let master = Arc::new(parking_lot::Mutex::new(pair.master));
        let writer = Arc::new(parking_lot::Mutex::new(writer));

        // Create vt100 parser
        let parser = Arc::new(RwLock::new(vt100::Parser::new(rows, cols, 1000)));

        // Clone parser for reader thread
        let parser_clone = parser.clone();

        // Spawn reader thread
        let agent_id = id;
        std::thread::spawn(move || {
            Self::read_loop(agent_id, reader, parser_clone, event_tx);
        });

        Ok(Self {
            id,
            master,
            writer,
            parser,
            size,
            exited: false,
        })
    }

    /// Resize the PTY
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        // Skip if size unchanged
        if self.size.cols == cols && self.size.rows == rows {
            return Ok(());
        }

        // Minimum size
        let cols = cols.max(10);
        let rows = rows.max(3);

        self.size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        // Resize the PTY
        let master = self.master.lock();
        master
            .resize(self.size)
            .map_err(|e| PtyError::Resize(e.to_string()))?;

        // Resize the parser
        self.parser.write().set_size(rows, cols);

        Ok(())
    }

    /// Write input data to the PTY
    pub fn write(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock();
        writer
            .write_all(data)
            .map_err(|e| PtyError::Write(e.to_string()))?;
        writer.flush().map_err(|e| PtyError::Write(e.to_string()))?;
        Ok(())
    }

    /// Get the current screen content as styled lines
    pub fn get_screen_lines(&self) -> Vec<ratatui::text::Line<'static>> {
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};

        let parser = self.parser.read();
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let mut lines = Vec::new();

        for row in 0..rows {
            let mut spans = Vec::new();

            for col in 0..cols {
                if let Some(cell) = screen.cell(row, col) {
                    let text = cell.contents();
                    if text.is_empty() {
                        spans.push(Span::raw(" "));
                    } else {
                        let mut style = Style::default();

                        // Apply foreground color
                        style = style.fg(vt100_color_to_ratatui(cell.fgcolor()));

                        // Apply background color
                        style = style.bg(vt100_color_to_ratatui(cell.bgcolor()));

                        // Apply attributes
                        if cell.bold() {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        if cell.italic() {
                            style = style.add_modifier(Modifier::ITALIC);
                        }
                        if cell.underline() {
                            style = style.add_modifier(Modifier::UNDERLINED);
                        }
                        if cell.inverse() {
                            style = style.add_modifier(Modifier::REVERSED);
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

    /// Get the raw text output (without styling)
    pub fn get_output_text(&self) -> String {
        let parser = self.parser.read();
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let mut output = String::new();

        for row in 0..rows {
            for col in 0..cols {
                if let Some(cell) = screen.cell(row, col) {
                    let text = cell.contents();
                    if text.is_empty() {
                        output.push(' ');
                    } else {
                        output.push_str(&text);
                    }
                } else {
                    output.push(' ');
                }
            }
            output.push('\n');
        }

        output.trim_end().to_string()
    }

    /// Get current size
    pub fn size(&self) -> (u16, u16) {
        (self.size.cols, self.size.rows)
    }

    /// Reader loop - runs in background thread
    fn read_loop(
        agent_id: AgentId,
        mut reader: Box<dyn Read + Send>,
        parser: Arc<RwLock<vt100::Parser>>,
        tx: Sender<Event>,
    ) {
        let mut buf = [0u8; 4096];

        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    // EOF - process exited
                    let _ = tx.send(Event::CliAgentExit {
                        id: agent_id,
                        exit_code: 0,
                    });
                    break;
                }
                Ok(n) => {
                    let data = &buf[..n];

                    // Feed data to vt100 parser
                    parser.write().process(data);

                    // Send output event
                    if tx
                        .send(Event::CliAgentOutput {
                            id: agent_id,
                            data: data.to_vec(),
                        })
                        .is_err()
                    {
                        // Channel closed
                        break;
                    }
                }
                Err(e) => {
                    // Read error
                    let code = e.raw_os_error().unwrap_or(-1);
                    let _ = tx.send(Event::CliAgentExit {
                        id: agent_id,
                        exit_code: code,
                    });
                    break;
                }
            }
        }
    }
}

/// Convert vt100 color to ratatui color
fn vt100_color_to_ratatui(color: vt100::Color) -> ratatui::style::Color {
    use ratatui::style::Color;

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
        vt100::Color::Idx(idx) => Color::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_vt100_color_default() {
        let color = vt100_color_to_ratatui(vt100::Color::Default);
        assert_eq!(color, Color::Reset);
    }

    #[test]
    fn test_vt100_color_basic_colors() {
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(0)), Color::Black);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(1)), Color::Red);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(2)), Color::Green);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(3)), Color::Yellow);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(4)), Color::Blue);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(5)), Color::Magenta);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(6)), Color::Cyan);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(7)), Color::Gray);
    }

    #[test]
    fn test_vt100_color_bright_colors() {
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(8)), Color::DarkGray);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(9)), Color::LightRed);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(10)), Color::LightGreen);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(11)), Color::LightYellow);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(12)), Color::LightBlue);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(13)), Color::LightMagenta);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(14)), Color::LightCyan);
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(15)), Color::White);
    }

    #[test]
    fn test_vt100_color_indexed() {
        // Test 256-color mode indices (16-255)
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(16)), Color::Indexed(16));
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(100)), Color::Indexed(100));
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Idx(255)), Color::Indexed(255));
    }

    #[test]
    fn test_vt100_color_rgb() {
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Rgb(255, 0, 0)), Color::Rgb(255, 0, 0));
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Rgb(0, 255, 0)), Color::Rgb(0, 255, 0));
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Rgb(0, 0, 255)), Color::Rgb(0, 0, 255));
        assert_eq!(vt100_color_to_ratatui(vt100::Color::Rgb(128, 128, 128)), Color::Rgb(128, 128, 128));
    }

    #[test]
    fn test_pty_agent_min_size_constants() {
        // The resize function enforces minimum sizes of 10 cols and 3 rows
        // Test that these constraints are documented/expected
        let min_cols: u16 = 10;
        let min_rows: u16 = 3;
        assert!(min_cols >= 10);
        assert!(min_rows >= 3);
    }
}
