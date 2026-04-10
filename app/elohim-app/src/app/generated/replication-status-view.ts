/* Generated from protocol schema: views/replication-status-view.schema.json -- DO NOT EDIT */

/**
 * Identity-driven content replication progress. Source of truth: in-memory ReplicationInner struct (Operational, Category C). Rebuilt from discovery on every process start.
 */
export interface ReplicationStatusView {
  /**
   * Content IDs discovered but not yet fetched
   */
  pending: number;
  /**
   * Content IDs successfully replicated
   */
  completed: number;
  /**
   * Content IDs that failed fetch (will retry)
   */
  failed: number;
  /**
   * True when all discovered content has been fetched or failed with max retries
   */
  caughtUp: boolean;
}
