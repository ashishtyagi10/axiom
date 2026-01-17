//! Developer Agent
//!
//! Writes code, fixes bugs, and executes commands.

use super::types::{AgentOperation, ChatMessage, DeveloperResponse};
use crate::Result;
use std::path::Path;

const DEVELOPER_SYSTEM_PROMPT: &str = r#"
You are the Developer Agent. Your job is to write code, fix bugs, and run commands.

**Capabilities:**
1. **File System**: Write/Overwrite files.
2. **Terminal**: Execute shell commands (e.g., npm install, npm test, ls -la).

**Output Format:**
You must respond with a strict JSON object (no markdown):
{
  "reasoning": "Explanation of your plan",
  "operations": [
    {
      "type": "write",
      "path": "/absolute/path/to/file.ts",
      "content": "file content here"
    },
    {
      "type": "execute",
      "command": "npm install"
    }
  ],
  "message": "Summary for the user"
}

Prioritize writing files before executing commands if they are dependencies.
"#;

/// Parse developer response from LLM output
pub fn parse_developer_response(content: &str) -> Result<DeveloperResponse> {
    // Clean up potential markdown code blocks
    let clean_content = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try to parse as JSON
    match serde_json::from_str::<serde_json::Value>(clean_content) {
        Ok(json) => {
            let reasoning = json
                .get("reasoning")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let message = json
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Task completed")
                .to_string();

            let operations = json
                .get("operations")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|op| parse_operation(op))
                        .collect()
                })
                .unwrap_or_default();

            Ok(DeveloperResponse {
                reasoning,
                operations,
                message,
            })
        }
        Err(_) => {
            // Fallback: treat as a message with no operations
            Ok(DeveloperResponse {
                reasoning: "Failed to parse response".to_string(),
                operations: vec![],
                message: content.to_string(),
            })
        }
    }
}

/// Parse a single operation from JSON
fn parse_operation(op: &serde_json::Value) -> Option<AgentOperation> {
    let op_type = op.get("type").and_then(|v| v.as_str())?;

    match op_type {
        "write" => {
            let path = op.get("path").and_then(|v| v.as_str())?;
            let content = op.get("content").and_then(|v| v.as_str())?;
            Some(AgentOperation::Write {
                path: path.into(),
                content: content.to_string(),
            })
        }
        "delete" => {
            let path = op.get("path").and_then(|v| v.as_str())?;
            Some(AgentOperation::Delete { path: path.into() })
        }
        "execute" => {
            let command = op.get("command").and_then(|v| v.as_str())?;
            Some(AgentOperation::Execute {
                command: command.to_string(),
            })
        }
        _ => None,
    }
}

/// Get a simple file tree for context
pub fn get_file_tree(workspace_path: &Path, max_depth: usize) -> Vec<String> {
    let mut files = Vec::new();
    collect_files(workspace_path, workspace_path, &mut files, 0, max_depth);
    files
}

fn collect_files(
    base: &Path,
    current: &Path,
    files: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }

    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip common directories to ignore
        if name.starts_with('.')
            || name == "node_modules"
            || name == "target"
            || name == ".git"
            || name == ".next"
            || name == "__pycache__"
            || name == "dist"
            || name == "build"
        {
            continue;
        }

        if path.is_dir() {
            collect_files(base, &path, files, depth + 1, max_depth);
        } else {
            if let Ok(relative) = path.strip_prefix(base) {
                files.push(relative.to_string_lossy().to_string());
            }
        }
    }
}

/// Build messages for developer with task and context
pub fn build_developer_messages(
    task: &str,
    workspace_path: &Path,
    file_list: &[String],
) -> Vec<ChatMessage> {
    let file_context = if file_list.is_empty() {
        "No files found in workspace".to_string()
    } else {
        file_list.join("\n")
    };

    vec![
        ChatMessage::system(DEVELOPER_SYSTEM_PROMPT),
        ChatMessage::user(format!(
            "Workspace Base Path: {}\n\nExisting Files:\n{}\n\nTASK: {}",
            workspace_path.display(),
            file_context,
            task
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_response() {
        let response = r#"{
            "reasoning": "Creating a new file",
            "operations": [
                {"type": "write", "path": "/tmp/test.txt", "content": "Hello"}
            ],
            "message": "File created"
        }"#;

        let result = parse_developer_response(response).unwrap();
        assert_eq!(result.operations.len(), 1);
        assert_eq!(result.message, "File created");
    }

    #[test]
    fn test_parse_with_execute() {
        let response = r#"{
            "reasoning": "Installing deps",
            "operations": [
                {"type": "execute", "command": "npm install"}
            ],
            "message": "Done"
        }"#;

        let result = parse_developer_response(response).unwrap();
        assert_eq!(result.operations.len(), 1);
        match &result.operations[0] {
            AgentOperation::Execute { command } => assert_eq!(command, "npm install"),
            _ => panic!("Expected Execute operation"),
        }
    }
}
