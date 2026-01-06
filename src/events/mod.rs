//! Event system with bounded channels
//!
//! Uses crossbeam bounded channels for backpressure to prevent memory bloat.

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use crossterm::event::{KeyEvent, MouseEvent};
use std::time::Duration;

/// Application events - unified event type
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input
    Key(KeyEvent),

    /// Mouse input
    Mouse(MouseEvent),

    /// Terminal resize (width, height)
    Resize(u16, u16),

    /// PTY output data
    PtyOutput(Vec<u8>),

    /// PTY process exited with code
    PtyExit(i32),

    /// LLM streaming chunk
    LlmChunk(String),

    /// LLM response complete
    LlmDone,

    /// LLM error occurred
    LlmError(String),

    /// Tick event for periodic updates
    Tick,

    /// Focus panel request
    FocusPanel(usize),

    /// Quit application
    Quit,
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
    /// Create a new event bus with specified capacity
    ///
    /// Capacity determines how many events can be buffered before senders block.
    /// Recommended: 1024 for responsive UI with some buffering.
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = bounded(capacity);
        Self { tx, rx }
    }

    /// Get a sender clone for spawning event producers
    pub fn sender(&self) -> Sender<Event> {
        self.tx.clone()
    }

    /// Receive next event, blocking until available or timeout
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Event> {
        self.rx.recv_timeout(timeout).ok()
    }

    /// Try to receive without blocking
    pub fn try_recv(&self) -> Option<Event> {
        match self.rx.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Drain up to max events from the queue
    ///
    /// Useful for batch processing to prevent event starvation.
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
