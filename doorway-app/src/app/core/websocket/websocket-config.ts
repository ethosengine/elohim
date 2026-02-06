/**
 * WebSocket Configuration
 *
 * Configuration for WebSocket connections, reconnection, and heartbeat.
 */

/**
 * Reconnection strategy configuration
 */
export interface ReconnectionConfig {
  /** Enable automatic reconnection */
  enabled: boolean;
  /** Maximum number of reconnection attempts (0 = infinite) */
  maxAttempts: number;
  /** Initial delay in ms before first reconnect */
  initialDelayMs: number;
  /** Maximum delay in ms between reconnects */
  maxDelayMs: number;
  /** Backoff multiplier for exponential backoff */
  backoffMultiplier: number;
}

/**
 * Heartbeat configuration
 */
export interface HeartbeatConfig {
  /** Enable heartbeat/ping */
  enabled: boolean;
  /** Interval in ms between pings */
  intervalMs: number;
  /** Timeout in ms to wait for pong response */
  timeoutMs: number;
}

/**
 * Complete WebSocket configuration
 */
export interface WebSocketConfig {
  reconnection: ReconnectionConfig;
  heartbeat: HeartbeatConfig;
  /** Connection timeout in ms */
  connectionTimeoutMs: number;
}

/**
 * Default WebSocket configuration
 */
export const DEFAULT_WEBSOCKET_CONFIG: WebSocketConfig = {
  reconnection: {
    enabled: true,
    maxAttempts: 10,
    initialDelayMs: 1000,
    maxDelayMs: 30000,
    backoffMultiplier: 1.5,
  },
  heartbeat: {
    enabled: true,
    intervalMs: 30000,
    timeoutMs: 5000,
  },
  connectionTimeoutMs: 10000,
};
