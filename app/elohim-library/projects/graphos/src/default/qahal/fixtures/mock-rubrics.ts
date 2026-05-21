/**
 * mock-rubrics.ts
 *
 * Versioned rubric data for each Tier-0 worked-example archetype:
 *   - Dowell household (intimate scale)
 *   - CofC congregation (plural-stewardship scale)
 *   - Hardins life-group (sub-Qahal nesting)
 *   - Wisdom commons federation (peer federation without hierarchy)
 *
 * Grounded in storyteller canonical narratives at:
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md
 * and UX design spec at:
 * genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md (Section 4.3, 6, 8.2)
 *
 * In production these would be versioned EPRs authored by the Qahal's stewards,
 * signed on-chain. These mocks provide stable fixtures for Library B pattern stories.
 */

import type { BloomTier } from './mock-imagodei-profiles.js';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * A mock rubric for a Qahal — the governable versioned EPR that defines what
 * standing looks like in this collective. Each archetype expresses its own
 * standing honors, mastery curve, and friction-gradient note.
 *
 * In production: a versioned EPR signed by the Qahal's stewards and ratified
 * by congregational/collective consent.
 */
export interface MockRubric {
  /** Stable kebab-case Qahal id this rubric belongs to. */
  qahalId: string;
  /** Human-readable rubric name. */
  name: string;
  /**
   * The three-to-five standing honors — plain-language statements of what
   * standing IS in this collective. Authored by the stewards. Ratified by
   * the collective.
   */
  standingHonors: string[];
  /**
   * Bloom's taxonomy mapping — what each of the six mastery tiers means
   * concretely in THIS collective's rubric. The substrate ships no opinion
   * about what mastery looks like; the stewards do.
   */
  bloomMapping: Record<BloomTier, string>;
  /**
   * Human-readable label for the standing decay cadence. Households are
   * gentle (old contributions honored); research institutions sharper.
   */
  cadenceLabel: string;
  /**
   * A brief friction-gradient note as it appears in the rules panel.
   * Describes the anti-accumulation substrate clause for this Qahal.
   */
  frictionGradientNote: string;
  /**
   * Stable imagodei ids of the stewards who configured (authored and maintain)
   * this rubric. For plural-stewardship Qahals, all stewards are listed.
   */
  configuredBy: string[];
  /** ISO date string of the last revision. */
  lastRevised: string;
  /**
   * External-link visibility map — per the UX spec Section 6 capability-gating
   * discipline. Maps capability tiers to visibility treatment. This is the
   * household stewardship declaration: "James (child) doesn't see external links;
   * Gertrude (elder_under_guardianship) sees them filtered through the co-steward."
   */
  externalLinkVisibility: Record<string, 'full' | 'filtered_via_co_steward' | 'hidden'>;
}

// ---------------------------------------------------------------------------
// Dowell household rubric
// ---------------------------------------------------------------------------

/**
 * DOWELL_HOUSEHOLD_RUBRIC — the Dowell family's Qahal rubric as described in
 * UX spec Section 4.3 and narrative 4.1. Short, intimate, humane. Standing honors
 * are the three phrases from the narrative: care contributed, presence shown up,
 * repair offered. The Bloom curve is grounded in family life, not institutional
 * credentialing. Matthew and Jessica are the co-stewards who authored it; it was
 * last revised 2026-04-15 to add the "repair offered" line.
 *
 * External-link visibility map follows spec Section 6:
 * - visitor: filtered_via_co_steward (substrate's care for the stranger)
 * - engaged / contributor / steward: full
 * - child (James): hidden — Matthew + Jessica protect him from open-web exposure
 * - elder_under_guardianship (Gertrude): filtered_via_co_steward — filtered with care
 * - idd_member / legal_steward_protected: hidden
 */
export const DOWELL_HOUSEHOLD_RUBRIC: MockRubric = {
  qahalId: 'dowell-household',
  name: 'Dowell Household — what we honor here',
  standingHonors: [
    'care contributed',
    'presence shown up',
    'repair offered when something has broken between us',
  ],
  bloomMapping: {
    remember: 'know who lives here, what we hold, our rhythms',
    understand: 'explain our care-economy patterns; recognize when help is needed',
    apply: 'do the daily work — meals, chores, attention given without prompt',
    analyze: "notice when something's off; see what's not being said",
    evaluate: 'judge contributions against what the household actually needs',
    create: 'propose new rhythms; design our family\'s rule of life',
  },
  cadenceLabel: 'gentle — standing decays slowly; old contributions are honored',
  frictionGradientNote:
    'No household member can accumulate disproportionate authority without commons-elohim flagging for discussion. Plural stewardship is structural.',
  configuredBy: ['matthew-dowell', 'jessica-dowell'],
  lastRevised: '2026-04-15',
  externalLinkVisibility: {
    visitor: 'filtered_via_co_steward',
    engaged: 'full',
    contributor: 'full',
    steward: 'full',
    child: 'hidden',
    elder_under_guardianship: 'filtered_via_co_steward',
    idd_member: 'filtered_via_co_steward',
    legal_steward_protected: 'hidden',
  },
};

// ---------------------------------------------------------------------------
// CofC congregation rubric
// ---------------------------------------------------------------------------

/**
 * COFC_CONGREGATION_RUBRIC — the plural-elder rubric of Matthew's local Churches
 * of Christ congregation. Authored by the four elders four years ago and revised
 * twice since. Ratified by congregational consent at a Wednesday-night meeting.
 * Standing honors: faith, practice, service, submission to apostolic teaching.
 * The Bloom curve is calibrated for spiritual maturity in the Restoration tradition.
 * Plural stewardship is structural: all four elders configure.
 */
export const COFC_CONGREGATION_RUBRIC: MockRubric = {
  qahalId: 'cofc-congregation',
  name: 'Local Churches of Christ — standing in this congregation',
  standingHonors: [
    'faith demonstrated in confession and life',
    'practice shown in fellowship and worship',
    'service offered to the body and to the neighborhood',
    'submission to the apostles\' teaching as we have received it',
  ],
  bloomMapping: {
    remember:
      'know the apostles\' teaching; name the confession; attend worship and fellowship regularly',
    understand:
      'explain the faith; articulate why we practice as we do; welcome a new convert with care',
    apply:
      'serve in the body; show up to the life-group; take a turn at communion; give to the neighborhood',
    analyze:
      'notice when teaching drifts; recognize when a brother or sister needs counsel; identify where the body is weak',
    evaluate:
      'judge contributions by the apostles\' standard; validate the work of those serving; assess whether a proposed practice fits our tradition',
    create:
      'author the rubric for the next generation; design catechesis for new converts; propose how the congregation serves the neighborhood this season',
  },
  cadenceLabel:
    'participation-shaped — standing reflects active fellowship; long absences decay gently but honestly',
  frictionGradientNote:
    'The substrate watches for accumulation: if one elder begins to carry more attestation-weight than the others, the commons-elohim writes a small note into the elders\' shared view. Plural-elder stewardship is protected by the substrate, not by good intentions alone.',
  configuredBy: ['brother-cal', 'elder-thompson', 'elder-davis', 'elder-rhodes'],
  lastRevised: '2024-09-10',
  externalLinkVisibility: {
    visitor: 'filtered_via_co_steward',
    engaged: 'full',
    contributor: 'full',
    steward: 'full',
    child: 'hidden',
    elder_under_guardianship: 'filtered_via_co_steward',
    idd_member: 'filtered_via_co_steward',
    legal_steward_protected: 'hidden',
  },
};

// ---------------------------------------------------------------------------
// Hardins life-group rubric
// ---------------------------------------------------------------------------

/**
 * LIFE_GROUP_RUBRIC — the Tuesday-evening life-group rubric. Inherits from the
 * congregation's rubric and adds a small local customization the group agreed to
 * at its second meeting three years ago. Standing honors: presence kept, prayer
 * attested, vulnerability offered, care shown across the week between meetings.
 * Anchored upward to the congregation's rubric so no life-group customization
 * can drift outside what the elders have authored. Configured by the eight households.
 */
export const LIFE_GROUP_RUBRIC: MockRubric = {
  qahalId: 'hardins-life-group',
  name: 'Tuesday Life-Group — what we hold together',
  standingHonors: [
    'presence kept — you show up on Tuesday evenings',
    'prayer attested — you pray with us and are prayed for',
    'vulnerability offered — you bring what you are actually carrying',
    'care shown across the week between meetings',
  ],
  bloomMapping: {
    remember:
      'know the names and stories of the eight households; know who is carrying what this week',
    understand:
      'explain why we study Scripture together; articulate the connection between Sunday morning and Tuesday evening',
    apply:
      'attend; pray; check in on a household during the week; bring a meal when someone is sick',
    analyze:
      'notice when a household goes quiet; recognize when the group is avoiding something; see the care that is not yet named',
    evaluate:
      'judge whether a discussion is faithful to the passage; assess whether care is landing; discern when the group is ready for mission-engagement',
    create:
      'propose how the group serves the neighborhood this season; design a study series for next quarter; invite another household to host',
  },
  cadenceLabel:
    'intimate — standing decays slowly; the group has been together three years and holds its history',
  frictionGradientNote:
    'The substrate notices hosting accumulation. John has hosted twenty-three of the last twenty-four Tuesdays; a gentle prompt will surface for him next week. No one should be the host by default. The Lees have offered.',
  configuredBy: [
    'john-hardin',
    'sheila-hardin',
    'jin-lee',
    'robertson-traveling',
    'hannah-kim',
  ],
  lastRevised: '2023-03-14',
  externalLinkVisibility: {
    visitor: 'filtered_via_co_steward',
    engaged: 'full',
    contributor: 'full',
    steward: 'full',
    child: 'hidden',
    elder_under_guardianship: 'filtered_via_co_steward',
    idd_member: 'filtered_via_co_steward',
    legal_steward_protected: 'hidden',
  },
};

// ---------------------------------------------------------------------------
// Wisdom commons federation rubric
// ---------------------------------------------------------------------------

/**
 * WISDOM_COMMONS_RUBRIC — the federation-template rubric for the wisdom commons of
 * autonomous Churches of Christ congregations. Not an override of any local rubric;
 * each participating congregation adopts, adapts, or declines. The federation Qahal
 * carries a shared rubric template authored over the years by the elders of participating
 * congregations. Standing honors: faithful proclamation, peaceable fellowship with
 * sister congregations, contribution offered to the commons, willingness to receive
 * correction. Configured by participating-congregation elders (represented here by
 * the four CofC elders + the Arkansas elder as a peer).
 */
export const WISDOM_COMMONS_RUBRIC: MockRubric = {
  qahalId: 'wisdom-commons',
  name: 'Wisdom Commons — standing in the fellowship of autonomous congregations',
  standingHonors: [
    'faithful proclamation of the apostles\' teaching as received in the Restoration tradition',
    'peaceable fellowship with sister congregations',
    'contribution offered to the commons — sermon series, practices, wisdom shared freely',
    'willingness to receive correction from sister congregations when sister congregations offer it',
  ],
  bloomMapping: {
    remember:
      'know the participating congregations; name the tradition we hold in common; acknowledge the fellowship',
    understand:
      'explain why autonomous congregations share wisdom horizontally; articulate the Restoration Movement\' polity choices',
    apply:
      'offer something to the commons — a sermon series, a ministry pattern, a prayer request; receive what others offer',
    analyze:
      'notice when a sister congregation\'s teaching drifts; recognize when the commons is being used for institutional leverage rather than peer wisdom',
    evaluate:
      'judge contributions by the apostles\' standard across the tradition; assess whether a concern is well-founded; discern when peer council is warranted',
    create:
      'author rubric-template revisions for the next generation of participating congregations; design the peer-council convening process; propose how the federation serves mission together',
  },
  cadenceLabel:
    'congregational — standing reflects active participation and contribution; long silence decays honestly',
  frictionGradientNote:
    'The federation has no hierarchy above any participating congregation. The substrate enforces this: no single congregation can accumulate authority over others through the commons. The peer council has no power to discipline; it can only witness. The friction-gradient makes any move toward institutional authority mechanically expensive.',
  configuredBy: ['brother-cal', 'elder-thompson', 'elder-davis', 'elder-rhodes', 'arkansas-elder'],
  lastRevised: '2025-11-03',
  externalLinkVisibility: {
    visitor: 'filtered_via_co_steward',
    engaged: 'full',
    contributor: 'full',
    steward: 'full',
    child: 'hidden',
    elder_under_guardianship: 'filtered_via_co_steward',
    idd_member: 'filtered_via_co_steward',
    legal_steward_protected: 'hidden',
  },
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/** All rubrics in one flat registry for O(1) lookup by Qahal id. */
const ALL_RUBRICS: MockRubric[] = [
  DOWELL_HOUSEHOLD_RUBRIC,
  COFC_CONGREGATION_RUBRIC,
  LIFE_GROUP_RUBRIC,
  WISDOM_COMMONS_RUBRIC,
];

/**
 * Look up a rubric by its Qahal's stable id.
 *
 * @param id - The Qahal kebab-case id (e.g., 'dowell-household', 'wisdom-commons')
 * @returns The matching rubric, or undefined if not found.
 *
 * @example
 * const rubric = rubricByQahalId('dowell-household');
 */
export function rubricByQahalId(id: string): MockRubric | undefined {
  return ALL_RUBRICS.find(r => r.qahalId === id);
}
