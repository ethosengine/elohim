/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/distribution-details.schema.json -- DO NOT EDIT */

/**
 * Hardware/deployment archetype carried on the AgentPeerBinding.
 */
export type DeviceArchetype = 'node' | 'desktop' | 'mobile' | 'steward';

/**
 * Lazy-fetched per-CID developer-grade distribution view. Strict superset of DistributionSummary: includes per-replica peer rows, projector identities, placement gaps, and recent projection events. Source of truth: composed from rea_commitments + economic_events + peer_identity_bindings + libp2p swarm state (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface DistributionDetails {
  summary: DistributionSummary;
  /**
   * Per-replica detail (peer id, archetype, freshness, hop hint, etc).
   */
  replicaPeers: ReplicaPeer[];
  /**
   * Per-doorway projection ack rows.
   */
  projectorIdentities: ProjectorIdentity[];
  /**
   * Open-shape placement-gap records describing missing target capacity. Schema kept loose during the lit-up topology bring-up; will graduate to a dedicated schema when stable.
   */
  placementGaps: {}[];
  /**
   * Recent rea projection-event records relevant to this CID. Open-shape during bring-up.
   */
  recentProjectionEvents: {}[];
  /**
   * CustodianCommitment CID references (rea_commitments rows) backing this distribution snapshot.
   */
  commitmentReferences?: string[];
}
/**
 * Inline summary — same shape returned on every CID-bearing response.
 */
export interface DistributionSummary {
  /**
   * Number of replicas currently observed for this CID.
   */
  replicaCount: number;
  /**
   * Target replica count derived from the CID's reach + policy.
   */
  replicaTarget: number;
  /**
   * Health bucket: healthy = at/above target, at_risk = below target, critical = below floor.
   */
  replicaHealth: 'healthy' | 'at_risk' | 'critical';
  /**
   * Number of doorways currently projecting this CID.
   */
  projectorCount: number;
  /**
   * Reach band the content is authored at — drives target replica count.
   */
  reachClass:
    | 'private'
    | 'intimate'
    | 'household'
    | 'neighborhood'
    | 'collective'
    | 'community'
    | 'district'
    | 'public';
  diversityHint: DiversityHint;
  /**
   * Where the bytes for this fetch came from.
   */
  thisFetchSource: 'projected_via_doorway' | 'peer_direct' | 'local_pantry';
  /**
   * Seconds since the last custodian probe verified the replica.
   */
  lastVerifiedSeconds: number;
  /**
   * Viewer's role for this CID, when known.
   */
  myRole?: 'sole_replica' | 'replica' | 'replica_and_projector' | 'not_hosting';
  /**
   * Net diff: positive = viewer hosts more than is hosted for them.
   */
  reciprocityHint?: number;
}
/**
 * Peer-diversity hint for the replica set.
 */
export interface DiversityHint {
  /**
   * Which diversity dimension the hint addresses.
   */
  kind: 'region_metro' | 'household_archetypes' | 'collective_member_count' | 'none';
  /**
   * Hint payload. Array of region/archetype strings, an integer count, or null/absent when kind=none. Serde tag/content adjacent tagging omits this key for unit variants.
   */
  value?: string[] | number | null;
}
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
/**
 * Per-doorway projector row for a CID's distribution-details view. Source of truth: doorway projection registry observed via libp2p (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface ProjectorIdentity {
  /**
   * Public hostname of the projecting doorway (e.g. 'matthew.elohim.host').
   */
  doorwayHostname: string;
  /**
   * Seconds since the projector last acknowledged this CID.
   */
  lastAckSeconds: number;
  /**
   * Coarse region tier of the doorway, when known.
   */
  regionTier?: string;
}
