//! Chat message types with multimodal support

use serde::{Deserialize, Serialize};

/// Role in the conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

/// Content part for multimodal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content
    Text { text: String },

    /// File attachment (for context)
    File {
        path: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        line_range: Option<(usize, usize)>,
    },
}

/// Chat message for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message sender
    pub role: Role,

    /// Message content (can be text or multipart)
    pub content: MessageContent,
}

/// Message content - either simple text or multipart
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),

    /// Multipart content (text + files)
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// Get the text content as a string
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Parts(parts) => {
                parts
                    .iter()
                    .filter_map(|p| match p {
                        ContentPart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }

    /// Check if this is simple text
    pub fn is_text(&self) -> bool {
        matches!(self, MessageContent::Text(_))
    }

    /// Get file attachments
    pub fn files(&self) -> Vec<&ContentPart> {
        match self {
            MessageContent::Text(_) => vec![],
            MessageContent::Parts(parts) => {
                parts
                    .iter()
                    .filter(|p| matches!(p, ContentPart::File { .. }))
                    .collect()
            }
        }
    }
}

impl ChatMessage {
    /// Create a new user message with text content
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a new assistant message with text content
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a new system message with text content
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a user message with file attachments
    pub fn user_with_files(text: impl Into<String>, files: Vec<(String, String)>) -> Self {
        let mut parts = vec![ContentPart::Text { text: text.into() }];

        for (path, content) in files {
            parts.push(ContentPart::File {
                path,
                content,
                line_range: None,
            });
        }

        Self {
            role: Role::User,
            content: MessageContent::Parts(parts),
        }
    }

    /// Get the text content of the message
    pub fn text(&self) -> String {
        self.content.as_text()
    }

    /// Get the role as a string (for API compatibility)
    pub fn role_str(&self) -> &str {
        self.role.as_str()
    }
}

/// Format file context for LLM prompt
///
/// Wraps file content in a structured format that LLMs can understand
pub fn format_file_context(path: &str, content: &str, line_range: Option<(usize, usize)>) -> String {
    let range_str = line_range
        .map(|(start, end)| format!(" (lines {}-{})", start, end))
        .unwrap_or_default();

    format!(
        "<file_context>\nFile: {}{}\n```\n{}\n```\n</file_context>",
        path, range_str, content
    )
}

/// Build a prompt with file attachments
pub fn build_prompt_with_context(user_text: &str, files: &[(String, String, Option<(usize, usize)>)]) -> String {
    if files.is_empty() {
        return user_text.to_string();
    }

    let mut prompt = String::new();

    // Add file context
    for (path, content, line_range) in files {
        prompt.push_str(&format_file_context(path, content, *line_range));
        prompt.push('\n');
    }

    // Add user's actual question/request
    prompt.push_str("\n");
    prompt.push_str(user_text);

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_message() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role_str(), "user");
        assert_eq!(msg.text(), "Hello");
    }

    #[test]
    fn test_user_with_files() {
        let msg = ChatMessage::user_with_files(
            "Review this code",
            vec![("main.rs".to_string(), "fn main() {}".to_string())],
        );
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content.files().len(), 1);
    }

    #[test]
    fn test_format_file_context() {
        let ctx = format_file_context("main.rs", "fn main() {}", None);
        assert!(ctx.contains("File: main.rs"));
        assert!(ctx.contains("fn main() {}"));
    }
}
