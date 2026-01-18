/**
 * Axiom API Types
 * These types mirror the Rust backend types from axiom-core
 */

// Workspace Types
export interface Workspace {
  id: string;
  name: string;
  path: string;
  workspace_type: 'local' | 'remote' | 'virtual';
  is_active: boolean;
  created_at: number; // Unix timestamp
  last_accessed: number; // Unix timestamp
  tags?: string[];
}

export interface WorkspaceView {
  id: string;
  name: string;
  path: string;
  workspace_type: 'local' | 'remote' | 'virtual';
  is_active: boolean;
}

export interface CreateWorkspaceRequest {
  name: string;
  path: string;
}

// File Types
export interface FileEntry {
  name: string;
  path: string;
  is_directory: boolean;
  isDirectory: boolean; // Alias for compatibility with fs.ts
  size: number;
  modified?: number;
  is_hidden?: boolean;
}

// Agent Types
export type AgentType = 'llm' | 'cli' | 'shell' | 'conductor';
export type AgentStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface AgentView {
  id: string;
  name: string;
  agent_type: AgentType;
  status: AgentStatus;
  line_count: number;
  elapsed_secs: number;
  parent_id?: string;
}

// Command Types (sent to backend via WebSocket)
export type Command =
  | { type: 'ProcessInput'; text: string }
  | { type: 'ExecuteShell'; command: string }
  | { type: 'InvokeCliAgent'; agent_id: string; prompt: string }
  | { type: 'SendPtyInput'; agent_id: string; data: number[] }
  | { type: 'ResizePty'; agent_id: string; cols: number; rows: number }
  | { type: 'ReadFile'; path: string }
  | { type: 'WriteFile'; path: string; content: string }
  | { type: 'CancelAgent'; agent_id: string }
  | { type: 'ListWorkspaces' }
  | { type: 'CreateWorkspace'; name: string; path: string }
  | { type: 'DeleteWorkspace'; workspace_id: string }
  | { type: 'ActivateWorkspace'; workspace_id: string }
  | { type: 'ListFiles'; path: string; include_hidden: boolean }
  | { type: 'SlashCommand'; command: SlashCommand };

// Notification Types (received from backend via WebSocket)
export type Notification =
  | { type: 'AgentSpawned'; id: string; name: string; agent_type: AgentType; parent_id?: string }
  | { type: 'AgentStatusChanged'; id: string; status: AgentStatus }
  | { type: 'AgentOutput'; id: string; chunk: string }
  | { type: 'PtyOutput'; id: string; data: number[] }
  | { type: 'PtyExited'; id: string; exit_code: number }
  | { type: 'FileModified'; path: string }
  | { type: 'FileLoaded'; path: string; content: string }
  | { type: 'Error'; message: string }
  | { type: 'Info'; message: string }
  | { type: 'WorkspaceList'; workspaces: WorkspaceView[]; active_id?: string }
  | { type: 'WorkspaceCreated'; workspace: Workspace }
  | { type: 'WorkspaceDeleted'; workspace_id: string }
  | { type: 'WorkspaceActivated'; workspace: Workspace }
  | { type: 'FileList'; path: string; entries: FileEntry[] }
  | { type: 'SlashCommandResult'; result: SlashCommandResult };

// Terminal Types
export interface TerminalLine {
  text: string;
  // ANSI formatting info can be added later
}

export interface TerminalScreen {
  lines: TerminalLine[];
  cursor?: [number, number];
  cols: number;
  rows: number;
}

// Command execution result
export interface CommandResult {
  stdout: string;
  stderr: string;
  exit_code: number;
}

// LLM Types
export interface LlmInfo {
  provider_id: string;
  model: string;
  status: 'connected' | 'disconnected' | 'error';
}

export interface LlmProvider {
  id: string;
  name: string;
  enabled: boolean;
  models: string[];
}

// API Response wrapper
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ========== Slash Command Types ==========

export type WorkspaceSubcommand =
  | { subcommand: 'List' }
  | { subcommand: 'Switch'; id?: string }
  | { subcommand: 'Create'; name: string; path: string };

export type ModelSubcommand =
  | { subcommand: 'List' }
  | { subcommand: 'Set'; model: string }
  | { subcommand: 'Current' };

export type ThemeSubcommand =
  | { subcommand: 'Toggle' }
  | { subcommand: 'Set'; variant: string };

export type SlashCommand =
  | { type: 'Help'; args: { command?: string } }
  | { type: 'Clear' }
  | { type: 'Settings' }
  | { type: 'Exit' }
  | { type: 'Version' }
  | { type: 'Init'; args: { path?: string } }
  | { type: 'Workspace'; args: WorkspaceSubcommand }
  | { type: 'Model'; args: ModelSubcommand }
  | { type: 'Theme'; args: ThemeSubcommand }
  | { type: 'Custom'; args: { name: string; args: string[] } };

export type UiAction =
  | { action: 'OpenSettings' }
  | { action: 'OpenModelSelector' }
  | { action: 'OpenWorkspaceSelector' }
  | { action: 'ClearOutput' }
  | { action: 'ToggleTheme' }
  | { action: 'SetTheme'; variant: string }
  | { action: 'FocusPanel'; panel: string };

export interface CommandHelp {
  name: string;
  aliases: string[];
  description: string;
  usage: string;
  examples: string[];
}

export interface WorkspaceInfo {
  id: string;
  name: string;
  path: string;
  is_active: boolean;
}

export type SlashCommandData =
  | { data_type: 'Help'; value: { commands: CommandHelp[] } }
  | { data_type: 'Version'; value: { version: string; commit?: string } }
  | { data_type: 'WorkspaceList'; value: WorkspaceInfo[] }
  | { data_type: 'ModelList'; value: { provider: string; models: string[]; active?: string } }
  | { data_type: 'Text'; value: string };

export type SlashCommandResult =
  | { type: 'Success'; message?: string }
  | { type: 'UiAction'; action: UiAction }
  | { type: 'Data'; data: SlashCommandData }
  | { type: 'Error'; message: string }
  | { type: 'Exit' };
