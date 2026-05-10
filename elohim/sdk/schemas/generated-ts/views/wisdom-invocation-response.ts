/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/wisdom-invocation-response.schema.json -- DO NOT EDIT */

/**
 * Source of truth: elohim-agent-service inference result (Operational, Category C). The phase field is the authoritative marker: 'elohim-active' ONLY when a real LLM call produced this response; 'dev-context' when the service returned a stub (no API key, backend unavailable, or error fallback). This phase is the signal downstream gate code uses for attestation phase-marking.
 */
export interface WisdomInvocationResponse {
  /**
   * Wisdom's verdict on the relational impact event
   */
  decision: 'allow' | 'decline' | 'escalate' | 'verdict' | 'need-deeper';
  /**
   * Authoritative phase marker. 'elohim-active' ONLY if a real LLM inference produced this response. 'dev-context' if stubbed, degraded, or no backend configured.
   */
  phase: 'dev-context' | 'elohim-active';
  /**
   * Reasoning audit trail for the wisdom decision
   */
  reasoning: {
    /**
     * The constitutional principle the wisdom call applied as its primary guide
     */
    primaryPrinciple: string;
    /**
     * Confidence score for the decision (0.0 = no confidence / stub; 1.0 = fully confident)
     */
    confidence: number;
    /**
     * Human-readable explanation of the reasoning
     */
    summary: string;
    /**
     * Contextual note about the phase — for dev-context: explains stub behaviour; for elohim-active: may be empty
     */
    phaseNote: string;
  };
  /**
   * Side effects to apply as a result of the verdict. Reserved for Phase 9+ when verdicts carry executable side-effect payloads. Callers MUST ignore unknown entries.
   */
  sideEffects?: unknown[];
}
