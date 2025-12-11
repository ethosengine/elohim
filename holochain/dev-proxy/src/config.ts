/**
 * Configuration for the Holochain dev proxy.
 * Loaded from environment variables with sensible defaults for local development.
 */

export interface Config {
  /** Port to listen on */
  port: number;
  /** Admin interface port on conductor (for local mode) */
  adminPort: number;
  /** Minimum allowed app interface port */
  appPortMin: number;
  /** Maximum allowed app interface port */
  appPortMax: number;
  /** Log level */
  logLevel: 'debug' | 'info' | 'warn' | 'error';
  /** Remote conductor URL (if set, proxies to remote instead of localhost) */
  conductorUrl?: string;
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
  const portRange = parsePortRange(
    getEnvOrDefault('ALLOWED_APP_PORTS', '4445-4500')
  );

  // CONDUCTOR_URL allows proxying to a remote conductor/proxy instead of localhost
  const conductorUrl = process.env.CONDUCTOR_URL || undefined;

  return {
    port: parseInt(getEnvOrDefault('DEV_PROXY_PORT', '8888'), 10),
    adminPort: parseInt(getEnvOrDefault('CONDUCTOR_ADMIN_PORT', '4444'), 10),
    appPortMin: portRange.min,
    appPortMax: portRange.max,
    logLevel: getEnvOrDefault('LOG_LEVEL', 'info') as Config['logLevel'],
    conductorUrl,
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
