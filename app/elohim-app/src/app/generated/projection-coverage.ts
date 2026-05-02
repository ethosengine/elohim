/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/projection-coverage.schema.json -- DO NOT EDIT */

/**
 * Projection cache and lag aggregate for a doorway. Source of truth: doorway projection cache + REA observation events (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface ProjectionCoverage {
  /**
   * Number of CIDs the doorway is currently projecting.
   */
  projectedCidCount: number;
  /**
   * Total CIDs the doorway is aware of (projected + known but unprojected).
   */
  knownCidCount: number;
  /**
   * Trailing 24h cache hit ratio (0.0-1.0).
   */
  cacheHitRate24h: number;
  /**
   * Average milliseconds between source observation and projection ack over last 24h.
   */
  projectionLagMsAvg: number;
}
