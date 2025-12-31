/**
 * WebSocket client for real-time import progress from doorway/elohim-storage
 *
 * ## Usage
 *
 * ```typescript
 * import { ProgressClient, waitForBatchCompletion } from './progress-client';
 *
 * const client = new ProgressClient(DOORWAY_URL);
 * await client.connect();
 * client.subscribe(batchId);
 *
 * const result = await waitForBatchCompletion(client, batchId, {
 *   onProgress: (update) => console.log(`Progress: ${update.processed_count}/${update.total_items}`)
 * });
 *
 * client.close();
 * ```
 */

import { EventEmitter } from 'events';
import WebSocket from 'ws';

// ============================================================================
// Types
// ============================================================================

/** Progress update message from server */
export interface ProgressMessage {
  type: 'progress' | 'complete' | 'error' | 'heartbeat' | 'initial_state';
  batch_id?: string;
  timestamp?: string;
  // For progress/complete
  status?: string;
  total_items?: number;
  processed_count?: number;
  error_count?: number;
  items_per_second?: number;
  elapsed_ms?: number;
  errors?: string[];
  // For initial_state
  batches?: BatchState[];
  // For error
  message?: string;
}

/** Batch state (from initial_state) */
export interface BatchState {
  batch_id: string;
  batch_type: string;
  status: string;
  total_items: number;
  processed_count: number;
  error_count: number;
  elapsed_ms: number;
  items_per_second: number;
}

/** Client configuration */
export interface ProgressClientConfig {
  /** Doorway URL (will be converted to WebSocket URL) */
  doorwayUrl: string;
  /** Initial reconnect delay in ms (default: 1000) */
  reconnectDelay?: number;
  /** Max reconnect delay in ms (default: 30000) */
  maxReconnectDelay?: number;
  /** Max reconnect attempts (default: 5) */
  maxReconnectAttempts?: number;
  /** Connection timeout in ms (default: 10000) */
  connectTimeout?: number;
}

/** Options for waitForBatchCompletion */
export interface WaitOptions {
  /** Timeout in ms (default: 300000 = 5 min) */
  timeoutMs?: number;
  /** Callback on each progress update */
  onProgress?: (update: ProgressMessage) => void;
}

// ============================================================================
// Progress Client
// ============================================================================

/**
 * WebSocket client for real-time import progress updates
 *
 * Events:
 * - 'connected' - Connection established
 * - 'disconnected' - Connection closed
 * - 'progress' - Progress update received
 * - 'complete' - Batch completed
 * - 'error' - Error received or connection error
 * - 'heartbeat' - Heartbeat received
 */
export class ProgressClient extends EventEmitter {
  private ws: WebSocket | null = null;
  private config: Required<ProgressClientConfig>;
  private reconnecting = false;
  private reconnectAttempts = 0;
  private subscriptions = new Set<string>();
  private closed = false;

  constructor(config: ProgressClientConfig | string) {
    super();

    if (typeof config === 'string') {
      config = { doorwayUrl: config };
    }

    this.config = {
      doorwayUrl: config.doorwayUrl,
      reconnectDelay: config.reconnectDelay ?? 1000,
      maxReconnectDelay: config.maxReconnectDelay ?? 30000,
      maxReconnectAttempts: config.maxReconnectAttempts ?? 5,
      connectTimeout: config.connectTimeout ?? 10000,
    };
  }

  /** Check if connected */
  get connected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  /** Connect to doorway WebSocket */
  async connect(): Promise<void> {
    if (this.connected) return;
    if (this.closed) throw new Error('Client is closed');

    const wsUrl = this.getWebSocketUrl();

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        if (this.ws) {
          this.ws.terminate();
        }
        reject(new Error(`Connection timeout (${this.config.connectTimeout}ms)`));
      }, this.config.connectTimeout);

      try {
        this.ws = new WebSocket(wsUrl);

        this.ws.on('open', () => {
          clearTimeout(timeout);
          this.reconnectAttempts = 0;
          this.emit('connected');

          // Re-subscribe to any previous subscriptions
          if (this.subscriptions.size > 0) {
            this.sendSubscribe([...this.subscriptions]);
          }

          resolve();
        });

        this.ws.on('message', (data) => {
          this.handleMessage(data.toString());
        });

        this.ws.on('close', () => {
          clearTimeout(timeout);
          this.emit('disconnected');
          this.attemptReconnect();
        });

        this.ws.on('error', (error) => {
          clearTimeout(timeout);
          this.emit('error', error);
          reject(error);
        });
      } catch (error) {
        clearTimeout(timeout);
        reject(error);
      }
    });
  }

  /** Subscribe to progress updates for specific batch(es) */
  subscribe(batchId: string | string[]): void {
    const ids = Array.isArray(batchId) ? batchId : [batchId];
    ids.forEach((id) => this.subscriptions.add(id));

    if (this.connected) {
      this.sendSubscribe(ids);
    }
  }

  /** Unsubscribe from batch(es) */
  unsubscribe(batchId: string | string[]): void {
    const ids = Array.isArray(batchId) ? batchId : [batchId];
    ids.forEach((id) => this.subscriptions.delete(id));

    if (this.connected) {
      this.sendMessage({ type: 'unsubscribe', batch_ids: ids });
    }
  }

  /** Send ping to keep connection alive */
  ping(): void {
    if (this.connected) {
      this.sendMessage({ type: 'ping' });
    }
  }

  /** Close the connection */
  close(): void {
    this.closed = true;
    this.subscriptions.clear();
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  // ==========================================================================
  // Private methods
  // ==========================================================================

  private getWebSocketUrl(): string {
    let url = this.config.doorwayUrl;

    // Convert HTTP URL to WebSocket URL
    if (url.startsWith('https://')) {
      url = url.replace('https://', 'wss://');
    } else if (url.startsWith('http://')) {
      url = url.replace('http://', 'ws://');
    } else if (!url.startsWith('ws://') && !url.startsWith('wss://')) {
      url = `ws://${url}`;
    }

    // Ensure path ends with /import/progress
    if (!url.endsWith('/import/progress')) {
      url = url.replace(/\/$/, '') + '/import/progress';
    }

    return url;
  }

  private sendMessage(msg: object): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  private sendSubscribe(batchIds: string[]): void {
    this.sendMessage({ type: 'subscribe', batch_ids: batchIds });
  }

  private handleMessage(data: string): void {
    try {
      const msg: ProgressMessage = JSON.parse(data);

      switch (msg.type) {
        case 'progress':
          this.emit('progress', msg);
          break;
        case 'complete':
          this.emit('complete', msg);
          this.emit('progress', msg); // Also emit as progress for unified handling
          break;
        case 'error':
          this.emit('error', new Error(msg.message ?? 'Unknown error'));
          break;
        case 'heartbeat':
          this.emit('heartbeat', msg);
          break;
        case 'initial_state':
          this.emit('initial_state', msg);
          // Also emit progress for each batch
          msg.batches?.forEach((batch) => {
            this.emit('progress', {
              type: 'progress',
              batch_id: batch.batch_id,
              status: batch.status,
              total_items: batch.total_items,
              processed_count: batch.processed_count,
              error_count: batch.error_count,
              elapsed_ms: batch.elapsed_ms,
              items_per_second: batch.items_per_second,
            });
          });
          break;
        default:
          // Unknown message type
          break;
      }
    } catch (error) {
      this.emit('error', new Error(`Failed to parse message: ${error}`));
    }
  }

  private attemptReconnect(): void {
    if (this.closed || this.reconnecting) return;
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      this.emit('error', new Error('Max reconnect attempts reached'));
      return;
    }

    this.reconnecting = true;
    this.reconnectAttempts++;

    const delay = Math.min(
      this.config.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1),
      this.config.maxReconnectDelay
    );

    setTimeout(async () => {
      this.reconnecting = false;
      if (!this.closed) {
        try {
          await this.connect();
        } catch (error) {
          // connect() will emit error, reconnect will be attempted on close
        }
      }
    }, delay);
  }
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Wait for a batch to complete with real-time progress updates
 *
 * @param client - Connected ProgressClient
 * @param batchId - Batch ID to wait for
 * @param options - Options including timeout and progress callback
 * @returns Final progress message when complete
 */
export async function waitForBatchCompletion(
  client: ProgressClient,
  batchId: string,
  options?: WaitOptions
): Promise<ProgressMessage> {
  const { timeoutMs = 300000, onProgress } = options ?? {};

  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      cleanup();
      reject(new Error(`Timeout waiting for batch ${batchId} (${timeoutMs}ms)`));
    }, timeoutMs);

    const handleProgress = (msg: ProgressMessage) => {
      if (msg.batch_id !== batchId) return;

      onProgress?.(msg);

      if (msg.type === 'complete' || msg.status === 'completed' || msg.status === 'completedwitherrors') {
        cleanup();
        resolve(msg);
      } else if (msg.type === 'error' || msg.status === 'failed') {
        cleanup();
        reject(new Error(msg.message ?? `Batch ${batchId} failed`));
      }
    };

    const handleError = (error: Error) => {
      cleanup();
      reject(error);
    };

    const cleanup = () => {
      clearTimeout(timeout);
      client.off('progress', handleProgress);
      client.off('complete', handleProgress);
      client.off('error', handleError);
    };

    client.on('progress', handleProgress);
    client.on('complete', handleProgress);
    client.on('error', handleError);

    // Subscribe to the batch
    client.subscribe(batchId);
  });
}

/**
 * Create a progress client and wait for batch with fallback to HTTP polling
 *
 * @param doorwayUrl - Doorway URL
 * @param batchId - Batch ID to wait for
 * @param options - Options
 * @param pollFn - Fallback polling function
 */
export async function waitWithFallback(
  doorwayUrl: string,
  batchId: string,
  options?: WaitOptions & { pollIntervalMs?: number },
  pollFn?: (batchId: string) => Promise<ProgressMessage | null>
): Promise<ProgressMessage> {
  const { pollIntervalMs = 5000, ...waitOptions } = options ?? {};

  const client = new ProgressClient(doorwayUrl);

  try {
    await client.connect();
    return await waitForBatchCompletion(client, batchId, waitOptions);
  } catch (wsError) {
    console.warn(`⚠️ WebSocket connection failed, falling back to HTTP polling: ${wsError}`);

    // Fallback to HTTP polling
    if (!pollFn) {
      throw wsError;
    }

    const startTime = Date.now();
    const timeoutMs = waitOptions?.timeoutMs ?? 300000;

    while (Date.now() - startTime < timeoutMs) {
      const status = await pollFn(batchId);
      if (!status) {
        await new Promise((r) => setTimeout(r, pollIntervalMs));
        continue;
      }

      waitOptions?.onProgress?.(status);

      if (status.status === 'completed' || status.status === 'completedwitherrors') {
        return status;
      } else if (status.status === 'failed') {
        throw new Error(`Batch ${batchId} failed`);
      }

      await new Promise((r) => setTimeout(r, pollIntervalMs));
    }

    throw new Error(`Timeout waiting for batch ${batchId}`);
  } finally {
    client.close();
  }
}
