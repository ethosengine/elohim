/**
 * Configuration loaded from environment variables
 *
 * Supports two modes:
 * - Production (DEV_MODE=false): Requires API keys, filters operations
 * - Development (DEV_MODE=true): No auth, passthrough all operations
 */
export interface Config {
  /** Development mode - skip auth and message filtering */
  devMode: boolean;
  /** WebSocket URL to the Holochain conductor admin interface */
  conductorUrl: string;
  /** Port to listen on */
  port: number;
  /** Admin interface port on conductor (for routing) */
  adminPort: number;
  /** Minimum allowed app interface port */
  appPortMin: number;
  /** Maximum allowed app interface port */
  appPortMax: number;
  /** API key for authenticated access (required in production mode) */
  apiKeyAuthenticated?: string;
  /** API key for admin access (required in production mode) */
  apiKeyAdmin?: string;
  /** Log level */
  logLevel: 'debug' | 'info' | 'warn' | 'error';

  // ==========================================================================
  // JWT Authentication (for hosted humans)
  // ==========================================================================

  /** Secret for signing JWT tokens (required in production) */
  jwtSecret: string;
  /** JWT token expiry in seconds (default: 3600 = 1 hour) */
  jwtExpirySeconds: number;
  /** Enable password-based authentication */
  enablePasswordAuth: boolean;
}

function getEnvOrDefault(name: string, defaultValue: string): string {
  return process.env[name] ?? defaultValue;
}

function parsePortRange(rangeStr: string): { min: number; max: number } {
  const [minStr, maxStr] = rangeStr.split('-');
  return {
    min: parseInt(minStr, 10),
    max: parseInt(maxStr ?? minStr, 10),
  };
}

export function loadConfig(): Config {
  const devMode = getEnvOrDefault('DEV_MODE', 'false').toLowerCase() === 'true';
  const portRange = parsePortRange(getEnvOrDefault('APP_PORT_RANGE', '4445-65535'));
  const enablePasswordAuth = getEnvOrDefault('ENABLE_PASSWORD_AUTH', 'true').toLowerCase() === 'true';

  // In production mode, API keys are required
  if (!devMode) {
    if (!process.env.API_KEY_AUTHENTICATED) {
      throw new Error('Missing required environment variable: API_KEY_AUTHENTICATED (required when DEV_MODE is not true)');
    }
    if (!process.env.API_KEY_ADMIN) {
      throw new Error('Missing required environment variable: API_KEY_ADMIN (required when DEV_MODE is not true)');
    }
  }

  // JWT secret: required in production, use default for dev
  const jwtSecret = process.env.JWT_SECRET ?? (devMode ? 'dev-jwt-secret-do-not-use-in-production' : '');
  if (!devMode && !jwtSecret) {
    throw new Error('Missing required environment variable: JWT_SECRET (required when DEV_MODE is not true)');
  }

  return {
    devMode,
    conductorUrl: getEnvOrDefault('CONDUCTOR_URL', 'ws://localhost:4444'),
    port: parseInt(getEnvOrDefault('PORT', '8080'), 10),
    adminPort: parseInt(getEnvOrDefault('CONDUCTOR_ADMIN_PORT', '4444'), 10),
    appPortMin: portRange.min,
    appPortMax: portRange.max,
    apiKeyAuthenticated: process.env.API_KEY_AUTHENTICATED,
    apiKeyAdmin: process.env.API_KEY_ADMIN,
    logLevel: getEnvOrDefault('LOG_LEVEL', 'info') as Config['logLevel'],

    // JWT settings
    jwtSecret,
    jwtExpirySeconds: parseInt(getEnvOrDefault('JWT_EXPIRY_SECONDS', '3600'), 10),
    enablePasswordAuth,
  };
}

/**
 * Detect if running in Eclipse Che environment.
 */
export function isCheEnvironment(): boolean {
  return !!(
    process.env.CHE_WORKSPACE_NAME ||
    process.env.DEVWORKSPACE_ID ||
    process.env.DEVFILE_FILENAME
  );
}

/**
 * Get Eclipse Che workspace info if available.
 */
export function getCheInfo(): { workspaceName?: string; workspaceId?: string } | null {
  if (!isCheEnvironment()) return null;

  return {
    workspaceName: process.env.CHE_WORKSPACE_NAME,
    workspaceId: process.env.DEVWORKSPACE_ID,
  };
}
