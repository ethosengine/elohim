/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/network-posture-view.schema.json -- DO NOT EDIT */

/**
 * Aggregate network posture computed from live PeerStatus rows and stewarded_nodes projection. Source of truth: computed per-request (operational Category C); PeerStatus DHT entries in infrastructure DNA back the peer counts. No persistence, no new entry type.
 */
export interface NetworkPostureView {
  /**
   * Distinct peers with a PeerStatus row.
   */
  totalPeers: number;
  /**
   * Peers currently online or degraded but responding.
   */
  activePeers: number;
  /**
   * Peers whose last update is older than 120 s.
   */
  stalePeers: number;
  /**
   * Peers in the general pool + currently active.
   */
  alwaysOnPeers: number;
  /**
   * Distinct households with at least one active peer.
   */
  householdsReciprocating: number;
  /**
   * At least one active peer accepts stewardship reserves.
   */
  computeAvailable: boolean;
  /**
   * Aggregate shard-locations pressure (0-1). Placeholder until shard_locations is populated.
   */
  storagePressure: number;
  computedAt: string;
}
