//! PTY Agent Manager
//!
//! Manages multiple PTY sessions for external CLI coding agents.
//! Handles spawning, input routing, resize events, and cleanup.

use super::pty_agent::PtyAgent;
use crate::config::CliAgentConfig;
use crate::error::Result;
use crate::events::Event;
use crate::types::{AgentId, TerminalScreen};
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::path::Path;

/// Manages multiple PTY agent sessions
pub struct PtyAgentManager {
    /// Active PTY agents
    agents: HashMap<AgentId, PtyAgent>,

    /// Event sender
    event_tx: Sender<Event>,

    /// Default PTY dimensions
    default_cols: u16,
    default_rows: u16,
}

impl PtyAgentManager {
    /// Create a new PTY agent manager
    pub fn new(event_tx: Sender<Event>) -> Self {
        Self {
            agents: HashMap::new(),
            event_tx,
            default_cols: 80,
            default_rows: 24,
        }
    }

    /// Set the default PTY dimensions for new agents
    pub fn set_default_size(&mut self, cols: u16, rows: u16) {
        self.default_cols = cols.max(20);
        self.default_rows = rows.max(5);
    }

    /// Get the default PTY dimensions
    pub fn default_size(&self) -> (u16, u16) {
        (self.default_cols, self.default_rows)
    }

    /// Start a new CLI agent PTY session
    ///
    /// # Arguments
    /// * `id` - The agent's runtime ID (from AgentRegistry)
    /// * `config` - CLI agent configuration
    /// * `prompt` - User's prompt to pass to the agent
    /// * `cwd` - Working directory for the agent
    pub fn start(
        &mut self,
        id: AgentId,
        config: &CliAgentConfig,
        prompt: &str,
        cwd: &Path,
    ) -> Result<()> {
        let agent = PtyAgent::new(
            id,
            config,
            prompt,
            cwd,
            self.default_cols,
            self.default_rows,
            self.event_tx.clone(),
        )?;

        self.agents.insert(id, agent);
        Ok(())
    }

    /// Write input data to a CLI agent
    pub fn write(&mut self, id: AgentId, data: &[u8]) -> Result<()> {
        if let Some(agent) = self.agents.get(&id) {
            agent.write(data)?;
        }
        Ok(())
    }

    /// Resize a CLI agent's PTY
    pub fn resize(&mut self, id: AgentId, cols: u16, rows: u16) -> Result<()> {
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.resize(cols, rows)?;
        }
        Ok(())
    }

    /// Resize all active PTY agents
    pub fn resize_all(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.default_cols = cols;
        self.default_rows = rows;

        for agent in self.agents.values_mut() {
            agent.resize(cols, rows)?;
        }
        Ok(())
    }

    /// Get the screen for a CLI agent (UI-agnostic TerminalScreen)
    pub fn get_screen(&self, id: AgentId) -> Option<TerminalScreen> {
        self.agents.get(&id).map(|agent| agent.get_screen())
    }

    /// Get the raw text output for a CLI agent
    pub fn get_output_text(&self, id: AgentId) -> Option<String> {
        self.agents.get(&id).map(|agent| agent.get_output_text())
    }

    /// Check if an agent ID is a CLI agent managed by this manager
    pub fn contains(&self, id: AgentId) -> bool {
        self.agents.contains_key(&id)
    }

    /// Mark an agent as exited
    pub fn mark_exited(&mut self, id: AgentId) {
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.exited = true;
        }
    }

    /// Remove a CLI agent session
    pub fn remove(&mut self, id: AgentId) {
        self.agents.remove(&id);
    }

    /// Remove all exited agents
    pub fn cleanup_exited(&mut self) {
        self.agents.retain(|_, agent| !agent.exited);
    }

    /// Get all active CLI agent IDs
    pub fn active_ids(&self) -> Vec<AgentId> {
        self.agents
            .iter()
            .filter(|(_, agent)| !agent.exited)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get a reference to a specific agent
    pub fn get(&self, id: AgentId) -> Option<&PtyAgent> {
        self.agents.get(&id)
    }

    /// Get number of active CLI agents
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if there are no active CLI agents
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let manager = PtyAgentManager::new(tx);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_contains() {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let manager = PtyAgentManager::new(tx);
        let id = AgentId::new(1);
        assert!(!manager.contains(id));
    }
}
