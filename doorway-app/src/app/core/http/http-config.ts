/**
 * HTTP Configuration
 *
 * Centralized configuration for HTTP requests, retries, and timeouts.
 */

/**
 * Retry strategy configuration
 */
export interface RetryConfig {
  /** Maximum number of retry attempts */
  maxAttempts: number;
  /** Delay in ms before first retry */
  initialDelayMs: number;
  /** Maximum delay in ms between retries */
  maxDelayMs: number;
  /** Backoff multiplier (exponential backoff) */
  backoffMultiplier: number;
  /** Which HTTP status codes should trigger a retry */
  retryableStatusCodes: number[];
}

/**
 * Timeout configuration
 */
export interface TimeoutConfig {
  /** Default timeout for all requests */
  defaultMs: number;
  /** Timeout for read operations */
  readMs: number;
  /** Timeout for write operations */
  writeMs: number;
  /** Timeout for long-running operations */
  longRunningMs: number;
}

/**
 * Circuit breaker configuration
 */
export interface CircuitBreakerConfig {
  /** Number of failures before opening circuit */
  failureThreshold: number;
  /** Time in ms to wait before attempting to close circuit */
  resetTimeoutMs: number;
  /** Time window in ms for counting failures */
  rollingWindowMs: number;
}

/**
 * Complete HTTP client configuration
 */
export interface HttpClientConfig {
  retry: RetryConfig;
  timeout: TimeoutConfig;
  circuitBreaker: CircuitBreakerConfig;
}

/**
 * Default HTTP configuration
 */
export const DEFAULT_HTTP_CONFIG: HttpClientConfig = {
  retry: {
    maxAttempts: 2,
    initialDelayMs: 1000,
    maxDelayMs: 5000,
    backoffMultiplier: 2,
    retryableStatusCodes: [408, 429, 500, 502, 503, 504],
  },
  timeout: {
    defaultMs: 30000,
    readMs: 30000,
    writeMs: 60000,
    longRunningMs: 120000,
  },
  circuitBreaker: {
    failureThreshold: 5,
    resetTimeoutMs: 60000,
    rollingWindowMs: 120000,
  },
};

/**
 * Operation types for selecting appropriate timeout
 */
export type OperationType = 'read' | 'write' | 'long-running';
