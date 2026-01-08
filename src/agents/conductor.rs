//! Conductor service for routing user input to agents
//!
//! The conductor receives user prompts and decides what agents to spawn.
//! It uses the active LLM provider to analyze requests and plan agent execution.

use crate::agents::{AgentSpawnRequest, AgentStatus, AgentType};
use crate::events::Event;
use crate::llm::{ChatMessage, MessageContent, ProviderRegistry, Role};
use crate::state::AgentId;
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use std::sync::Arc;

/// The conductor service
///
/// Receives user prompts and spawns appropriate agents to handle them.
pub struct Conductor {
    /// LLM provider registry
    llm_registry: Arc<RwLock<ProviderRegistry>>,

    /// Event sender for emitting events
    event_tx: Sender<Event>,

    /// Conversation history for context
    history: Vec<ChatMessage>,

    /// Maximum history length to retain
    max_history: usize,

    /// Persistent conductor agent ID (reused across inputs)
    agent_id: Option<AgentId>,
}

impl Conductor {
    /// Create a new conductor
    pub fn new(llm_registry: Arc<RwLock<ProviderRegistry>>, event_tx: Sender<Event>) -> Self {
        Self {
            llm_registry,
            event_tx,
            history: Vec::new(),
            max_history: 20,
            agent_id: None,
        }
    }

    /// Set the persistent conductor agent ID
    pub fn set_agent_id(&mut self, id: AgentId) {
        self.agent_id = Some(id);
    }

    /// Get the persistent conductor agent ID
    pub fn agent_id(&self) -> Option<AgentId> {
        self.agent_id
    }

    /// Process a user request
    ///
    /// Analyzes the request and spawns appropriate agents.
    /// Reuses the existing conductor agent if available.
    pub fn process(&mut self, input: String) {
        // Add user message to history
        self.history.push(ChatMessage {
            role: Role::User,
            content: MessageContent::Text(input.clone()),
        });

        // Trim history if needed
        while self.history.len() > self.max_history {
            self.history.remove(0);
        }

        // Check if we have a persistent conductor agent
        if let Some(id) = self.agent_id {
            // Wake existing conductor agent
            let _ = self.event_tx.send(Event::AgentWake(id));
            // Execute with the new input
            self.execute(id, &input);
        } else {
            // First time - spawn new conductor agent
            let request = AgentSpawnRequest {
                agent_type: AgentType::Conductor,
                name: "Conductor".to_string(),
                description: "AI Assistant".to_string(),
                parameters: Some(input.clone()),
                parent_id: None, // Conductor is the root agent
            };

            let _ = self.event_tx.send(Event::AgentSpawn(request));
        }
    }

    /// Execute the conductor agent's logic
    ///
    /// Called by the executor when the conductor agent starts running.
    pub fn execute(&self, agent_id: AgentId, task: &str) {
        // Set agent status to Running
        let _ = self.event_tx.send(Event::AgentUpdate {
            id: agent_id,
            status: AgentStatus::Running,
        });

        // Output user's question first (chat interface style, right-aligned box)
        let _ = self.event_tx.send(Event::AgentOutput {
            id: agent_id,
            chunk: format!(">>>user\n{}\n<<<\n\n", task),
        });

        let event_tx = self.event_tx.clone();
        let llm_registry = self.llm_registry.clone();
        let history = self.history.clone();
        let task = task.to_string();

        // Run in background thread to not block UI
        std::thread::spawn(move || {
            execute_conductor(agent_id, &task, history, llm_registry, event_tx);
        });
    }

    /// Get the conversation history
    pub fn history(&self) -> &[ChatMessage] {
        &self.history
    }

    /// Clear conversation history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Add assistant response to history
    pub fn add_response(&mut self, response: String) {
        self.history.push(ChatMessage {
            role: Role::Assistant,
            content: MessageContent::Text(response),
        });

        while self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }
}

/// Execute the conductor agent
fn execute_conductor(
    agent_id: AgentId,
    task: &str,
    history: Vec<ChatMessage>,
    llm_registry: Arc<RwLock<ProviderRegistry>>,
    event_tx: Sender<Event>,
) {
    // Build the system prompt for the conductor
    let system_prompt = build_system_prompt();

    // Build messages for the LLM
    let mut messages = vec![ChatMessage {
        role: Role::System,
        content: MessageContent::Text(system_prompt),
    }];

    // Add conversation history
    messages.extend(history);

    // Get the active provider
    let registry = llm_registry.read();
    let provider = match registry.active() {
        Some(p) => p.clone(),
        None => {
            let _ = event_tx.send(Event::AgentOutput {
                id: agent_id,
                chunk: "Error: No LLM provider available".to_string(),
            });
            // Set to Idle so conductor can be reused
            let _ = event_tx.send(Event::AgentUpdate {
                id: agent_id,
                status: AgentStatus::Idle,
            });
            return;
        }
    };
    drop(registry);

    // Create a channel for LLM responses
    let (llm_tx, llm_rx) = crossbeam_channel::unbounded();

    // Send to LLM
    provider.send_message(messages, llm_tx);

    // Output Axiom prefix (chat interface style)
    let _ = event_tx.send(Event::AgentOutput {
        id: agent_id,
        chunk: "**Axiom:** ".to_string(),
    });

    // Stream responses to agent output
    let mut full_response = String::new();
    loop {
        match llm_rx.recv() {
            Ok(Event::LlmChunk(chunk)) => {
                full_response.push_str(&chunk);
                let _ = event_tx.send(Event::AgentOutput {
                    id: agent_id,
                    chunk,
                });
            }
            Ok(Event::LlmDone) => {
                // Parse response for agent spawn commands (pass conductor_id as parent)
                parse_and_spawn_agents(&full_response, &event_tx, agent_id);
                // Set to Idle so conductor can be reused for next input
                let _ = event_tx.send(Event::AgentUpdate {
                    id: agent_id,
                    status: AgentStatus::Idle,
                });
                break;
            }
            Ok(Event::LlmError(e)) => {
                let _ = event_tx.send(Event::AgentOutput {
                    id: agent_id,
                    chunk: format!("\nError: {}", e),
                });
                // Set to Idle so conductor can be reused
                let _ = event_tx.send(Event::AgentUpdate {
                    id: agent_id,
                    status: AgentStatus::Idle,
                });
                break;
            }
            Err(_) => {
                // Set to Idle so conductor can be reused
                let _ = event_tx.send(Event::AgentUpdate {
                    id: agent_id,
                    status: AgentStatus::Idle,
                });
                break;
            }
            _ => {}
        }
    }
}

/// Build the system prompt for the conductor
fn build_system_prompt() -> String {
    r#"You are an AI assistant integrated into Axiom, a terminal-based IDE. You help users with code, shell commands, and file operations.

When you need to perform actions, you can spawn specialized agents:

1. **Shell Agent**: Execute shell commands
   Format: `@shell <command>`
   Example: `@shell ls -la`

2. **Coder Agent**: Modify or create code files
   Format: `@coder <description>` followed by code blocks
   Example:
   @coder Update the main function
   ```rust:src/main.rs
   fn main() {
       println!("Hello!");
   }
   ```

3. **Search Agent**: Search files or content
   Format: `@search <query>`
   Example: `@search TODO`

4. **FileOps Agent**: Read, write, or manage files
   Format: `@fileops <operation> <path>`
   Example: `@fileops read src/main.rs`

You can spawn multiple agents in a single response. Always explain what you're doing before spawning agents.

If the user's request doesn't require any agent actions, just respond conversationally."#.to_string()
}

/// Parse LLM response and spawn any requested agents
fn parse_and_spawn_agents(response: &str, event_tx: &Sender<Event>, parent_id: AgentId) {
    let mut lines = response.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Check for shell agent
        if let Some(cmd) = trimmed.strip_prefix("@shell ") {
            let request = AgentSpawnRequest {
                agent_type: AgentType::Shell,
                name: "Shell".to_string(),
                description: truncate(cmd, 50),
                parameters: Some(cmd.to_string()),
                parent_id: Some(parent_id),
            };
            let _ = event_tx.send(Event::AgentSpawn(request));
        }

        // Check for search agent
        if let Some(query) = trimmed.strip_prefix("@search ") {
            let request = AgentSpawnRequest {
                agent_type: AgentType::Search,
                name: "Search".to_string(),
                description: format!("Searching: {}", truncate(query, 40)),
                parameters: Some(query.to_string()),
                parent_id: Some(parent_id),
            };
            let _ = event_tx.send(Event::AgentSpawn(request));
        }

        // Check for fileops agent
        if let Some(op) = trimmed.strip_prefix("@fileops ") {
            let request = AgentSpawnRequest {
                agent_type: AgentType::FileOps,
                name: "FileOps".to_string(),
                description: truncate(op, 50),
                parameters: Some(op.to_string()),
                parent_id: Some(parent_id),
            };
            let _ = event_tx.send(Event::AgentSpawn(request));
        }

        // Check for coder agent (may span multiple lines with code blocks)
        if let Some(desc) = trimmed.strip_prefix("@coder ") {
            let mut code_content = String::new();
            let mut in_code_block = false;
            let mut current_path = String::new();

            // Collect following code blocks
            while let Some(&next_line) = lines.peek() {
                if next_line.trim().starts_with("```") && !in_code_block {
                    in_code_block = true;
                    // Extract path from code block header
                    let header = next_line.trim().trim_start_matches('`');
                    if let Some(path_part) = header.split(':').nth(1) {
                        current_path = path_part.to_string();
                    } else if header.contains('/') {
                        current_path = header.to_string();
                    }
                    lines.next();
                } else if next_line.trim() == "```" && in_code_block {
                    in_code_block = false;
                    lines.next();
                    break;
                } else if in_code_block {
                    code_content.push_str(next_line);
                    code_content.push('\n');
                    lines.next();
                } else if next_line.trim().starts_with('@') {
                    break;
                } else {
                    lines.next();
                }
            }

            let params = if !current_path.is_empty() {
                format!("{}|{}", current_path, code_content)
            } else {
                desc.to_string()
            };

            let request = AgentSpawnRequest {
                agent_type: AgentType::Coder,
                name: "Coder".to_string(),
                description: truncate(desc, 50),
                parameters: Some(params),
                parent_id: Some(parent_id),
            };
            let _ = event_tx.send(Event::AgentSpawn(request));
        }
    }
}

/// Truncate a string to the specified length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_parse_shell_command() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let response = "@shell ls -la";
        let parent_id = AgentId::new(1);
        parse_and_spawn_agents(response, &tx, parent_id);

        if let Ok(Event::AgentSpawn(req)) = rx.try_recv() {
            assert_eq!(req.agent_type, AgentType::Shell);
            assert_eq!(req.parameters, Some("ls -la".to_string()));
            assert_eq!(req.parent_id, Some(parent_id));
        } else {
            panic!("Expected AgentSpawn event");
        }
    }

    #[test]
    fn test_parse_multiple_agents() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let response = r#"Let me help you with that.

@shell git status
@search TODO comments"#;
        let parent_id = AgentId::new(1);
        parse_and_spawn_agents(response, &tx, parent_id);

        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 2);
    }
}
