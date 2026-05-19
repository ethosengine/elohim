/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/distribution-summary.schema.json -- DO NOT EDIT */

/**
 * Inline per-CID distribution payload, hydrated onto every EPR/content response. ~100 bytes/item; drives badge state and simple-tier tooltips. Source of truth: composed from rea_commitments + economic_events using the T03d action conventions (Operational, Category C). Reconstructed per request; not persisted as a new entity.
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
