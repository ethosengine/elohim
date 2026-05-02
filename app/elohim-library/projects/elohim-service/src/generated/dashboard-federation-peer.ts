/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/dashboard-federation-peer.schema.json -- DO NOT EDIT */

/**
 * Per-doorway federation row in a doorway dashboard. Source of truth: doorway-to-doorway federation registry observed via libp2p (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface DashboardFederationPeer {
  doorwayHostname: string;
  online: boolean;
  /**
   * Which way bytes flow across this federation edge.
   */
  direction: 'bidirectional' | 'outbound_only' | 'inbound_only';
  /**
   * Count of CIDs both doorways project.
   */
  sharedCidCount: number;
}
