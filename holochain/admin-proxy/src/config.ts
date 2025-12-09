/**
 * Configuration loaded from environment variables
 */
export interface Config {
  /** WebSocket URL to the Holochain conductor */
  conductorUrl: string;
  /** Port to listen on */
  port: number;
  /** API key for authenticated access (normal dev operations) */
  apiKeyAuthenticated: string;
  /** API key for admin access (destructive operations) */
  apiKeyAdmin: string;
  /** Log level */
  logLevel: 'debug' | 'info' | 'warn' | 'error';
}

function getEnvOrThrow(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`Missing required environment variable: ${name}`);
  }
  return value;
}

function getEnvOrDefault(name: string, defaultValue: string): string {
  return process.env[name] ?? defaultValue;
}

export function loadConfig(): Config {
  return {
    conductorUrl: getEnvOrDefault('CONDUCTOR_URL', 'ws://localhost:4444'),
    port: parseInt(getEnvOrDefault('PORT', '8080'), 10),
    apiKeyAuthenticated: getEnvOrThrow('API_KEY_AUTHENTICATED'),
    apiKeyAdmin: getEnvOrThrow('API_KEY_ADMIN'),
    logLevel: getEnvOrDefault('LOG_LEVEL', 'info') as Config['logLevel'],
  };
}
