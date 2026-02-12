/**
 * Circuit Breaker Pattern Implementation
 *
 * Prevents cascading failures by temporarily stopping requests
 * to a failing service, allowing it time to recover.
 */

import { CircuitBreakerConfig } from './http-config';
import { CircuitBreakerOpenError } from '../errors/doorway-errors';

/**
 * Circuit breaker states
 */
export enum CircuitState {
  CLOSED = 'CLOSED', // Normal operation
  OPEN = 'OPEN', // Blocking requests
  HALF_OPEN = 'HALF_OPEN', // Testing if service recovered
}

/**
 * Failure record for rolling window
 */
interface FailureRecord {
  timestamp: number;
  error: Error;
}

/**
 * Circuit breaker for protecting against cascading failures
 */
export class CircuitBreaker {
  private state: CircuitState = CircuitState.CLOSED;
  private failures: FailureRecord[] = [];
  private lastStateChange: number = Date.now();
  private consecutiveSuccesses = 0;

  constructor(
    private readonly name: string,
    private readonly config: CircuitBreakerConfig
  ) {}

  /**
   * Check if request should be allowed
   */
  async execute<T>(fn: () => Promise<T>): Promise<T> {
    // Clean up old failures outside rolling window
    this.cleanupFailures();

    // Check circuit state
    if (this.state === CircuitState.OPEN) {
      // Check if enough time has passed to try again
      const timeSinceOpen = Date.now() - this.lastStateChange;
      if (timeSinceOpen >= this.config.resetTimeoutMs) {
        this.state = CircuitState.HALF_OPEN;
        this.lastStateChange = Date.now();
      } else {
        const resetIn = this.config.resetTimeoutMs - timeSinceOpen;
        throw new CircuitBreakerOpenError(
          `Circuit breaker '${this.name}' is OPEN`,
          resetIn
        );
      }
    }

    try {
      const result = await fn();
      this.recordSuccess();
      return result;
    } catch (error) {
      this.recordFailure(error as Error);
      throw error;
    }
  }

  /**
   * Record successful request
   */
  private recordSuccess(): void {
    if (this.state === CircuitState.HALF_OPEN) {
      this.consecutiveSuccesses++;
      // After successful test, close the circuit
      if (this.consecutiveSuccesses >= 1) {
        this.state = CircuitState.CLOSED;
        this.lastStateChange = Date.now();
        this.failures = [];
        this.consecutiveSuccesses = 0;
      }
    } else {
      this.consecutiveSuccesses = 0;
    }
  }

  /**
   * Record failed request
   */
  private recordFailure(error: Error): void {
    this.failures.push({
      timestamp: Date.now(),
      error,
    });

    this.consecutiveSuccesses = 0;

    // Check if we should open the circuit
    if (
      this.state !== CircuitState.OPEN &&
      this.failures.length >= this.config.failureThreshold
    ) {
      this.state = CircuitState.OPEN;
      this.lastStateChange = Date.now();
      console.error(
        `[CircuitBreaker:${this.name}] OPEN - Too many failures (${this.failures.length})`
      );
    }
  }

  /**
   * Remove failures outside the rolling window
   */
  private cleanupFailures(): void {
    const cutoff = Date.now() - this.config.rollingWindowMs;
    this.failures = this.failures.filter((f) => f.timestamp >= cutoff);
  }

  /**
   * Get current state
   */
  getState(): CircuitState {
    return this.state;
  }

  /**
   * Get failure count in current window
   */
  getFailureCount(): number {
    this.cleanupFailures();
    return this.failures.length;
  }

  /**
   * Reset circuit breaker (for testing or manual intervention)
   */
  reset(): void {
    this.state = CircuitState.CLOSED;
    this.failures = [];
    this.consecutiveSuccesses = 0;
    this.lastStateChange = Date.now();
  }
}
