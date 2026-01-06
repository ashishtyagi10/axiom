//! PTY wrapper with proper resize support
//!
//! CRITICAL: This fixes the 24x80 hardcoded issue from nterm.
//! The PTY is resized whenever the terminal panel area changes.

use crate::core::{PtyError, Result};
use crate::events::Event;
use crossbeam_channel::Sender;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;

/// PTY wrapper with dynamic resize support
pub struct Pty {
    /// Master PTY handle for resize operations
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,

    /// Writer for sending input to PTY
    writer: Arc<Mutex<Box<dyn Write + Send>>>,

    /// Current PTY size
    size: PtySize,

    /// Reader thread handle (for cleanup)
    _reader_handle: std::thread::JoinHandle<()>,
}

impl Pty {
    /// Create a new PTY with initial size
    ///
    /// # Arguments
    /// * `cols` - Initial number of columns
    /// * `rows` - Initial number of rows
    /// * `event_tx` - Channel to send PTY output events
    /// * `cwd` - Working directory for the shell
    ///
    /// # Returns
    /// * `Ok(Pty)` on success
    /// * `Err(PtyError)` if PTY creation fails
    pub fn new(cols: u16, rows: u16, event_tx: Sender<Event>, cwd: &Path) -> Result<Self> {
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

        // Get shell from environment
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

        // Spawn shell process in the specified directory
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd);
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

        let master = Arc::new(Mutex::new(pair.master));
        let writer = Arc::new(Mutex::new(writer));

        // Spawn reader thread with bounded buffer
        let reader_handle = std::thread::spawn(move || {
            Self::read_loop(reader, event_tx);
        });

        Ok(Self {
            master,
            writer,
            size,
            _reader_handle: reader_handle,
        })
    }

    /// Resize the PTY
    ///
    /// CRITICAL: This must be called when the terminal panel area changes.
    /// The PTY will send SIGWINCH to child processes so they reflow text.
    ///
    /// # Arguments
    /// * `cols` - New number of columns
    /// * `rows` - New number of rows
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        // Skip if size unchanged (avoid unnecessary syscalls)
        if self.size.cols == cols && self.size.rows == rows {
            return Ok(());
        }

        // Minimum size to prevent issues
        let cols = cols.max(10);
        let rows = rows.max(3);

        self.size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        // Resize the PTY - this sends SIGWINCH to child
        let master = self.master.lock();
        master
            .resize(self.size)
            .map_err(|e| PtyError::Resize(e.to_string()))?;

        Ok(())
    }

    /// Write data to the PTY
    ///
    /// Typically keyboard input is written here.
    pub fn write(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock();
        writer
            .write_all(data)
            .map_err(|e: std::io::Error| PtyError::Write(e.to_string()))?;
        writer
            .flush()
            .map_err(|e: std::io::Error| PtyError::Write(e.to_string()))?;
        Ok(())
    }

    /// Write a single character
    pub fn write_char(&self, c: char) -> Result<()> {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.write(s.as_bytes())
    }

    /// Get current size
    pub fn size(&self) -> (u16, u16) {
        (self.size.cols, self.size.rows)
    }

    /// Reader loop - runs in background thread
    fn read_loop(mut reader: Box<dyn Read + Send>, tx: Sender<Event>) {
        let mut buf = [0u8; 4096]; // Larger buffer for efficiency

        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    // EOF - shell exited
                    let _ = tx.send(Event::PtyExit(0));
                    break;
                }
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    if tx.send(Event::PtyOutput(data)).is_err() {
                        // Channel closed, exit
                        break;
                    }
                }
                Err(e) => {
                    // Read error - likely PTY closed
                    let code = e.raw_os_error().unwrap_or(-1);
                    let _ = tx.send(Event::PtyExit(code));
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_pty_size_min() {
        let (tx, _rx) = crossbeam_channel::bounded(10);
        let cwd = PathBuf::from("/tmp");
        let mut pty = Pty::new(80, 24, tx, &cwd).unwrap();

        // Resize to very small should clamp to minimum
        pty.resize(5, 1).unwrap();
        let (cols, rows) = pty.size();
        assert!(cols >= 10);
        assert!(rows >= 3);
    }

    #[test]
    fn test_pty_resize_skip_same() {
        let (tx, _rx) = crossbeam_channel::bounded(10);
        let cwd = PathBuf::from("/tmp");
        let mut pty = Pty::new(80, 24, tx, &cwd).unwrap();

        // Same size should not error
        pty.resize(80, 24).unwrap();
    }
}
