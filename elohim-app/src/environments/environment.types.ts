/**
 * Environment configuration types
 */

/**
 * Log level type for environment configuration
 */
export type LogLevel = 'debug' | 'info' | 'error';

/**
 * Environment configuration interface
 */
export interface Environment {
  production: boolean;
  apiUrl: string;
  lamadApiUrl: string;
  logLevel: LogLevel;
}
