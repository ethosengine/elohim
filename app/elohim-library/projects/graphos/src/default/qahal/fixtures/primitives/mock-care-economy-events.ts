/**
 * mock-care-economy-events.ts
 *
 * Stream events for each canonical worked-example scene:
 *   - Dowell household Tuesday morning (narrative 4.1)
 *   - CofC congregation Sunday morning (narrative 4.2)
 *   - Hardins life-group Tuesday evening (narrative 4.3)
 *   - Wisdom commons Thursday afternoon (narrative 4.4)
 *
 * Grounded in storyteller canonical narratives at:
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md
 *
 * REA events record care given and care received — the small accounting motion
 * the elohim catches before it can become a debt. Tokens are illustrative;
 * actual REA flow values are substrate-computed.
 *
 * Note: The canonical narrative places the Dowell scene on a Wednesday morning
 * ("It is a Wednesday morning in early autumn"), but the fixture is named
 * "Tuesday morning" to align with the roadmap's canonical fixture naming
 * convention (dowell-household-tuesday-morning). The narrative content is
 * faithfully reproduced from the canonical source.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * A REA (Resource-Event-Agent) economic event — the protocol's primitive for
 * recording care, presence, repair, growth, and time as substrate-visible flows.
 * These accumulate in the Qahal's care-economy ledger over years into the slow
 * truth the family already knows: who cares, how, and how often.
 */
export interface MockReaEvent {
  /** REA event kind — the care vocabulary this Qahal recognizes. */
  kind: 'care' | 'presence' | 'repair' | 'growth' | 'time';
  /**
   * Token quantity — illustrative unit; substrate computes actual value.
   * Higher tokens = more care weight recognized by the rubric.
   */
  tokens: number;
}

/**
 * A single event in a Qahal's commons stream. Events are authored by members
 * or observed by the commons-elohim co-steward. They carry optional REA
 * economic accounting, optional thread threading, and optional pending-acknowledgment
 * state (items that "exist in the stream waiting for a small gesture").
 */
export interface MockCareEconomyEvent {
  /** Stable kebab-case id for downstream fixture composition. */
  id: string;
  /** The Qahal this event belongs to. */
  qahalId: string;
  /**
   * Author id — an imagodei profile id (e.g., 'matthew-dowell'), or
   * 'commons-elohim' for observations composed by the Qahal's co-steward.
   */
  authorId: string;
  /** ISO datetime string of event creation. */
  timestamp: string;
  /** Human-readable stream content. */
  content: string;
  /**
   * Optional REA economic event — the substrate's accounting motion.
   * When present, this event flows shefa into the Qahal's care-economy ledger.
   */
  rea?: MockReaEvent;
  /**
   * True if this event is pending acknowledgment from the stream's primary viewer.
   * The commons-elohim surfaces these in its right-nav panel ("three care contributions
   * are pending acknowledgment — Sheila's recipe, Gertrude's check-in, the neighbor's
   * offer to bring dinner").
   */
  acknowledgmentPending?: boolean;
  /**
   * Optional parent event id — for threaded replies. Prayer attestations thread
   * under the original prayer request; acknowledgments thread under the acknowledged item.
   */
  threadParentId?: string;
}

// ---------------------------------------------------------------------------
// 4.1 — Dowell household Tuesday morning stream
// ---------------------------------------------------------------------------

/**
 * DOWELL_TUESDAY_MORNING_STREAM — the Dowell household commons stream as Matthew
 * sees it when he opens the elohim-app for the first time that Wednesday morning.
 *
 * Narrative 4.1 elements reproduced:
 * - Yesterday's elohim-composed entry about James being sick + shift notes
 * - Sheila's chicken-and-rice soup recipe (pending acknowledgment, REA care)
 * - Gertrude's check-in via elohim-to-elohim conversation (pending acknowledgment)
 * - Neighbor's dinner offer surfaced by the co-steward (pending acknowledgment)
 * - Commons-elohim's right-nav note: "The household is steady"
 *
 * When Matthew acknowledges Sheila's recipe, the substrate flows shefa — a tiny REA
 * event — into the household's care-economy ledger, where it will accumulate over
 * the years into the slow truth the family already knows.
 */
export const DOWELL_TUESDAY_MORNING_STREAM: MockCareEconomyEvent[] = [
  {
    id: 'dowell-james-sick-composed',
    qahalId: 'dowell-household',
    authorId: 'commons-elohim',
    timestamp: '2026-05-20T20:15:00Z',
    content:
      'Sick kid. James was up twice. Jessica took the morning shift, Matthew took the afternoon, James napped from 1–3.',
    rea: { kind: 'care', tokens: 12 },
  },
  {
    id: 'dowell-sheila-soup-recipe',
    qahalId: 'dowell-household',
    authorId: 'sheila-hardin',
    timestamp: '2026-05-21T07:42:00Z',
    content:
      "I made it last night and James ate two bowls before he got sick. Maybe this helps. [chicken-and-rice soup recipe — Gertrude's version]",
    rea: { kind: 'care', tokens: 8 },
    acknowledgmentPending: true,
  },
  {
    id: 'dowell-gertrude-checkin',
    qahalId: 'dowell-household',
    authorId: 'commons-elohim',
    timestamp: '2026-05-21T07:58:00Z',
    content:
      'Gertrude checked in this morning. She is glad James is being read to.',
    acknowledgmentPending: true,
  },
  {
    id: 'dowell-neighbor-dinner-offer',
    qahalId: 'dowell-household',
    authorId: 'commons-elohim',
    timestamp: '2026-05-21T08:05:00Z',
    content:
      'A neighbor has offered to bring dinner tonight. Reply when you have a moment. There is no urgency.',
    acknowledgmentPending: true,
  },
  {
    id: 'dowell-jessica-morning-note',
    qahalId: 'dowell-household',
    authorId: 'jessica-dowell',
    timestamp: '2026-05-21T08:12:00Z',
    content:
      'James is at the kitchen table. We are reading about volcanoes. He is eating toast in small bites. He will be okay.',
    rea: { kind: 'care', tokens: 5 },
  },
];

// ---------------------------------------------------------------------------
// 4.2 — CofC congregation Sunday morning stream
// ---------------------------------------------------------------------------

/**
 * COFC_SUNDAY_MORNING_STREAM — the congregation's commons stream as Matthew sees
 * it Sunday morning at eight, coffee in hand, before sending James upstairs to
 * brush his teeth.
 *
 * Narrative 4.2 elements reproduced:
 * - Prayer requests: Sarah's father in hospital; Robertsons' traveling mercies; Hardins' dog Buster
 * - Deacon's youth retreat note (18 kids, need 2 drivers + 4 sleeping bags; James invited)
 * - Quote from last week's Romans 12 sermon + recording link
 * - Commons-elohim's deacon-like witness paragraph
 */
export const COFC_SUNDAY_MORNING_STREAM: MockCareEconomyEvent[] = [
  {
    id: 'cofc-prayer-sarahs-father',
    qahalId: 'cofc-congregation',
    authorId: 'brother-cal',
    timestamp: '2026-05-21T07:00:00Z',
    content:
      "Prayer requests this week: Sarah's father is back in the hospital. Please hold the Johnson family in prayer.",
    rea: { kind: 'presence', tokens: 1 },
  },
  {
    id: 'cofc-prayer-robertsons-travel',
    qahalId: 'cofc-congregation',
    authorId: 'brother-cal',
    timestamp: '2026-05-21T07:01:00Z',
    content:
      'The Robertsons are traveling and ask for traveling mercies. They will join us by video from the road.',
    rea: { kind: 'presence', tokens: 1 },
    threadParentId: 'cofc-prayer-sarahs-father',
  },
  {
    id: 'cofc-prayer-hardin-dog-buster',
    qahalId: 'cofc-congregation',
    authorId: 'brother-cal',
    timestamp: '2026-05-21T07:02:00Z',
    content:
      'The Hardin family is grieving the loss of their dog Buster, who was fifteen and a good dog.',
    rea: { kind: 'care', tokens: 3 },
    threadParentId: 'cofc-prayer-sarahs-father',
  },
  {
    id: 'cofc-youth-retreat-note',
    qahalId: 'cofc-congregation',
    authorId: 'commons-elohim',
    timestamp: '2026-05-21T07:15:00Z',
    content:
      "Youth retreat update from Deacon Williams: Eighteen kids signed up. We need two more drivers and four more sleeping bags. James — we'd love to have you.",
    acknowledgmentPending: false,
  },
  {
    id: 'cofc-romans-12-sermon-link',
    qahalId: 'cofc-congregation',
    authorId: 'brother-cal',
    timestamp: '2026-05-21T07:30:00Z',
    content:
      "From last week's Romans 12 sermon (week 4 of the series): \"I appeal to you therefore, brothers, by the mercies of God, to present your bodies as a living sacrifice, holy and acceptable to God, which is your spiritual worship.\" Recording available for anyone who missed it or wants to hear a passage again.",
    rea: { kind: 'growth', tokens: 4 },
  },
  {
    id: 'cofc-elohim-witness-sunday',
    qahalId: 'cofc-congregation',
    authorId: 'commons-elohim',
    timestamp: '2026-05-21T08:00:00Z',
    content:
      "The congregation's reach into the neighborhood is rising slightly. Three life-groups are at the cohesion threshold the rubric describes as ready-for-mission-engagement. Brother Cal's sermon series on Romans is in its fourth week; the commons attestations suggest the body is receiving it. The youth retreat needs two more drivers.",
  },
];

// ---------------------------------------------------------------------------
// 4.3 — Hardins life-group Tuesday evening stream
// ---------------------------------------------------------------------------

/**
 * LIFE_GROUP_TUESDAY_EVENING_STREAM — the life-group commons stream from Tuesday
 * evening at John Hardin's house.
 *
 * Narrative 4.3 elements reproduced:
 * - John reads Romans 12:1 to open
 * - Sarah shares about her father in the hospital (vulnerable, Romans 12 frame)
 * - The Robertsons' video contribution about being a presented body while traveling
 * - Hannah Kim's tired-but-grateful check-in
 * - Prayer attestations (REA presence +1 per amen)
 * - Post-meeting commons-elohim witness: cohesion threshold reached
 *
 * Prayers are not published; they are encrypted to the life-group and available
 * only to those present. Prayer attestations — cryptographically signed — accumulate
 * in the group's record: "we held this together."
 */
export const LIFE_GROUP_TUESDAY_EVENING_STREAM: MockCareEconomyEvent[] = [
  {
    id: 'lg-john-opens-romans12',
    qahalId: 'hardins-life-group',
    authorId: 'john-hardin',
    timestamp: '2026-05-20T19:30:00Z',
    content:
      'Romans 12:1 — "I appeal to you therefore, brothers, by the mercies of God, to present your bodies as a living sacrifice, holy and acceptable to God, which is your spiritual worship." Let\'s sit with that for a moment.',
  },
  {
    id: 'lg-sarah-father-share',
    qahalId: 'hardins-life-group',
    authorId: 'commons-elohim',
    timestamp: '2026-05-20T19:45:00Z',
    content:
      "Sarah shared about her father in the hospital — what it means to offer her own anxiety as a living sacrifice. The room held it together.",
    rea: { kind: 'care', tokens: 6 },
  },
  {
    id: 'lg-robertsons-video-contribution',
    qahalId: 'hardins-life-group',
    authorId: 'robertson-traveling',
    timestamp: '2026-05-20T20:00:00Z',
    content:
      'From the road — what it means to be a body presented when you are physically absent from your usual fellowship. We are with you from here.',
    rea: { kind: 'presence', tokens: 3 },
  },
  {
    id: 'lg-hannah-tired-grateful',
    qahalId: 'hardins-life-group',
    authorId: 'hannah-kim',
    timestamp: '2026-05-20T20:10:00Z',
    content:
      "I'm grateful to be here even if I can't follow every sentence tonight. [toddler running in background] Thank you for having me.",
    rea: { kind: 'presence', tokens: 2 },
  },
  {
    id: 'lg-sarah-prayer-request',
    qahalId: 'hardins-life-group',
    authorId: 'john-hardin',
    timestamp: '2026-05-20T20:30:00Z',
    content:
      "Prayer for Sarah's father and the Johnson family. [attested by 12 adults + 4 older children]",
    rea: { kind: 'care', tokens: 16 },
  },
  {
    id: 'lg-prayer-amen-matthew',
    qahalId: 'hardins-life-group',
    authorId: 'matthew-dowell',
    timestamp: '2026-05-20T20:31:00Z',
    content: 'amen',
    rea: { kind: 'presence', tokens: 1 },
    threadParentId: 'lg-sarah-prayer-request',
  },
  {
    id: 'lg-prayer-amen-jessica',
    qahalId: 'hardins-life-group',
    authorId: 'jessica-dowell',
    timestamp: '2026-05-20T20:31:30Z',
    content: 'amen',
    rea: { kind: 'presence', tokens: 1 },
    threadParentId: 'lg-sarah-prayer-request',
  },
  {
    id: 'lg-elohim-witness-cohesion',
    qahalId: 'hardins-life-group',
    authorId: 'commons-elohim',
    timestamp: '2026-05-20T21:00:00Z',
    content:
      "This life-group has reached the cohesion threshold the congregation's rubric describes as ready-for-mission-engagement. The elders' note on this threshold suggests that the next step is for the life-group to choose a neighborhood concern to carry together — meals for a family, support for a school, presence at a struggling sister congregation. There is no urgency. The rubric describes this as a discernment season, not a deadline.",
  },
];

// ---------------------------------------------------------------------------
// 4.4 — Wisdom commons Thursday afternoon stream
// ---------------------------------------------------------------------------

/**
 * WISDOM_COMMONS_THURSDAY_STREAM — the wisdom commons federation Qahal stream
 * from a Thursday afternoon in October. Brother Cal is in his study with the
 * wisdom-commons Qahal open on his laptop.
 *
 * Narrative 4.4 elements reproduced:
 * - Tennessee sermon-series offer
 * - Oklahoma neighborhood-meals ministry note
 * - Texas elder's theological essay on Romans 12 "living sacrifice"
 * - Ohio congregation's call-for-minister prayer request
 * - West Virginia long-serving-elder death grief note
 * - Brother Cal's concern surface entry (the load-bearing moment of 4.4)
 *   + the commons-elohim's plain factual witness line (public to participating congregations)
 *
 * All items are REA wisdom-exchange events — gifts of practice and prayer,
 * attested by the offering congregation, available to receive or leave alone.
 * No item is commanded. All are offered.
 */
export const WISDOM_COMMONS_THURSDAY_STREAM: MockCareEconomyEvent[] = [
  {
    id: 'wc-tennessee-sermon-series',
    qahalId: 'wisdom-commons',
    authorId: 'commons-elohim',
    timestamp: '2026-05-15T09:00:00Z',
    content:
      'A congregation in Tennessee has just finished a twelve-week sermon series on the book of Romans and is offering the full outline to anyone who would like to use it. Attached: series overview, weekly outlines, discussion questions, and recommended commentaries.',
    rea: { kind: 'growth', tokens: 10 },
  },
  {
    id: 'wc-oklahoma-neighborhood-meals',
    qahalId: 'wisdom-commons',
    authorId: 'commons-elohim',
    timestamp: '2026-05-16T14:30:00Z',
    content:
      'A congregation in Oklahoma has run a successful neighborhood-meals ministry for two years and offers the practical details: how it was organized, what the costs were, how volunteers were recruited, what they learned. Available for any congregation that would like to try something similar.',
    rea: { kind: 'care', tokens: 8 },
  },
  {
    id: 'wc-texas-essay-living-sacrifice',
    qahalId: 'wisdom-commons',
    authorId: 'commons-elohim',
    timestamp: '2026-05-17T11:15:00Z',
    content:
      "An elder in Texas has written a theological essay on the meaning of \"living sacrifice\" in Romans 12:1 — what Paul's Greek accomplishes that English translations soften, how the Restoration tradition has historically read this passage, and what it asks of a congregation's life together. Offered to the commons for any who find it useful.",
    rea: { kind: 'growth', tokens: 6 },
  },
  {
    id: 'wc-ohio-minister-search',
    qahalId: 'wisdom-commons',
    authorId: 'commons-elohim',
    timestamp: '2026-05-18T10:00:00Z',
    content:
      "A small congregation in Ohio is in the process of calling a new minister and asks for prayer. They have been without a minister for six months. They are a faithful body and they are tired. Please hold them.",
    rea: { kind: 'care', tokens: 5 },
  },
  {
    id: 'wc-wv-elder-death',
    qahalId: 'wisdom-commons',
    authorId: 'commons-elohim',
    timestamp: '2026-05-20T08:00:00Z',
    content:
      'A congregation in West Virginia has lost their long-serving elder this week. He served for thirty-one years. They are grateful for the prayers of the brotherhood and ask that he be remembered.',
    rea: { kind: 'care', tokens: 7 },
  },
  {
    id: 'wc-brother-cal-concern-surface',
    qahalId: 'wisdom-commons',
    authorId: 'brother-cal',
    timestamp: '2026-05-21T14:22:00Z',
    content:
      "I write as one who has been in fellowship with this brother and this congregation for thirty years. I write seeking to understand and to be understood. I do not write to expel, and I have no authority to expel. I write because the tradition we share asks brothers to speak to brothers when something has gone wrong.\n\nAn elder in a sister congregation in Arkansas has recently published a teaching that I believe is doctrinally adrift from the apostles' teaching as our tradition has received it. I have read the teaching twice. I have prayed about it. I have talked with two of my fellow elders. I now bring this concern as a peer to peers.\n\n[Concern surface attached — cites teaching, cites apostolic passages, names the elder and congregation, names posture: not to expel, to understand and be understood. Rubric reference: willingness to receive correction from sister congregations.]",
    acknowledgmentPending: true,
  },
  {
    id: 'wc-elohim-peer-council-witness',
    qahalId: 'wisdom-commons',
    authorId: 'commons-elohim',
    timestamp: '2026-05-21T14:25:00Z',
    content:
      'Brother Cal of [congregation] has surfaced a concern regarding teaching at [Arkansas sister congregation]. A peer council is being convened at Brother Cal\'s request. The convening is voluntary on every side. The peer council has no authority to bind any congregation. Elders from participating congregations who have asked to be available for this kind of convening will be invited.',
    threadParentId: 'wc-brother-cal-concern-surface',
  },
];

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/** All streams keyed by Qahal id for O(1) lookup. */
const STREAMS: Record<string, MockCareEconomyEvent[]> = {
  'dowell-household': DOWELL_TUESDAY_MORNING_STREAM,
  'cofc-congregation': COFC_SUNDAY_MORNING_STREAM,
  'hardins-life-group': LIFE_GROUP_TUESDAY_EVENING_STREAM,
  'wisdom-commons': WISDOM_COMMONS_THURSDAY_STREAM,
};

/**
 * Return all stream events for a given Qahal id.
 *
 * @param qahalId - The Qahal's stable kebab-case id
 * @returns Array of stream events for this Qahal (empty array if Qahal not found)
 *
 * @example
 * const stream = streamForQahal('dowell-household');
 */
export function streamForQahal(qahalId: string): MockCareEconomyEvent[] {
  return STREAMS[qahalId] ?? [];
}
