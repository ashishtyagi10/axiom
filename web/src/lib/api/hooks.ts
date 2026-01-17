'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { axiomApi } from './client';
import { AxiomWebSocket, createWorkspaceConnection, ConnectionState } from './websocket';
import type { WorkspaceView, FileEntry, Notification, Command } from './types';

// ========== Workspace Hooks ==========

export function useWorkspaces() {
  const [workspaces, setWorkspaces] = useState<WorkspaceView[]>([]);
  const [activeId, setActiveId] = useState<string | undefined>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchWorkspaces = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await axiomApi.listWorkspaces();
      setWorkspaces(response.workspaces);
      setActiveId(response.active_id);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchWorkspaces();
  }, [fetchWorkspaces]);

  const createWorkspace = useCallback(async (name: string, path: string) => {
    try {
      const response = await axiomApi.createWorkspace({ name, path });
      if (response.success && response.workspace) {
        await fetchWorkspaces();
        return response.workspace;
      }
      throw new Error(response.error || 'Failed to create workspace');
    } catch (e: any) {
      setError(e.message);
      throw e;
    }
  }, [fetchWorkspaces]);

  const deleteWorkspace = useCallback(async (id: string) => {
    try {
      const response = await axiomApi.deleteWorkspace(id);
      if (response.success) {
        await fetchWorkspaces();
      } else {
        throw new Error(response.error || 'Failed to delete workspace');
      }
    } catch (e: any) {
      setError(e.message);
      throw e;
    }
  }, [fetchWorkspaces]);

  const activateWorkspace = useCallback(async (id: string) => {
    try {
      const response = await axiomApi.activateWorkspace(id);
      if (response.success) {
        setActiveId(id);
      } else {
        throw new Error(response.error || 'Failed to activate workspace');
      }
    } catch (e: any) {
      setError(e.message);
      throw e;
    }
  }, []);

  return {
    workspaces,
    activeId,
    loading,
    error,
    refresh: fetchWorkspaces,
    createWorkspace,
    deleteWorkspace,
    activateWorkspace,
  };
}

// ========== File Browser Hook ==========

export function useFileBrowser(workspaceId: string | undefined) {
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [currentPath, setCurrentPath] = useState<string | undefined>();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const navigate = useCallback(async (path?: string) => {
    if (!workspaceId) return;

    try {
      setLoading(true);
      setError(null);
      const response = await axiomApi.listFiles(workspaceId, path);
      setEntries(response.entries);
      setCurrentPath(path);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  useEffect(() => {
    if (workspaceId) {
      navigate();
    }
  }, [workspaceId, navigate]);

  return {
    entries,
    currentPath,
    loading,
    error,
    navigate,
    refresh: () => navigate(currentPath),
  };
}

// ========== WebSocket Hook ==========

export function useWorkspaceConnection(workspaceId: string | undefined) {
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected');
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const wsRef = useRef<AxiomWebSocket | null>(null);

  useEffect(() => {
    if (!workspaceId) {
      if (wsRef.current) {
        wsRef.current.disconnect();
        wsRef.current = null;
      }
      return;
    }

    const ws = createWorkspaceConnection(workspaceId, {
      onNotification: (notification) => {
        setNotifications((prev) => [...prev, notification]);
      },
      onStateChange: setConnectionState,
      onError: (error) => {
        console.error('WebSocket error:', error);
      },
    });

    ws.connect();
    wsRef.current = ws;

    return () => {
      ws.disconnect();
      wsRef.current = null;
    };
  }, [workspaceId]);

  const sendCommand = useCallback((command: Command) => {
    if (wsRef.current) {
      return wsRef.current.send(command);
    }
    return false;
  }, []);

  const clearNotifications = useCallback(() => {
    setNotifications([]);
  }, []);

  return {
    connectionState,
    isConnected: connectionState === 'connected',
    notifications,
    sendCommand,
    clearNotifications,
  };
}

// ========== File Content Hook ==========

export function useFileContent(workspaceId: string | undefined, filePath: string | undefined) {
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadFile = useCallback(async () => {
    if (!workspaceId || !filePath) return;

    try {
      setLoading(true);
      setError(null);
      const response = await axiomApi.readFile(workspaceId, filePath);
      setContent(response.content);
    } catch (e: any) {
      setError(e.message);
      setContent(null);
    } finally {
      setLoading(false);
    }
  }, [workspaceId, filePath]);

  const saveFile = useCallback(async (newContent: string) => {
    if (!workspaceId || !filePath) return;

    try {
      setLoading(true);
      setError(null);
      const response = await axiomApi.writeFile(workspaceId, filePath, newContent);
      if (response.success) {
        setContent(newContent);
      } else {
        throw new Error(response.error || 'Failed to save file');
      }
    } catch (e: any) {
      setError(e.message);
      throw e;
    } finally {
      setLoading(false);
    }
  }, [workspaceId, filePath]);

  useEffect(() => {
    loadFile();
  }, [loadFile]);

  return {
    content,
    loading,
    error,
    reload: loadFile,
    saveFile,
  };
}
