//! Internal event system for axiom-core
//!
//! This module defines backend-only events that don't depend on any UI framework.
//! UI events (Key, Mouse, etc.) are handled in axiom-tui, not here.

use crate::types::{AgentId, AgentSpawnRequest, AgentStatus, OutputContext};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use std::time::Duration;

/// Internal backend events
///
/// These events are used for communication within the backend.
/// They do not include UI events (Key, Mouse, Resize) which belong in axiom-tui.
#[derive(Debug, Clone)]
pub enum Event {
    /// Tick event for periodic updates
    Tick,

    // ===== LLM Events =====
    /// LLM streaming response chunk
    LlmChunk(String),

    /// LLM response complete
    LlmDone,

    /// LLM error occurred
    LlmError(String),

    /// File modification request from LLM
    FileModification { path: String, content: String },

    // ===== Agent Events =====
    /// Request conductor to process user input
    ConductorRequest(String),

    /// Spawn a new agent
    AgentSpawn(AgentSpawnRequest),

    /// Agent status update
    AgentUpdate {
        id: AgentId,
        status: AgentStatus,
    },

    /// Agent output chunk (streaming)
    AgentOutput {
        id: AgentId,
        chunk: String,
    },

    /// Agent completed
    AgentComplete {
        id: AgentId,
    },

    /// Wake an idle agent
    AgentWake(AgentId),

    /// Conductor response complete
    ConductorResponse(String),

    // ===== CLI Agent Events =====
    /// Invoke a CLI agent
    CliAgentInvoke {
        agent_id: String,
        prompt: String,
    },

    /// CLI agent PTY output
    CliAgentOutput {
        id: AgentId,
        data: Vec<u8>,
    },

    /// CLI agent PTY exited
    CliAgentExit {
        id: AgentId,
        exit_code: i32,
    },

    /// Send input to CLI agent
    CliAgentInput {
        id: AgentId,
        data: Vec<u8>,
    },

    // ===== Context Events =====
    /// Switch output context
    SwitchContext(OutputContext),

    /// Execute shell command
    ShellExecute(String),

    /// File changed on disk
    FileChanged(PathBuf),

    /// Quit signal
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
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = bounded(capacity);
        Self { tx, rx }
    }

    /// Get a sender clone for spawning event producers
    pub fn sender(&self) -> Sender<Event> {
        self.tx.clone()
    }

    /// Receive the next event with timeout
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Event> {
        self.rx.recv_timeout(timeout).ok()
    }

    /// Try to receive without blocking
    pub fn try_recv(&self) -> Option<Event> {
        self.rx.try_recv().ok()
    }

    /// Drain up to `max` events
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
    fn test_event_bus() {
        let bus = EventBus::new(10);
        bus.sender().send(Event::Tick).unwrap();
        assert!(bus.try_recv().is_some());
    }
}
