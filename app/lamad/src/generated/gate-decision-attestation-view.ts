/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/gate-decision-attestation-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: DHT (mishpat DNA, GateDecisionAttestation entry, Notarized, Category A). This record is a read-optimised projection populated by the MishpatSignal::GateDecisionCreated post-commit signal. If this projection and the DHT disagree, the DHT wins.
 */
export interface GateDecisionAttestationView {
  /**
   * CID of this attestation (self-addressing, globally unique)
   */
  decisionId: string;
  /**
   * Deployment phase under which the gate evaluated the request
   */
  phase: 'dev-context' | 'elohim-active';
  /**
   * AgentPubKey (base64) of the deciding elohim agent
   */
  elohimId: string;
  /**
   * CID of the substance declaration (model + constitution) active at decision time
   */
  elohimSubstanceCid: string;
  /**
   * Identifier of the gate that evaluated the request (e.g. 'discernment-gate-v1-mechanical')
   */
  gateName: string;
  /**
   * CID of the GateProcessDeclaration DAG that was executed
   */
  gateProcessCid: string;
  /**
   * Serialised RequestRef (opaque JSON string)
   */
  requestRefJson: string;
  /**
   * Outcome of the gate evaluation
   */
  decision: 'allow' | 'decline' | 'escalate' | 'verdict';
  /**
   * Full ConstitutionalReasoning (opaque JSON string)
   */
  reasoningJson: string;
  /**
   * CID of the privacy-respecting GateContext snapshot
   */
  contextSummaryCid: string;
  /**
   * ISO 8601 timestamp of when the decision was made
   */
  decidedAt: string;
  /**
   * CID of the universal-band DAG declaration active at decision time
   */
  universalBandCid: string;
  /**
   * ActionHash (base64) of the upstream DHT entry — client can use this to verify provenance
   */
  dhtAnchorHash: string;
  /**
   * ISO 8601 timestamp of when this projection row was inserted
   */
  createdAt: string;
}
