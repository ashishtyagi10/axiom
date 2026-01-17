//! PTY session wrapper for CLI agents
//!
//! Manages a single PTY session running an external CLI coding agent
//! (Claude Code, Gemini CLI, etc.) with terminal emulation.
//!
//! This module is UI-agnostic and returns `TerminalScreen` for rendering.

use crate::config::CliAgentConfig;
use crate::error::{AxiomError, Result};
use crate::events::Event;
use crate::types::{AgentId, TerminalCell, TerminalColor, TerminalLine, TerminalScreen};
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
            .map_err(|e| AxiomError::pty(format!("Failed to create PTY: {}", e)))?;

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
            .map_err(|e| AxiomError::pty(format!("Failed to spawn command: {}", e)))?;

        // Clone reader for background thread
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| AxiomError::pty(format!("Failed to clone reader: {}", e)))?;

        // Get writer for sending input
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AxiomError::pty(format!("Failed to get writer: {}", e)))?;

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
            .map_err(|e| AxiomError::pty(format!("Failed to resize: {}", e)))?;

        // Resize the parser
        self.parser.write().set_size(rows, cols);

        Ok(())
    }

    /// Write input data to the PTY
    pub fn write(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock();
        writer
            .write_all(data)
            .map_err(|e| AxiomError::pty(format!("Write error: {}", e)))?;
        writer
            .flush()
            .map_err(|e| AxiomError::pty(format!("Flush error: {}", e)))?;
        Ok(())
    }

    /// Get the current screen content as a UI-agnostic TerminalScreen
    pub fn get_screen(&self) -> TerminalScreen {
        let parser = self.parser.read();
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let mut lines = Vec::with_capacity(rows as usize);

        for row in 0..rows {
            let mut cells = Vec::with_capacity(cols as usize);

            for col in 0..cols {
                if let Some(cell) = screen.cell(row, col) {
                    let text = cell.contents();
                    let char = if text.is_empty() {
                        ' '
                    } else {
                        text.chars().next().unwrap_or(' ')
                    };

                    cells.push(TerminalCell {
                        char,
                        fg: vt100_color_to_terminal(cell.fgcolor()),
                        bg: vt100_color_to_terminal(cell.bgcolor()),
                        bold: cell.bold(),
                        italic: cell.italic(),
                        underline: cell.underline(),
                        inverse: cell.inverse(),
                    });
                } else {
                    cells.push(TerminalCell::default());
                }
            }

            lines.push(TerminalLine { cells });
        }

        // Get cursor position
        let cursor_pos = screen.cursor_position();
        let cursor = Some((cursor_pos.1, cursor_pos.0)); // (col, row)

        TerminalScreen {
            lines,
            cursor,
            cols,
            rows,
        }
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

/// Convert vt100 color to our UI-agnostic TerminalColor
fn vt100_color_to_terminal(color: vt100::Color) -> TerminalColor {
    match color {
        vt100::Color::Default => TerminalColor::Default,
        vt100::Color::Idx(0) => TerminalColor::Black,
        vt100::Color::Idx(1) => TerminalColor::Red,
        vt100::Color::Idx(2) => TerminalColor::Green,
        vt100::Color::Idx(3) => TerminalColor::Yellow,
        vt100::Color::Idx(4) => TerminalColor::Blue,
        vt100::Color::Idx(5) => TerminalColor::Magenta,
        vt100::Color::Idx(6) => TerminalColor::Cyan,
        vt100::Color::Idx(7) => TerminalColor::White,
        vt100::Color::Idx(8) => TerminalColor::BrightBlack,
        vt100::Color::Idx(9) => TerminalColor::BrightRed,
        vt100::Color::Idx(10) => TerminalColor::BrightGreen,
        vt100::Color::Idx(11) => TerminalColor::BrightYellow,
        vt100::Color::Idx(12) => TerminalColor::BrightBlue,
        vt100::Color::Idx(13) => TerminalColor::BrightMagenta,
        vt100::Color::Idx(14) => TerminalColor::BrightCyan,
        vt100::Color::Idx(15) => TerminalColor::BrightWhite,
        vt100::Color::Idx(index) => TerminalColor::Indexed { index },
        vt100::Color::Rgb(r, g, b) => TerminalColor::Rgb { r, g, b },
    }
}
