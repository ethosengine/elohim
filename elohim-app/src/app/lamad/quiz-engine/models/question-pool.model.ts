/**
 * Question Pool Model - Questions associated with ContentNodes.
 *
 * Each ContentNode can have an optional question pool containing
 * Perseus items that assess comprehension of that content.
 *
 * Question pools support:
 * - Practice quizzes (unlimited, from hierarchy)
 * - Mastery quizzes (limited attempts, weighted)
 * - Inline quizzes (post-content attestation)
 *
 * @example
 * ```typescript
 * const pool: QuestionPool = {
 *   contentId: 'governance-basics',
 *   questions: [...perseusItems],
 *   metadata: {
 *     minPracticeQuestions: 3,
 *     minMasteryQuestions: 5
 *   }
 * };
 * ```
 */

import type {
  PerseusItem,
  BloomsLevel,
  QuestionDifficulty,
} from '../../content-io/plugins/sophia/sophia-moment.model';

// @coverage: 31.0% (2026-02-04)

// ─────────────────────────────────────────────────────────────────────────────
// Question Pool
// ─────────────────────────────────────────────────────────────────────────────

/**
 * QuestionPool - Questions associated with a ContentNode.
 *
 * Every ContentNode can have an optional question pool for assessment.
 */
export interface QuestionPool {
  /** ContentNode ID this pool belongs to */
  contentId: string;

  /** Questions in this pool */
  questions: PerseusItem[];

  /** Pool metadata and configuration */
  metadata: QuestionPoolMetadata;

  /** When pool was last updated */
  updatedAt: string;

  /** When pool was created */
  createdAt: string;

  /** Version for optimistic locking */
  version: number;
}

/**
 * Metadata and configuration for a question pool.
 */
export interface QuestionPoolMetadata {
  /** Minimum questions needed for practice quiz */
  minPracticeQuestions: number;

  /** Minimum questions needed for mastery quiz */
  minMasteryQuestions: number;

  /** Distribution targets by Bloom's level */
  bloomsDistribution: BloomsDistribution;

  /** Distribution targets by difficulty */
  difficultyDistribution: DifficultyDistribution;

  /** Whether this pool is complete (has enough questions) */
  isComplete: boolean;

  /** Tags for filtering */
  tags: string[];

  /** Source document(s) questions were derived from */
  sourceDocs: string[];
}

/**
 * Target distribution of questions by Bloom's taxonomy level.
 */
export interface BloomsDistribution {
  remember: number;
  understand: number;
  apply: number;
  analyze: number;
  evaluate?: number;
  create?: number;
}

/**
 * Target distribution of questions by difficulty.
 */
export interface DifficultyDistribution {
  easy: number;
  medium: number;
  hard: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Hierarchical Question Source
// ─────────────────────────────────────────────────────────────────────────────

/**
 * HierarchicalQuestionSource - For drawing questions from content hierarchy.
 *
 * Practice quizzes draw from content below the current position in the path,
 * allowing learners to demonstrate knowledge of prerequisite material.
 */
export interface HierarchicalQuestionSource {
  /** Current position in hierarchy */
  currentContentId: string;

  /** Path context for hierarchy awareness */
  pathId: string;

  /** Section ID within the path */
  sectionId?: string;

  /** Chapter ID within the path */
  chapterId?: string;

  /** Content IDs eligible for question selection */
  eligibleContentIds: string[];

  /** Combined question pool from all eligible content */
  combinedPool: PerseusItem[];

  /** Statistics about the combined pool */
  stats: HierarchicalPoolStats;
}

/**
 * Statistics about a hierarchical question pool.
 */
export interface HierarchicalPoolStats {
  /** Total questions available */
  totalQuestions: number;

  /** Questions by content ID */
  questionsByContent: Map<string, number>;

  /** Questions by Bloom's level */
  questionsByBlooms: Record<BloomsLevel, number>;

  /** Questions by difficulty */
  questionsByDifficulty: Record<QuestionDifficulty, number>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Question Selection
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Options for selecting questions from a pool.
 */
export interface QuestionSelectionOptions {
  /** Number of questions to select */
  count: number;

  /** Filter by Bloom's level(s) */
  bloomsLevels?: BloomsLevel[];

  /** Filter by difficulty */
  difficulty?: QuestionDifficulty[];

  /** Filter by tags */
  tags?: string[];

  /** Exclude specific question IDs (already answered) */
  excludeIds?: string[];

  /** Weight toward specific content IDs (for mastery quizzes) */
  weightedContentIds?: Map<string, number>;

  /** Randomize order */
  randomize?: boolean;

  /** Ensure variety (don't repeat content IDs) */
  ensureVariety?: boolean;
}

/**
 * Result of question selection.
 */
export interface QuestionSelectionResult {
  /** Selected questions */
  questions: PerseusItem[];

  /** Whether selection met all criteria */
  selectionComplete: boolean;

  /** Reasons if selection was incomplete */
  selectionNotes?: string[];

  /** Content IDs represented in selection */
  contentIds: string[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Pool Operations
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Request to create or update a question pool.
 */
export interface QuestionPoolUpsertRequest {
  /** Target content ID */
  contentId: string;

  /** Questions to add/replace */
  questions: PerseusItem[];

  /** Whether to replace existing pool or merge */
  mode: 'replace' | 'merge';

  /** Source information */
  sourceDoc?: string;
}

/**
 * Query for finding question pools.
 */
export interface QuestionPoolQuery {
  /** Filter by content IDs */
  contentIds?: string[];

  /** Filter by tags */
  tags?: string[];

  /** Minimum question count */
  minQuestions?: number;

  /** Filter by completeness */
  isComplete?: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Create an empty question pool for a content node.
 */
export function createEmptyPool(contentId: string): QuestionPool {
  const now = new Date().toISOString();
  return {
    contentId,
    questions: [],
    metadata: {
      minPracticeQuestions: 3,
      minMasteryQuestions: 5,
      bloomsDistribution: {
        remember: 2,
        understand: 2,
        apply: 1,
        analyze: 0,
      },
      difficultyDistribution: {
        easy: 2,
        medium: 2,
        hard: 1,
      },
      isComplete: false,
      tags: [],
      sourceDocs: [],
    },
    createdAt: now,
    updatedAt: now,
    version: 1,
  };
}

/**
 * Check if a pool has enough questions for practice.
 */
export function canPractice(pool: QuestionPool): boolean {
  return pool.questions.length >= pool.metadata.minPracticeQuestions;
}

/**
 * Check if a pool has enough questions for mastery quiz.
 */
export function canMastery(pool: QuestionPool): boolean {
  return pool.questions.length >= pool.metadata.minMasteryQuestions;
}

/**
 * Calculate pool completeness percentage.
 */
export function calculateCompleteness(pool: QuestionPool): number {
  const { bloomsDistribution, difficultyDistribution } = pool.metadata;

  // Total target questions
  const bloomsTotal = Object.values(bloomsDistribution).reduce((a, b) => a + b, 0);
  const difficultyTotal = Object.values(difficultyDistribution).reduce((a, b) => a + b, 0);
  const targetTotal = Math.max(bloomsTotal, difficultyTotal);

  if (targetTotal === 0) return 100;

  // Calculate actual distribution
  const actualBlooms: Record<string, number> = {};
  const actualDifficulty: Record<string, number> = {};

  for (const q of pool.questions) {
    const level = (q.metadata?.['bloomsLevel'] as string) ?? 'understand';
    const diff = (q.metadata?.['difficulty'] as string) ?? 'medium';
    actualBlooms[level] = (actualBlooms[level] ?? 0) + 1;
    actualDifficulty[diff] = (actualDifficulty[diff] ?? 0) + 1;
  }

  // Score is based on meeting targets
  let metTargets = 0;
  let totalTargets = 0;

  for (const [level, target] of Object.entries(bloomsDistribution)) {
    if (target > 0) {
      totalTargets++;
      if ((actualBlooms[level] ?? 0) >= target) {
        metTargets++;
      }
    }
  }

  for (const [diff, target] of Object.entries(difficultyDistribution)) {
    if (target > 0) {
      totalTargets++;
      if ((actualDifficulty[diff] ?? 0) >= target) {
        metTargets++;
      }
    }
  }

  return totalTargets > 0 ? Math.round((metTargets / totalTargets) * 100) : 100;
}
