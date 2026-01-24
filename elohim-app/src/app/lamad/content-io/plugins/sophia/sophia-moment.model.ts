/**
 * Sophia Moment Model - Type definitions for Sophia assessment integration.
 *
 * This module provides type definitions for Angular components to work with
 * the Sophia assessment system without requiring the full sophia-core package.
 *
 * NOTE: These types are also available from sophia-element-loader.ts which
 * provides access to the bundled Psyche API with additional functions like
 * `aggregateReflections()` and `getPrimarySubscale()`. For psychometric
 * processing, prefer using the Psyche API from sophia-element-loader.
 *
 * @see ./sophia-element-loader.ts for the Psyche API
 */

// ─────────────────────────────────────────────────────────────────────────────
// Assessment Purpose
// ─────────────────────────────────────────────────────────────────────────────

export type AssessmentPurpose = 'mastery' | 'discovery' | 'reflection' | 'invitation';

// ─────────────────────────────────────────────────────────────────────────────
// Widget and Renderer Types
// ─────────────────────────────────────────────────────────────────────────────

export interface PerseusRenderer {
  content: string;
  widgets: Record<string, PerseusWidget>;
  images?: Record<string, PerseusImage>;
}

export interface PerseusWidget {
  type: string;
  options: Record<string, unknown>;
  graded?: boolean;
  alignment?: string;
  static?: boolean;
  version?: { major: number; minor: number };
}

export interface PerseusImage {
  url: string;
  width?: number;
  height?: number;
  alt?: string;
  title?: string;
}

export interface Hint {
  content: string;
  widgets: Record<string, PerseusWidget>;
  images?: Record<string, PerseusImage>;
  replace?: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Subscale Types
// ─────────────────────────────────────────────────────────────────────────────

export type SubscaleMappings = Record<string, ChoiceSubscaleMap>;
export type ChoiceSubscaleMap = Record<string, SubscaleContribution>;
export type SubscaleContribution = Record<string, number>;

// ─────────────────────────────────────────────────────────────────────────────
// Moment
// ─────────────────────────────────────────────────────────────────────────────

export interface Moment {
  id: string;
  purpose: AssessmentPurpose;
  content: PerseusRenderer;
  hints?: Hint[];
  subscaleContributions?: SubscaleMappings;
  metadata?: MomentMetadata;
}

export interface MomentMetadata {
  tags?: string[];
  assessesContentId?: string;
  estimatedTimeSeconds?: number;
  instrumentId?: string;
  category?: string;
  [key: string]: unknown;
}

// ─────────────────────────────────────────────────────────────────────────────
// Recognition
// ─────────────────────────────────────────────────────────────────────────────

export interface Recognition {
  momentId: string;
  purpose: AssessmentPurpose;
  mastery?: MasteryResult;
  resonance?: ResonanceResult;
  reflection?: ReflectionResult;
  userInput: Record<string, unknown>;
  timestamp?: number;
}

export interface MasteryResult {
  demonstrated: boolean;
  score: number;
  total: number;
  message?: string;
}

export interface ResonanceResult {
  subscaleContributions: Record<string, number>;
  selectedChoiceIds?: string[];
  confidence?: number;
}

export interface ReflectionResult {
  subscaleContributions: Record<string, number>;
  rawValue?: number;
  selectedChoiceIds?: string[];
  openResponse?: string;
  confidence?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Aggregated Results
// ─────────────────────────────────────────────────────────────────────────────

export interface AggregatedReflection {
  subscaleTotals: Record<string, number>;
  subscaleCounts: Record<string, number>;
  normalizedScores: Record<string, number>;
  momentCount: number;
  momentIds: string[];
  aggregatedAt: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility Types
// ─────────────────────────────────────────────────────────────────────────────

export function isMasteryMoment(moment: Moment): boolean {
  return moment.purpose === 'mastery';
}

export function isDiscoveryMoment(moment: Moment): boolean {
  return moment.purpose === 'discovery' || moment.purpose === 'reflection';
}

/**
 * Get the primary (highest scoring) subscale from a totals map.
 *
 * @deprecated Prefer using `getPsycheAPI().getPrimarySubscale(aggregated)` from
 * sophia-element-loader.ts which accepts the full AggregatedReflection object
 * and provides more robust handling.
 */
export function getPrimarySubscale(totals: Record<string, number>): string | undefined {
  const entries = Object.entries(totals);
  if (entries.length === 0) return undefined;

  let maxSubscale = entries[0][0];
  let maxValue = entries[0][1];

  for (const [subscale, value] of entries) {
    if (value > maxValue) {
      maxValue = value;
      maxSubscale = subscale;
    }
  }

  return maxSubscale;
}

// ─────────────────────────────────────────────────────────────────────────────
// Backward Compatibility Aliases (Perseus → Sophia)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * PerseusItem is now Moment - aliased for backward compatibility.
 * A "Moment" is a unit of assessment that may be a mastery question,
 * discovery prompt, or reflection item.
 */
export interface PerseusItem extends Moment {
  /** @deprecated Use purpose instead */
  question?: PerseusRenderer;
}

/**
 * PerseusScoreResult is now Recognition - aliased for backward compatibility.
 * "Recognition" acknowledges what the learner demonstrated, not just correctness.
 */
export interface PerseusScoreResult {
  /** Whether the answer was correct (for mastery) */
  correct: boolean;
  /** Score message */
  message?: string;
  /** Empty state indicator */
  empty?: boolean;
  /** Guess indicator */
  guess?: boolean;
}

/** @deprecated Use Moment instead */
export type PerseusQuestion = Moment;

// Bloom's taxonomy levels for question classification
export type BloomsLevel = 'remember' | 'understand' | 'apply' | 'analyze' | 'evaluate' | 'create';

// Question difficulty levels
export type QuestionDifficulty = 'easy' | 'medium' | 'hard';

/**
 * Convert Recognition to legacy PerseusScoreResult format.
 */
export function recognitionToScoreResult(recognition: Recognition): PerseusScoreResult {
  return {
    correct: recognition.mastery?.demonstrated ?? false,
    message: recognition.mastery?.message,
    empty: Object.keys(recognition.userInput).length === 0,
    guess: false
  };
}

/**
 * Convert legacy PerseusItem format to Moment.
 * Handles both the old { question: {...} } format and new { content: {...} } format.
 */
export function perseusItemToMoment(item: Record<string, unknown>): Moment {
  // Handle legacy format with 'question' property
  if (item['question'] && !item['content']) {
    return {
      id: (item['id'] as string) ?? crypto.randomUUID(),
      purpose: 'mastery',
      content: item['question'] as PerseusRenderer,
      hints: item['hints'] as Hint[] | undefined,
      metadata: item['metadata'] as MomentMetadata | undefined
    };
  }

  // Already in Moment format
  return item as unknown as Moment;
}
