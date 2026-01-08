//! Event system with bounded channels
//!
//! Uses crossbeam bounded channels for backpressure to prevent memory bloat.

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use crossterm::event::{KeyEvent, MouseEvent};
use std::time::Duration;

use crate::agents::{AgentSpawnRequest, AgentStatus};
use crate::state::{AgentId, OutputContext};

/// Application events - unified event type
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input event
    Key(KeyEvent),

    /// Mouse input event
    Mouse(MouseEvent),

    /// Terminal resize event with new dimensions (width, height)
    Resize(u16, u16),

    /// PTY output data (raw bytes from the pseudo-terminal)
    PtyOutput(Vec<u8>),

    /// PTY process exited with the specified exit code
    PtyExit(i32),

    /// LLM streaming response chunk (partial token/text)
    LlmChunk(String),

    /// LLM response generation complete
    LlmDone,

    /// LLM error occurred with description
    LlmError(String),

    /// File modification request from LLM
    ///
    /// Contains the file path and the new content to be applied.
    FileModification { path: String, content: String },

    /// File changed on disk (detected by file watcher)
    ///
    /// Contains the path to the file that was created or modified.
    FileChanged(std::path::PathBuf),

    /// Tick event for periodic updates (e.g., UI animations, polling)
    Tick,

    /// Request to focus a specific panel by index
    FocusPanel(usize),

    /// Quit application request
    Quit,

    // ===== Agent System Events =====

    /// Request conductor to process user input
    ConductorRequest(String),

    /// Spawn a new agent
    AgentSpawn(AgentSpawnRequest),

    /// Agent status update
    AgentUpdate {
        /// The agent ID
        id: AgentId,
        /// New status
        status: AgentStatus,
    },

    /// Agent output chunk (streaming)
    AgentOutput {
        /// The agent ID
        id: AgentId,
        /// Output chunk
        chunk: String,
    },

    /// Agent completed
    AgentComplete {
        /// The agent ID
        id: AgentId,
    },

    /// Wake an idle agent (used for persistent Conductor)
    AgentWake(AgentId),

    /// Switch output context (what's displayed in output area)
    SwitchContext(OutputContext),

    /// Execute shell command (from input routing)
    ShellExecute(String),
}

/// Event bus using bounded crossbeam channels
///
/// Bounded channels provide backpressure - if the receiver is slow,
/// senders will block, preventing unbounded memory growth.
pub struct EventBus {
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl EventBus {
    /// Create a new event bus with specified capacity.
    ///
    /// Capacity determines how many events can be buffered before senders block.
    /// Recommended: 1024 for responsive UI with some buffering.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The maximum number of events the bus can hold before blocking senders.
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = bounded(capacity);
        Self { tx, rx }
    }

    /// Get a sender clone for spawning event producers.
    ///
    /// This sender can be cloned and sent to other threads to produce events.
    pub fn sender(&self) -> Sender<Event> {
        self.tx.clone()
    }

    /// Receive the next event, blocking until one is available or the timeout expires.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The maximum duration to wait for an event.
    ///
    /// # Returns
    ///
    /// * `Some(Event)` if an event was received.
    /// * `None` if the timeout expired or the channel is disconnected.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Event> {
        self.rx.recv_timeout(timeout).ok()
    }

    /// Try to receive the next event without blocking.
    ///
    /// # Returns
    ///
    /// * `Some(Event)` if an event is immediately available.
    /// * `None` if the channel is empty or disconnected.
    pub fn try_recv(&self) -> Option<Event> {
        match self.rx.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Drain up to `max` events from the queue.
    ///
    /// Useful for batch processing to prevent event starvation or to handle
    /// multiple accumulated events (like resize or pty output) at once.
    ///
    /// # Arguments
    ///
    /// * `max` - The maximum number of events to retrieve.
    ///
    /// # Returns
    ///
    /// A vector containing the drained events.
    pub fn drain(&self, max: usize) -> Vec<Event> {
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
    fn test_event_bus_bounded() {
        let bus = EventBus::new(10);

        // Should be able to send up to capacity
        for _ in 0..10 {
            bus.sender().try_send(Event::Tick).unwrap();
        }

        // 11th send should fail (channel full)
        assert!(bus.sender().try_send(Event::Tick).is_err());

        // Drain should get all 10
        let events = bus.drain(50);
        assert_eq!(events.len(), 10);
    }

    #[test]
    fn test_drain_partial() {
        let bus = EventBus::new(100);

        for _ in 0..5 {
            bus.sender().try_send(Event::Tick).unwrap();
        }

        let events = bus.drain(3);
        assert_eq!(events.len(), 3);

        let remaining = bus.drain(10);
        assert_eq!(remaining.len(), 2);
    }
}
