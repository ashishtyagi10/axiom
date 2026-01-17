/**
 * Axiom WebSocket Client
 * Real-time streaming connection to the Rust backend
 */

import type { Command, Notification } from './types';

export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

export interface WebSocketClientOptions {
  onNotification?: (notification: Notification) => void;
  onStateChange?: (state: ConnectionState) => void;
  onError?: (error: Error) => void;
  reconnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
}

const DEFAULT_OPTIONS: Required<Omit<WebSocketClientOptions, 'onNotification' | 'onStateChange' | 'onError'>> = {
  reconnect: true,
  reconnectInterval: 2000,
  maxReconnectAttempts: 5,
};

export class AxiomWebSocket {
  private ws: WebSocket | null = null;
  private url: string;
  private options: Required<Omit<WebSocketClientOptions, 'onNotification' | 'onStateChange' | 'onError'>> & WebSocketClientOptions;
  private state: ConnectionState = 'disconnected';
  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private messageQueue: Command[] = [];

  constructor(url: string, options: WebSocketClientOptions = {}) {
    this.url = url;
    this.options = { ...DEFAULT_OPTIONS, ...options };
  }

  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return;
    }

    this.setState('connecting');

    try {
      this.ws = new WebSocket(this.url);
      this.setupEventHandlers();
    } catch (error) {
      this.handleError(error as Error);
    }
  }

  disconnect(): void {
    this.options.reconnect = false;
    this.clearReconnectTimer();

    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }

    this.setState('disconnected');
  }

  send(command: Command): boolean {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(command));
      return true;
    }

    // Queue message if not connected
    if (this.options.reconnect) {
      this.messageQueue.push(command);
    }

    return false;
  }

  getState(): ConnectionState {
    return this.state;
  }

  isConnected(): boolean {
    return this.state === 'connected';
  }

  private setupEventHandlers(): void {
    if (!this.ws) return;

    this.ws.onopen = () => {
      this.setState('connected');
      this.reconnectAttempts = 0;
      this.flushMessageQueue();
    };

    this.ws.onclose = (event) => {
      this.setState('disconnected');

      if (this.options.reconnect && !event.wasClean) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = (event) => {
      this.handleError(new Error('WebSocket error'));
    };

    this.ws.onmessage = (event) => {
      try {
        const notification = JSON.parse(event.data) as Notification;
        this.options.onNotification?.(notification);
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
      }
    };
  }

  private setState(state: ConnectionState): void {
    this.state = state;
    this.options.onStateChange?.(state);
  }

  private handleError(error: Error): void {
    this.setState('error');
    this.options.onError?.(error);
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
      console.error('Max reconnect attempts reached');
      return;
    }

    this.clearReconnectTimer();

    const delay = this.options.reconnectInterval * Math.pow(2, this.reconnectAttempts);
    this.reconnectAttempts++;

    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, delay);
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0) {
      const command = this.messageQueue.shift();
      if (command) {
        this.send(command);
      }
    }
  }
}

// Factory function to create WebSocket connections
export function createWorkspaceConnection(
  workspaceId: string,
  options: WebSocketClientOptions = {}
): AxiomWebSocket {
  const baseUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';
  const wsUrl = baseUrl.replace(/^http/, 'ws') + `/api/workspaces/${workspaceId}/ws`;
  return new AxiomWebSocket(wsUrl, options);
}

// Singleton manager for multiple workspace connections
class WebSocketManager {
  private connections = new Map<string, AxiomWebSocket>();

  getConnection(workspaceId: string, options?: WebSocketClientOptions): AxiomWebSocket {
    let connection = this.connections.get(workspaceId);

    if (!connection) {
      connection = createWorkspaceConnection(workspaceId, options);
      this.connections.set(workspaceId, connection);
    }

    return connection;
  }

  closeConnection(workspaceId: string): void {
    const connection = this.connections.get(workspaceId);
    if (connection) {
      connection.disconnect();
      this.connections.delete(workspaceId);
    }
  }

  closeAll(): void {
    for (const [id, connection] of this.connections) {
      connection.disconnect();
    }
    this.connections.clear();
  }
}

export const wsManager = new WebSocketManager();
