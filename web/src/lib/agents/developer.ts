import { chatCompletion, LLMMessage } from '../llm/api';
import { DeveloperResponse } from './types';
import { listDirectory } from '../fs';
import path from 'path';

const SYSTEM_PROMPT = `
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
      "content": "content"
    },
    {
      "type": "execute",
      "command": "npm install"
    }
  ],
  "message": "Summary for the user"
}

Prioritize writing files before executing commands if they are dependencies.
`;

async function getSimpleFileTree(dir: string, depth = 0): Promise<string[]> {
  if (depth > 3) return [];
  try {
    const entries = await listDirectory(dir);
    const files = [];
    for (const entry of entries) {
      if (entry.name === 'node_modules' || entry.name === '.git' || entry.name === '.next') continue;
      
      if (entry.isDirectory) {
        const subFiles = await getSimpleFileTree(entry.path, depth + 1);
        files.push(...subFiles);
      } else {
        files.push(entry.path);
      }
    }
    return files;
  } catch (e) {
    return [];
  }
}

export async function runDeveloperAgent(task: string, workspacePath: string): Promise<DeveloperResponse> {
  // 1. Get Context
  const allFiles = await getSimpleFileTree(workspacePath);
  const fileList = allFiles.map(f => f.replace(workspacePath, '')).join('\n');

  // 2. Construct Prompt
  const messages: LLMMessage[] = [
    { role: 'system', content: SYSTEM_PROMPT },
    { role: 'user', content: `Workspace Base Path: ${workspacePath}\n\nExisting Files:\n${fileList}\n\nTASK: ${task}` }
  ];

  // 3. Call LLM
  const response = await chatCompletion('developer', messages);

  // 4. Parse
  try {
    const cleanContent = response.content.replace(/```json/g, '').replace(/```/g, '').trim();
    return JSON.parse(cleanContent);
  } catch (e) {
    console.error("Failed to parse developer response", response.content);
    return {
      reasoning: "Failed to parse response",
      operations: [],
      message: "I tried to generate a response but failed the format check."
    };
  }
}
