/**
 * Circuit Breaker Service
 *
 * Implements the circuit breaker pattern to prevent cascading failures.
 * When a service fails repeatedly, the circuit "opens" and fails fast
 * instead of waiting for timeouts.
 *
 * States:
 * - CLOSED: Normal operation, requests pass through
 * - OPEN: Too many failures, requests fail immediately
 * - HALF_OPEN: Testing if service has recovered
 *
 * Usage:
 * ```typescript
 * const result = await this.circuitBreaker.execute(
 *   'holochain-zome',
 *   () => this.holochainClient.callZome(input)
 * );
 *
 * if (result.circuitOpen) {
 *   // Circuit is open, service is unavailable
 *   this.showOfflineMode();
 * }
 * ```
 */

import { Injectable, inject, signal } from '@angular/core';

// @coverage: 100.0% (2026-01-31)

import { LoggerService } from './logger.service';

// =============================================================================
// Types
// =============================================================================

/** Circuit breaker states */
export type CircuitState = 'CLOSED' | 'OPEN' | 'HALF_OPEN';

/** Configuration for a circuit breaker */
export interface CircuitBreakerConfig {
  /** Number of failures before opening circuit */
  failureThreshold: number;
  /** Time in ms before attempting recovery */
  resetTimeoutMs: number;
  /** Number of successful calls to close circuit */
  successThreshold: number;
  /** Time window for counting failures (ms) */
  failureWindowMs: number;
}

/** Result of a circuit breaker execution */
export interface CircuitBreakerResult<T> {
  /** Whether the call succeeded */
  success: boolean;
  /** Result data if successful */
  data?: T;
  /** Error message if failed */
  error?: string;
  /** Whether the circuit is currently open */
  circuitOpen: boolean;
  /** Current circuit state */
  state: CircuitState;
}

/** Internal circuit state */
interface CircuitInstance {
  state: CircuitState;
  failures: number[]; // Timestamps of failures
  successes: number; // Consecutive successes in half-open state
  lastFailure: number | null;
  lastStateChange: number;
  config: CircuitBreakerConfig;
}

// =============================================================================
// Default Configuration
// =============================================================================

const DEFAULT_CONFIG: CircuitBreakerConfig = {
  failureThreshold: 5,
  resetTimeoutMs: 30000, // 30 seconds
  successThreshold: 3,
  failureWindowMs: 60000, // 1 minute
};

// =============================================================================
// Service Implementation
// =============================================================================

@Injectable({ providedIn: 'root' })
export class CircuitBreakerService {
  private readonly logger = inject(LoggerService).createChild('CircuitBreaker');

  /** Registered circuits by name */
  private readonly circuits = new Map<string, CircuitInstance>();

  /** Observable circuit states */
  private readonly _circuitStates = signal<Map<string, CircuitState>>(new Map());

  /** Read-only access to circuit states */
  readonly circuitStates = this._circuitStates.asReadonly();

  // ===========================================================================
  // Public API
  // ===========================================================================

  /**
   * Register a new circuit breaker with custom configuration.
   */
  register(name: string, config?: Partial<CircuitBreakerConfig>): void {
    if (this.circuits.has(name)) {
      return; // Already registered
    }

    const circuit: CircuitInstance = {
      state: 'CLOSED',
      failures: [],
      successes: 0,
      lastFailure: null,
      lastStateChange: Date.now(),
      config: { ...DEFAULT_CONFIG, ...config },
    };

    this.circuits.set(name, circuit);
    this.updateStateSignal(name, 'CLOSED');
    this.logger.debug('Circuit registered', { name, config: circuit.config });
  }

  /**
   * Execute a function with circuit breaker protection.
   */
  async execute<T>(
    name: string,
    fn: () => Promise<T>,
    config?: Partial<CircuitBreakerConfig>
  ): Promise<CircuitBreakerResult<T>> {
    // Auto-register if not exists
    if (!this.circuits.has(name)) {
      this.register(name, config);
    }

    const circuit = this.circuits.get(name)!;

    // Check if circuit should transition
    this.checkStateTransition(name, circuit);

    // If circuit is OPEN, fail fast
    if (circuit.state === 'OPEN') {
      this.logger.debug('Circuit open, failing fast', { name });
      return {
        success: false,
        error: `Circuit breaker "${name}" is open`,
        circuitOpen: true,
        state: 'OPEN',
      };
    }

    // Execute the function
    try {
      const data = await fn();
      this.recordSuccess(name, circuit);
      return {
        success: true,
        data,
        circuitOpen: false,
        state: circuit.state,
      };
    } catch (err) {
      this.recordFailure(name, circuit, err);
      // State may have changed to OPEN after recordFailure
      // Use string comparison to avoid TypeScript's control flow narrowing
      const currentState = circuit.state as string;
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
        circuitOpen: currentState === 'OPEN',
        state: circuit.state,
      };
    }
  }

  /**
   * Get current state of a circuit.
   */
  getState(name: string): CircuitState | null {
    const circuit = this.circuits.get(name);
    if (!circuit) return null;

    // Check for state transitions before returning
    this.checkStateTransition(name, circuit);
    return circuit.state;
  }

  /**
   * Get statistics for a circuit.
   */
  getStats(name: string): {
    state: CircuitState;
    recentFailures: number;
    consecutiveSuccesses: number;
    timeSinceLastFailure: number | null;
    timeSinceStateChange: number;
  } | null {
    const circuit = this.circuits.get(name);
    if (!circuit) return null;

    // Clean old failures
    this.cleanOldFailures(circuit);

    return {
      state: circuit.state,
      recentFailures: circuit.failures.length,
      consecutiveSuccesses: circuit.successes,
      timeSinceLastFailure: circuit.lastFailure ? Date.now() - circuit.lastFailure : null,
      timeSinceStateChange: Date.now() - circuit.lastStateChange,
    };
  }

  /**
   * Force reset a circuit to CLOSED state.
   */
  reset(name: string): void {
    const circuit = this.circuits.get(name);
    if (!circuit) return;

    circuit.state = 'CLOSED';
    circuit.failures = [];
    circuit.successes = 0;
    circuit.lastStateChange = Date.now();
    this.updateStateSignal(name, 'CLOSED');
    this.logger.info('Circuit manually reset', { name });
  }

  /**
   * Get all registered circuit names.
   */
  getCircuitNames(): string[] {
    return Array.from(this.circuits.keys());
  }

  // ===========================================================================
  // Internal Methods
  // ===========================================================================

  private recordSuccess(name: string, circuit: CircuitInstance): void {
    if (circuit.state === 'HALF_OPEN') {
      circuit.successes++;

      if (circuit.successes >= circuit.config.successThreshold) {
        this.transitionTo(name, circuit, 'CLOSED');
        this.logger.info('Circuit closed after recovery', {
          name,
          successes: circuit.successes,
        });
      }
    }

    // Clear failures on success in CLOSED state
    if (circuit.state === 'CLOSED') {
      circuit.failures = [];
    }
  }

  private recordFailure(name: string, circuit: CircuitInstance, err: unknown): void {
    const now = Date.now();
    circuit.lastFailure = now;
    circuit.failures.push(now);
    circuit.successes = 0; // Reset success counter

    // Clean old failures outside the window
    this.cleanOldFailures(circuit);

    this.logger.debug('Failure recorded', {
      name,
      recentFailures: circuit.failures.length,
      threshold: circuit.config.failureThreshold,
      error: err instanceof Error ? err.message : String(err),
    });

    // Check if we should open the circuit
    if ((circuit.state === 'CLOSED' || circuit.state === 'HALF_OPEN') && circuit.failures.length >= circuit.config.failureThreshold) {
      this.transitionTo(name, circuit, 'OPEN');
      this.logger.warn('Circuit opened due to failures', {
        name,
        failures: circuit.failures.length,
      });
    }
  }

  private checkStateTransition(name: string, circuit: CircuitInstance): void {
    if (circuit.state === 'OPEN') {
      const timeSinceOpen = Date.now() - circuit.lastStateChange;

      if (timeSinceOpen >= circuit.config.resetTimeoutMs) {
        this.transitionTo(name, circuit, 'HALF_OPEN');
        this.logger.info('Circuit half-opened for testing', { name });
      }
    }
  }

  private transitionTo(name: string, circuit: CircuitInstance, newState: CircuitState): void {
    const oldState = circuit.state;
    circuit.state = newState;
    circuit.lastStateChange = Date.now();

    if (newState === 'CLOSED') {
      circuit.failures = [];
      circuit.successes = 0;
    } else if (newState === 'HALF_OPEN') {
      circuit.successes = 0;
    }

    this.updateStateSignal(name, newState);
    this.logger.debug('Circuit state transition', { name, from: oldState, to: newState });
  }

  private cleanOldFailures(circuit: CircuitInstance): void {
    const cutoff = Date.now() - circuit.config.failureWindowMs;
    circuit.failures = circuit.failures.filter(ts => ts > cutoff);
  }

  private updateStateSignal(name: string, state: CircuitState): void {
    this._circuitStates.update(map => {
      const newMap = new Map(map);
      newMap.set(name, state);
      return newMap;
    });
  }
}
