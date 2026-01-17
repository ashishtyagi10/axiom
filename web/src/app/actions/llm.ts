'use server';

// For Phase 3, LLM settings remain in the frontend config
// In Phase 4, these will be moved to the Rust backend

import {
  getLLMSettings as getLLMSettingsLib,
  updateProvider as updateProviderLib,
  updateAgentMapping as updateAgentMappingLib,
  LLMProvider
} from '@/lib/workspace-config';

export async function getLLMSettingsAction() {
  return await getLLMSettingsLib();
}

export async function updateProviderAction(providerId: string, updates: Partial<LLMProvider>) {
  return await updateProviderLib(providerId, updates);
}

export async function updateAgentMappingAction(agentId: string, providerId: string, modelId: string) {
  return await updateAgentMappingLib(agentId, providerId, modelId);
}
