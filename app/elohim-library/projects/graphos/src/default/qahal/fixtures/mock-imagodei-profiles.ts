/**
 * mock-imagodei-profiles.ts
 *
 * Typed mock imagodei profiles for the Tier-0 worked examples:
 *   - Dowell household (Matthew, Jessica, James, Sheila, Gertrude)
 *   - CofC congregation elders (Brother Cal, Thompson, Davis, Rhodes)
 *   - Life-group families (Hardins, Lees, Robertsons, Kims)
 *   - Arkansas sister-congregation elder (the concern-surface moment in narrative 4.4)
 *
 * Grounded in storyteller canonical narratives at:
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md
 *
 * IDs use kebab-case stable identifiers for downstream fixture composition.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * Standing tier within a given Qahal — earned through lamad-attested mastery
 * on the Bloom curve. Computed per-Qahal at request time; this field is the
 * mock stand-in for the derived StandingView.
 */
export type StandingTier = 'visitor' | 'engaged' | 'contributor' | 'steward';

/**
 * Bloom's taxonomy tier — six-level mastery curve underlying all standing computation.
 * Each level maps to a capability surface declared in the Qahal's rubric.
 */
export type BloomTier =
  | 'remember'
  | 'understand'
  | 'apply'
  | 'analyze'
  | 'evaluate'
  | 'create';

/**
 * Capability tier — governs what a human can see and do, including protected-tier
 * overrides for those whose agency requires support (children, elders under guardianship,
 * IDD members, humans under legal stewardship). These are distinct from Bloom tiers:
 * a human can have full standing in a care-economy while still being `elder_under_guardianship`
 * for external-link exposure purposes.
 */
export type CapabilityTier =
  | 'visitor'
  | 'engaged'
  | 'contributor'
  | 'steward'
  | 'elohim-support'
  | 'child'
  | 'idd_member'
  | 'elder_under_guardianship'
  | 'legal_steward_protected';

/**
 * A mock imagodei profile — the protocol-level identity surface for a human.
 * In production, this is a computed view (Category C) derived from imagodei
 * DHT entries. These mocks provide stable fixtures for Library B pattern stories.
 */
export interface MockImagodeiProfile {
  /** Stable kebab-case identifier for downstream fixture composition. */
  id: string;
  /** Human display name. */
  name: string;
  /** Optional avatar URL. If absent, UI renders initials. */
  avatarUrl?: string;
  /**
   * Standing tier within the primary worked-example Qahal for this profile.
   * Downstream stories may override per Qahal context.
   */
  standingTier: StandingTier;
  /**
   * Bloom mastery tier — the human's attested level in the rubric's mastery curve.
   * Steward-tier standing implies Bloom-Create or Bloom-Evaluate at minimum.
   */
  bloomTier: BloomTier;
  /**
   * Capability tier — governs UI affordances. Protected tiers (child, elder_under_guardianship,
   * idd_member, legal_steward_protected) override capability regardless of standing.
   */
  capabilityTier: CapabilityTier;
  /**
   * IDs of Qahals this human is affiliated with. Used for left-nav collective-switcher
   * and imagodei lens composition.
   */
  affiliations: string[];
}

// ---------------------------------------------------------------------------
// Dowell family
// ---------------------------------------------------------------------------

/**
 * Matthew Dowell — household co-steward, congregation member of eleven years,
 * life-group participant. The point-of-view character in the canonical narratives.
 * Steward standing in the household; contributor standing in the congregation.
 */
const matthewDowell: MockImagodeiProfile = {
  id: 'matthew-dowell',
  name: 'Matthew Dowell',
  standingTier: 'steward',
  bloomTier: 'create',
  capabilityTier: 'steward',
  affiliations: [
    'dowell-household',
    'cofc-congregation',
    'hardins-life-group',
    'wisdom-commons',
  ],
};

/**
 * Jessica Dowell — household co-steward alongside Matthew. Full ring this week;
 * the narrative notes she took the morning shift with sick James without prompting.
 */
const jessicaDowell: MockImagodeiProfile = {
  id: 'jessica-dowell',
  name: 'Jessica Dowell',
  standingTier: 'steward',
  bloomTier: 'create',
  capabilityTier: 'steward',
  affiliations: ['dowell-household', 'cofc-congregation', 'hardins-life-group'],
};

/**
 * James Dowell — Matthew and Jessica's eleven-year-old son. His household standing
 * is small this week (sore throat, hard week of school), but the family knows it
 * will be larger next week. Protected capability tier: child — external links are
 * hidden per household rubric configuration.
 */
const jamesDowell: MockImagodeiProfile = {
  id: 'james-dowell',
  name: 'James Dowell',
  standingTier: 'engaged',
  bloomTier: 'remember',
  capabilityTier: 'child',
  affiliations: ['dowell-household'],
};

/**
 * Sheila Hardin (née Dowell) — Matthew's sister. Core-family visibility into the
 * household without being a member. The narrative shows her as the person who
 * sends recipes and makes chicken-and-rice soup when someone is sick. Contributor
 * standing in the household care-economy.
 */
const sheilaHardin: MockImagodeiProfile = {
  id: 'sheila-hardin',
  name: 'Sheila Hardin',
  standingTier: 'contributor',
  bloomTier: 'apply',
  capabilityTier: 'contributor',
  affiliations: ['dowell-household', 'cofc-congregation', 'hardins-life-group'],
};

/**
 * Gertrude Dowell — Matthew's mother, elder under guardianship. Lives half a
 * continent away; her hub holds a recovery share for the Dowells; the Dowells'
 * hub holds one for hers. External links are filtered through the co-steward per
 * household rubric (elder_under_guardianship protected tier).
 */
const gertrudeDowell: MockImagodeiProfile = {
  id: 'gertrude-dowell',
  name: 'Gertrude Dowell',
  standingTier: 'contributor',
  bloomTier: 'understand',
  capabilityTier: 'elder_under_guardianship',
  affiliations: ['dowell-household'],
};

/**
 * DOWELL_FAMILY — the five members of the Dowell household Qahal as the canonical
 * narrative renders them: Matthew and Jessica as co-stewards, James as their child,
 * Sheila as core-family, Gertrude as elder under guardianship.
 */
export const DOWELL_FAMILY: MockImagodeiProfile[] = [
  matthewDowell,
  jessicaDowell,
  jamesDowell,
  sheilaHardin,
  gertrudeDowell,
];

// ---------------------------------------------------------------------------
// CofC elders
// ---------------------------------------------------------------------------

/**
 * Brother Cal — one of four elders at Matthew's local Churches of Christ congregation.
 * The narrative shows him anchoring a Romans 12 sermon series (now in its fourth week).
 * He surfaces the concern regarding the Arkansas sister congregation in narrative 4.4.
 * The friction-gradient watches that he does not accumulate disproportionate attestation
 * weight among the four elders.
 */
const brotherCal: MockImagodeiProfile = {
  id: 'brother-cal',
  name: 'Brother Cal',
  standingTier: 'steward',
  bloomTier: 'create',
  capabilityTier: 'steward',
  affiliations: ['cofc-congregation', 'wisdom-commons'],
};

/**
 * Elder Thompson — one of the four plural stewards of the congregation.
 * Named in the congregation Qahal's rubric as a co-signer.
 */
const elderThompson: MockImagodeiProfile = {
  id: 'elder-thompson',
  name: 'Elder Thompson',
  standingTier: 'steward',
  bloomTier: 'create',
  capabilityTier: 'steward',
  affiliations: ['cofc-congregation', 'wisdom-commons'],
};

/**
 * Elder Davis — one of the four plural stewards of the congregation.
 * Named in the congregation Qahal's rubric as a co-signer.
 */
const elderDavis: MockImagodeiProfile = {
  id: 'elder-davis',
  name: 'Elder Davis',
  standingTier: 'steward',
  bloomTier: 'create',
  capabilityTier: 'steward',
  affiliations: ['cofc-congregation', 'wisdom-commons'],
};

/**
 * Elder Rhodes — one of the four plural stewards of the congregation.
 * Named in the congregation Qahal's rubric as a co-signer.
 */
const elderRhodes: MockImagodeiProfile = {
  id: 'elder-rhodes',
  name: 'Elder Rhodes',
  standingTier: 'steward',
  bloomTier: 'create',
  capabilityTier: 'steward',
  affiliations: ['cofc-congregation', 'wisdom-commons'],
};

/**
 * COFC_ELDERS — the four plural stewards of Matthew's local congregation. They share
 * stewardship as peers; none is ranked above the others. The substrate watches for
 * accumulation and the commons-elohim writes a small line when one begins to carry
 * more attestation-weight than the others.
 */
export const COFC_ELDERS: MockImagodeiProfile[] = [
  brotherCal,
  elderThompson,
  elderDavis,
  elderRhodes,
];

// ---------------------------------------------------------------------------
// Life-group families
// ---------------------------------------------------------------------------

/**
 * John Hardin — host steward of the Tuesday-evening life-group. The narrative notes
 * he has hosted twenty-three of the last twenty-four Tuesdays; the substrate gently
 * prompts him to invite another household to host, and he says yes.
 */
const johnHardin: MockImagodeiProfile = {
  id: 'john-hardin',
  name: 'John Hardin',
  standingTier: 'steward',
  bloomTier: 'create',
  capabilityTier: 'steward',
  affiliations: ['cofc-congregation', 'hardins-life-group'],
};

/**
 * Jin Lee — member of the Lee household in the life-group. The Lees are present
 * in person on Tuesday evening and offer to host the next meeting when John's
 * hosting accumulation is gently surfaced.
 */
const jinLee: MockImagodeiProfile = {
  id: 'jin-lee',
  name: 'Jin Lee',
  standingTier: 'contributor',
  bloomTier: 'apply',
  capabilityTier: 'contributor',
  affiliations: ['cofc-congregation', 'hardins-life-group'],
};

/**
 * Robertson household representative — the Robertsons are traveling and dial in
 * by video. They speak to what it means to be a body presented when physically
 * absent from usual fellowship (the Romans 12 discussion).
 */
const robertsonTraveling: MockImagodeiProfile = {
  id: 'robertson-traveling',
  name: 'Robertson (traveling)',
  standingTier: 'contributor',
  bloomTier: 'apply',
  capabilityTier: 'contributor',
  affiliations: ['cofc-congregation', 'hardins-life-group'],
};

/**
 * Hannah Kim — the Kim family's representative on Tuesday evening. Her youngest is
 * sick and she is at home with him. She joins by video, half-listening while her
 * toddler runs in the background, grateful to be present even if she cannot follow
 * every sentence.
 */
const hannahKim: MockImagodeiProfile = {
  id: 'hannah-kim',
  name: 'Hannah Kim',
  standingTier: 'contributor',
  bloomTier: 'apply',
  capabilityTier: 'contributor',
  affiliations: ['cofc-congregation', 'hardins-life-group'],
};

/**
 * LIFE_GROUP_FAMILIES — the core life-group members from the Tuesday-evening gathering
 * narrative. The group has been meeting for three years; the substrate knows this. The
 * commons stream shows the texture of an old fellowship.
 */
export const LIFE_GROUP_FAMILIES: MockImagodeiProfile[] = [
  johnHardin,
  sheilaHardin,
  jinLee,
  robertsonTraveling,
  hannahKim,
];

// ---------------------------------------------------------------------------
// Arkansas sister-congregation elder
// ---------------------------------------------------------------------------

/**
 * Arkansas sister-congregation elder — the elder whose teaching surfaced concern
 * in narrative 4.4. Brother Cal's congregation has been in fellowship with this
 * congregation for thirty years. The elder attends the voluntary peer council
 * convened by the wisdom-commons substrate, sits with the council's witness for
 * two months, and writes back that the congregation has reconsidered the teaching.
 * The resolution is recorded as an REA reconciliation event.
 */
export const ARKANSAS_SISTER_CONGREGATION_ELDER: MockImagodeiProfile = {
  id: 'arkansas-elder',
  name: 'Elder (Arkansas congregation)',
  standingTier: 'steward',
  bloomTier: 'create',
  capabilityTier: 'steward',
  affiliations: ['arkansas-sister-congregation', 'wisdom-commons'],
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/** All profiles in one flat registry for O(1) lookup. */
const ALL_PROFILES: MockImagodeiProfile[] = [
  ...DOWELL_FAMILY,
  ...COFC_ELDERS,
  ...LIFE_GROUP_FAMILIES,
  ARKANSAS_SISTER_CONGREGATION_ELDER,
];

/**
 * Look up a profile by its stable id.
 *
 * @param id - The kebab-case id (e.g., 'matthew-dowell', 'brother-cal')
 * @returns The matching profile, or undefined if not found.
 *
 * @example
 * const matthew = profileById('matthew-dowell');
 */
export function profileById(id: string): MockImagodeiProfile | undefined {
  return ALL_PROFILES.find(p => p.id === id);
}
