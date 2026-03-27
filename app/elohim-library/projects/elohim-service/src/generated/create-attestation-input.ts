/* Generated from protocol schema: inputs/create-attestation-input.schema.json -- DO NOT EDIT */

/**
 * Input for creating content attestations. Must match Rust CreateAttestationInputView in views.rs. Attestations are Category A (DHT-notarized) — they are the protocol's trust/reach mechanism.
 */
export interface CreateAttestationInput {
  /**
   * Content being attested
   */
  contentId: string;
  /**
   * ContributorPresence making the attestation
   */
  attestorPresenceId: string;
  /**
   * Scope of attestation (e.g. quality, accuracy, relevance)
   */
  scope: string;
  /**
   * Type of attestation (e.g. peer-review, steward-endorsement, community-validation)
   */
  attestationType: string;
  /**
   * Supporting evidence for the attestation (parsed object)
   */
  evidence?: {};
  /**
   * Grantor context — who authorized this attestation and under what governance model (parsed object)
   */
  grantor?: {};
}
