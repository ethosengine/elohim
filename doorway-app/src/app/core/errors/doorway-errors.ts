/**
 * Doorway Error Hierarchy
 *
 * Typed error classes for different failure modes in the doorway admin system.
 * Enables proper error handling, recovery strategies, and user feedback.
 */

/**
 * Base error class for all doorway-related errors
 */
export abstract class DoorwayError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly recoverable: boolean = false,
    public readonly cause?: Error
  ) {
    super(message);
    this.name = this.constructor.name;
    Object.setPrototypeOf(this, new.target.prototype);
  }

  /**
   * Get user-friendly error message
   */
  abstract getUserMessage(): string;
}

/**
 * Network-related errors (timeouts, connection failures)
 */
export class NetworkError extends DoorwayError {
  constructor(
    message: string,
    public readonly statusCode?: number,
    cause?: Error
  ) {
    super(message, 'NETWORK_ERROR', true, cause);
  }

  getUserMessage(): string {
    if (this.statusCode === 503) {
      return 'Service temporarily unavailable. The orchestrator may be disabled.';
    }
    if (this.statusCode && this.statusCode >= 500) {
      return 'Server error occurred. Please try again later.';
    }
    return 'Network connection failed. Please check your internet connection.';
  }
}

/**
 * Authentication/Authorization errors
 */
export class AuthenticationError extends DoorwayError {
  constructor(
    message: string,
    public readonly statusCode: number,
    cause?: Error
  ) {
    super(message, 'AUTH_ERROR', false, cause);
  }

  getUserMessage(): string {
    if (this.statusCode === 401) {
      return 'Authentication required. Please log in again.';
    }
    if (this.statusCode === 403) {
      return 'You do not have permission to perform this action.';
    }
    return 'Authentication failed.';
  }
}

/**
 * WebSocket connection errors
 */
export class WebSocketError extends DoorwayError {
  constructor(
    message: string,
    public readonly event?: Event,
    cause?: Error
  ) {
    super(message, 'WEBSOCKET_ERROR', true, cause);
  }

  getUserMessage(): string {
    return 'Real-time connection lost. Attempting to reconnect...';
  }
}

/**
 * Request timeout errors
 */
export class TimeoutError extends DoorwayError {
  constructor(
    message: string,
    public readonly timeoutMs: number,
    cause?: Error
  ) {
    super(message, 'TIMEOUT_ERROR', true, cause);
  }

  getUserMessage(): string {
    return `Request timed out after ${this.timeoutMs / 1000}s. Please try again.`;
  }
}

/**
 * Validation errors (bad request data)
 */
export class ValidationError extends DoorwayError {
  constructor(
    message: string,
    public readonly field?: string,
    cause?: Error
  ) {
    super(message, 'VALIDATION_ERROR', false, cause);
  }

  getUserMessage(): string {
    if (this.field) {
      return `Invalid value for ${this.field}: ${this.message}`;
    }
    return `Validation failed: ${this.message}`;
  }
}

/**
 * Resource not found errors
 */
export class NotFoundError extends DoorwayError {
  constructor(
    message: string,
    public readonly resourceType: string,
    public readonly resourceId: string,
    cause?: Error
  ) {
    super(message, 'NOT_FOUND', false, cause);
  }

  getUserMessage(): string {
    return `${this.resourceType} '${this.resourceId}' not found.`;
  }
}

/**
 * Rate limiting errors
 */
export class RateLimitError extends DoorwayError {
  constructor(
    message: string,
    public readonly retryAfterMs?: number,
    cause?: Error
  ) {
    super(message, 'RATE_LIMIT', true, cause);
  }

  getUserMessage(): string {
    if (this.retryAfterMs) {
      const seconds = Math.ceil(this.retryAfterMs / 1000);
      return `Rate limit exceeded. Please try again in ${seconds} seconds.`;
    }
    return 'Too many requests. Please slow down.';
  }
}

/**
 * Service unavailable (orchestrator disabled, maintenance mode)
 */
export class ServiceUnavailableError extends DoorwayError {
  constructor(
    message: string,
    public readonly reason?: string,
    cause?: Error
  ) {
    super(message, 'SERVICE_UNAVAILABLE', true, cause);
  }

  getUserMessage(): string {
    return this.reason ?? 'Service is currently unavailable. Please try again later.';
  }
}

/**
 * Unknown/unexpected errors
 */
export class UnknownError extends DoorwayError {
  constructor(message: string, cause?: Error) {
    super(message, 'UNKNOWN_ERROR', false, cause);
  }

  getUserMessage(): string {
    return 'An unexpected error occurred. Please contact support if this persists.';
  }
}

/**
 * Circuit breaker open (too many failures)
 */
export class CircuitBreakerOpenError extends DoorwayError {
  constructor(
    message: string,
    public readonly resetAfterMs: number,
    cause?: Error
  ) {
    super(message, 'CIRCUIT_BREAKER_OPEN', true, cause);
  }

  getUserMessage(): string {
    const seconds = Math.ceil(this.resetAfterMs / 1000);
    return `Service temporarily disabled due to repeated failures. Retry in ${seconds}s.`;
  }
}
