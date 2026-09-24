/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/content-search-view.schema.json -- DO NOT EDIT */

/**
 * The fold the ranking read, as an answer envelope (../objects/answer.schema.json) narrowed to this view: present with the fold's measure, state, attestation and time; absent when the store holds no fold; unreachable when it could not be read.
 */
export type ContentSearchFoldAnswer = ContentSearchFoldPresent | AnswerAbsent | AnswerUnreachable;
/**
 * How far the fold is behind the content rows, as an answer envelope (../objects/answer.schema.json) narrowed to this view: present with the count behind and the measure's declared bound; absent when there is no fold to be behind; unreachable when the lag could not be measured.
 */
export type ContentSearchFoldLagAnswer =
  | ContentSearchFoldLagPresent
  | AnswerAbsent
  | AnswerUnreachable;
/**
 * Content type vocabulary. Core types are DNA-notarized (three-leg coupled: knowledge+value+governance). Storage-only types are cross-domain entity references. Extensible types are community vocabulary.
 */
export type ContentType =
  | 'epic'
  | 'concept'
  | 'lesson'
  | 'scenario'
  | 'assessment'
  | 'reflection'
  | 'discussion'
  | 'exercise'
  | 'article'
  | 'path'
  | 'human'
  | 'role'
  | 'collective'
  | 'example'
  | 'reference'
  | 'feature'
  | 'practice'
  | 'contributor'
  | 'video'
  | 'audio'
  | 'book'
  | 'book-chapter'
  | 'documentary'
  | 'bible-verse'
  | 'activity'
  | 'narrative'
  | 'course-module'
  | 'module'
  | 'quiz'
  | 'podcast'
  | 'simulation'
  | 'node-context'
  | 'stewardship-context'
  | 'work-story'
  | 'work-project'
  | 'issue-report'
  | 'application'
  | 'gate-process-declaration'
  | 'universal-band-declaration'
  | 'gate-rules-declaration'
  | 'aggregation-spec'
  | 'escalation-target-spec'
  | 'element-registry';
/**
 * Content reach/visibility level. Ordered from most restrictive to most open. Source of truth: DNA-notarized CORE_REACH_LEVELS constant in content_store_integrity zome. Category A — enumeration values are part of the protocol vocabulary enforced by gateways without parsing payload.
 */
export type Reach =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';

/**
 * Source of truth: assembled read projection (Operational, Category C) over the storage content projection's derived lexical fold — the content-lexical-index measure's fold store, itself rebuilt from the content rows it folds and never synced. Rendered per request and never persisted. Response to GET /db/content/search?q&lens&contentType&reach&tags&limit&offset&recipe. Every answer names the recipe it ranked under (name and CID), the lens it resolved and why, the selection rule, the fold it read and how far behind the content rows that fold is. A fold that is absent or unreachable answers rankingKnown: false with no candidates, never an empty list dressed as a ranking. Candidates are reach-gated per reader after ranking; what the gate withheld is named in omissions, never silently dropped. facets are counted over the admitted set only.
 */
export interface ContentSearchView {
  /**
   * The question as the reader asked it (the q parameter), unchanged.
   */
  query: string;
  /**
   * True only when the candidates were ordered by the declared recipe over a present fold. False whenever the fold is absent or unreachable; candidates is then empty.
   */
  rankingKnown: boolean;
  recipe: ContentSearchRecipeView;
  lens: ContentSearchLensView;
  /**
   * The selection rule, one line: which candidates this page shows of how many ranked, and by what order.
   */
  selection: string;
  fold: ContentSearchFoldAnswer;
  foldLag: ContentSearchFoldLagAnswer;
  /**
   * The admitted candidates of this page, in the recipe's order. Empty when rankingKnown is false.
   */
  candidates: ContentSearchCandidateView[];
  facets: ContentSearchFacetsView;
  /**
   * One named line per thing the view withheld or could not vouch for (a reach refusal, a fold that is behind). Present even when empty.
   */
  omissions: string[];
  /**
   * One named line per request parameter the view could not honour as asked (a recipe CID pin that names another recipe). Present even when empty.
   */
  unresolved: string[];
  /**
   * How many candidates the reach gate admitted before the page was cut; candidates never exceed it.
   */
  totalCount: number;
}
/**
 * The declared fusion recipe the ranking ran under.
 */
export interface ContentSearchRecipeView {
  /**
   * The recipe id, e.g. rrf-v2.
   */
  name: string;
  /**
   * The recipe's content address (atom_cid of its addressed keys).
   */
  cid: string;
  /**
   * The reciprocal-rank constant.
   */
  k: number;
  /**
   * Fusion orders; it never sums a producer's score.
   */
  orderOnly: boolean;
  /**
   * The producers fused, in recipe order.
   *
   * @minItems 1
   */
  producers: [ContentSearchProducerView, ...ContentSearchProducerView[]];
}
/**
 * One producer of a ranking and the method it ranks under.
 */
export interface ContentSearchProducerView {
  /**
   * The producer id as the recipe names it, e.g. lexical.
   */
  id: string;
  /**
   * The method the producer ranks under: its IndexMeasure CID.
   */
  method: string;
}
/**
 * The lens level the view resolved. A lens is never refused: an absent or unknown level resolves to the recipe's default with provenance defaulted.
 */
export interface ContentSearchLensView {
  /**
   * A level of the recipe's lens table, e.g. minimal, standard, whole.
   */
  level: string;
  /**
   * How many candidates the level shows at most.
   */
  choiceCount: number;
  /**
   * The content address of the lens table the level was read from.
   */
  cid: string;
  /**
   * requested when the reader named this level; defaulted when the reader named none, or one the table does not declare (never a refusal).
   */
  provenance: string;
}
export interface ContentSearchFoldPresent {
  state: 'present';
  value: ContentSearchFoldView;
}
/**
 * The fold the ranking read.
 */
export interface ContentSearchFoldView {
  /**
   * The IndexMeasure CID the fold executed.
   */
  measure: string;
  /**
   * The state the fold's latest attestation reports (FoldState: complete, degraded or failed).
   */
  state: string;
  /**
   * The content address of that attestation.
   */
  attestationCid: string;
  /**
   * Unix epoch seconds the attestation was made.
   */
  at: number;
}
/**
 * The answer arrived and reports the value does not exist. A POSITIVE claim — absence was observed — so its only legal reason is `observed_absent`, and it never carries a value.
 */
export interface AnswerAbsent {
  state: 'absent';
  reason: 'observed_absent';
}
/**
 * No answer arrived; nothing is established in either direction. Always carries a reason — a non-present answer with no reason is exactly the unreadable gauge C8 forbids — and never a value.
 */
export interface AnswerUnreachable {
  state: 'unreachable';
  reason: 'timeout' | 'transport_error' | 'refused' | 'unverifiable' | 'not_yet_delivered';
}
export interface ContentSearchFoldLagPresent {
  state: 'present';
  value: ContentSearchFoldLagView;
}
/**
 * How far the fold is behind, against the measure's declared bound.
 */
export interface ContentSearchFoldLagView {
  /**
   * Content rows changed since the fold's watermark.
   */
  behind: number;
  /**
   * The measure's declared fold-lag limit.
   */
  limit: number;
  /**
   * What the limit counts, e.g. units.
   */
  unit: string;
  /**
   * True when behind is within the limit.
   */
  within: boolean;
}
/**
 * One admitted candidate with its provenance: which producer ranked it, under which method.
 */
export interface ContentSearchCandidateView {
  /**
   * The content row's id.
   */
  contentId: string;
  title: string;
  contentType: ContentType;
  reach: Reach;
  /**
   * The row's trust legibility label as ContentView.trust carries it: notarized, published or unconfirmed. Never a number.
   */
  trust: string;
  /**
   * The row's tags; an empty array is the honest 'no tags' answer.
   */
  tags: string[];
  /**
   * The fused reciprocal-rank score, for ORDER only; comparable within one answer, never across answers.
   */
  score: number;
  /**
   * The producer that ranked the candidate best.
   */
  producer: string;
  /**
   * That producer's method: its IndexMeasure CID.
   */
  method: string;
  /**
   * The section the ranking producer matched, or null when it located none (one provenance: the fold was read and no section matched).
   */
  bestSection: null | ContentSearchSectionView;
}
/**
 * The section of a candidate a match landed in.
 */
export interface ContentSearchSectionView {
  /**
   * The section's name: head, tags, a heading, or a line window.
   */
  title: string;
  /**
   * A short excerpt of the section's text.
   */
  snippet: string;
}
/**
 * Counts over the admitted candidates only; a withheld row never counts.
 */
export interface ContentSearchFacetsView {
  contentType: FacetCountView[];
  reach: FacetCountView[];
  tags: FacetCountView[];
}
/**
 * One facet value and how many admitted candidates carry it.
 */
export interface FacetCountView {
  value: string;
  count: number;
}
