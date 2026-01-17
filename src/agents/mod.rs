//! Agent system for managing spawned AI agents
//!
//! The agent system tracks all agents spawned by the conductor,
//! manages their lifecycle, and stores their output.

mod conductor;
mod executor;
mod pty_agent;
mod pty_manager;
mod types;

pub use conductor::Conductor;
pub use executor::Executor;
pub use pty_agent::PtyAgent;
pub use pty_manager::PtyAgentManager;
pub use types::{AgentStatus, AgentType};

use crate::state::AgentId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// A spawned agent instance
#[derive(Debug)]
pub struct Agent {
    /// Unique identifier for this agent
    pub id: AgentId,

    /// Type of agent (determines its capabilities)
    pub agent_type: AgentType,

    /// Human-readable name for display
    pub name: String,

    /// Description of what this agent is doing
    pub description: String,

    /// Current execution status
    pub status: AgentStatus,

    /// Accumulated output from the agent
    pub output: String,

    /// When the agent was created
    pub created_at: Instant,

    /// When the agent finished (if terminal)
    pub completed_at: Option<Instant>,

    /// Token count (estimated from output characters)
    pub token_count: usize,

    /// Output line count (for progress tracking)
    pub line_count: usize,

    /// Progress percentage (0-100, optional based on agent type)
    pub progress: Option<u8>,

    /// Parent agent that spawned this agent (for aggregated output)
    pub parent_id: Option<AgentId>,
}

impl Agent {
    /// Create a new agent
    pub fn new(
        id: AgentId,
        agent_type: AgentType,
        name: String,
        description: String,
        parent_id: Option<AgentId>,
    ) -> Self {
        Self {
            id,
            agent_type,
            name,
            description,
            status: AgentStatus::Pending,
            output: String::new(),
            created_at: Instant::now(),
            completed_at: None,
            token_count: 0,
            line_count: 0,
            progress: None,
            parent_id,
        }
    }

    /// Start the agent (set status to Running)
    pub fn start(&mut self) {
        self.status = AgentStatus::Running;
    }

    /// Append output to the agent
    pub fn append_output(&mut self, chunk: &str) {
        self.output.push_str(chunk);
        // Estimate tokens (~4 chars per token on average)
        self.token_count = self.output.len() / 4;
        // Count lines
        self.line_count = self.output.lines().count();
    }

    /// Set progress percentage (0-100)
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = Some(progress.min(100));
    }

    /// Mark the agent as completed
    pub fn complete(&mut self) {
        self.status = AgentStatus::Completed;
        self.completed_at = Some(Instant::now());
    }

    /// Mark the agent as errored
    pub fn error(&mut self, message: String) {
        self.status = AgentStatus::Error(message);
        self.completed_at = Some(Instant::now());
    }

    /// Mark the agent as cancelled
    pub fn cancel(&mut self) {
        self.status = AgentStatus::Cancelled;
        self.completed_at = Some(Instant::now());
    }

    /// Get the duration since creation
    pub fn elapsed(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }

    /// Get the run duration (if completed)
    pub fn run_duration(&self) -> Option<std::time::Duration> {
        self.completed_at.map(|end| end.duration_since(self.created_at))
    }
}

/// Request to spawn a new agent
#[derive(Debug, Clone)]
pub struct AgentSpawnRequest {
    /// Type of agent to spawn
    pub agent_type: AgentType,

    /// Human-readable name
    pub name: String,

    /// Description of the task
    pub description: String,

    /// Task-specific parameters (JSON-serializable)
    pub parameters: Option<String>,

    /// Parent agent that is spawning this agent
    pub parent_id: Option<AgentId>,
}

/// Manages the lifecycle of all agents
pub struct AgentRegistry {
    /// All agents indexed by ID
    agents: HashMap<AgentId, Agent>,

    /// Counter for generating unique IDs
    next_id: AtomicU64,

    /// Currently selected agent (for display in output area)
    selected: Option<AgentId>,

    /// Order of agents for display (most recent first)
    order: Vec<AgentId>,
}

impl AgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            next_id: AtomicU64::new(1),
            selected: None,
            order: Vec::new(),
        }
    }

    /// Spawn a new agent and return its ID
    pub fn spawn(&mut self, request: AgentSpawnRequest) -> AgentId {
        let id = AgentId::new(self.next_id.fetch_add(1, Ordering::SeqCst));
        let agent = Agent::new(
            id,
            request.agent_type,
            request.name,
            request.description,
            request.parent_id,
        );

        self.order.insert(0, id); // Add to front (most recent)
        self.agents.insert(id, agent);

        // Auto-select if this is the first agent
        if self.selected.is_none() {
            self.selected = Some(id);
        }

        id
    }

    /// Get an agent by ID
    pub fn get(&self, id: AgentId) -> Option<&Agent> {
        self.agents.get(&id)
    }

    /// Get mutable agent by ID
    pub fn get_mut(&mut self, id: AgentId) -> Option<&mut Agent> {
        self.agents.get_mut(&id)
    }

    /// Get the currently selected agent
    pub fn selected(&self) -> Option<&Agent> {
        self.selected.and_then(|id| self.agents.get(&id))
    }

    /// Get the selected agent ID
    pub fn selected_id(&self) -> Option<AgentId> {
        self.selected
    }

    /// Select an agent by ID
    pub fn select(&mut self, id: AgentId) {
        if self.agents.contains_key(&id) {
            self.selected = Some(id);
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Get all agents in display order (most recent first)
    pub fn agents(&self) -> impl Iterator<Item = &Agent> {
        self.order.iter().filter_map(|id| self.agents.get(id))
    }

    /// Get all children of a parent agent (for aggregated output)
    pub fn children(&self, parent_id: AgentId) -> Vec<&Agent> {
        self.agents
            .values()
            .filter(|a| a.parent_id == Some(parent_id))
            .collect()
    }

    /// Get the number of agents
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Get the number of running agents
    pub fn running_count(&self) -> usize {
        self.agents.values().filter(|a| a.status.is_running()).count()
    }

    /// Start an agent
    pub fn start(&mut self, id: AgentId) {
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.start();
        }
    }

    /// Append output to an agent
    pub fn append_output(&mut self, id: AgentId, chunk: &str) {
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.append_output(chunk);
        }
    }

    /// Mark an agent as complete
    pub fn complete(&mut self, id: AgentId) {
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.complete();
        }
    }

    /// Mark an agent as errored
    pub fn error(&mut self, id: AgentId, message: String) {
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.error(message);
        }
    }

    /// Cancel an agent
    pub fn cancel(&mut self, id: AgentId) {
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.cancel();
        }
    }

    /// Clear all agents from the registry
    pub fn clear(&mut self) {
        self.agents.clear();
        self.order.clear();
        self.selected = None;
    }

    /// Remove completed/cancelled agents older than the specified age
    pub fn cleanup_old(&mut self, max_age: std::time::Duration) {
        let now = Instant::now();
        let to_remove: Vec<AgentId> = self
            .agents
            .iter()
            .filter(|(_, a)| {
                a.status.is_terminal()
                    && a.completed_at
                        .map(|t| now.duration_since(t) > max_age)
                        .unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect();

        for id in to_remove {
            self.agents.remove(&id);
            self.order.retain(|&i| i != id);
            if self.selected == Some(id) {
                self.selected = self.order.first().copied();
            }
        }
    }

    /// Remove all children of a parent agent (for new interaction cleanup)
    pub fn remove_children(&mut self, parent_id: AgentId) {
        let to_remove: Vec<AgentId> = self
            .agents
            .iter()
            .filter(|(_, a)| a.parent_id == Some(parent_id))
            .map(|(id, _)| *id)
            .collect();

        for id in to_remove {
            self.agents.remove(&id);
            self.order.retain(|&i| i != id);
            if self.selected == Some(id) {
                self.selected = Some(parent_id); // Select parent instead
            }
        }
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_agent() {
        let mut registry = AgentRegistry::new();

        let request = AgentSpawnRequest {
            agent_type: AgentType::Shell,
            name: "Test Shell".to_string(),
            description: "Running ls".to_string(),
            parameters: None,
            parent_id: None,
        };

        let id = registry.spawn(request);
        assert_eq!(id.value(), 1);

        let agent = registry.get(id).unwrap();
        assert_eq!(agent.name, "Test Shell");
        assert!(matches!(agent.status, AgentStatus::Pending));
    }

    #[test]
    fn test_agent_lifecycle() {
        let mut registry = AgentRegistry::new();

        let id = registry.spawn(AgentSpawnRequest {
            agent_type: AgentType::Coder,
            name: "Coder".to_string(),
            description: "Writing code".to_string(),
            parameters: None,
            parent_id: None,
        });

        registry.start(id);
        assert!(registry.get(id).unwrap().status.is_running());

        registry.append_output(id, "Hello ");
        registry.append_output(id, "World");
        assert_eq!(registry.get(id).unwrap().output, "Hello World");

        registry.complete(id);
        assert!(registry.get(id).unwrap().status.is_terminal());
    }

    #[test]
    fn test_selection() {
        let mut registry = AgentRegistry::new();

        // First agent should be auto-selected
        let id1 = registry.spawn(AgentSpawnRequest {
            agent_type: AgentType::Shell,
            name: "First".to_string(),
            description: "".to_string(),
            parameters: None,
            parent_id: None,
        });
        assert_eq!(registry.selected_id(), Some(id1));

        // Second agent doesn't change selection
        let id2 = registry.spawn(AgentSpawnRequest {
            agent_type: AgentType::Shell,
            name: "Second".to_string(),
            description: "".to_string(),
            parameters: None,
            parent_id: None,
        });
        assert_eq!(registry.selected_id(), Some(id1));

        // Explicit selection works
        registry.select(id2);
        assert_eq!(registry.selected_id(), Some(id2));
    }
}
