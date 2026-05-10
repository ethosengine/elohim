/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/elohim-strength.schema.json -- DO NOT EDIT */

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
