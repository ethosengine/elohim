/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/bounds-validation-result-view.schema.json -- DO NOT EDIT */

/**
 * Wire shape for POST /api/v1/diagnostics/validate-bounds. Reports whether a given EconomicEvent passes bounds validation against the Commitment named by its bounded_by field. Source of truth: pure function output; no persisted entity. Walks bounded_by -> Commitment (Holochain DHT) -> checks {active, scope-includes-event, reach-ceiling-respected, rate-not-exceeded, key-rotation-current, revoked?}.
 */
export interface BoundsValidationResultView {
  /**
   * True iff all bounds checks passed.
   */
  pass: boolean;
  /**
   * CID of the Commitment that was walked.
   */
  commitment_cid: string;
  /**
   * Populated iff pass=false. Names the first violation encountered.
   */
  violation?: {
    kind:
      | 'commitment_inactive'
      | 'scope_not_included'
      | 'reach_ceiling_exceeded'
      | 'rate_limit_exceeded'
      | 'key_rotation_stale'
      | 'commitment_revoked'
      | 'commitment_not_found';
    /**
     * Human-readable explanation including any relevant numeric bounds.
     */
    summary: string;
  } | null;
  /**
   * Per-check status. All true iff pass=true; first false short-circuits and populates violation.
   */
  checks: {
    commitment_found: boolean;
    active: boolean;
    scope_includes_event: boolean;
    reach_ceiling_ok: boolean;
    rate_within_limit: boolean;
    key_rotation_current: boolean;
    not_revoked: boolean;
  };
}
