/* Generated from protocol schema: inputs/wisdom-invocation-input.schema.json -- DO NOT EDIT */

/**
 * Source of truth: elohim-agent-service (Operational, Category C). Input payload for POST /wisdom/invoke. Not persisted; the service produces a WisdomInvocationResponse that may be used for DHT attestation in Phase 9+.
 */
export interface WisdomInvocationInput {
  /**
   * CID of the constitution ContentNode that governs this wisdom invocation
   */
  constitutionCid: string;
  /**
   * CID of the framing ContentNode that scopes the specific wisdom question
   */
  framingCid: string;
  /**
   * Which GateContext keys the wisdom call should consider
   */
  contextKeys: string[];
  /**
   * Serialized subset of GateContext values for the declared contextKeys — pre-assembled by the gate executor
   */
  contextJson: string;
  /**
   * Short natural-language description of the relational impact event being judged
   */
  eventSummary: string;
}
