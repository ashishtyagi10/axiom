'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { axiomApi } from './client';
import { AxiomWebSocket, createWorkspaceConnection, ConnectionState } from './websocket';
import type {
  WorkspaceView,
  FileEntry,
  Notification,
  Command,
  RalphState,
  RalphConfig
} from './types';

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

// ========== Ralph Loop Hook ==========

export interface UseRalphLoopOptions {
  workspaceId: string;
  onIterationStart?: (iteration: number, task: string) => void;
  onIterationComplete?: (iteration: number, status: string, summary: string) => void;
  onLoopComplete?: (totalIterations: number, reason: any) => void;
  onError?: (error: string) => void;
}

export function useRalphLoop(options: UseRalphLoopOptions) {
  const { workspaceId, onIterationStart, onIterationComplete, onLoopComplete, onError } = options;

  const [state, setState] = useState<RalphState | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Polling interval ref for status updates
  const pollingRef = useRef<NodeJS.Timeout | null>(null);

  // Fetch current Ralph status
  const fetchStatus = useCallback(async () => {
    try {
      const response = await axiomApi.getRalphStatus(workspaceId);
      if (response.state) {
        setState(response.state);

        // Stop polling if loop is complete or errored
        if (response.state.status === 'Complete' || response.state.status === 'Error') {
          if (pollingRef.current) {
            clearInterval(pollingRef.current);
            pollingRef.current = null;
          }
        }
      }
    } catch (e: any) {
      // Silently handle fetch errors during polling
      console.error('Failed to fetch Ralph status:', e);
    }
  }, [workspaceId]);

  // Start Ralph Loop
  const startLoop = useCallback(async (task: string, config?: Partial<RalphConfig>) => {
    if (!task.trim()) {
      setError('Task description is required');
      return;
    }

    try {
      setIsStarting(true);
      setError(null);

      const response = await axiomApi.startRalphLoop(workspaceId, task, config);

      if (response.success && response.state) {
        setState(response.state);

        // Start polling for status updates
        if (!pollingRef.current) {
          pollingRef.current = setInterval(fetchStatus, 2000);
        }
      } else {
        throw new Error(response.error || 'Failed to start Ralph Loop');
      }
    } catch (e: any) {
      setError(e.message);
      onError?.(e.message);
    } finally {
      setIsStarting(false);
    }
  }, [workspaceId, fetchStatus, onError]);

  // Stop Ralph Loop
  const stopLoop = useCallback(async () => {
    try {
      setIsStopping(true);
      setError(null);

      const response = await axiomApi.stopRalphLoop(workspaceId);

      if (response.success) {
        await fetchStatus();
      } else {
        throw new Error(response.error || 'Failed to stop Ralph Loop');
      }
    } catch (e: any) {
      setError(e.message);
      onError?.(e.message);
    } finally {
      setIsStopping(false);
    }
  }, [workspaceId, fetchStatus, onError]);

  // Update feedback for next iteration
  const updateFeedback = useCallback(async (feedback: string) => {
    try {
      setError(null);
      const response = await axiomApi.updateRalphFeedback(workspaceId, feedback);

      if (!response.success) {
        throw new Error(response.error || 'Failed to update feedback');
      }
    } catch (e: any) {
      setError(e.message);
      onError?.(e.message);
    }
  }, [workspaceId, onError]);

  // Reset state (for starting a new loop after completion)
  const reset = useCallback(() => {
    setState(null);
    setError(null);
    if (pollingRef.current) {
      clearInterval(pollingRef.current);
      pollingRef.current = null;
    }
  }, []);

  // Initial status fetch and cleanup
  useEffect(() => {
    if (workspaceId) {
      fetchStatus();
    }

    return () => {
      if (pollingRef.current) {
        clearInterval(pollingRef.current);
        pollingRef.current = null;
      }
    };
  }, [workspaceId, fetchStatus]);

  // Handle state changes for callbacks
  useEffect(() => {
    if (!state) return;

    const lastIteration = state.iteration_history[state.iteration_history.length - 1];

    // Call callbacks based on state changes
    if (state.status === 'Running' && lastIteration) {
      onIterationComplete?.(
        lastIteration.iteration,
        lastIteration.status,
        lastIteration.summary
      );
    }

    if (state.status === 'Complete' && state.completion_reason) {
      onLoopComplete?.(state.current_iteration, state.completion_reason);
    }
  }, [state, onIterationComplete, onLoopComplete]);

  return {
    state,
    isStarting,
    isStopping,
    isRunning: state?.status === 'Running',
    isComplete: state?.status === 'Complete' || state?.status === 'Error',
    error,
    startLoop,
    stopLoop,
    updateFeedback,
    refresh: fetchStatus,
    reset,
  };
}
