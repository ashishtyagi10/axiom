'use server';

import { axiomApi } from '@/lib/api';

// Types for orchestration
export type AgentRole = 'orchestrator' | 'po' | 'architect' | 'developer';

export interface OrchestratorDecision {
  nextAgent: AgentRole | 'user';
  reasoning: string;
  task?: string;
}

export interface AgentOperation {
  type: 'write' | 'delete' | 'execute';
  path?: string;
  content?: string;
  command?: string;
  success?: boolean;
  error?: string;
}

export interface DeveloperResponse {
  reasoning: string;
  operations: AgentOperation[];
  message: string;
}

export interface LLMMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

/**
 * Run the orchestrator to decide the next action
 */
export async function orchestrateAction(
  workspaceId: string,
  messages: LLMMessage[]
): Promise<OrchestratorDecision> {
  try {
    const response = await axiomApi.orchestrate(workspaceId, messages);

    if (response.error) {
      throw new Error(response.error);
    }

    // Convert backend format to frontend format
    return {
      nextAgent: response.next_agent as AgentRole | 'user',
      reasoning: response.reasoning,
      task: response.task,
    };
  } catch (error: any) {
    console.error('Orchestration error:', error);
    // Fallback to user response on error
    return {
      nextAgent: 'user',
      reasoning: 'Error occurred during orchestration',
      task: error.message || 'An error occurred. Please try again.',
    };
  }
}

/**
 * Run the developer agent to write code
 */
export async function developerAction(
  workspaceId: string,
  task: string
): Promise<DeveloperResponse> {
  try {
    const response = await axiomApi.runDeveloper(workspaceId, task);

    if (response.error) {
      throw new Error(response.error);
    }

    // Convert backend format to frontend format
    return {
      reasoning: response.reasoning,
      operations: response.operations.map((op) => ({
        type: op.type as 'write' | 'delete' | 'execute',
        path: op.path,
        command: op.command,
        success: op.success,
        error: op.error,
      })),
      message: response.message,
    };
  } catch (error: any) {
    console.error('Developer agent error:', error);
    return {
      reasoning: 'Failed to execute developer agent',
      operations: [],
      message: error.message || 'An error occurred. Please try again.',
    };
  }
}

/**
 * Get LLM settings for the workspace
 */
export async function getLlmSettingsAction(workspaceId: string) {
  try {
    return await axiomApi.getLlmSettings(workspaceId);
  } catch (error: any) {
    console.error('Failed to get LLM settings:', error);
    throw error;
  }
}

/**
 * Update a provider's configuration
 */
export async function updateProviderAction(
  workspaceId: string,
  providerId: string,
  updates: {
    api_key?: string;
    base_url?: string;
    default_model?: string;
    enabled?: boolean;
  }
) {
  try {
    return await axiomApi.updateProvider(workspaceId, providerId, updates);
  } catch (error: any) {
    console.error('Failed to update provider:', error);
    throw error;
  }
}

/**
 * Update an agent's LLM mapping
 */
export async function updateAgentMappingAction(
  workspaceId: string,
  agentId: string,
  providerId: string,
  modelId: string
) {
  try {
    return await axiomApi.updateAgentMapping(
      workspaceId,
      agentId,
      providerId,
      modelId
    );
  } catch (error: any) {
    console.error('Failed to update agent mapping:', error);
    throw error;
  }
}
