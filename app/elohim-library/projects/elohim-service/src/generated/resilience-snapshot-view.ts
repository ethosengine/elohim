/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/resilience-snapshot-view.schema.json -- DO NOT EDIT */

/**
 * Resilience claim consumed by <elohim-resilience-snapshot>. Collective-general: stewards of the content can be any collective kind (household, church, patron-circle, DAO, …) — any group that holds DHT-notarized REA commitments under the social-compute epic. Source of truth: computed projection. Operational Category C.
 */
export interface ResilienceSnapshotView {
  contentId: string;
  /**
   * unmeasured = content has never entered the distribution plane (no shard manifest) — every count below is a non-measurement, NOT a measured zero. measured = a shard manifest exists and the counts are real. Renderers MUST show a distinct 'not yet distributed' state for unmeasured, never a fake at-risk verdict. Diagnostic tell preserved: a measured content with regionless stewards puts steward counts in regionalDistribution.unknown, so unknown == 0 with all-zeros implies unmeasured.
   */
  distributionState: 'unmeasured' | 'measured';
  /**
   * Distinct collectives (of any kind) with at least one peer holding this content.
   */
  stewardingCollectives: number;
  /**
   * Distinct collectives with an active REA commitment covering this content.
   */
  commitmentBackedCollectives: number;
  /**
   * 0..1 — ratio of achieved distinct-collective placements to requested.
   */
  diversityScore: number;
  regionalDistribution: {
    local: number;
    regional: number;
    global: number;
    unknown: number;
  };
  placementGaps: PlacementGapView[];
  protectionStatus: 'at-risk' | 'partial' | 'protected';
  /**
   * Collectives that both steward this content AND are stewarded by the viewer's collective (mutual reciprocation). Optional; omitted when no viewer context is supplied.
   */
  reciprocatingCollectives?: number;
  details?: {
    /**
     * Collectives currently stewarding this content, with their kind so the UI can label per-kind ('household', 'church', 'patron-circle', etc.).
     */
    stewardingCollectives?: {
      id: string;
      /**
       * Collective kind: 'household', 'church', 'patron-circle', 'dao', or any future collective kind declared in the collectives table.
       */
      kind: string;
      /**
       * Optional human-readable display label.
       */
      label?: string;
      /**
       * Distinct agents/devices WITHIN this collective holding the content — the intra-hub resiliency lens. stewardingCollectives (top-level) is the inter-hub count; this is the intra count per entry. Folded from the holder-relation's retained per-agent dimension (2026-06-19-resilience-facings design).
       */
      intraHubPeers?: number;
    }[];
    /**
     * Live/known peer pair — honest denominators. live = peers with an active online|degraded PeerStatus across the stewarding collectives; known = stewarded nodes registered across those collectives. Renderers show 'live/known peers live' (e.g. '2/3'); '0/0' only when genuinely nothing is known.
     */
    onlinePeers?: {
      live: number;
      known: number;
    };
    healthScore?: number;
  };
  /**
   * How many additional distinct-collective slots short of the diversity floor (CoverageRollup deficit measure). Derived in the graph-backed branch via CoverageRollup.deficit.measure(). Present with value 0 when the graph branch measures the floor exactly met; absent (not-selected) only on the relational household_resilience::snapshot path.
   */
  coverageShortfall?: number;
  /**
   * Household-addressed felt projection of the resilience claim — names, not nines. Computed in household_resilience.rs::snapshot(). Honest by construction: floor-relative (reassurance is measured against the content's resilience floor, not a flat threshold) and unmeasured-aware (distributionState 'unmeasured' → reassurance 'not-yet-seen', never a fake verdict). The inverse of the operator debug Lens; consumed by <elohim-memory-safety>.
   */
  feltStatus?: {
    /**
     * Canonical content-neutral felt sentence — the single source of truth for the translation. The consuming component adds warm framing ('your memories', the album name); it never re-translates the state.
     */
    headline: string;
    /**
     * Floor-relative felt state. not-yet-seen = unmeasured (never entered the distribution plane). protected = meets the floor with no active gap. watching = a holder lapsed but coverage survives, OR below the floor but still holding. needs-help = critically thin (<=1 household).
     */
    reassurance: 'protected' | 'watching' | 'needs-help' | 'not-yet-seen';
    /**
     * The collectives currently holding this content, with display labels — names, not nines. Same shape as details.stewardingCollectives.
     */
    heldBy: {
      id: string;
      kind: string;
      /**
       * Human-readable collective name (collectives.name).
       */
      label?: string;
      /**
       * Distinct agents/devices WITHIN this collective holding the content — the intra-hub resiliency lens (same shape as details.stewardingCollectives).
       */
      intraHubPeers?: number;
    }[];
    /**
     * The resilience FLOOR this content is measured against — the content-relative denominator. Expresses 'held by M of the N homes this should live in'. See genesis/data/timeline/backlog/resilience-tier-content-declared-floor.md.
     */
    floor: {
      /**
       * Resilience tier (value-driven, owner-declarable): 'standard' (default) until the content declares its own value-tier (vault/keepsake/ephemeral/...). NOT derived from reach.
       */
      tier: string;
      /**
       * false = the tier is a default, not the owner's declared intent (honesty marker — the surface must not imply the household chose this floor).
       */
      tierDeclared: boolean;
      /**
       * Households this tier wants holding the content (the floor).
       */
      wantsHouseholds: number;
      /**
       * Households currently holding it (the achievement against the floor).
       */
      hasHouseholds: number;
    };
    /**
     * One pro-social action offered when protection is short or unseen (e.g. 'Invite a household to help hold these'). Omitted when none needed.
     */
    suggestedAction?: string;
  };
}
/**
 * Structured shefa signal: a content item's achieved placement falls short of its requested stewarding-collective diversity. The 'steward' can be any collective kind (household, church, patron-circle, DAO, …) — any group that can hold DHT-notarized REA commitments. Source of truth: computed projection from shard_locations + rea_commitments + humans → collectives. Operational Category C — no DHT entry.
 */
export interface PlacementGapView {
  /**
   * UUID of this gap record.
   */
  id: string;
  contentId: string;
  shardHash: string;
  /**
   * Target number of distinct stewarding collectives (any kind) the placement should reach.
   */
  requestedStewardCount: number;
  /**
   * Actual number of distinct stewarding collectives that accepted the shard.
   */
  achievedStewardCount: number;
  /**
   * Fraction of requested diversity backed by active REA commitments.
   */
  contractCoverage: number;
  gapKind: 'under-committed' | 'contracts-short' | 'peers-unavailable';
  firstSeenAt: string;
  lastSeenAt: string;
}
