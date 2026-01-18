//! Event system with bounded channels
//!
//! Uses crossbeam bounded channels for backpressure to prevent memory bloat.

use axiom_core::SlashCommand;
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use crossterm::event::{KeyEvent, MouseEvent};
use std::path::PathBuf;
use std::time::Duration;

use crate::agents::{AgentSpawnRequest, AgentStatus};
use crate::state::{AgentId, OutputContext, PanelId, WorkspaceId};

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

    /// Request to focus a specific panel (smart focus based on context)
    FocusPanel(PanelId),

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

    /// Conductor response complete - add to history for context
    ConductorResponse(String),

    /// Switch output context (what's displayed in output area)
    SwitchContext(OutputContext),

    /// Execute shell command (from input routing)
    ShellExecute(String),

    // ===== CLI Agent Events =====

    /// Invoke a CLI agent with a prompt
    CliAgentInvoke {
        /// The agent ID (e.g., "claude", "gemini")
        agent_id: String,
        /// The user's prompt
        prompt: String,
    },

    /// CLI agent PTY output
    CliAgentOutput {
        /// The agent's runtime ID
        id: AgentId,
        /// Raw PTY output data
        data: Vec<u8>,
    },

    /// CLI agent PTY exited
    CliAgentExit {
        /// The agent's runtime ID
        id: AgentId,
        /// Exit code
        exit_code: i32,
    },

    /// Send input to a CLI agent
    CliAgentInput {
        /// The agent's runtime ID
        id: AgentId,
        /// Raw input data to send
        data: Vec<u8>,
    },

    // ===== Workspace Events =====

    /// Request to switch to a different workspace
    WorkspaceSwitch(WorkspaceId),

    /// Create a new workspace
    WorkspaceCreate {
        /// Workspace name
        name: String,
        /// Workspace path
        path: PathBuf,
    },

    /// Delete a workspace (by ID)
    WorkspaceDelete(WorkspaceId),

    /// Workspace switched successfully (notification)
    WorkspaceSwitched {
        /// The workspace ID that was switched to
        id: WorkspaceId,
        /// The workspace path
        path: PathBuf,
    },

    // ===== Slash Command Events =====

    /// Execute a slash command (e.g., /help, /exit, /settings)
    SlashCommand(SlashCommand),
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

    #[test]
    fn test_event_bus_capacity() {
        let bus = EventBus::new(5);

        for _ in 0..5 {
            assert!(bus.sender().try_send(Event::Tick).is_ok());
        }

        // Channel should be full
        assert!(bus.sender().try_send(Event::Tick).is_err());
    }

    #[test]
    fn test_event_bus_recv_timeout() {
        let bus = EventBus::new(10);

        // Nothing in channel - should timeout
        let result = bus.recv_timeout(Duration::from_millis(10));
        assert!(result.is_none());

        // Send an event
        bus.sender().try_send(Event::Quit).unwrap();

        // Should receive it
        let result = bus.recv_timeout(Duration::from_millis(100));
        assert!(matches!(result, Some(Event::Quit)));
    }

    #[test]
    fn test_event_bus_try_recv_empty() {
        let bus = EventBus::new(10);

        // Empty channel
        assert!(bus.try_recv().is_none());

        // Send something
        bus.sender().try_send(Event::Tick).unwrap();

        // Should receive it
        assert!(bus.try_recv().is_some());

        // Now empty again
        assert!(bus.try_recv().is_none());
    }

    #[test]
    fn test_event_bus_drain_empty() {
        let bus = EventBus::new(10);

        let events = bus.drain(100);
        assert!(events.is_empty());
    }

    #[test]
    fn test_event_bus_multiple_senders() {
        let bus = EventBus::new(10);

        let tx1 = bus.sender();
        let tx2 = bus.sender();

        tx1.try_send(Event::Tick).unwrap();
        tx2.try_send(Event::Tick).unwrap();

        let events = bus.drain(10);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_event_clone() {
        let event = Event::LlmChunk("hello".to_string());
        let cloned = event.clone();

        if let Event::LlmChunk(text) = cloned {
            assert_eq!(text, "hello");
        } else {
            panic!("Clone failed");
        }
    }

    #[test]
    fn test_event_debug() {
        let event = Event::Tick;
        let debug = format!("{:?}", event);
        assert!(debug.contains("Tick"));
    }

    #[test]
    fn test_event_variants_agent() {
        let id = AgentId::new(1);

        let spawn = Event::AgentSpawn(AgentSpawnRequest {
            agent_type: crate::agents::AgentType::Shell,
            name: "test".to_string(),
            description: "test".to_string(),
            parameters: None,
            parent_id: None,
        });
        assert!(matches!(spawn, Event::AgentSpawn(_)));

        let update = Event::AgentUpdate {
            id,
            status: AgentStatus::Running,
        };
        assert!(matches!(update, Event::AgentUpdate { .. }));

        let output = Event::AgentOutput {
            id,
            chunk: "output".to_string(),
        };
        assert!(matches!(output, Event::AgentOutput { .. }));

        let complete = Event::AgentComplete { id };
        assert!(matches!(complete, Event::AgentComplete { .. }));
    }

    #[test]
    fn test_event_variants_llm() {
        let chunk = Event::LlmChunk("text".to_string());
        assert!(matches!(chunk, Event::LlmChunk(_)));

        let done = Event::LlmDone;
        assert!(matches!(done, Event::LlmDone));

        let error = Event::LlmError("error".to_string());
        assert!(matches!(error, Event::LlmError(_)));
    }

    #[test]
    fn test_event_variants_file() {
        let modification = Event::FileModification {
            path: "/test.rs".to_string(),
            content: "fn main() {}".to_string(),
        };
        assert!(matches!(modification, Event::FileModification { .. }));

        let changed = Event::FileChanged(std::path::PathBuf::from("/test.rs"));
        assert!(matches!(changed, Event::FileChanged(_)));
    }

    #[test]
    fn test_event_variants_cli_agent() {
        let invoke = Event::CliAgentInvoke {
            agent_id: "claude".to_string(),
            prompt: "help me".to_string(),
        };
        assert!(matches!(invoke, Event::CliAgentInvoke { .. }));

        let output = Event::CliAgentOutput {
            id: AgentId::new(1),
            data: vec![65, 66, 67],
        };
        assert!(matches!(output, Event::CliAgentOutput { .. }));

        let exit = Event::CliAgentExit {
            id: AgentId::new(1),
            exit_code: 0,
        };
        assert!(matches!(exit, Event::CliAgentExit { .. }));

        let input = Event::CliAgentInput {
            id: AgentId::new(1),
            data: vec![13],
        };
        assert!(matches!(input, Event::CliAgentInput { .. }));
    }

    #[test]
    fn test_event_variants_slash_command() {
        let help = Event::SlashCommand(SlashCommand::Help { command: None });
        assert!(matches!(help, Event::SlashCommand(SlashCommand::Help { .. })));

        let exit = Event::SlashCommand(SlashCommand::Exit);
        assert!(matches!(exit, Event::SlashCommand(SlashCommand::Exit)));

        let clear = Event::SlashCommand(SlashCommand::Clear);
        assert!(matches!(clear, Event::SlashCommand(SlashCommand::Clear)));

        let settings = Event::SlashCommand(SlashCommand::Settings);
        assert!(matches!(settings, Event::SlashCommand(SlashCommand::Settings)));
    }
}
