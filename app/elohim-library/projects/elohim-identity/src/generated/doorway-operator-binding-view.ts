/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/doorway-operator-binding-view.schema.json -- DO NOT EDIT */

/**
 * Denormalized read shape for active doorway-operator authority. Source of truth: derived from rea_commitments where action='operate-doorway' and state='active' (Operational, Category C — reconstructed from the notarized Commitment entry type, no new physical projection table). Each binding ties an operator agent to a single doorway with a capability set, scope, and succession role. The doorway's auth layer queries this view at JWT refresh time and embeds the capability snapshot into the issued token.
 */
export interface DoorwayOperatorBindingView {
  /**
   * rea_commitments.id of the underlying Commitment entry.
   */
  commitmentId: string;
  /**
   * The doorway this operator authority applies to. Parsed from in_scope_of (the 'doorway:<id>' entry). MUST match an existing DoorwayRegistration.id.
   */
  doorwayId: string;
  /**
   * Agent public key of the operator (Commitment.provider).
   */
  operatorAgent: string;
  /**
   * Active capability strings carried by this binding (Commitment.resource_classified_as).
   */
  capabilities: string[];
  /**
   * Role within the doorway's succession plan (parsed from Commitment.metadata_json).
   */
  successionRole: 'primary' | 'deputy' | 'recovery-witness';
  /**
   * Who can audit this binding (parsed from Commitment.metadata_json).
   */
  reachScope: 'operator-private' | 'stewards-only' | 'public';
  /**
   * Commitment.state.
   */
  state: 'active' | 'breached' | 'completed' | 'cancelled';
  /**
   * The doorway's stewardship Agreement (Commitment.clause_of). One Agreement umbrellas all operator commitments for a doorway.
   */
  agreementId: string;
  /**
   * ISO8601 — when this operator authority takes effect.
   */
  hasBeginning?: string | null;
  /**
   * ISO8601 — when this operator authority lapses (null = open-ended).
   */
  hasEnd?: string | null;
  /**
   * ActionHash of the underlying Commitment entry on the DHT. Category A discipline: a binding without dht_anchor_hash is invalid.
   */
  dhtAnchorHash: string;
  /**
   * ActionHash of the current HardwareCustodyAttestation this operator binding serves under. Null during the schemaVersion=1 transition window; non-null once the doorway has bootstrapped its custody attestation. The auth resolver compares this against the latest valid custody attestation for the doorway; mismatch indicates the binding is orphaned by a custody transition and the caller MUST re-authenticate.
   */
  custodyAttestationHash?: string | null;
  /**
   * ActionHash of the current StewardOfRecordAttestation this operator binding serves under. Null during transition. Independently revocable from custody; a stewardship transfer orphans operator commitments without affecting custody.
   */
  stewardAttestationHash?: string | null;
  /**
   * ISO8601 — when this binding was projected.
   */
  createdAt: string;
}
