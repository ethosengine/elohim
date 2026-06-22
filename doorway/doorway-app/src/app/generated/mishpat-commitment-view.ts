/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/mishpat-commitment-view.schema.json -- DO NOT EDIT */

/**
 * Read projection of a single notarized mishpat Commitment (the compute/recovery-class ledger). One row of the `mishpat_commitments` table — the previously write-only/unread-surfaced ledger the REA economic facing reads. Operational Category C — no new DHT entry type; the Commitment itself is already notarized (`Mishpat::Commitment`, cid = entry_hash) and this is a read-only projection over the existing operational table.
 */
export interface MishpatCommitmentView {
  /**
   * Commitment content address — the Holochain entry_hash of the Commitment entry. The natural join key (economic_events.bounded_by references this).
   */
  cid: string;
  /**
   * Action discriminator: delegates-compute | replicates-dwelling | provide | project-epr | custody-blob | … (mishpat coordinator vocabulary; never a closed whitelist).
   */
  action: string;
  /**
   * The commitment's own scope / in_scope_of. NOT the classification list (membership is matched against resource_classified_as, never this field).
   */
  scope: string;
  /**
   * Provider agent identity — an agent_cid (uhCAk…), never a transport id.
   */
  provider: string;
  /**
   * Recipient agent identity — an agent_cid (uhCAk…), never a transport id.
   */
  recipient: string;
  /**
   * Action-specific policy envelope (rate limits, reach ceiling, rotation TTL, …). Parsed from the bounds_json column; an opaque object whose shape varies by action — deliberately NOT additionalProperties:false. Absent (field omitted) when the column held no parseable object.
   */
  bounds?: Record<string, unknown>;
  /**
   * ISO-8601 start of the commitment validity window.
   */
  validFrom: string;
  /**
   * ISO-8601 end of the commitment validity window.
   */
  validUntil: string;
  /**
   * Lifecycle state: proposed | active | … (commitment_backed requires active).
   */
  state: string;
  /**
   * DHT provenance — the action_hash of the notarized Commitment entry. Optional: omitted (field absent) on an un-notarized row (a NULL dht_anchor_hash, on which ProjectionCommitmentFetcher refuses a bounds-pass — the local-stack dht-anchor gap).
   */
  dhtAnchorHash?: string;
  /**
   * ISO-8601 timestamp at which this projection row was written.
   */
  createdAt: string;
}
