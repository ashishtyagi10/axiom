//! Orchestrator Agent
//!
//! Analyzes user requests and delegates to appropriate agents.

use super::types::{ChatMessage, NextAgent, OrchestratorDecision};
use crate::Result;

const ORCHESTRATOR_SYSTEM_PROMPT: &str = r#"
You are the Orchestrator of an Agile software development team.
Your goal is to analyze the user's request and decide which agent is best suited to handle the next step.

The agents at your disposal are:
1. **Product Owner (po)**: Responsible for defining requirements, user stories, and acceptance criteria. Call this agent first for new feature requests or vague ideas.
2. **Architect (architect)**: Responsible for technical design, file structure, and technology choices. Call this agent after requirements are clear.
3. **Developer (developer)**: Responsible for writing code, fixing bugs, and running tests. Call this agent when the design is ready or for specific code tasks.

**Output Format:**
You must respond with a strict JSON object (no markdown formatting) in the following format:
{
  "next_agent": "po" | "architect" | "developer" | "user",
  "reasoning": "Explanation of why you chose this agent",
  "task": "Specific instructions for the agent (or the final answer to the user if next_agent is 'user')"
}

If the user says "hello" or asks a general question unrelated to coding/project, set "next_agent": "user" and provide a friendly answer in "task".
"#;

/// Parse orchestrator response from LLM output
pub fn parse_orchestrator_response(content: &str) -> Result<OrchestratorDecision> {
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
            let next_agent_str = json
                .get("next_agent")
                .or_else(|| json.get("nextAgent"))
                .and_then(|v| v.as_str())
                .unwrap_or("user");

            let next_agent = next_agent_str.parse().unwrap_or(NextAgent::User);

            let reasoning = json
                .get("reasoning")
                .and_then(|v| v.as_str())
                .unwrap_or("Direct response")
                .to_string();

            let task = json
                .get("task")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Ok(OrchestratorDecision {
                next_agent,
                reasoning,
                task,
            })
        }
        Err(_) => {
            // Fallback: treat the whole response as a message to the user
            Ok(OrchestratorDecision {
                next_agent: NextAgent::User,
                reasoning: "Direct response".to_string(),
                task: Some(content.to_string()),
            })
        }
    }
}

/// Build messages for orchestrator with conversation history
pub fn build_orchestrator_messages(conversation: &[ChatMessage]) -> Vec<ChatMessage> {
    let mut messages = vec![ChatMessage::system(ORCHESTRATOR_SYSTEM_PROMPT)];
    messages.extend(conversation.iter().cloned());
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_response() {
        let response = r#"{"next_agent": "developer", "reasoning": "Code task", "task": "Fix the bug"}"#;
        let decision = parse_orchestrator_response(response).unwrap();
        assert_eq!(decision.next_agent, NextAgent::Developer);
        assert_eq!(decision.task, Some("Fix the bug".to_string()));
    }

    #[test]
    fn test_parse_with_markdown() {
        let response = r#"```json
{"next_agent": "po", "reasoning": "New feature", "task": "Define requirements"}
```"#;
        let decision = parse_orchestrator_response(response).unwrap();
        assert_eq!(decision.next_agent, NextAgent::Po);
    }

    #[test]
    fn test_fallback_response() {
        let response = "I don't understand what you want me to do.";
        let decision = parse_orchestrator_response(response).unwrap();
        assert_eq!(decision.next_agent, NextAgent::User);
        assert_eq!(decision.task, Some(response.to_string()));
    }
}
