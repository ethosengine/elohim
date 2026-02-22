/**
 * Console Logger - Default logger for connection strategies.
 *
 * Provides level-filtered console output when no Angular LoggerService is available.
 * Used as the default in elohim-library (framework-agnostic) contexts.
 *
 * @packageDocumentation
 */

import type { Logger, LogLevel } from './connection-strategy';

/** Numeric log level for comparison */
const LOG_LEVEL_ORDER: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

/**
 * Console-based logger with level filtering.
 */
export class ConsoleLogger implements Logger {
  private readonly minLevel: number;

  constructor(
    private readonly prefix: string,
    level: LogLevel = 'info'
  ) {
    this.minLevel = LOG_LEVEL_ORDER[level];
  }

  debug(message: string, context?: Record<string, unknown>): void {
    if (this.minLevel <= LOG_LEVEL_ORDER.debug) {
      if (context) {
        console.debug(`[${this.prefix}] ${message}`, context);
      } else {
        console.debug(`[${this.prefix}] ${message}`);
      }
    }
  }

  info(message: string, context?: Record<string, unknown>): void {
    if (this.minLevel <= LOG_LEVEL_ORDER.info) {
      if (context) {
        console.log(`[${this.prefix}] ${message}`, context);
      } else {
        console.log(`[${this.prefix}] ${message}`);
      }
    }
  }

  warn(message: string, context?: Record<string, unknown>): void {
    if (this.minLevel <= LOG_LEVEL_ORDER.warn) {
      if (context) {
        console.warn(`[${this.prefix}] ${message}`, context);
      } else {
        console.warn(`[${this.prefix}] ${message}`);
      }
    }
  }

  error(message: string, error?: unknown, context?: Record<string, unknown>): void {
    if (this.minLevel <= LOG_LEVEL_ORDER.error) {
      if (error && context) {
        console.error(`[${this.prefix}] ${message}`, error, context);
      } else if (error) {
        console.error(`[${this.prefix}] ${message}`, error);
      } else {
        console.error(`[${this.prefix}] ${message}`);
      }
    }
  }
}
