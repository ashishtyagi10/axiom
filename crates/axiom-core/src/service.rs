//! AxiomService - Main backend facade
//!
//! This module provides the primary interface for any UI (TUI, Web, GUI) to interact
//! with the Axiom backend. It wraps all internal components and exposes a clean,
//! command-based API.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────────┐   Command     ┌──────────────────┐
//! │   Any UI          │ ─────────────→│   AxiomService   │
//! │ (TUI, Web, GUI)   │               │                  │
//! │                   │ ←─────────────│   (Backend)      │
//! └───────────────────┘  Notification └──────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use axiom_core::{AxiomService, Command};
//!
//! // Create the service
//! let mut service = AxiomService::new(config, cwd)?;
//!
//! // Send commands
//! service.send(Command::ProcessInput { text: "hello".to_string() })?;
//!
//! // Poll for notifications
//! while let Some(notif) = service.poll_notification() {
//!     match notif {
//!         Notification::AgentOutput { id, chunk } => {
//!             println!("Agent {}: {}", id, chunk);
//!         }
//!         _ => {}
//!     }
//! }
//! ```

use crate::agents::{AgentRegistry, Conductor, Executor, PtyAgentManager};
use crate::commands::Command;
use crate::config::AxiomConfig;
use crate::error::{AxiomError, Result};
use crate::events::Event;
use crate::llm::ProviderRegistry;
use crate::notifications::Notification;
use crate::types::{
    AgentId, AgentSpawnRequest, AgentStatus, AgentType, AgentView, CliAgentInfo, OutputContext,
    TerminalScreen,
};
use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Main backend service facade
///
/// This is the primary interface for UIs to interact with the Axiom backend.
/// It processes commands, manages internal state, and emits notifications.
pub struct AxiomService {
    /// Internal event bus sender
    event_tx: Sender<Event>,

    /// Internal event bus receiver
    event_rx: Receiver<Event>,

    /// Notification sender (to UI)
    notification_tx: Sender<Notification>,

    /// Notification receiver (for UI to poll)
    notification_rx: Receiver<Notification>,

    /// Agent registry
    agent_registry: Arc<RwLock<AgentRegistry>>,

    /// PTY agent manager
    pty_manager: Arc<RwLock<PtyAgentManager>>,

    /// LLM provider registry
    llm_registry: Arc<RwLock<ProviderRegistry>>,

    /// Conductor for routing user input
    conductor: Arc<RwLock<Conductor>>,

    /// Executor for running agents
    executor: Arc<RwLock<Executor>>,

    /// Current working directory
    cwd: PathBuf,

    /// Configuration
    config: AxiomConfig,

    /// Current output context
    output_context: Arc<RwLock<OutputContext>>,
}

impl AxiomService {
    /// Create a new AxiomService
    ///
    /// # Arguments
    /// * `config` - Axiom configuration
    /// * `cwd` - Current working directory
    pub fn new(config: AxiomConfig, cwd: PathBuf) -> Result<Self> {
        // Create internal event bus (bounded for backpressure)
        let (event_tx, event_rx) = bounded(1000);

        // Create notification channel for UI
        let (notification_tx, notification_rx) = bounded(1000);

        // Create agent registry
        let agent_registry = Arc::new(RwLock::new(AgentRegistry::new()));

        // Create PTY manager
        let pty_manager = Arc::new(RwLock::new(PtyAgentManager::new(event_tx.clone())));

        // Create LLM provider registry
        let llm_registry = Arc::new(RwLock::new(ProviderRegistry::from_config(&config)));

        // Create conductor
        let conductor = Arc::new(RwLock::new(Conductor::new(
            llm_registry.clone(),
            event_tx.clone(),
        )));

        // Create executor
        let executor = Arc::new(RwLock::new(Executor::new(
            event_tx.clone(),
            agent_registry.clone(),
            cwd.clone(),
        )));

        // Initialize LLM providers based on config
        Self::init_llm_providers(&config, &llm_registry);

        Ok(Self {
            event_tx,
            event_rx,
            notification_tx,
            notification_rx,
            agent_registry,
            pty_manager,
            llm_registry,
            conductor,
            executor,
            cwd,
            config,
            output_context: Arc::new(RwLock::new(OutputContext::Empty)),
        })
    }

    /// Initialize LLM providers from configuration
    fn init_llm_providers(config: &AxiomConfig, registry: &Arc<RwLock<ProviderRegistry>>) {
        let mut reg = registry.write();

        // Initialize Ollama if configured
        if let Some(ollama_config) = config.llm.providers.get("ollama") {
            if ollama_config.enabled {
                let model = ollama_config
                    .default_model
                    .as_deref()
                    .unwrap_or("gemma3:4b");
                let provider = Arc::new(crate::llm::OllamaProvider::with_model(model));
                reg.register(provider);
            }
        }

        // Initialize Claude if configured
        if let Some(claude_config) = config.llm.providers.get("claude") {
            if claude_config.enabled {
                if let Some(api_key) = &claude_config.api_key {
                    let model = claude_config
                        .default_model
                        .as_deref()
                        .unwrap_or("claude-sonnet-4-20250514");
                    let provider = Arc::new(crate::llm::ClaudeProvider::new(api_key, model));
                    reg.register(provider);
                }
            }
        }

        // Initialize Gemini if configured
        if let Some(gemini_config) = config.llm.providers.get("gemini") {
            if gemini_config.enabled {
                if let Some(api_key) = &gemini_config.api_key {
                    let model = gemini_config
                        .default_model
                        .as_deref()
                        .unwrap_or("gemini-2.0-flash");
                    let provider = Arc::new(crate::llm::GeminiProvider::new(api_key, model));
                    reg.register(provider);
                }
            }
        }
    }

    /// Send a command to the backend
    pub fn send(&mut self, command: Command) -> Result<()> {
        match command {
            Command::ProcessInput { text } => {
                self.process_input(text)?;
            }
            Command::ExecuteShell { command } => {
                self.execute_shell(command)?;
            }
            Command::InvokeCliAgent { agent_id, prompt } => {
                self.invoke_cli_agent(&agent_id, &prompt)?;
            }
            Command::SendPtyInput { agent_id, data } => {
                self.send_pty_input(agent_id, &data)?;
            }
            Command::ResizePty { agent_id, cols, rows } => {
                self.resize_pty(agent_id, cols, rows)?;
            }
            Command::ReadFile { path } => {
                self.read_file(path)?;
            }
            Command::WriteFile { path, content } => {
                self.write_file(path, content)?;
            }
            Command::SetLlmModel { provider_id, model } => {
                self.set_llm_model(&provider_id, &model)?;
            }
            Command::CancelAgent { agent_id } => {
                self.cancel_agent(agent_id)?;
            }
            Command::SelectContext { context } => {
                self.switch_context(context)?;
            }
            Command::ListProviders => {
                // Send providers via notification
                let providers = self.llm_providers();
                for p in providers {
                    let _ = self.notification_tx.send(Notification::Info {
                        message: format!("Provider: {} ({})", p.name, p.id),
                    });
                }
            }
            Command::ListCliAgents => {
                // Send CLI agents via notification
                let agents = self.cli_agents();
                for a in agents {
                    let _ = self.notification_tx.send(Notification::Info {
                        message: format!("CLI Agent: {} ({})", a.name, a.id),
                    });
                }
            }
            Command::GetSnapshot => {
                // Could send a full state snapshot as notification
            }
            Command::Shutdown => {
                self.shutdown()?;
            }

            // Workspace commands - these are handled by WorkspaceManager at a higher level
            // When AxiomService is used standalone (without WorkspaceManager), these are no-ops
            // that just emit info notifications
            Command::ListWorkspaces => {
                let _ = self.notification_tx.send(Notification::Info {
                    message: "ListWorkspaces: Use WorkspaceManager for workspace operations".into(),
                });
            }
            Command::CreateWorkspace { name, path } => {
                let _ = self.notification_tx.send(Notification::Info {
                    message: format!("CreateWorkspace: {} at {}", name, path.display()),
                });
            }
            Command::DeleteWorkspace { workspace_id } => {
                let _ = self.notification_tx.send(Notification::Info {
                    message: format!("DeleteWorkspace: {}", workspace_id),
                });
            }
            Command::ActivateWorkspace { workspace_id } => {
                let _ = self.notification_tx.send(Notification::Info {
                    message: format!("ActivateWorkspace: {}", workspace_id),
                });
            }
            Command::DeactivateWorkspace => {
                let _ = self.notification_tx.send(Notification::Info {
                    message: "DeactivateWorkspace".into(),
                });
            }
            Command::GetWorkspace { workspace_id } => {
                let _ = self.notification_tx.send(Notification::Info {
                    message: format!("GetWorkspace: {}", workspace_id),
                });
            }
            Command::UpdateWorkspace { workspace_id, name } => {
                let _ = self.notification_tx.send(Notification::Info {
                    message: format!("UpdateWorkspace: {} name={:?}", workspace_id, name),
                });
            }
            Command::ListFiles { path, include_hidden } => {
                self.list_files(path, include_hidden)?;
            }
        }
        Ok(())
    }

    /// List files in a directory
    fn list_files(&mut self, path: PathBuf, include_hidden: bool) -> Result<()> {
        use crate::notifications::FileEntry;

        let full_path = if path.is_absolute() {
            path.clone()
        } else {
            self.cwd.join(&path)
        };

        let mut entries = Vec::new();

        if let Ok(read_dir) = std::fs::read_dir(&full_path) {
            for entry in read_dir.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files if not requested
                if !include_hidden && file_name.starts_with('.') {
                    continue;
                }

                let metadata = entry.metadata().ok();
                let is_directory = metadata.as_ref().map_or(false, |m| m.is_dir());
                let size = metadata.as_ref().map_or(0, |m| m.len());
                let modified = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs() as i64);

                entries.push(FileEntry {
                    name: file_name.clone(),
                    path: entry.path(),
                    is_directory,
                    size,
                    modified,
                    is_hidden: file_name.starts_with('.'),
                });
            }
        }

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        let _ = self.notification_tx.send(Notification::FileList { path, entries });
        Ok(())
    }

    /// Process internal events and emit notifications
    ///
    /// Call this periodically (e.g., in your UI event loop) to process
    /// backend events and convert them to notifications.
    pub fn process_events(&mut self) -> Result<()> {
        // Drain all pending events
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event)?;
        }
        Ok(())
    }

    /// Process events with a timeout
    ///
    /// Blocks for up to `timeout` waiting for an event, then processes all pending events.
    pub fn process_events_timeout(&mut self, timeout: Duration) -> Result<()> {
        // Wait for first event with timeout
        if let Ok(event) = self.event_rx.recv_timeout(timeout) {
            self.handle_event(event)?;
        }

        // Then drain all pending events
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event)?;
        }

        Ok(())
    }

    /// Poll for a notification (non-blocking)
    pub fn poll_notification(&self) -> Option<Notification> {
        self.notification_rx.try_recv().ok()
    }

    /// Get the notification receiver for external polling
    pub fn notifications(&self) -> &Receiver<Notification> {
        &self.notification_rx
    }

    /// Get a snapshot of all agents
    pub fn agents(&self) -> Vec<AgentView> {
        let registry = self.agent_registry.read();
        registry
            .agents()
            .map(|agent| AgentView {
                id: agent.id,
                name: agent.name.clone(),
                agent_type: agent.agent_type.clone(),
                status: agent.status.clone(),
                line_count: agent.line_count,
                elapsed_secs: agent.elapsed().as_secs_f64(),
                parent_id: agent.parent_id,
                token_count: agent.token_count,
            })
            .collect()
    }

    /// Get a specific agent's view
    pub fn agent(&self, id: AgentId) -> Option<AgentView> {
        let registry = self.agent_registry.read();
        registry.get(id).map(|agent| AgentView {
            id: agent.id,
            name: agent.name.clone(),
            agent_type: agent.agent_type.clone(),
            status: agent.status.clone(),
            line_count: agent.line_count,
            elapsed_secs: agent.elapsed().as_secs_f64(),
            parent_id: agent.parent_id,
            token_count: agent.token_count,
        })
    }

    /// Get agent output text
    pub fn agent_output(&self, id: AgentId) -> Option<String> {
        let registry = self.agent_registry.read();
        registry.get(id).map(|agent| agent.output.clone())
    }

    /// Get PTY screen for a CLI agent
    pub fn pty_screen(&self, id: AgentId) -> Option<TerminalScreen> {
        let manager = self.pty_manager.read();
        manager.get_screen(id)
    }

    /// Get current output context
    pub fn output_context(&self) -> OutputContext {
        self.output_context.read().clone()
    }

    /// Get available LLM providers
    pub fn llm_providers(&self) -> Vec<crate::llm::ProviderInfo> {
        let registry = self.llm_registry.read();
        registry.provider_info()
    }

    /// Get active LLM provider ID
    pub fn active_llm_provider(&self) -> Option<String> {
        let registry = self.llm_registry.read();
        let id = registry.active_id();
        if id.is_empty() {
            None
        } else {
            Some(id)
        }
    }

    /// Get available CLI agents from config
    pub fn cli_agents(&self) -> Vec<CliAgentInfo> {
        self.config
            .cli_agents
            .agents
            .iter()
            .filter(|(_, cfg)| cfg.enabled)
            .map(|(id, cfg)| CliAgentInfo {
                id: id.clone(),
                name: cfg.name.clone(),
                icon: cfg.icon.clone(),
                enabled: cfg.enabled,
            })
            .collect()
    }

    /// Get current working directory
    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }

    /// Set current working directory
    pub fn set_cwd(&mut self, cwd: PathBuf) {
        self.cwd = cwd;
    }

    /// Get configuration
    pub fn config(&self) -> &AxiomConfig {
        &self.config
    }

    // ========== Internal command handlers ==========

    fn process_input(&mut self, text: String) -> Result<()> {
        // Check for CLI agent invocation (#agent syntax)
        if let Some(rest) = text.strip_prefix('#') {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if !parts.is_empty() {
                let agent_id = parts[0];
                let prompt = parts.get(1).unwrap_or(&"").to_string();
                return self.invoke_cli_agent(agent_id, &prompt);
            }
        }

        // Regular input - send to conductor
        let mut conductor = self.conductor.write();
        conductor.process(text);
        Ok(())
    }

    fn execute_shell(&mut self, command: String) -> Result<()> {
        let request = AgentSpawnRequest {
            agent_type: AgentType::Shell,
            name: "Shell".to_string(),
            description: command.clone(),
            parameters: Some(command),
            parent_id: None,
        };

        let _ = self.event_tx.send(Event::AgentSpawn(request));
        Ok(())
    }

    fn invoke_cli_agent(&mut self, agent_id: &str, prompt: &str) -> Result<()> {
        // Get CLI agent config
        let config = self
            .config
            .cli_agents
            .agents
            .get(agent_id)
            .ok_or_else(|| AxiomError::not_found(format!("CLI agent '{}' not found", agent_id)))?
            .clone();

        if !config.enabled {
            return Err(AxiomError::invalid_operation(format!(
                "CLI agent '{}' is disabled",
                agent_id
            )));
        }

        // Spawn the agent in registry
        let request = AgentSpawnRequest {
            agent_type: AgentType::CliAgent {
                config_id: agent_id.to_string(),
            },
            name: config.name.clone(),
            description: prompt.to_string(),
            parameters: Some(prompt.to_string()),
            parent_id: None,
        };

        let id = {
            let mut registry = self.agent_registry.write();
            let id = registry.spawn(request);
            registry.start(id);
            id
        };

        // Start PTY session
        {
            let mut manager = self.pty_manager.write();
            manager.start(id, &config, prompt, &self.cwd)?;
        }

        // Switch context to the new agent
        *self.output_context.write() = OutputContext::Agent { agent_id: id };

        // Emit notification
        let _ = self.notification_tx.send(Notification::AgentSpawned {
            id,
            name: config.name,
            agent_type: AgentType::CliAgent {
                config_id: agent_id.to_string(),
            },
            parent_id: None,
        });

        Ok(())
    }

    fn send_pty_input(&mut self, agent_id: AgentId, data: &[u8]) -> Result<()> {
        let mut manager = self.pty_manager.write();
        manager.write(agent_id, data)?;
        Ok(())
    }

    fn resize_pty(&mut self, agent_id: AgentId, cols: u16, rows: u16) -> Result<()> {
        let mut manager = self.pty_manager.write();
        manager.resize(agent_id, cols, rows)?;
        Ok(())
    }

    fn read_file(&mut self, path: PathBuf) -> Result<()> {
        let content = std::fs::read_to_string(&path)?;
        let _ = self.notification_tx.send(Notification::FileLoaded {
            path: path.clone(),
            content,
        });

        // Switch context to file
        *self.output_context.write() = OutputContext::File { path };

        Ok(())
    }

    fn write_file(&mut self, path: PathBuf, content: String) -> Result<()> {
        std::fs::write(&path, &content)?;
        let _ = self.notification_tx.send(Notification::FileModified { path });
        Ok(())
    }

    fn set_llm_model(&mut self, provider_id: &str, model: &str) -> Result<()> {
        let registry = self.llm_registry.read();
        registry
            .set_model(provider_id, model)
            .map_err(|e| AxiomError::llm(e.to_string()))?;
        Ok(())
    }

    fn cancel_agent(&mut self, agent_id: AgentId) -> Result<()> {
        // Cancel in registry
        {
            let mut registry = self.agent_registry.write();
            if let Some(agent) = registry.get_mut(agent_id) {
                agent.cancel();
            }
        }

        // Remove PTY if it's a CLI agent
        {
            let mut manager = self.pty_manager.write();
            if manager.contains(agent_id) {
                manager.remove(agent_id);
            }
        }

        let _ = self.notification_tx.send(Notification::AgentStatusChanged {
            id: agent_id,
            status: AgentStatus::Cancelled,
        });

        Ok(())
    }

    fn switch_context(&mut self, context: OutputContext) -> Result<()> {
        *self.output_context.write() = context;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Clean up PTY sessions
        {
            let mut manager = self.pty_manager.write();
            for id in manager.active_ids() {
                manager.remove(id);
            }
        }

        Ok(())
    }

    // ========== Internal event handlers ==========

    fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::AgentSpawn(request) => {
                self.handle_agent_spawn(request)?;
            }
            Event::AgentUpdate { id, status } => {
                self.handle_agent_update(id, status)?;
            }
            Event::AgentOutput { id, chunk } => {
                self.handle_agent_output(id, chunk)?;
            }
            Event::AgentComplete { id } => {
                self.handle_agent_complete(id)?;
            }
            Event::AgentWake(id) => {
                // Wake agent - just update status
                let mut registry = self.agent_registry.write();
                if let Some(agent) = registry.get_mut(id) {
                    agent.status = AgentStatus::Running;
                }
            }
            Event::CliAgentOutput { id, data } => {
                // PTY output - emit notification
                let _ = self.notification_tx.send(Notification::PtyOutput { id, data });
            }
            Event::CliAgentExit { id, exit_code } => {
                self.handle_cli_agent_exit(id, exit_code)?;
            }
            Event::CliAgentInput { id, data } => {
                // Forward input to PTY
                let mut manager = self.pty_manager.write();
                let _ = manager.write(id, &data);
            }
            Event::ConductorResponse(response) => {
                // Add response to conductor history
                let mut conductor = self.conductor.write();
                conductor.add_response(response);
            }
            Event::LlmChunk(chunk) => {
                // LLM chunks are handled by conductor internally
                let _ = self.notification_tx.send(Notification::Info {
                    message: chunk,
                });
            }
            Event::LlmDone => {
                // LLM done - nothing to do here
            }
            Event::LlmError(error) => {
                let _ = self.notification_tx.send(Notification::Error { message: error });
            }
            Event::FileModification { path, content: _ } => {
                let _ = self.notification_tx.send(Notification::FileModified {
                    path: PathBuf::from(path),
                });
            }
            Event::SwitchContext(context) => {
                *self.output_context.write() = context;
            }
            Event::ShellExecute(command) => {
                self.execute_shell(command)?;
            }
            Event::FileChanged(path) => {
                let _ = self.notification_tx.send(Notification::FileModified { path });
            }
            Event::Tick => {
                // Periodic tick - could be used for cleanup, etc.
            }
            Event::Quit => {
                self.shutdown()?;
            }
            Event::CliAgentInvoke { agent_id, prompt } => {
                self.invoke_cli_agent(&agent_id, &prompt)?;
            }
            Event::ConductorRequest(input) => {
                self.process_input(input)?;
            }
        }
        Ok(())
    }

    fn handle_agent_spawn(&mut self, request: AgentSpawnRequest) -> Result<()> {
        let id = {
            let mut registry = self.agent_registry.write();
            registry.spawn(request.clone())
        };

        // Emit notification
        let _ = self.notification_tx.send(Notification::AgentSpawned {
            id,
            name: request.name.clone(),
            agent_type: request.agent_type.clone(),
            parent_id: request.parent_id,
        });

        // If this is a conductor agent, set it up
        if request.agent_type == AgentType::Conductor {
            let mut conductor = self.conductor.write();
            conductor.set_agent_id(id);

            // Execute with the initial task
            if let Some(params) = &request.parameters {
                conductor.execute(id, params);
            }
        } else if !matches!(request.agent_type, AgentType::CliAgent { .. }) {
            // Execute non-CLI agents
            let executor = self.executor.read();
            executor.execute(id, &request);
        }

        // Switch context to new agent
        *self.output_context.write() = OutputContext::Agent { agent_id: id };

        Ok(())
    }

    fn handle_agent_update(&mut self, id: AgentId, status: AgentStatus) -> Result<()> {
        {
            let mut registry = self.agent_registry.write();
            if let Some(agent) = registry.get_mut(id) {
                agent.status = status.clone();
            }
        }

        let _ = self
            .notification_tx
            .send(Notification::AgentStatusChanged { id, status });

        Ok(())
    }

    fn handle_agent_output(&mut self, id: AgentId, chunk: String) -> Result<()> {
        {
            let mut registry = self.agent_registry.write();
            registry.append_output(id, &chunk);
        }

        let _ = self
            .notification_tx
            .send(Notification::AgentOutput { id, chunk });

        Ok(())
    }

    fn handle_agent_complete(&mut self, id: AgentId) -> Result<()> {
        {
            let mut registry = self.agent_registry.write();
            registry.complete(id);
        }

        let _ = self.notification_tx.send(Notification::AgentStatusChanged {
            id,
            status: AgentStatus::Completed,
        });

        Ok(())
    }

    fn handle_cli_agent_exit(&mut self, id: AgentId, exit_code: i32) -> Result<()> {
        // Mark as exited in PTY manager
        {
            let mut manager = self.pty_manager.write();
            manager.mark_exited(id);
        }

        // Update agent status
        {
            let mut registry = self.agent_registry.write();
            if let Some(agent) = registry.get_mut(id) {
                if exit_code == 0 {
                    agent.complete();
                } else {
                    agent.error(format!("Exited with code {}", exit_code));
                }
            }
        }

        let _ = self.notification_tx.send(Notification::PtyExited { id, exit_code });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let config = AxiomConfig::default();
        let cwd = std::env::current_dir().unwrap();
        let service = AxiomService::new(config, cwd);
        assert!(service.is_ok());
    }

    #[test]
    fn test_agents_empty() {
        let config = AxiomConfig::default();
        let cwd = std::env::current_dir().unwrap();
        let service = AxiomService::new(config, cwd).unwrap();
        assert!(service.agents().is_empty());
    }

    #[test]
    fn test_cli_agents_from_config() {
        let config = AxiomConfig::default();
        let cwd = std::env::current_dir().unwrap();
        let service = AxiomService::new(config, cwd).unwrap();
        let agents = service.cli_agents();
        // Default config has claude and gemini enabled
        assert!(!agents.is_empty());
    }
}
