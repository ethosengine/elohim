/**
 * Environment configuration types
 */

/**
 * Log level type for environment configuration
 */
export type LogLevel = 'debug' | 'info' | 'error';

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
}

/**
 * Environment configuration interface
 */
export interface Environment {
  production: boolean;
  apiUrl: string;
  lamadApiUrl: string;
  logLevel: LogLevel;
  holochain?: HolochainEnvironmentConfig;
}
