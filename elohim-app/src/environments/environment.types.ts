/**
 * Environment configuration types
 */

/**
 * Log level type for environment configuration
 */
export type LogLevel = 'debug' | 'info' | 'error';

/**
 * Connection mode for Holochain deployment
 *
 * - 'auto': Auto-detect based on environment (Tauri→direct, browser→doorway)
 * - 'doorway': Route through Doorway proxy (web deployments)
 * - 'direct': Connect directly to local conductor (native deployments)
 */
export type ConnectionMode = 'auto' | 'doorway' | 'direct';

/**
 * Holochain connection configuration
 */
export interface HolochainEnvironmentConfig {
  /** Admin WebSocket URL (may point to proxy) */
  adminUrl: string;
  /** App WebSocket URL (direct conductor access after auth) */
  appUrl: string;
  /** HTTP base URL for auth endpoints (admin-proxy /auth/*) */
  authUrl?: string;
  /** API key for admin proxy authentication */
  proxyApiKey?: string;
  /** Use local dev-proxy in Eclipse Che (auto-detected if true) */
  useLocalProxy?: boolean;
  /** Connection mode: auto-detect, force doorway, or force direct */
  connectionMode?: ConnectionMode;
  /** elohim-storage sidecar URL (for direct mode blob storage) */
  storageUrl?: string;
}

/**
 * Projection API configuration
 */
export interface ProjectionApiConfig {
  /** Whether projection API is enabled */
  enabled: boolean;
  /** Base URL for projection API (defaults to authUrl/api/v1/projection) */
  baseUrl?: string;
  /** Request timeout in ms */
  timeout?: number;
  /** Cache mode: prefer-projection (default) or prefer-holochain */
  cacheMode?: 'prefer-projection' | 'prefer-holochain';
}

/**
 * Environment configuration interface
 */
export interface Environment {
  production: boolean;
  logLevel: LogLevel;
  /** Environment name (development, alpha, staging, production) */
  environment: string;
  /** Git commit hash for version tracking */
  gitHash: string;
  holochain?: HolochainEnvironmentConfig;
  /** Projection API configuration (fast read path) */
  projectionApi?: ProjectionApiConfig;
  /** Doorway API base URL (defaults to same origin if not set) */
  doorwayUrl?: string;
}
