/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: commitments/replicates-dwelling.schema.json -- DO NOT EDIT */

/**
 * Payload shape for a Mishpat::Commitment entry with action='replicates-dwelling'. Storage-availability commitment between two dwelling-hubs. Does NOT presuppose recipient can decrypt — that's the encryption-layer (separate sprint). Source of truth: Holochain DHT (existing Mishpat::Commitment entry type, action discriminator). Spec: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md §4.
 */
export interface ReplicatesDwellingCommitment {
  action: 'replicates-dwelling';
  provider_dwelling_hub_id: string;
  recipient_dwelling_hub_id: string;
  provider_role: 'steward_mutual' | 'collective_steward';
  via_collective_hub_id?: string | null;
  capacity_bytes: number;
  scope_filter: {
    epr_kinds?: (
      | 'Content'
      | 'Manifest'
      | 'Claim'
      | 'Observation'
      | 'EconomicEvent'
      | 'Commitment'
      | 'Attestation'
      | 'Delegation'
      | 'FeedbackSignal'
    )[];
    bytes_per_blob_max?: number;
    requires_attestations?: string[];
    kinds_excluded?: string[];
  };
  valid_from: string;
  valid_until: string;
  grace_period_days: number;
  rotation_ttl_days: number;
  ratio_attestation: {
    commons_pct: number;
    dwelling_pct: number;
    collective_pct: number;
    free_pct: number;
    effective_ratio_cid: string;
  };
}
