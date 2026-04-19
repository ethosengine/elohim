/* Generated from protocol schema: views/elohim-capability-profile.schema.json -- DO NOT EDIT */

/**
 * Source of truth: operator-configured at elohim-storage startup (Operational, Category C). Not auto-detected — the operator declares the model running here. Phase 10+ may derive this automatically from a live elohim-agent-service.
 */
export interface ElohimCapabilityProfile {
  /**
   * Fully-qualified model name. E.g. claude-opus-4-7, llama-3.1-70b-q4, gpt-4o
   */
  modelName: string;
  /**
   * Model family/vendor. E.g. claude, llama, gpt, mistral, gemini
   */
  modelFamily: string;
  /**
   * Maximum input context in tokens for this model instance
   */
  contextWindowTokens: number;
  /**
   * CID of the constitution document priming this elohim. Null when no specific constitution is applied (base model behavior)
   */
  constitutionCid?: string | null;
  /**
   * Quantization descriptor. E.g. q4_K_M, q8_0, bf16, f32. Null for hosted/closed-weight models where quantization is not operator-visible
   */
  quantizationSpec?: string | null;
  /**
   * Free-form host descriptor indicating where this elohim runs. E.g. tauri-desktop-mac-arm64, elohim-node-linux-x86_64, doorway-hosted-cloud
   */
  deploymentContext?: string | null;
  /**
   * Domains this elohim is primed for via constitution or fine-tuning. E.g. child-safety, medical, code-review, curriculum-design. Free-form in Phase 9; Phase 10+ may enumerate a controlled vocabulary
   */
  specialties?: string[];
  /**
   * Named capabilities this elohim can dispatch. E.g. content-safety-review, discernment-evaluation, mastery-assessment. Maps to gate names or service names that gate-client uses for dispatch
   */
  skills?: string[];
  /**
   * Observed strengths accrued from attestation history. Populated by the protocol over time as gate decisions accumulate; operator-initialized as empty
   */
  strengths?: string[];
  /**
   * ISO 8601 timestamp when this elohim instance became active at this peer
   */
  activeSince: string;
  /**
   * Reach of this specific elohim instance. Distinct from the peer node's overall reach — an elohim running at a well-connected node may still have limited reach for certain gate decisions. Null until reach attestations accrue
   */
  reachLevel?: string | null;
}
