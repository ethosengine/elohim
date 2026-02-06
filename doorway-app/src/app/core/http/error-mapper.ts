/**
 * HTTP Error Mapper
 *
 * Maps HTTP errors to typed domain errors with proper recovery strategies.
 */

import { HttpErrorResponse } from '@angular/common/http';
import {
  DoorwayError,
  NetworkError,
  AuthenticationError,
  NotFoundError,
  ValidationError,
  RateLimitError,
  ServiceUnavailableError,
  TimeoutError,
  UnknownError,
} from '../errors/doorway-errors';

/**
 * Maps HttpErrorResponse to typed DoorwayError
 */
export class HttpErrorMapper {
  /**
   * Convert HTTP error to domain error
   */
  static mapError(error: HttpErrorResponse, operation: string): DoorwayError {
    // Timeout errors
    if (error.name === 'TimeoutError' || error.status === 408) {
      return new TimeoutError(
        `Operation '${operation}' timed out`,
        30000,
        error.error
      );
    }

    // Authentication/Authorization errors
    if (error.status === 401 || error.status === 403) {
      return new AuthenticationError(
        `Authentication failed for '${operation}': ${error.message}`,
        error.status,
        error.error
      );
    }

    // Not found errors
    if (error.status === 404) {
      const resourceId = this.extractResourceId(error);
      return new NotFoundError(
        `Resource not found in '${operation}'`,
        operation,
        resourceId,
        error.error
      );
    }

    // Validation errors
    if (error.status === 400 || error.status === 422) {
      const field = this.extractValidationField(error);
      return new ValidationError(
        error.error?.error ?? error.message,
        field,
        error.error
      );
    }

    // Rate limiting
    if (error.status === 429) {
      const retryAfter = this.extractRetryAfter(error);
      return new RateLimitError(
        `Rate limit exceeded for '${operation}'`,
        retryAfter,
        error.error
      );
    }

    // Service unavailable (orchestrator disabled, etc.)
    if (error.status === 503) {
      return new ServiceUnavailableError(
        `Service unavailable for '${operation}'`,
        'The orchestrator may be disabled on this doorway',
        error.error
      );
    }

    // Server errors
    if (error.status >= 500) {
      return new NetworkError(
        `Server error in '${operation}': ${error.message}`,
        error.status,
        error.error
      );
    }

    // Network/connection errors
    if (error.status === 0 || !error.status) {
      return new NetworkError(
        `Network error in '${operation}': ${error.message}`,
        undefined,
        error.error
      );
    }

    // Unknown errors
    return new UnknownError(
      `Unexpected error in '${operation}': ${error.message}`,
      error.error
    );
  }

  /**
   * Extract resource ID from error (if available)
   */
  private static extractResourceId(error: HttpErrorResponse): string {
    // Try to extract from URL path
    const urlParts = error.url?.split('/');
    return urlParts?.[urlParts.length - 1] ?? 'unknown';
  }

  /**
   * Extract validation field from error response
   */
  private static extractValidationField(error: HttpErrorResponse): string | undefined {
    // Check if error response has field information
    if (error.error?.field) {
      return error.error.field;
    }
    if (error.error?.errors?.[0]?.field) {
      return error.error.errors[0].field;
    }
    return undefined;
  }

  /**
   * Extract retry-after header value
   */
  private static extractRetryAfter(error: HttpErrorResponse): number | undefined {
    const retryAfter = error.headers.get('Retry-After');
    if (!retryAfter) return undefined;

    // Parse as seconds
    const seconds = parseInt(retryAfter, 10);
    if (!isNaN(seconds)) {
      return seconds * 1000;
    }

    // Parse as date
    const date = new Date(retryAfter);
    if (!isNaN(date.getTime())) {
      return date.getTime() - Date.now();
    }

    return undefined;
  }
}
