/* Generated from protocol schema: views/drain-status-view.schema.json -- DO NOT EDIT */

/**
 * Drain queue state for DHT publication. Source of truth: SQLite aggregate query over p2p_published_at column (Operational, Category C).
 */
export interface DrainStatusView {
  /**
   * Total rows in the local content projection (scoped to lamad app)
   */
  total: number;
  /**
   * Rows successfully published to libp2p Kad DHT
   */
  published: number;
  /**
   * Rows not yet drained. 0 and stable = caught up
   */
  pending: number;
}
