/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/replica-peer.schema.json -- DO NOT EDIT */

/**
 * Hardware/deployment archetype carried on the AgentPeerBinding.
 */
export type DeviceArchetype = 'node' | 'desktop' | 'mobile' | 'steward';

/**
 * Per-peer replica row for a CID's distribution-details view. Source of truth: peer_identity_bindings (Category A from imagodei DHT, projected via signal stream) joined with libp2p swarm liveness (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface ReplicaPeer {
  /**
   * libp2p PeerId of the replica.
   */
  peerId: string;
  deviceArchetype: DeviceArchetype;
  /**
   * Seconds since this peer was last observed online.
   */
  lastSeenSeconds: number;
  /**
   * Approximate libp2p Kad hop distance from the requester.
   */
  hopHint?: number;
  /**
   * Household this peer belongs to, when known.
   */
  householdId?: string;
  /**
   * Coarse region tier (e.g. 'us-central') for diversity reasoning.
   */
  regionTier?: string;
}
