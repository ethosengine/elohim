/* Generated from protocol schema: views/elohim-capability-profile.schema.json -- DO NOT EDIT */

/**
 * Domains an elohim is primed for via constitution or fine-tuning. Core specialties are protocol-known and affect dispatch routing. Extensible specialties are community-declared and accepted by storage without DNA validation.
 */
export type ElohimSpecialty =
  | 'child-safety'
  | 'family-dynamics'
  | 'content-safety'
  | 'discernment'
  | 'reach-evaluation'
  | 'medical'
  | 'legal'
  | 'crisis'
  | 'education'
  | 'code-review'
  | 'financial-advice'
  | 'creative-writing'
  | 'technical-documentation'
  | 'translation'
  | 'poetry'
  | 'philosophy'
  | 'governance'
  | 'conflict-resolution'
  | 'curriculum-design'
  | 'counseling'
  | 'theology'
  | 'science'
  | 'mathematics'
  | 'history'
  | 'language-learning'
  | 'mental-health'
  | 'disability-support'
  | 'elder-care'
  | 'research'
  | 'journalism';
/**
 * Named gate capabilities an elohim can dispatch. Core skills map directly to registered gate interfaces the protocol orchestrates (content-safety-gate, discernment-gate-v1-mechanical, reach-gate-v1). Extensible skills map to the full ElohimCapability variant set and any community-registered capabilities.
 */
export type ElohimSkill =
  | 'content-safety-review'
  | 'discernment-evaluation'
  | 'reach-negotiation'
  | 'attestation-recommendation'
  | 'spiral-detection'
  | 'care-connection'
  | 'graduated-intervention'
  | 'constitutional-verification'
  | 'accuracy-verification'
  | 'knowledge-map-synthesis'
  | 'affinity-analysis'
  | 'path-recommendation'
  | 'cross-layer-validation'
  | 'existential-boundary-enforcement'
  | 'governance-ratification'
  | 'path-analysis'
  | 'learning-objective-validation'
  | 'prerequisite-verification'
  | 'mastery-assessment-design'
  | 'family-value-alignment'
  | 'personal-agent-support'
  | 'feedback-profile-negotiation'
  | 'feedback-profile-enforcement'
  | 'feedback-profile-upgrade'
  | 'feedback-profile-downgrade'
  | 'place-attestation'
  | 'place-naming-governance'
  | 'geographic-reach-assignment'
  | 'bioregional-enforcement';
/**
 * Observed strengths accrued from attestation history. Core strengths are protocol-named reputation dimensions the imagodei layer tracks. Extensible strengths are community-named emergent patterns. Phase 10 ships a minimal core vocabulary; Phase 11+ will codify additional values from real attestation-history analysis.
 */
export type ElohimStrength =
  | 'high-confidence-judgments'
  | 'appeals-sustained'
  | 'consensus-alignment'
  | 'novel-pattern-detection'
  | 'steady-baseline'
  | 'consistent-constitutional-reasoning'
  | 'low-false-positive-rate'
  | 'fast-resolution'
  | 'cross-context-consistency'
  | 'escalation-accuracy';

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
   * Domains this elohim is primed for via constitution or fine-tuning. Values constrained to elohim-specialty vocabulary (Task 10.3). Core values affect gate dispatch routing; extensible values inform peer-diversity selection
   */
  specialties?: ElohimSpecialty[];
  /**
   * Named gate capabilities this elohim can dispatch. Values constrained to elohim-skill vocabulary (Task 10.3). Core values map to active gate interfaces (content-safety-gate, discernment-gate-v1-mechanical, reach-gate-v1); extensible values map to ElohimCapability variants
   */
  skills?: ElohimSkill[];
  /**
   * Observed strengths accrued from attestation history. Values constrained to elohim-strength vocabulary (Task 10.3). Populated by the protocol over time as gate decisions accumulate; operator-initialized as empty
   */
  strengths?: ElohimStrength[];
  /**
   * ISO 8601 timestamp when this elohim instance became active at this peer
   */
  activeSince: string;
  /**
   * Reach of this specific elohim instance. Distinct from the peer node's overall reach — an elohim running at a well-connected node may still have limited reach for certain gate decisions. Null until reach attestations accrue
   */
  reachLevel?: string | null;
}
