/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/freshness.schema.json -- DO NOT EDIT */

/**
 * Liveness/staleness indicator carried on cluster + topology + slice views. Source of truth: composed from libp2p swarm liveness signals + slice timestamps (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface Freshness {
  /**
   * Liveness bucket. live = fresh signal; stale = past freshness window; offline = no signal; cached_offline_until_reconnect = served from cache awaiting reconnect; unverifiable = signal received but signature/freshness cannot be checked; all_offline = entire device set is offline.
   */
  state:
    | 'live'
    | 'stale'
    | 'offline'
    | 'cached_offline_until_reconnect'
    | 'unverifiable'
    | 'all_offline';
  /**
   * Milliseconds since the last fresh signal, when state != live.
   */
  staleSinceMs?: number;
}
