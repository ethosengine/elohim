/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/dashboard-steward.schema.json -- DO NOT EDIT */

/**
 * Hardware/deployment archetype of the device presenting an AgentPeerBinding. Source of truth: DHT (Notarized, Category A — imagodei integrity zome). DNA-notarized as part of the AgentPeerBinding entry type. Values: 'node' = dedicated elohim-node server/blade; 'desktop' = Tauri desktop app (workstation or laptop); 'mobile' = mobile client (phone or tablet); 'steward' = steward process managing collective infrastructure.
 */
export type DeviceArchetype = 'node' | 'desktop' | 'mobile' | 'steward';

/**
 * Per-storage-steward row in a doorway dashboard. Source of truth: peer_identity_bindings (Category A from imagodei DHT) joined with hosting counts derived from rea_commitments (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface DashboardSteward {
  peerId: string;
  archetype: DeviceArchetype;
  displayName?: string;
  online: boolean;
  /**
   * Count of CIDs this steward currently hosts.
   */
  hostingCount: number;
  /**
   * Approximate libp2p Kad hop distance from the doorway.
   */
  hopHint?: number;
}
