import { chatCompletion, LLMMessage } from '../llm/api';
import { OrchestratorDecision } from './types';

const SYSTEM_PROMPT = `
You are the Orchestrator of an Agile software development team.
Your goal is to analyze the user's request and decide which agent is best suited to handle the next step.

The agents at your disposal are:
1. **Product Owner (po)**: Responsible for defining requirements, user stories, and acceptance criteria. Call this agent first for new feature requests or vague ideas.
2. **Architect (architect)**: Responsible for technical design, file structure, and technology choices. Call this agent after requirements are clear.
3. **Developer (developer)**: Responsible for writing code, fixing bugs, and running tests. Call this agent when the design is ready or for specific code tasks.

**Output Format:**
You must respond with a strict JSON object (no markdown formatting) in the following format:
{
  "nextAgent": "po" | "architect" | "developer" | "user",
  "reasoning": "Explanation of why you chose this agent",
  "task": "Specific instructions for the agent (or the final answer to the user if nextAgent is 'user')"
}

If the user says "hello" or asks a general question unrelated to coding/project, set "nextAgent": "user" and provide a friendly answer in "task".
`;

export async function orchestrate(messages: LLMMessage[]): Promise<OrchestratorDecision> {
  const response = await chatCompletion('orchestrator', [
    { role: 'system', content: SYSTEM_PROMPT },
    ...messages
  ]);

  try {
    // Clean up potential markdown code blocks
    const cleanContent = response.content.replace(/```json/g, '').replace(/```/g, '').trim();
    return JSON.parse(cleanContent);
  } catch (e) {
    console.error("Failed to parse orchestrator response", response.content);
    // Fallback: If parsing fails, treat the whole response as a message to the user
    return {
      nextAgent: 'user',
      reasoning: 'Direct response',
      task: response.content
    };
  }
}
