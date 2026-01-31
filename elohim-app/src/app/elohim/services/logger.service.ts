/**
 * Logger Service - Centralized structured logging for the Elohim app.
 *
 * Features:
 * - Log levels (debug, info, warn, error)
 * - Structured context objects
 * - Correlation ID support for request tracing
 * - Configurable output (console, future: remote aggregation)
 * - Performance timing utilities
 *
 * Usage:
 * ```typescript
 * // Basic logging
 * this.logger.info('User logged in', { userId: 'abc' });
 * this.logger.error('Failed to fetch content', error, { contentId: '123' });
 *
 * // With correlation ID (for request tracing)
 * this.logger.setCorrelationId('req-12345');
 * this.logger.info('Processing request'); // Automatically includes correlationId
 *
 * // Performance timing
 * const timer = this.logger.startTimer('zome-call');
 * await someAsyncOperation();
 * timer.end({ zomeName: 'content_store' }); // Logs duration automatically
 * ```
 */

import { Injectable, signal } from '@angular/core';

// @coverage: 3.2% (2026-02-05)

// =============================================================================
// Types
// =============================================================================

/** Log levels in order of severity */
export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

/** Numeric values for log level comparison */
const LOG_LEVEL_VALUES: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

/** Structured log entry */
export interface LogEntry {
  /** ISO timestamp */
  timestamp: string;
  /** Log level */
  level: LogLevel;
  /** Log message */
  message: string;
  /** Optional error object */
  error?: Error;
  /** Optional context data */
  context?: Record<string, unknown>;
  /** Correlation ID for request tracing */
  correlationId?: string;
  /** Source component/service */
  source?: string;
  /** Duration in ms (for timed operations) */
  durationMs?: number;
}

/** Timer instance for performance measurement */
export interface LogTimer {
  /** End the timer and log the duration */
  end(context?: Record<string, unknown>): void;
  /** Get elapsed time without ending */
  elapsed(): number;
}

/** Logger configuration */
export interface LoggerConfig {
  /** Minimum log level to output */
  minLevel: LogLevel;
  /** Whether to include timestamps in console output */
  includeTimestamp: boolean;
  /** Whether to output as JSON (for log aggregation) */
  jsonOutput: boolean;
  /** Default source name */
  defaultSource: string;
}

// =============================================================================
// Default Configuration
// =============================================================================

const DEFAULT_CONFIG: LoggerConfig = {
  minLevel: 'debug',
  includeTimestamp: true,
  jsonOutput: false,
  defaultSource: 'elohim-app',
};

// =============================================================================
// Logger Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class LoggerService {
  private config: LoggerConfig = { ...DEFAULT_CONFIG };
  private correlationId: string | null = null;
  private source: string | null = null;

  /** Recent log entries (for debugging/inspection) */
  private readonly recentLogs = signal<LogEntry[]>([]);
  private readonly maxRecentLogs = 100;

  // ==========================================================================
  // Configuration
  // ==========================================================================

  /**
   * Configure the logger.
   */
  configure(config: Partial<LoggerConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Set minimum log level.
   */
  setMinLevel(level: LogLevel): void {
    this.config.minLevel = level;
  }

  /**
   * Set correlation ID for request tracing.
   * All subsequent logs will include this ID.
   */
  setCorrelationId(id: string | null): void {
    this.correlationId = id;
  }

  /**
   * Get current correlation ID.
   */
  getCorrelationId(): string | null {
    return this.correlationId;
  }

  /**
   * Generate a new correlation ID.
   */
  generateCorrelationId(): string {
    const randomBytes = crypto.getRandomValues(new Uint8Array(8));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 9);
    const id = `${Date.now()}-${randomStr}`;
    this.correlationId = id;
    return id;
  }

  /**
   * Set source name for logs from this context.
   */
  setSource(source: string | null): void {
    this.source = source;
  }

  /**
   * Create a child logger with a specific source.
   * Inherits correlation ID but has its own source.
   */
  createChild(source: string): ChildLogger {
    return new ChildLogger(this, source);
  }

  // ==========================================================================
  // Log Methods
  // ==========================================================================

  /**
   * Log a debug message.
   */
  debug(message: string, context?: Record<string, unknown>): void {
    this.log('debug', message, undefined, context);
  }

  /**
   * Log an info message.
   */
  info(message: string, context?: Record<string, unknown>): void {
    this.log('info', message, undefined, context);
  }

  /**
   * Log a warning message.
   */
  warn(message: string, context?: Record<string, unknown>): void {
    this.log('warn', message, undefined, context);
  }

  /**
   * Log an error message.
   */
  error(message: string, error?: Error | unknown, context?: Record<string, unknown>): void {
    const err = error instanceof Error ? error : undefined;
    this.log('error', message, err, context);
  }

  // ==========================================================================
  // Performance Timing
  // ==========================================================================

  /**
   * Start a timer for measuring operation duration.
   */
  startTimer(operationName: string): LogTimer {
    const startTime = performance.now();

    return {
      end: (context?: Record<string, unknown>) => {
        const durationMs = Math.round(performance.now() - startTime);
        this.info(`${operationName} completed`, {
          ...context,
          durationMs,
          operation: operationName,
        });
      },
      elapsed: () => Math.round(performance.now() - startTime),
    };
  }

  /**
   * Measure an async operation.
   */
  async time<T>(
    operationName: string,
    operation: () => Promise<T>,
    context?: Record<string, unknown>
  ): Promise<T> {
    const timer = this.startTimer(operationName);
    try {
      const result = await operation();
      timer.end({ ...context, success: true });
      return result;
    } catch (err) {
      timer.end({
        ...context,
        success: false,
        error: err instanceof Error ? err.message : String(err),
      });
      throw err;
    }
  }

  // ==========================================================================
  // Log Inspection
  // ==========================================================================

  /**
   * Get recent log entries (for debugging).
   */
  getRecentLogs(): LogEntry[] {
    return this.recentLogs();
  }

  /**
   * Clear recent logs.
   */
  clearRecentLogs(): void {
    this.recentLogs.set([]);
  }

  // ==========================================================================
  // Internal
  // ==========================================================================

  /**
   * Core log method.
   */
  private log(
    level: LogLevel,
    message: string,
    error?: Error,
    context?: Record<string, unknown>
  ): void {
    // Check log level
    if (LOG_LEVEL_VALUES[level] < LOG_LEVEL_VALUES[this.config.minLevel]) {
      return;
    }

    const entry: LogEntry = {
      timestamp: new Date().toISOString(),
      level,
      message,
      error,
      context,
      correlationId: this.correlationId ?? undefined,
      source: this.source ?? this.config.defaultSource,
    };

    // Store in recent logs
    this.addToRecentLogs(entry);

    // Output to console
    this.outputToConsole(entry);
  }

  /**
   * Add entry to recent logs (circular buffer).
   */
  private addToRecentLogs(entry: LogEntry): void {
    this.recentLogs.update((logs: LogEntry[]) => {
      const newLogs = [...logs, entry];
      if (newLogs.length > this.maxRecentLogs) {
        return newLogs.slice(-this.maxRecentLogs);
      }
      return newLogs;
    });
  }

  /**
   * Output log entry to console.
   */
  private outputToConsole(entry: LogEntry): void {
    if (this.config.jsonOutput) {
      // JSON output for log aggregation
      // eslint-disable-next-line no-console
      console.log(JSON.stringify(entry));
      return;
    }

    // Human-readable output
    const parts: string[] = [];

    // Timestamp
    if (this.config.includeTimestamp) {
      const time = entry.timestamp.split('T')[1].split('.')[0];
      parts.push(`[${time}]`);
    }

    // Source
    if (entry.source) {
      parts.push(`[${entry.source}]`);
    }

    // Correlation ID
    if (entry.correlationId) {
      parts.push(`[${entry.correlationId.slice(0, 8)}]`);
    }

    // Message
    parts.push(entry.message);

    const prefix = parts.join(' ');

    // Build arguments list (only include non-empty values)
    const args: unknown[] = [prefix];
    if (entry.context) {
      args.push(entry.context);
    }
    if (entry.error) {
      args.push(entry.error);
    }

    // Choose console method based on level
    switch (entry.level) {
      case 'debug':
        // eslint-disable-next-line no-console
        console.debug(...args);
        break;

      case 'info':
        // eslint-disable-next-line no-console
        console.info(...args);
        break;

      case 'warn':
        console.warn(...args);
        break;

      case 'error':
        console.error(...args);
        break;
    }
  }
}

// =============================================================================
// Child Logger
// =============================================================================

/**
 * Child logger with a specific source.
 * Delegates to parent logger but overrides source.
 */
export class ChildLogger {
  constructor(
    private readonly parent: LoggerService,
    private readonly source: string
  ) {}

  debug(message: string, context?: Record<string, unknown>): void {
    this.parent.setSource(this.source);
    this.parent.debug(message, context);
    this.parent.setSource(null);
  }

  info(message: string, context?: Record<string, unknown>): void {
    this.parent.setSource(this.source);
    this.parent.info(message, context);
    this.parent.setSource(null);
  }

  warn(message: string, context?: Record<string, unknown>): void {
    this.parent.setSource(this.source);
    this.parent.warn(message, context);
    this.parent.setSource(null);
  }

  error(message: string, error?: Error | unknown, context?: Record<string, unknown>): void {
    this.parent.setSource(this.source);
    this.parent.error(message, error, context);
    this.parent.setSource(null);
  }

  startTimer(operationName: string): LogTimer {
    this.parent.setSource(this.source);
    const timer = this.parent.startTimer(operationName);
    this.parent.setSource(null);
    return timer;
  }
}
