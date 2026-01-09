/**
 * Storage Response Models
 *
 * Type definitions for elohim-storage HTTP API responses.
 * These align with the backend service layer in holochain/elohim-storage/src/services/
 */

/**
 * Paginated list response from elohim-storage.
 * All list endpoints (GET /db/content, /db/paths, /db/relationships) return this shape.
 */
export interface ListResponse<T> {
  /** Array of items matching the query */
  items: T[];
  /** Number of items in this response (items.length) */
  count: number;
  /** Pagination limit that was applied */
  limit: number;
  /** Pagination offset that was applied */
  offset: number;
}

/**
 * Result from bulk create operations.
 * Returned by POST /db/content/bulk, /db/paths/bulk, /db/relationships/bulk
 */
export interface BulkCreateResult {
  /** Number of items successfully inserted */
  inserted: number;
  /** Number of items skipped (already exist) */
  skipped: number;
  /** Error messages for any failed items */
  errors?: string[];
}
