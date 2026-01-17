//! TUI event system
//!
//! Handles UI-specific events (keyboard, mouse, resize) and bridges
//! to the axiom-core backend via Commands and Notifications.

use axiom_core::{AgentId, OutputContext};
use crossbeam_channel::{bounded, Receiver, Sender};
use crossterm::event::{KeyEvent, MouseEvent};
use std::path::PathBuf;
use std::time::Duration;

use crate::state::PanelId;

/// TUI-specific events
///
/// These events are for the terminal UI layer only.
/// Backend events are received as Notifications from AxiomService.
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// Keyboard input event
    Key(KeyEvent),

    /// Mouse input event
    Mouse(MouseEvent),

    /// Terminal resize event with new dimensions (width, height)
    Resize(u16, u16),

    /// Tick event for periodic updates (e.g., UI animations)
    Tick,

    /// Request to focus a specific panel
    FocusPanel(PanelId),

    /// Switch output context (what's displayed in output area)
    SwitchContext(OutputContext),

    /// File changed on disk (detected by file watcher)
    FileChanged(PathBuf),

    /// Quit application request
    Quit,

    // ===== Backend Bridge Events =====
    // These are converted from AxiomService Notifications

    /// Agent was spawned
    AgentSpawned {
        id: AgentId,
        name: String,
    },

    /// Agent status changed
    AgentStatusChanged {
        id: AgentId,
        is_complete: bool,
        is_error: bool,
    },

    /// Agent produced output
    AgentOutput {
        id: AgentId,
        chunk: String,
    },

    /// PTY output from CLI agent
    PtyOutput {
        id: AgentId,
        data: Vec<u8>,
    },

    /// PTY exited
    PtyExited {
        id: AgentId,
        exit_code: i32,
    },

    /// File was modified
    FileModified {
        path: PathBuf,
    },

    /// Error message from backend
    Error(String),

    /// Info message from backend
    Info(String),
}

/// TUI event bus using bounded crossbeam channels
///
/// Bounded channels provide backpressure - if the receiver is slow,
/// senders will block, preventing unbounded memory growth.
pub struct TuiEventBus {
    tx: Sender<TuiEvent>,
    rx: Receiver<TuiEvent>,
}

impl TuiEventBus {
    /// Create a new event bus with specified capacity
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = bounded(capacity);
        Self { tx, rx }
    }

    /// Get a sender clone for spawning event producers
    pub fn sender(&self) -> Sender<TuiEvent> {
        self.tx.clone()
    }

    /// Receive the next event with timeout
    pub fn recv_timeout(&self, timeout: Duration) -> Option<TuiEvent> {
        self.rx.recv_timeout(timeout).ok()
    }

    /// Try to receive without blocking
    pub fn try_recv(&self) -> Option<TuiEvent> {
        self.rx.try_recv().ok()
    }

    /// Drain up to `max` events
    pub fn drain(&self, max: usize) -> Vec<TuiEvent> {
        let mut events = Vec::with_capacity(max);
        while events.len() < max {
            match self.rx.try_recv() {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus() {
        let bus = TuiEventBus::new(10);
        bus.sender().send(TuiEvent::Tick).unwrap();
        assert!(bus.try_recv().is_some());
    }
}
