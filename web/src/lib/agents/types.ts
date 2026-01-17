export type AgentRole = 'orchestrator' | 'po' | 'architect' | 'developer';

export interface AgentState {
  id: AgentRole;
  name: string;
  status: 'idle' | 'thinking' | 'working' | 'waiting';
  currentTask?: string;
  lastMessage?: string;
}

export interface OrchestratorDecision {
  nextAgent: AgentRole | 'user';
  reasoning: string;
  task?: string;
}

export interface AgentOperation {
  type: 'write' | 'delete' | 'execute';
  path?: string; // For write/delete
  content?: string; // For write
  command?: string; // For execute
}

export interface DeveloperResponse {
  reasoning: string;
  operations: AgentOperation[];
  message: string;
}