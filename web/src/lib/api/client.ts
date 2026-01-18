/**
 * Axiom API Client
 * HTTP client for communicating with the Rust backend (axiom-server)
 */

import type {
  Workspace,
  WorkspaceView,
  CreateWorkspaceRequest,
  FileEntry,
  CommandResult,
  ApiResponse,
} from './types';

// Default to localhost in development, can be configured for production
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

class AxiomApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  private async fetch<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;

    const response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    if (!response.ok) {
      const errorBody = await response.text();
      throw new Error(`API error (${response.status}): ${errorBody}`);
    }

    return response.json();
  }

  // ========== Health Check ==========

  async health(): Promise<{ status: string }> {
    return this.fetch('/api/health');
  }

  // ========== Workspace Operations ==========

  async listWorkspaces(): Promise<{
    workspaces: WorkspaceView[];
    active_id?: string;
  }> {
    return this.fetch('/api/workspaces');
  }

  async createWorkspace(
    request: CreateWorkspaceRequest
  ): Promise<{ success: boolean; workspace?: Workspace; error?: string }> {
    return this.fetch('/api/workspaces', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async getWorkspace(
    id: string
  ): Promise<{ workspace?: Workspace; error?: string }> {
    return this.fetch(`/api/workspaces/${id}`);
  }

  async activateWorkspace(
    id: string
  ): Promise<{ success: boolean; error?: string }> {
    return this.fetch(`/api/workspaces/${id}/activate`, {
      method: 'POST',
    });
  }

  async deleteWorkspace(
    id: string
  ): Promise<{ success: boolean; error?: string }> {
    return this.fetch(`/api/workspaces/${id}`, {
      method: 'DELETE',
    });
  }

  // ========== File Operations ==========

  async listFiles(
    workspaceId: string,
    path?: string,
    includeHidden: boolean = false
  ): Promise<{ entries: FileEntry[] }> {
    const params = new URLSearchParams();
    if (path) params.set('path', path);
    if (includeHidden) params.set('include_hidden', 'true');

    const query = params.toString();
    const endpoint = `/api/workspaces/${workspaceId}/files${query ? `?${query}` : ''}`;

    return this.fetch(endpoint);
  }

  async readFile(
    workspaceId: string,
    path: string
  ): Promise<{ content: string }> {
    const params = new URLSearchParams({ path });
    return this.fetch(`/api/workspaces/${workspaceId}/file?${params}`);
  }

  async writeFile(
    workspaceId: string,
    path: string,
    content: string
  ): Promise<{ success: boolean; error?: string }> {
    return this.fetch(`/api/workspaces/${workspaceId}/file`, {
      method: 'PUT',
      body: JSON.stringify({ path, content }),
    });
  }

  // ========== Terminal/Command Operations ==========

  async runCommand(
    workspaceId: string,
    command: string
  ): Promise<CommandResult> {
    return this.fetch(`/api/workspaces/${workspaceId}/command`, {
      method: 'POST',
      body: JSON.stringify({ command }),
    });
  }

  // ========== Agent Operations ==========

  async invokeAgent(
    workspaceId: string,
    agentId: string,
    prompt: string
  ): Promise<{ agent_id: string; success: boolean; error?: string }> {
    return this.fetch(`/api/workspaces/${workspaceId}/agents`, {
      method: 'POST',
      body: JSON.stringify({ agent_id: agentId, prompt }),
    });
  }

  async cancelAgent(
    workspaceId: string,
    agentId: string
  ): Promise<{ success: boolean; error?: string }> {
    return this.fetch(`/api/workspaces/${workspaceId}/agents/${agentId}/cancel`, {
      method: 'POST',
    });
  }

  async listAgents(
    workspaceId: string
  ): Promise<{ agents: import('./types').AgentView[] }> {
    return this.fetch(`/api/workspaces/${workspaceId}/agents`);
  }

  // ========== WebSocket URL ==========

  getWebSocketUrl(workspaceId: string): string {
    const wsBase = this.baseUrl.replace(/^http/, 'ws');
    return `${wsBase}/api/workspaces/${workspaceId}/ws`;
  }

  // ========== Orchestration Operations ==========

  async orchestrate(
    workspaceId: string,
    messages: Array<{ role: string; content: string }>
  ): Promise<{
    next_agent: string;
    reasoning: string;
    task?: string;
    error?: string;
  }> {
    return this.fetch(`/api/workspaces/${workspaceId}/orchestrate`, {
      method: 'POST',
      body: JSON.stringify({ messages }),
    });
  }

  async runDeveloper(
    workspaceId: string,
    task: string
  ): Promise<{
    reasoning: string;
    operations: Array<{
      type: string;
      path?: string;
      command?: string;
      success?: boolean;
      error?: string;
    }>;
    message: string;
    error?: string;
  }> {
    return this.fetch(`/api/workspaces/${workspaceId}/agents/developer`, {
      method: 'POST',
      body: JSON.stringify({ task }),
    });
  }

  async getLlmSettings(workspaceId: string): Promise<{
    providers: Array<{
      id: string;
      name: string;
      base_url?: string;
      default_model: string;
      enabled: boolean;
      has_api_key: boolean;
    }>;
    agent_mappings: Array<{
      agent_id: string;
      provider_id: string;
      model_id: string;
    }>;
  }> {
    return this.fetch(`/api/workspaces/${workspaceId}/llm/settings`);
  }

  async updateProvider(
    workspaceId: string,
    providerId: string,
    updates: {
      api_key?: string;
      base_url?: string;
      default_model?: string;
      enabled?: boolean;
    }
  ): Promise<{ success: boolean; message?: string; error?: string }> {
    return this.fetch(
      `/api/workspaces/${workspaceId}/llm/providers/${providerId}`,
      {
        method: 'PUT',
        body: JSON.stringify(updates),
      }
    );
  }

  async updateAgentMapping(
    workspaceId: string,
    agentId: string,
    providerId: string,
    modelId: string
  ): Promise<{ success: boolean; message?: string; error?: string }> {
    return this.fetch(
      `/api/workspaces/${workspaceId}/llm/mappings/${agentId}`,
      {
        method: 'PUT',
        body: JSON.stringify({ provider_id: providerId, model_id: modelId }),
      }
    );
  }

  // ========== Slash Command Operations ==========

  async executeSlashCommand(
    workspaceId: string,
    command: string
  ): Promise<import('./types').SlashCommandResult> {
    return this.fetch(`/api/workspaces/${workspaceId}/slash`, {
      method: 'POST',
      body: JSON.stringify({ command }),
    });
  }
}

// Singleton instance
export const axiomApi = new AxiomApiClient();

// Also export the class for custom instances
export { AxiomApiClient };
