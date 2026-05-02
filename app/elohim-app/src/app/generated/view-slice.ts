/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/view-slice.schema.json -- DO NOT EDIT */

/**
 * Per-device slice returned over the view-federation/1.0.0 libp2p protocol; signed by the responding peer's agent key. Source of truth: composed at the responding peer from its local projection of the requested view kind (Operational, Category C — the meta-shape that federates cluster + topology views across household peers). Reconstructed per request; not persisted.
 */
export interface ViewSlice {
  /**
   * libp2p PeerId of the responding peer.
   */
  peerId: string;
  /**
   * Which view kind this slice represents — drives the payload's expected shape.
   */
  viewKind: 'cluster' | 'peer_topology';
  freshness: Freshness;
  /**
   * View-kind-specific slice body. Open shape at the envelope level — the consumer interprets payload per viewKind.
   */
  payload: {};
  /**
   * Base64 signature over canonical_bytes(viewKind, peerId, freshness, payload).
   */
  signature: string;
}
/**
 * Liveness/staleness of the slice as observed at the responding peer.
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
