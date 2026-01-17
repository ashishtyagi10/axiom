/**
 * Axiom API Module
 * Main exports for the Axiom backend API client
 */

// Types
export * from './types';

// HTTP Client
export { axiomApi, AxiomApiClient } from './client';

// WebSocket Client
export {
  AxiomWebSocket,
  createWorkspaceConnection,
  wsManager,
  type ConnectionState,
  type WebSocketClientOptions,
} from './websocket';

// React Hooks
export {
  useWorkspaces,
  useFileBrowser,
  useWorkspaceConnection,
  useFileContent,
} from './hooks';
