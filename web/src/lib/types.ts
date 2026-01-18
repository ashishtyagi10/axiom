// Browser-compatible types (no Node.js dependencies)

export interface WorkspaceConfig {
  id: string;
  title: string;
  path: string;
  type: 'local' | 'remote';
  lastAccessed: string;
  createdAt: string;
}

export interface LLMProvider {
  id: string;
  name: string;
  apiKey: string;
  baseUrl?: string;
  defaultModel: string;
  enabled: boolean;
}

export interface AgentMapping {
  agentId: string; // orchestrator, po, architect, developer
  providerId: string;
  modelId: string;
}
