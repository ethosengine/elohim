/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/lens-market-view.schema.json -- DO NOT EDIT */

/**
 * The plural-Mishpat-lens market over one EPR — the lenses governing it, their context-relative affinity, the controversy (contention) spread, the regime-drift status, and any open fresh-lens bounty. Source of truth: assembled read projection (Operational, Category C). Lens identities are DHT-notarized Mishpat Commitments (Category A); affinity, contention, and regimeStatus are reconstructable facing folds over the notarized selection/verdict tallies. Reconstructed per request; not persisted. GET /api/v1/epr/{eprScope}/lens-market.
 */
export interface LensMarketView {
  /**
   * The EPR slug-id this market governs (e.g. 'epr:lamad-spa') — the scope key, NOT the dag-cbor CID (plan A3).
   */
  eprScope: string;
  /**
   * The lenses governing this EPR, side by side (plural, co-valid — a shared table of readings, never a ranked leaderboard). Ordered by affinity desc with a deterministic lensCid tie-break; an invalid lens is present with valid=false (degraded row), never dropped.
   */
  lenses: {
    /**
     * The lens entry_hash (= CID, the read/scope key; NEVER action_hash — plan A5 gospel).
     */
    lensCid: string;
    /**
     * The collective / school-of-thought label (e.g. 'georgist', 'beerian'); feeds the contributor-card attribution.
     */
    school: string;
    /**
     * Governance role: 'lens' | 'floor' | 'ceiling' (view-local status string).
     */
    role: string;
    /**
     * Short human summary of what this lens steers toward (its telos).
     */
    telosSummary: string;
    /**
     * Earned affinity in THIS EPR context — distinct attested selectors (gaming-resistant). 0 = warm-but-not-active here (folds::lens_affinity).
     */
    affinityInContext: number;
    /**
     * This lens's current deterministic reading over the EPR; null when the lens has not yet fired in this context.
     */
    currentVerdict: string | null;
    /**
     * Fail-closed flag — false means the lens row is degraded (poisoned/unparseable) and excluded from selection, but still surfaced (the EprRouter-empties lesson; plan A5).
     */
    valid: boolean;
  }[];
  /**
   * Controversy SPREAD over this EPR in [0,1]: 1.0 = perfectly split (max contention), 0.0 = unanimous/none. NOT a net score (folds::lens_contention). Feeds the ControversyBadge.
   */
  contentionIndex: number;
  /**
   * Cross-surface regime-drift status: 'stable' | 'drifting' | 'breached'. 'breached' is the JOINT affinity-decay AND contention-rise condition (folds::lens_selector::classify_regime — the net-new primitive) that opens a bounty.
   */
  regimeStatus: string;
  /**
   * Commitment CID of an open fresh-lens pull-bounty for this EPR (opened on a ContentionBreach), or null when none is open.
   */
  openBountyCid: string | null;
  /**
   * RFC3339 timestamp this projection was assembled (Operational-C; reconstructable, not persisted).
   */
  computedAt: string;
}
