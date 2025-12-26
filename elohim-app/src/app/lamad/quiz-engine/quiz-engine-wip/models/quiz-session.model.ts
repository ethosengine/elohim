/**
 * Quiz Session Model - State machine for quiz flows.
 *
 * Manages the lifecycle of practice, mastery, and inline quizzes
 * with state transitions, response tracking, and result aggregation.
 *
 * Session Types:
 * - Practice: Unlimited attempts, draws from content hierarchy
 * - Mastery: Limited attempts (2/day), gates section progression
 * - Inline: Post-content attestation, 3-in-a-row target
 * - PreAssessment: Skip-ahead assessment at path start
 *
 * @example
 * ```typescript
 * const session = createQuizSession('practice', humanId, pathContext);
 * session.state = 'in_progress';
 * // ... user answers questions
 * const result = completeSession(session);
 * ```
 */

import type { PerseusItem, PerseusScoreResult } from '../../content-io/plugins/perseus/perseus-item.model';
import type { MasteryLevel } from '../../models/content-mastery.model';

// ─────────────────────────────────────────────────────────────────────────────
// Session Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Types of quiz sessions supported.
 */
export type QuizSessionType =
  | 'practice'        // Practice quiz - unlimited, from hierarchy
  | 'mastery'         // Mastery quiz - limited attempts, gates progression
  | 'inline'          // Post-content attestation (3-in-a-row)
  | 'pre-assessment'; // Skip-ahead assessment at path start

/**
 * States in the quiz session lifecycle.
 */
export type QuizSessionState =
  | 'not_started'   // Session created but not begun
  | 'in_progress'   // User is actively answering questions
  | 'paused'        // Session paused (can resume)
  | 'completed'     // All questions answered
  | 'abandoned'     // User left without completing
  | 'passed'        // Completed with passing score
  | 'failed';       // Completed but did not pass

// ─────────────────────────────────────────────────────────────────────────────
// Quiz Session
// ─────────────────────────────────────────────────────────────────────────────

/**
 * QuizSession - Active quiz state machine.
 *
 * Tracks the full lifecycle of a quiz attempt including questions,
 * responses, timing, and context.
 */
export interface QuizSession {
  /** Unique session ID */
  id: string;

  /** Human taking the quiz */
  humanId: string;

  /** Quiz type determines flow and rules */
  type: QuizSessionType;

  /** Current state in lifecycle */
  state: QuizSessionState;

  /** Questions in this session (ordered) */
  questions: SessionQuestion[];

  /** Current question index (0-based) */
  currentIndex: number;

  /** Responses recorded so far */
  responses: QuizResponse[];

  /** Session timing */
  timing: SessionTiming;

  /** Context for path-aware quizzes */
  pathContext?: PathQuizContext;

  /** For mastery quizzes: attempt tracking */
  attemptInfo?: AttemptInfo;

  /** For inline quizzes: streak tracking */
  streakInfo?: StreakInfo;

  /** Configuration for this session */
  config: QuizSessionConfig;

  /** Serialized state for persistence */
  savedState?: unknown;
}

/**
 * A question within a session.
 */
export interface SessionQuestion {
  /** Perseus item */
  item: PerseusItem;

  /** Source content ID */
  contentId: string;

  /** Question index in session */
  index: number;

  /** Whether answered */
  answered: boolean;

  /** Whether correct (after scoring) */
  correct?: boolean;

  /** Score (0-1 for partial credit) */
  score?: number;

  /** Time spent on this question (ms) */
  timeSpentMs: number;

  /** Whether hint was used */
  hintUsed: boolean;

  /** Attempt count for this question */
  attempts: number;
}

/**
 * A user's response to a question.
 */
export interface QuizResponse {
  /** Question ID */
  questionId: string;

  /** Session question index */
  questionIndex: number;

  /** Content ID the question assesses */
  contentId: string;

  /** User's answer (Perseus widget-specific format) */
  response: unknown;

  /** Whether the answer was correct */
  correct: boolean;

  /** Score from 0 to 1 */
  score: number;

  /** When the response was submitted */
  answeredAt: string;

  /** Time spent on this question (ms) */
  timeSpentMs: number;

  /** Attempt number (1 = first try) */
  attemptNumber: number;

  /** Whether hint was viewed before answering */
  hintViewed: boolean;

  /** Full Perseus score result */
  perseusResult?: PerseusScoreResult;
}

/**
 * Session timing information.
 */
export interface SessionTiming {
  /** When session was created */
  createdAt: string;

  /** When session was started (first question viewed) */
  startedAt?: string;

  /** When session was completed/abandoned */
  endedAt?: string;

  /** Total time spent in session (ms) */
  totalTimeMs: number;

  /** Time limit if any (ms) */
  timeLimitMs?: number;

  /** Whether time limit was exceeded */
  timeExceeded?: boolean;
}

/**
 * Path context for quizzes within a learning path.
 */
export interface PathQuizContext {
  /** Learning path ID */
  pathId: string;

  /** Current section ID */
  sectionId: string;

  /** Current chapter ID */
  chapterId?: string;

  /** Current module ID */
  moduleId?: string;

  /** Step index in path */
  stepIndex?: number;

  /** Content IDs in current section */
  sectionContentIds: string[];
}

/**
 * Attempt tracking for mastery quizzes.
 */
export interface AttemptInfo {
  /** Current attempt number */
  attemptNumber: number;

  /** Maximum attempts allowed */
  maxAttempts: number;

  /** When cooldown ends (if in cooldown) */
  cooldownEndsAt?: string;

  /** Previous attempt scores */
  previousScores: number[];
}

/**
 * Streak tracking for inline quizzes.
 */
export interface StreakInfo {
  /** Current consecutive correct answers */
  currentStreak: number;

  /** Target streak for attestation */
  targetStreak: number;

  /** Maximum streak achieved in session */
  maxStreak: number;

  /** History of recent answers (true = correct) */
  recentAnswers: boolean[];

  /** Whether target was achieved */
  targetAchieved: boolean;
}

/**
 * Configuration for a quiz session.
 */
export interface QuizSessionConfig {
  /** Passing score threshold (0-1) */
  passingScore: number;

  /** Allow retrying incorrect answers */
  allowRetry: boolean;

  /** Maximum retries per question */
  maxRetriesPerQuestion: number;

  /** Show feedback after each answer */
  showImmediateFeedback: boolean;

  /** Show correct answer after wrong attempt */
  showCorrectAnswer: boolean;

  /** Randomize question order */
  randomizeQuestions: boolean;

  /** Time limit per question (ms, 0 = none) */
  timeLimitPerQuestion: number;

  /** Total session time limit (ms, 0 = none) */
  totalTimeLimit: number;

  /** Allow skipping questions */
  allowSkip: boolean;

  /** Can navigate back to previous questions */
  allowBackNavigation: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Quiz Result
// ─────────────────────────────────────────────────────────────────────────────

/**
 * QuizResult - Final result of a completed quiz session.
 */
export interface QuizResult {
  /** Session ID */
  sessionId: string;

  /** Quiz type */
  type: QuizSessionType;

  /** Human who took the quiz */
  humanId: string;

  /** Overall score (0-1) */
  score: number;

  /** Whether passed based on threshold */
  passed: boolean;

  /** Passing threshold used */
  passingThreshold: number;

  /** Number of correct answers */
  correctCount: number;

  /** Total questions */
  totalQuestions: number;

  /** Per-content performance */
  contentScores: ContentScore[];

  /** Mastery level changes triggered */
  masteryChanges: MasteryChange[];

  /** Attestations granted */
  attestationsGranted: string[];

  /** For inline quizzes: streak achievement */
  streakResult?: StreakResult;

  /** Path context if applicable */
  pathContext?: PathQuizContext;

  /** Timing summary */
  timing: ResultTiming;

  /** When result was generated */
  completedAt: string;
}

/**
 * Per-content score breakdown.
 */
export interface ContentScore {
  /** Content ID */
  contentId: string;

  /** Questions answered for this content */
  questionsAnswered: number;

  /** Correct answers */
  correctAnswers: number;

  /** Average score */
  averageScore: number;

  /** Bloom's levels tested */
  bloomsLevelsTested: string[];
}

/**
 * Mastery level change triggered by quiz.
 */
export interface MasteryChange {
  /** Content ID */
  contentId: string;

  /** Previous mastery level */
  fromLevel: MasteryLevel;

  /** New mastery level */
  toLevel: MasteryLevel;

  /** Direction of change */
  direction: 'up' | 'down' | 'same';

  /** Evidence for the change */
  evidence: string;
}

/**
 * Streak achievement result.
 */
export interface StreakResult {
  /** Whether target streak was achieved */
  achieved: boolean;

  /** Final streak count */
  finalStreak: number;

  /** Target streak */
  targetStreak: number;

  /** Maximum streak during session */
  maxStreak: number;
}

/**
 * Timing summary for results.
 */
export interface ResultTiming {
  /** Total session duration (ms) */
  totalDurationMs: number;

  /** Average time per question (ms) */
  averageTimePerQuestion: number;

  /** Fastest answer (ms) */
  fastestAnswerMs: number;

  /** Slowest answer (ms) */
  slowestAnswerMs: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// State Transitions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Valid state transitions for quiz sessions.
 */
export const VALID_TRANSITIONS: Record<QuizSessionState, QuizSessionState[]> = {
  not_started: ['in_progress', 'abandoned'],
  in_progress: ['paused', 'completed', 'abandoned', 'passed', 'failed'],
  paused: ['in_progress', 'abandoned'],
  completed: ['passed', 'failed'], // Terminal, but can resolve to pass/fail
  abandoned: [], // Terminal
  passed: [], // Terminal
  failed: [] // Terminal
};

/**
 * Check if a state transition is valid.
 */
export function isValidTransition(from: QuizSessionState, to: QuizSessionState): boolean {
  return VALID_TRANSITIONS[from]?.includes(to) ?? false;
}

/**
 * Check if a state is terminal (no further transitions).
 */
export function isTerminalState(state: QuizSessionState): boolean {
  return VALID_TRANSITIONS[state]?.length === 0;
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Default configurations by quiz type.
 */
export const DEFAULT_CONFIGS: Record<QuizSessionType, QuizSessionConfig> = {
  practice: {
    passingScore: 0.7,
    allowRetry: true,
    maxRetriesPerQuestion: 2,
    showImmediateFeedback: true,
    showCorrectAnswer: true,
    randomizeQuestions: true,
    timeLimitPerQuestion: 0,
    totalTimeLimit: 0,
    allowSkip: true,
    allowBackNavigation: true
  },
  mastery: {
    passingScore: 0.8,
    allowRetry: false,
    maxRetriesPerQuestion: 0,
    showImmediateFeedback: false,
    showCorrectAnswer: false,
    randomizeQuestions: true,
    timeLimitPerQuestion: 120000, // 2 minutes
    totalTimeLimit: 0,
    allowSkip: false,
    allowBackNavigation: false
  },
  inline: {
    passingScore: 0.7,
    allowRetry: true,
    maxRetriesPerQuestion: 1,
    showImmediateFeedback: true,
    showCorrectAnswer: false,
    randomizeQuestions: true,
    timeLimitPerQuestion: 0,
    totalTimeLimit: 0,
    allowSkip: false,
    allowBackNavigation: false
  },
  'pre-assessment': {
    passingScore: 0.7,
    allowRetry: false,
    maxRetriesPerQuestion: 0,
    showImmediateFeedback: false,
    showCorrectAnswer: false,
    randomizeQuestions: true,
    timeLimitPerQuestion: 60000, // 1 minute
    totalTimeLimit: 1800000, // 30 minutes
    allowSkip: true,
    allowBackNavigation: false
  }
};

/**
 * Create a new quiz session.
 */
export function createQuizSession(
  type: QuizSessionType,
  humanId: string,
  questions: PerseusItem[],
  pathContext?: PathQuizContext,
  configOverrides?: Partial<QuizSessionConfig>
): QuizSession {
  const now = new Date().toISOString();
  const config = { ...DEFAULT_CONFIGS[type], ...configOverrides };

  // Map questions to session questions
  const sessionQuestions: SessionQuestion[] = questions.map((item, index) => ({
    item,
    contentId: item.metadata?.assessesContentId ?? '',
    index,
    answered: false,
    timeSpentMs: 0,
    hintUsed: false,
    attempts: 0
  }));

  // Randomize if configured
  if (config.randomizeQuestions) {
    shuffleArray(sessionQuestions);
    sessionQuestions.forEach((q, i) => q.index = i);
  }

  const session: QuizSession = {
    id: generateSessionId(),
    humanId,
    type,
    state: 'not_started',
    questions: sessionQuestions,
    currentIndex: 0,
    responses: [],
    timing: {
      createdAt: now,
      totalTimeMs: 0,
      timeLimitMs: config.totalTimeLimit || undefined
    },
    config
  };

  // Add path context if provided
  if (pathContext) {
    session.pathContext = pathContext;
  }

  // Initialize type-specific info
  if (type === 'inline') {
    session.streakInfo = {
      currentStreak: 0,
      targetStreak: 3,
      maxStreak: 0,
      recentAnswers: [],
      targetAchieved: false
    };
  }

  if (type === 'mastery') {
    session.attemptInfo = {
      attemptNumber: 1,
      maxAttempts: 2,
      previousScores: []
    };
  }

  return session;
}

/**
 * Generate a unique session ID.
 */
function generateSessionId(): string {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 8);
  return `qs-${timestamp}-${random}`;
}

/**
 * Fisher-Yates shuffle for randomizing questions.
 */
function shuffleArray<T>(array: T[]): void {
  for (let i = array.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [array[i], array[j]] = [array[j], array[i]];
  }
}

/**
 * Calculate result from a completed session.
 */
export function calculateQuizResult(session: QuizSession): QuizResult {
  const responses = session.responses;
  const correctCount = responses.filter(r => r.correct).length;
  const score = responses.length > 0 ? correctCount / responses.length : 0;
  const passed = score >= session.config.passingScore;

  // Calculate per-content scores
  const contentScoreMap = new Map<string, { correct: number; total: number; scores: number[]; blooms: Set<string> }>();

  for (const response of responses) {
    const existing = contentScoreMap.get(response.contentId) ?? {
      correct: 0,
      total: 0,
      scores: [],
      blooms: new Set<string>()
    };
    existing.total++;
    existing.scores.push(response.score);
    if (response.correct) existing.correct++;

    // Get Bloom's level from question
    const question = session.questions.find(q => q.item.id === response.questionId);
    if (question?.item.metadata?.bloomsLevel) {
      existing.blooms.add(question.item.metadata.bloomsLevel);
    }

    contentScoreMap.set(response.contentId, existing);
  }

  const contentScores: ContentScore[] = Array.from(contentScoreMap.entries()).map(([contentId, data]) => ({
    contentId,
    questionsAnswered: data.total,
    correctAnswers: data.correct,
    averageScore: data.scores.reduce((a, b) => a + b, 0) / data.scores.length,
    bloomsLevelsTested: Array.from(data.blooms)
  }));

  // Calculate timing
  const times = responses.map(r => r.timeSpentMs);
  const timing: ResultTiming = {
    totalDurationMs: session.timing.totalTimeMs,
    averageTimePerQuestion: times.length > 0 ? times.reduce((a, b) => a + b, 0) / times.length : 0,
    fastestAnswerMs: times.length > 0 ? Math.min(...times) : 0,
    slowestAnswerMs: times.length > 0 ? Math.max(...times) : 0
  };

  const result: QuizResult = {
    sessionId: session.id,
    type: session.type,
    humanId: session.humanId,
    score,
    passed,
    passingThreshold: session.config.passingScore,
    correctCount,
    totalQuestions: session.questions.length,
    contentScores,
    masteryChanges: [], // Populated by mastery service
    attestationsGranted: [], // Populated by attestation service
    pathContext: session.pathContext,
    timing,
    completedAt: new Date().toISOString()
  };

  // Add streak result for inline quizzes
  if (session.type === 'inline' && session.streakInfo) {
    result.streakResult = {
      achieved: session.streakInfo.targetAchieved,
      finalStreak: session.streakInfo.currentStreak,
      targetStreak: session.streakInfo.targetStreak,
      maxStreak: session.streakInfo.maxStreak
    };
  }

  return result;
}
