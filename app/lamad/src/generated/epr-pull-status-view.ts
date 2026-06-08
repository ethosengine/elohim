/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/epr-pull-status.schema.json -- DO NOT EDIT */

/**
 * Per-EPR acquisition pull progress, grouped by head_ref with shared content counted once. Served on GET /api/v1/pins/{eprId}/pull (own node only). Source of truth: in-memory AcquisitionState GapTrackers (Operational, Category C). Recomputed per request from active pins × local inventory; not persisted.
 */
export interface EprPullStatusView {
  /**
   * The pinned head_ref this rollup groups. Echoed from the {eprId} path segment.
   */
  eprId: string;
  /**
   * Distinct desired content ids across all pins of this EPR (shared ids counted once). null = cannot compute (no resolved desired set yet) — the tri-state wait-for-drain contract; never caught up while null.
   */
  total: number | null;
  /**
   * Distinct content ids byte-arrival complete (deduped across pins of this EPR).
   */
  fetched: number;
  /**
   * Distinct content ids still in flight (deduped across pins of this EPR).
   */
  pending: number;
  /**
   * Distinct content ids that failed fetch (deduped). Not necessarily terminal — re-queued next cycle while fail_count < max_retries.
   */
  failed: number;
  /**
   * True only when total > 0 and every distinct desired id is fetched (byte-arrival, R-A). null when total is null. A failed/transiently-empty-pending item never reports caught up.
   */
  caughtUp: boolean | null;
}
