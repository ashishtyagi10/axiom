'use server';

import { axiomApi } from '@/lib/api';
import type { Workspace, WorkspaceView, FileEntry } from '@/lib/api/types';

// Re-export types with OSMO-compatible names
export interface WorkspaceConfig {
  id: string;
  title: string;
  path: string;
  type: 'local' | 'remote';
  lastAccessed: string;
  createdAt: string;
}

// Convert Axiom Workspace to OSMO-compatible WorkspaceConfig
function toWorkspaceConfig(ws: WorkspaceView | Workspace): WorkspaceConfig {
  const w = ws as Workspace;
  return {
    id: ws.id,
    title: ws.name,
    path: ws.path,
    type: ws.workspace_type === 'virtual' ? 'local' : ws.workspace_type,
    lastAccessed: w.last_accessed
      ? new Date(w.last_accessed * 1000).toISOString()
      : new Date().toISOString(),
    createdAt: w.created_at
      ? new Date(w.created_at * 1000).toISOString()
      : new Date().toISOString(),
  };
}

export async function getWorkspacesAction(): Promise<WorkspaceConfig[]> {
  try {
    const response = await axiomApi.listWorkspaces();
    return response.workspaces.map(toWorkspaceConfig);
  } catch (error) {
    console.error('Failed to get workspaces:', error);
    return [];
  }
}

export async function getHomeDirAction(): Promise<string> {
  // This needs to be fetched from the server
  // For now, return a placeholder - will be implemented in axiom-server
  return process.env.HOME || '/';
}

export async function readFileAction(path: string): Promise<string> {
  // This requires knowing the workspace ID
  // For now, we'll need to refactor components to pass workspace ID
  throw new Error('readFileAction requires workspace ID - use component with workspace context');
}

export async function writeFileAction(path: string, content: string): Promise<void> {
  // This requires knowing the workspace ID
  throw new Error('writeFileAction requires workspace ID - use component with workspace context');
}

export async function getWorkspaceByIdAction(id: string): Promise<WorkspaceConfig | undefined> {
  try {
    const response = await axiomApi.getWorkspace(id);
    if (response.workspace) {
      return toWorkspaceConfig(response.workspace);
    }
    return undefined;
  } catch (error) {
    console.error('Failed to get workspace:', error);
    return undefined;
  }
}

export async function addWorkspaceAction(
  title: string,
  path: string,
  type: 'local' | 'remote' = 'local'
): Promise<WorkspaceConfig> {
  const response = await axiomApi.createWorkspace({
    name: title,
    path: path,
  });

  if (!response.success || !response.workspace) {
    throw new Error(response.error || 'Failed to create workspace');
  }

  return toWorkspaceConfig(response.workspace);
}

export async function removeWorkspaceAction(id: string): Promise<void> {
  const response = await axiomApi.deleteWorkspace(id);
  if (!response.success) {
    throw new Error(response.error || 'Failed to delete workspace');
  }
}

export async function listFilesAction(workspaceId: string, path?: string): Promise<FileEntry[]> {
  try {
    const response = await axiomApi.listFiles(workspaceId, path);
    return response.entries;
  } catch (error) {
    console.error('Failed to list files:', error);
    return [];
  }
}
