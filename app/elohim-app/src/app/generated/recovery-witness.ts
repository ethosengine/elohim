/* Generated from protocol schema: views/recovery-witness.schema.json -- DO NOT EDIT */

/**
 * Projection of imagodei HumanityWitness DHT entry when submitted under a RecoveryRequest (IntimateQuorum path). Source of truth: DHT.
 */
export interface RecoveryWitnessView {
  /**
   * ActionHash of the HumanityWitness entry (base64).
   */
  dhtAnchorHash: string;
  /**
   * ActionHash of the RecoveryRequest this witness is linked to (base64).
   */
  recoveryRequestHash: string;
  /**
   * AgentPubKey of the submitting authorizer (base64).
   */
  witnessAgentId: string;
  /**
   * Legacy String id of the human being witnessed for — mirrors the request's resolved human_id.
   */
  humanId: string;
  /**
   * Optional human-readable note from the authorizer (e.g., 'Sarah called me, recognized the dog's name').
   */
  note?: string | null;
  /**
   * ISO-8601 timestamp of witness submission.
   */
  submittedAt: string;
}
