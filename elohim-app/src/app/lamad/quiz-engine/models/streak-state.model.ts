/**
 * Streak State Model - Tracking consecutive correct answers.
 *
 * Used for inline quizzes where learners must get 3-5 questions
 * correct in a row to earn the "practiced" attestation.
 *
 * Features:
 * - Configurable target streak (default: 3)
 * - Maximum questions before forcing completion
 * - History tracking for UI display
 * - Persistence for session recovery
 *
 * @example
 * ```typescript
 * const streak = createStreakState('concept-123', { targetStreak: 3 });
 * streak = recordAnswer(streak, true);  // correct
 * streak = recordAnswer(streak, true);  // correct
 * streak = recordAnswer(streak, true);  // correct - achieved!
 * console.log(isStreakAchieved(streak)); // true
 * ```
 */

// ─────────────────────────────────────────────────────────────────────────────
// Streak State
// ─────────────────────────────────────────────────────────────────────────────

/**
 * StreakState - Tracks progress toward streak-based attestation.
 */
export interface StreakState {
  /** Content ID being practiced */
  contentId: string;

  /** Human ID */
  humanId: string;

  /** Current consecutive correct count */
  currentStreak: number;

  /** Target streak to achieve attestation */
  targetStreak: number;

  /** Maximum questions before auto-complete (0 = unlimited) */
  maxQuestions: number;

  /** Total correct answers in session */
  totalCorrect: number;

  /** Total questions attempted */
  totalAttempted: number;

  /** Recent answer history (for UI display) */
  recentAnswers: StreakAnswer[];

  /** Best streak achieved in this session */
  bestStreak: number;

  /** Whether target has been achieved */
  achieved: boolean;

  /** When streak tracking started */
  startedAt: string;

  /** When target was achieved (if achieved) */
  achievedAt?: string;
}

/**
 * Individual answer record in streak history.
 */
export interface StreakAnswer {
  /** Question ID */
  questionId: string;

  /** Whether correct */
  correct: boolean;

  /** Streak count after this answer */
  streakAfter: number;

  /** When answered */
  answeredAt: string;
}

/**
 * Configuration for streak tracking.
 */
export interface StreakConfig {
  /** Number of consecutive correct answers needed */
  targetStreak: number;

  /** Maximum questions before forcing completion (0 = unlimited) */
  maxQuestions: number;

  /** Reset streak on wrong answer (true) or just don't increment (false) */
  resetOnWrong: boolean;

  /** How many recent answers to keep in history */
  historyLength: number;
}

/**
 * Default streak configuration.
 */
export const DEFAULT_STREAK_CONFIG: StreakConfig = {
  targetStreak: 3,
  maxQuestions: 10,
  resetOnWrong: true,
  historyLength: 10
};

// ─────────────────────────────────────────────────────────────────────────────
// Streak Progress
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Progress toward streak achievement.
 */
export interface StreakProgress {
  /** Current streak count */
  current: number;

  /** Target streak */
  target: number;

  /** Progress percentage (0-100) */
  percentage: number;

  /** Whether achieved */
  achieved: boolean;

  /** Questions remaining before max (null if unlimited) */
  questionsRemaining: number | null;

  /** Visual indicators for UI */
  indicators: StreakIndicator[];
}

/**
 * Visual indicator for streak display.
 */
export interface StreakIndicator {
  /** Index in streak (0-based) */
  index: number;

  /** State of this position */
  state: 'empty' | 'correct' | 'incorrect' | 'current';
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Create initial streak state for a content node.
 */
export function createStreakState(
  contentId: string,
  humanId: string,
  config: Partial<StreakConfig> = {}
): StreakState {
  const fullConfig = { ...DEFAULT_STREAK_CONFIG, ...config };

  return {
    contentId,
    humanId,
    currentStreak: 0,
    targetStreak: fullConfig.targetStreak,
    maxQuestions: fullConfig.maxQuestions,
    totalCorrect: 0,
    totalAttempted: 0,
    recentAnswers: [],
    bestStreak: 0,
    achieved: false,
    startedAt: new Date().toISOString()
  };
}

/**
 * Record an answer and update streak state.
 * Returns a new state object (immutable).
 */
export function recordAnswer(
  state: StreakState,
  questionId: string,
  correct: boolean,
  config: Partial<StreakConfig> = {}
): StreakState {
  const fullConfig = { ...DEFAULT_STREAK_CONFIG, ...config };

  // Calculate new streak
  let newStreak: number;
  if (correct) {
    newStreak = state.currentStreak + 1;
  } else if (fullConfig.resetOnWrong) {
    newStreak = 0;
  } else {
    newStreak = state.currentStreak;
  }

  // Track best streak
  const newBestStreak = Math.max(state.bestStreak, newStreak);

  // Check if achieved
  const achieved = newStreak >= state.targetStreak;

  // Create answer record
  const answer: StreakAnswer = {
    questionId,
    correct,
    streakAfter: newStreak,
    answeredAt: new Date().toISOString()
  };

  // Update history (keep last N)
  const newHistory = [...state.recentAnswers, answer].slice(-fullConfig.historyLength);

  return {
    ...state,
    currentStreak: newStreak,
    totalCorrect: state.totalCorrect + (correct ? 1 : 0),
    totalAttempted: state.totalAttempted + 1,
    recentAnswers: newHistory,
    bestStreak: newBestStreak,
    achieved: achieved || state.achieved, // Once achieved, stays achieved
    achievedAt: achieved && !state.achieved ? new Date().toISOString() : state.achievedAt
  };
}

/**
 * Check if streak target has been achieved.
 */
export function isStreakAchieved(state: StreakState): boolean {
  return state.achieved;
}

/**
 * Check if max questions reached.
 */
export function isMaxQuestionsReached(state: StreakState): boolean {
  return state.maxQuestions > 0 && state.totalAttempted >= state.maxQuestions;
}

/**
 * Check if streak session is complete (achieved or max reached).
 */
export function isStreakComplete(state: StreakState): boolean {
  return state.achieved || isMaxQuestionsReached(state);
}

/**
 * Get progress toward streak achievement.
 */
export function getStreakProgress(state: StreakState): StreakProgress {
  const percentage = Math.min(100, Math.round((state.currentStreak / state.targetStreak) * 100));

  const questionsRemaining = state.maxQuestions > 0
    ? Math.max(0, state.maxQuestions - state.totalAttempted)
    : null;

  // Build indicators for UI
  const indicators: StreakIndicator[] = [];
  for (let i = 0; i < state.targetStreak; i++) {
    let indicatorState: StreakIndicator['state'] = 'empty';

    if (i < state.currentStreak) {
      indicatorState = 'correct';
    } else if (i === state.currentStreak && state.recentAnswers.length > 0) {
      const lastAnswer = state.recentAnswers[state.recentAnswers.length - 1];
      if (!lastAnswer.correct && i === 0) {
        indicatorState = 'incorrect';
      }
    }

    indicators.push({ index: i, state: indicatorState });
  }

  return {
    current: state.currentStreak,
    target: state.targetStreak,
    percentage,
    achieved: state.achieved,
    questionsRemaining,
    indicators
  };
}

/**
 * Reset streak state for retry.
 */
export function resetStreak(state: StreakState): StreakState {
  return {
    ...state,
    currentStreak: 0,
    totalCorrect: 0,
    totalAttempted: 0,
    recentAnswers: [],
    achieved: false,
    achievedAt: undefined,
    startedAt: new Date().toISOString()
  };
}

/**
 * Serialize streak state for persistence.
 */
export function serializeStreakState(state: StreakState): string {
  return JSON.stringify(state);
}

/**
 * Deserialize streak state from persistence.
 */
export function deserializeStreakState(json: string): StreakState | null {
  try {
    return JSON.parse(json) as StreakState;
  } catch {
    return null;
  }
}
