import fs from 'fs/promises';
import path from 'path';
import { ensureConfigDir } from './fs';

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

export interface AppConfig {
  workspaces: WorkspaceConfig[];
  llmSettings: {
    providers: LLMProvider[];
    agentMappings: AgentMapping[];
  };
}

const CONFIG_FILE_NAME = 'config.json';

const DEFAULT_CONFIG: AppConfig = {
  workspaces: [],
  llmSettings: {
    providers: [
      { id: 'openai', name: 'OpenAI', apiKey: '', defaultModel: 'gpt-4o', enabled: false },
      { id: 'anthropic', name: 'Anthropic', apiKey: '', defaultModel: 'claude-3-5-sonnet-20240620', enabled: false },
      { id: 'gemini', name: 'Google Gemini', apiKey: '', defaultModel: 'gemini-1.5-pro', enabled: false },
      { id: 'ollama', name: 'Ollama (Local)', apiKey: 'na', baseUrl: 'http://localhost:11434', defaultModel: 'llama3', enabled: false },
    ],
    agentMappings: [
      { agentId: 'orchestrator', providerId: 'openai', modelId: 'gpt-4o' },
      { agentId: 'po', providerId: 'openai', modelId: 'gpt-4o' },
      { agentId: 'architect', providerId: 'openai', modelId: 'gpt-4o' },
      { agentId: 'developer', providerId: 'openai', modelId: 'gpt-4o' },
    ]
  }
};

async function getConfigPath(): Promise<string> {
  const configDir = await ensureConfigDir();
  return path.join(configDir, CONFIG_FILE_NAME);
}

async function readConfig(): Promise<AppConfig> {
  const configPath = await getConfigPath();
  try {
    const data = await fs.readFile(configPath, 'utf-8');
    const parsed = JSON.parse(data);
    // Merge with defaults to handle new fields
    return {
      ...DEFAULT_CONFIG,
      ...parsed,
      llmSettings: {
        ...DEFAULT_CONFIG.llmSettings,
        ...(parsed.llmSettings || {})
      }
    };
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      return DEFAULT_CONFIG;
    }
    throw error;
  }
}

async function writeConfig(config: AppConfig): Promise<void> {
  const configPath = await getConfigPath();
  await fs.writeFile(configPath, JSON.stringify(config, null, 2), 'utf-8');
}

// Workspace Logic
export async function getWorkspaces(): Promise<WorkspaceConfig[]> {
  const config = await readConfig();
  return config.workspaces.sort((a, b) => 
    new Date(b.lastAccessed).getTime() - new Date(a.lastAccessed).getTime()
  );
}

export async function getWorkspaceById(id: string): Promise<WorkspaceConfig | undefined> {
  const config = await readConfig();
  return config.workspaces.find(w => w.id === id);
}

export async function addWorkspace(workspace: Omit<WorkspaceConfig, 'id' | 'createdAt' | 'lastAccessed'>): Promise<WorkspaceConfig> {
  const config = await readConfig();
  
  const newWorkspace: WorkspaceConfig = {
    ...workspace,
    id: crypto.randomUUID(),
    createdAt: new Date().toISOString(),
    lastAccessed: new Date().toISOString(),
  };

  config.workspaces.push(newWorkspace);
  await writeConfig(config);
  return newWorkspace;
}

export async function removeWorkspace(id: string): Promise<void> {
  const config = await readConfig();
  config.workspaces = config.workspaces.filter(w => w.id !== id);
  await writeConfig(config);
}

// LLM Settings Logic
export async function getLLMSettings() {
  const config = await readConfig();
  return config.llmSettings;
}

export async function updateProvider(providerId: string, updates: Partial<LLMProvider>) {
  const config = await readConfig();
  const index = config.llmSettings.providers.findIndex(p => p.id === providerId);
  if (index !== -1) {
    config.llmSettings.providers[index] = { ...config.llmSettings.providers[index], ...updates };
    await writeConfig(config);
  }
}

export async function updateAgentMapping(agentId: string, providerId: string, modelId: string) {
  const config = await readConfig();
  const index = config.llmSettings.agentMappings.findIndex(m => m.agentId === agentId);
  if (index !== -1) {
    config.llmSettings.agentMappings[index] = { agentId, providerId, modelId };
  } else {
    config.llmSettings.agentMappings.push({ agentId, providerId, modelId });
  }
  await writeConfig(config);
}