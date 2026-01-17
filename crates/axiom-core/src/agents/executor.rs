//! Agent executor for running spawned agents
//!
//! Handles the actual execution of different agent types (Shell, Coder, Search, FileOps).

use super::AgentRegistry;
use crate::events::Event;
use crate::types::{AgentId, AgentSpawnRequest, AgentStatus, AgentType};
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

/// Agent executor
///
/// Executes spawned agents based on their type.
pub struct Executor {
    /// Event sender
    event_tx: Sender<Event>,

    /// Agent registry for updating agent state
    agent_registry: Arc<RwLock<AgentRegistry>>,

    /// Current working directory
    cwd: PathBuf,
}

impl Executor {
    /// Create a new executor
    pub fn new(
        event_tx: Sender<Event>,
        agent_registry: Arc<RwLock<AgentRegistry>>,
        cwd: PathBuf,
    ) -> Self {
        Self {
            event_tx,
            agent_registry,
            cwd,
        }
    }

    /// Execute an agent based on its type
    pub fn execute(&self, agent_id: AgentId, request: &AgentSpawnRequest) {
        let event_tx = self.event_tx.clone();
        let agent_registry = self.agent_registry.clone();
        let cwd = self.cwd.clone();
        let request = request.clone();

        // Mark agent as running
        {
            let mut registry = agent_registry.write();
            registry.start(agent_id);
        }

        // Update status
        let _ = event_tx.send(Event::AgentUpdate {
            id: agent_id,
            status: AgentStatus::Running,
        });

        // Execute based on type
        std::thread::spawn(move || {
            let result = match request.agent_type {
                AgentType::Shell => execute_shell(agent_id, &request, &cwd, &event_tx),
                AgentType::Coder => execute_coder(agent_id, &request, &cwd, &event_tx),
                AgentType::Search => execute_search(agent_id, &request, &cwd, &event_tx),
                AgentType::FileOps => execute_fileops(agent_id, &request, &cwd, &event_tx),
                AgentType::Conductor => {
                    // Conductor is handled by the Conductor service
                    Ok(())
                }
                AgentType::Custom { ref name } => {
                    let _ = event_tx.send(Event::AgentOutput {
                        id: agent_id,
                        chunk: format!("Custom agent '{}' not implemented", name),
                    });
                    Ok(())
                }
                AgentType::CliAgent { .. } => {
                    // CLI agents are handled by PtyAgentManager, not this executor
                    Ok(())
                }
            };

            // Update agent status based on result
            let mut registry = agent_registry.write();
            match result {
                Ok(()) => registry.complete(agent_id),
                Err(e) => registry.error(agent_id, e),
            }

            let _ = event_tx.send(Event::AgentComplete { id: agent_id });
        });
    }
}

/// Execute a shell command
fn execute_shell(
    agent_id: AgentId,
    request: &AgentSpawnRequest,
    cwd: &PathBuf,
    event_tx: &Sender<Event>,
) -> Result<(), String> {
    let cmd = request.parameters.as_deref().unwrap_or("");
    if cmd.is_empty() {
        return Err("No command provided".to_string());
    }

    let _ = event_tx.send(Event::AgentOutput {
        id: agent_id,
        chunk: format!("$ {}\n", cmd),
    });

    // Execute the command
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    match output {
        Ok(mut child) => {
            // Stream stdout
            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = event_tx.send(Event::AgentOutput {
                            id: agent_id,
                            chunk: format!("{}\n", line),
                        });
                    }
                }
            }

            // Collect stderr
            if let Some(stderr) = child.stderr.take() {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = event_tx.send(Event::AgentOutput {
                            id: agent_id,
                            chunk: format!("{}\n", line),
                        });
                    }
                }
            }

            // Wait for completion
            match child.wait() {
                Ok(status) => {
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!("Command exited with status: {}", status))
                    }
                }
                Err(e) => Err(format!("Failed to wait for command: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to execute command: {}", e)),
    }
}

/// Execute a coder agent (file modification)
fn execute_coder(
    agent_id: AgentId,
    request: &AgentSpawnRequest,
    cwd: &PathBuf,
    event_tx: &Sender<Event>,
) -> Result<(), String> {
    let params = request.parameters.as_deref().unwrap_or("");

    // Parse path|content format
    if let Some((path, content)) = params.split_once('|') {
        let file_path = if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            cwd.join(path)
        };

        let _ = event_tx.send(Event::AgentOutput {
            id: agent_id,
            chunk: format!("Writing to: {}\n", file_path.display()),
        });

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }
        }

        // Write the file
        std::fs::write(&file_path, content)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        let _ = event_tx.send(Event::AgentOutput {
            id: agent_id,
            chunk: format!("File written successfully ({} bytes)\n", content.len()),
        });

        // Emit file modification event for output panel to update
        let _ = event_tx.send(Event::FileModification {
            path: file_path.to_string_lossy().to_string(),
            content: content.to_string(),
        });

        Ok(())
    } else {
        let _ = event_tx.send(Event::AgentOutput {
            id: agent_id,
            chunk: format!("Coder: {}\n", params),
        });
        Ok(())
    }
}

/// Execute a search agent
fn execute_search(
    agent_id: AgentId,
    request: &AgentSpawnRequest,
    cwd: &PathBuf,
    event_tx: &Sender<Event>,
) -> Result<(), String> {
    let query = request.parameters.as_deref().unwrap_or("");
    if query.is_empty() {
        return Err("No search query provided".to_string());
    }

    let _ = event_tx.send(Event::AgentOutput {
        id: agent_id,
        chunk: format!("Searching for: {}\n\n", query),
    });

    // Use ripgrep if available, otherwise grep
    let output = Command::new("rg")
        .args(["--line-number", "--with-filename", query])
        .current_dir(cwd)
        .output()
        .or_else(|_| {
            Command::new("grep")
                .args(["-rn", query, "."])
                .current_dir(cwd)
                .output()
        });

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !stdout.is_empty() {
                // Limit output to first 50 matches
                let lines: Vec<&str> = stdout.lines().take(50).collect();
                let total_matches = stdout.lines().count();

                for line in &lines {
                    let _ = event_tx.send(Event::AgentOutput {
                        id: agent_id,
                        chunk: format!("{}\n", line),
                    });
                }

                if total_matches > 50 {
                    let _ = event_tx.send(Event::AgentOutput {
                        id: agent_id,
                        chunk: format!("\n... and {} more matches\n", total_matches - 50),
                    });
                }

                let _ = event_tx.send(Event::AgentOutput {
                    id: agent_id,
                    chunk: format!("\nFound {} matches\n", total_matches),
                });
            } else if !stderr.is_empty() {
                let _ = event_tx.send(Event::AgentOutput {
                    id: agent_id,
                    chunk: stderr.to_string(),
                });
            } else {
                let _ = event_tx.send(Event::AgentOutput {
                    id: agent_id,
                    chunk: "No matches found\n".to_string(),
                });
            }

            Ok(())
        }
        Err(e) => Err(format!("Search failed: {}", e)),
    }
}

/// Execute a file operations agent
fn execute_fileops(
    agent_id: AgentId,
    request: &AgentSpawnRequest,
    cwd: &PathBuf,
    event_tx: &Sender<Event>,
) -> Result<(), String> {
    let params = request.parameters.as_deref().unwrap_or("");
    let parts: Vec<&str> = params.splitn(2, ' ').collect();

    if parts.is_empty() {
        return Err("No operation specified".to_string());
    }

    let operation = parts[0];
    let path = parts.get(1).unwrap_or(&"");

    match operation {
        "read" => {
            let file_path = if path.starts_with('/') {
                PathBuf::from(path)
            } else {
                cwd.join(path)
            };

            let _ = event_tx.send(Event::AgentOutput {
                id: agent_id,
                chunk: format!("Reading: {}\n\n", file_path.display()),
            });

            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    // Limit output to first 100 lines
                    let lines: Vec<&str> = content.lines().take(100).collect();
                    let total_lines = content.lines().count();

                    for (i, line) in lines.iter().enumerate() {
                        let _ = event_tx.send(Event::AgentOutput {
                            id: agent_id,
                            chunk: format!("{:4} | {}\n", i + 1, line),
                        });
                    }

                    if total_lines > 100 {
                        let _ = event_tx.send(Event::AgentOutput {
                            id: agent_id,
                            chunk: format!("\n... {} more lines\n", total_lines - 100),
                        });
                    }

                    Ok(())
                }
                Err(e) => Err(format!("Failed to read file: {}", e)),
            }
        }
        "list" | "ls" => {
            let dir_path = if path.is_empty() {
                cwd.clone()
            } else if path.starts_with('/') {
                PathBuf::from(path)
            } else {
                cwd.join(path)
            };

            let _ = event_tx.send(Event::AgentOutput {
                id: agent_id,
                chunk: format!("Listing: {}\n\n", dir_path.display()),
            });

            match std::fs::read_dir(&dir_path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let file_type = if entry.path().is_dir() { "d" } else { "-" };
                        let _ = event_tx.send(Event::AgentOutput {
                            id: agent_id,
                            chunk: format!(
                                "{} {}\n",
                                file_type,
                                entry.file_name().to_string_lossy()
                            ),
                        });
                    }
                    Ok(())
                }
                Err(e) => Err(format!("Failed to list directory: {}", e)),
            }
        }
        "exists" => {
            let file_path = if path.starts_with('/') {
                PathBuf::from(path)
            } else {
                cwd.join(path)
            };

            let exists = file_path.exists();
            let file_type = if file_path.is_dir() {
                "directory"
            } else if file_path.is_file() {
                "file"
            } else {
                "unknown"
            };

            let _ = event_tx.send(Event::AgentOutput {
                id: agent_id,
                chunk: format!(
                    "{}: {} ({})\n",
                    file_path.display(),
                    if exists { "exists" } else { "not found" },
                    file_type
                ),
            });

            Ok(())
        }
        "delete" | "rm" => {
            let file_path = if path.starts_with('/') {
                PathBuf::from(path)
            } else {
                cwd.join(path)
            };

            let _ = event_tx.send(Event::AgentOutput {
                id: agent_id,
                chunk: format!("Deleting: {}\n", file_path.display()),
            });

            if file_path.is_dir() {
                std::fs::remove_dir_all(&file_path)
                    .map_err(|e| format!("Failed to delete directory: {}", e))?;
            } else {
                std::fs::remove_file(&file_path)
                    .map_err(|e| format!("Failed to delete file: {}", e))?;
            }

            let _ = event_tx.send(Event::AgentOutput {
                id: agent_id,
                chunk: "Deleted successfully\n".to_string(),
            });

            Ok(())
        }
        _ => Err(format!("Unknown operation: {}", operation)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_shell_execution() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let cwd = env::current_dir().unwrap();

        let request = AgentSpawnRequest {
            agent_type: AgentType::Shell,
            name: "Test".to_string(),
            description: "Test".to_string(),
            parameters: Some("echo hello".to_string()),
            parent_id: None,
        };

        let result = execute_shell(AgentId::new(1), &request, &cwd, &tx);
        assert!(result.is_ok());

        // Check output contains "hello"
        let mut found_hello = false;
        while let Ok(event) = rx.try_recv() {
            if let Event::AgentOutput { chunk, .. } = event {
                if chunk.contains("hello") {
                    found_hello = true;
                }
            }
        }
        assert!(found_hello);
    }

    #[test]
    fn test_search_execution() {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let cwd = env::current_dir().unwrap();

        let request = AgentSpawnRequest {
            agent_type: AgentType::Search,
            name: "Search".to_string(),
            description: "Test".to_string(),
            parameters: Some("fn main".to_string()),
            parent_id: None,
        };

        let result = execute_search(AgentId::new(1), &request, &cwd, &tx);
        assert!(result.is_ok());
    }
}
