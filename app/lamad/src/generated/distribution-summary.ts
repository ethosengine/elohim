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
   * Health bucket: healthy = at/above target, at_risk = below target, critical = below floor, over_replicated = more replicas than commitments justify; release shards to reclaim budget.
   */
  replicaHealth: 'healthy' | 'at_risk' | 'critical' | 'over_replicated';
  /**
   * Number of doorways currently projecting this CID.
   */
  projectorCount: number;
  /**
   * Reach band the content is authored at — drives target replica count. Schema-8 vocabulary (elohim/sdk/schemas/v1/enums/reach.schema.json); see reach-vocabulary-frontend-strand backlog for the multi-vocabulary reconciliation history.
   */
  reachClass:
    | 'private'
    | 'self'
    | 'intimate'
    | 'trusted'
    | 'familiar'
    | 'community'
    | 'public'
    | 'commons';
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
  /**
   * Federation-level projection coverage. local = 1-2 projectors in same cluster; regional = 3+ projectors but ≤1 fault domain; global = projectors spanning ≥2 fault domains. Computed from projectorCount + projector geography when available.
   */
  projectionTier: 'local' | 'regional' | 'global';
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
