/**
 * Attempt Record Model - Tracking mastery quiz attempts and cooldowns.
 *
 * Mastery quizzes are limited to prevent brute-force memorization:
 * - 2 attempts per day (configurable)
 * - 4-hour cooldown between attempts
 * - Daily reset at midnight
 *
 * This creates meaningful spaced repetition where learners must
 * actually learn the material rather than just retrying immediately.
 *
 * @example
 * ```typescript
 * const record = getAttemptRecord('concept-123', 'human-456');
 * const canAttempt = checkCanAttempt(record);
 * if (canAttempt.allowed) {
 *   // Start mastery quiz
 *   const newRecord = recordAttempt(record, quizResult);
 * } else {
 * }
 * ```
 */

import type { QuizResult } from './quiz-session.model';

// @coverage: 66.7% (2026-02-05)

// ─────────────────────────────────────────────────────────────────────────────
// Attempt Record
// ─────────────────────────────────────────────────────────────────────────────

/**
 * AttemptRecord - Tracks mastery quiz attempts per content.
 */
export interface AttemptRecord {
  /** Content node ID */
  contentId: string;

  /** Human ID */
  humanId: string;

  /** Attempts made today */
  attemptsToday: number;

  /** Maximum attempts allowed per day */
  maxAttemptsPerDay: number;

  /** When attempts reset (midnight local or configured time) */
  resetsAt: string;

  /** Timestamp of last attempt */
  lastAttemptAt: string | null;

  /** Cooldown end time (if in cooldown) */
  cooldownEndsAt: string | null;

  /** Best score achieved */
  bestScore: number;

  /** Whether content has been mastered (passed mastery quiz) */
  mastered: boolean;

  /** When mastery was achieved */
  masteredAt: string | null;

  /** Historical attempt records */
  attemptHistory: AttemptHistoryEntry[];
}

/**
 * Single attempt in history.
 */
export interface AttemptHistoryEntry {
  /** Session ID of the attempt */
  sessionId: string;

  /** When attempt was made */
  attemptedAt: string;

  /** Score achieved (0-1) */
  score: number;

  /** Whether passed */
  passed: boolean;

  /** Time spent (ms) */
  durationMs: number;

  /** Questions answered correctly */
  correctCount: number;

  /** Total questions */
  totalQuestions: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Cooldown Configuration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Configuration for attempt limits and cooldowns.
 */
export interface CooldownConfig {
  /** Maximum mastery quiz attempts per day */
  masteryAttemptsPerDay: number;

  /** Cooldown between attempts (hours) */
  cooldownHours: number;

  /** Hour of day when attempts reset (0-23, local time) */
  dailyResetHour: number;

  /** Minimum time between attempts even without cooldown (minutes) */
  minTimeBetweenAttempts: number;
}

/**
 * Default cooldown configuration.
 */
export const DEFAULT_COOLDOWN_CONFIG: CooldownConfig = {
  masteryAttemptsPerDay: 2,
  cooldownHours: 4,
  dailyResetHour: 0, // Midnight
  minTimeBetweenAttempts: 5,
};

// ─────────────────────────────────────────────────────────────────────────────
// Attempt Check Result
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Result of checking if an attempt is allowed.
 */
export interface AttemptCheckResult {
  /** Whether attempt is allowed */
  allowed: boolean;

  /** Reason if not allowed */
  reason?: AttemptDenialReason;

  /** When cooldown ends (if in cooldown) */
  cooldownEndsAt?: string;

  /** Remaining attempts today */
  remainingAttempts: number;

  /** When attempts reset */
  resetsAt: string;

  /** Human-readable message */
  message: string;
}

/**
 * Reasons an attempt may be denied.
 */
export type AttemptDenialReason =
  | 'max_attempts_reached' // Used all attempts for the day
  | 'in_cooldown' // Cooldown period active
  | 'too_soon' // Too soon after last attempt
  | 'already_mastered'; // Already passed mastery quiz

// ─────────────────────────────────────────────────────────────────────────────
// Cooldown Status
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Current cooldown status for display.
 */
export interface CooldownStatus {
  /** Whether currently in cooldown */
  inCooldown: boolean;

  /** When cooldown ends */
  cooldownEndsAt: string | null;

  /** Time remaining in cooldown (ms) */
  remainingMs: number;

  /** Formatted time remaining (e.g., "2h 15m") */
  remainingFormatted: string;

  /** Attempts used today */
  attemptsUsed: number;

  /** Attempts remaining today */
  attemptsRemaining: number;

  /** When attempts reset */
  resetsAt: string;

  /** Time until reset (ms) */
  timeUntilResetMs: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Create a new attempt record for a content node.
 */
export function createAttemptRecord(
  contentId: string,
  humanId: string,
  config: Partial<CooldownConfig> = {}
): AttemptRecord {
  const fullConfig = { ...DEFAULT_COOLDOWN_CONFIG, ...config };

  return {
    contentId,
    humanId,
    attemptsToday: 0,
    maxAttemptsPerDay: fullConfig.masteryAttemptsPerDay,
    resetsAt: calculateResetTime(fullConfig.dailyResetHour),
    lastAttemptAt: null,
    cooldownEndsAt: null,
    bestScore: 0,
    mastered: false,
    masteredAt: null,
    attemptHistory: [],
  };
}

/**
 * Calculate the next reset time based on configured hour.
 */
function calculateResetTime(resetHour: number): string {
  const now = new Date();
  const reset = new Date(now);

  reset.setHours(resetHour, 0, 0, 0);

  // If reset time has passed today, use tomorrow
  if (reset <= now) {
    reset.setDate(reset.getDate() + 1);
  }

  return reset.toISOString();
}

/**
 * Check if an attempt is currently allowed.
 */
export function checkCanAttempt(
  record: AttemptRecord,
  config: Partial<CooldownConfig> = {}
): AttemptCheckResult {
  const fullConfig = { ...DEFAULT_COOLDOWN_CONFIG, ...config };
  const now = new Date();

  // Check if already mastered
  if (record.mastered) {
    return {
      allowed: false,
      reason: 'already_mastered',
      remainingAttempts: 0,
      resetsAt: record.resetsAt,
      message: 'You have already mastered this content.',
    };
  }

  // Check if reset time has passed (new day)
  const resetTime = new Date(record.resetsAt);
  let attemptsToday = record.attemptsToday;

  if (now >= resetTime) {
    // New day - reset attempts
    attemptsToday = 0;
  }

  // Check max attempts
  if (attemptsToday >= record.maxAttemptsPerDay) {
    return {
      allowed: false,
      reason: 'max_attempts_reached',
      remainingAttempts: 0,
      resetsAt: record.resetsAt,
      message: `You've used all ${record.maxAttemptsPerDay} attempts for today. Try again tomorrow.`,
    };
  }

  // Check cooldown
  if (record.cooldownEndsAt) {
    const cooldownEnd = new Date(record.cooldownEndsAt);
    if (now < cooldownEnd) {
      const remainingMs = cooldownEnd.getTime() - now.getTime();
      const remainingFormatted = formatDuration(remainingMs);

      return {
        allowed: false,
        reason: 'in_cooldown',
        cooldownEndsAt: record.cooldownEndsAt,
        remainingAttempts: record.maxAttemptsPerDay - attemptsToday,
        resetsAt: record.resetsAt,
        message: `Please wait ${remainingFormatted} before your next attempt.`,
      };
    }
  }

  // Check minimum time between attempts
  if (record.lastAttemptAt) {
    const lastAttempt = new Date(record.lastAttemptAt);
    const minTime = fullConfig.minTimeBetweenAttempts * 60 * 1000; // Convert to ms
    const timeSince = now.getTime() - lastAttempt.getTime();

    if (timeSince < minTime) {
      const waitTime = minTime - timeSince;
      return {
        allowed: false,
        reason: 'too_soon',
        cooldownEndsAt: new Date(now.getTime() + waitTime).toISOString(),
        remainingAttempts: record.maxAttemptsPerDay - attemptsToday,
        resetsAt: record.resetsAt,
        message: `Please wait ${formatDuration(waitTime)} before your next attempt.`,
      };
    }
  }

  // Attempt allowed
  return {
    allowed: true,
    remainingAttempts: record.maxAttemptsPerDay - attemptsToday,
    resetsAt: record.resetsAt,
    message: `You have ${record.maxAttemptsPerDay - attemptsToday} attempt(s) remaining today.`,
  };
}

/**
 * Record an attempt and update the record.
 * Returns a new record object (immutable).
 */
export function recordAttempt(
  record: AttemptRecord,
  result: QuizResult,
  config: Partial<CooldownConfig> = {}
): AttemptRecord {
  const fullConfig = { ...DEFAULT_COOLDOWN_CONFIG, ...config };
  const now = new Date();

  // Check if reset time has passed
  const resetTime = new Date(record.resetsAt);
  let attemptsToday = record.attemptsToday;

  if (now >= resetTime) {
    // New day - reset
    attemptsToday = 0;
  }

  // Create history entry
  const historyEntry: AttemptHistoryEntry = {
    sessionId: result.sessionId,
    attemptedAt: now.toISOString(),
    score: result.score,
    passed: result.passed,
    durationMs: result.timing.totalDurationMs,
    correctCount: result.correctCount,
    totalQuestions: result.totalQuestions,
  };

  // Calculate cooldown end
  const cooldownEndsAt = new Date(now.getTime() + fullConfig.cooldownHours * 60 * 60 * 1000);

  // Calculate new reset time if needed
  const newResetsAt =
    now >= resetTime ? calculateResetTime(fullConfig.dailyResetHour) : record.resetsAt;

  return {
    ...record,
    attemptsToday: attemptsToday + 1,
    lastAttemptAt: now.toISOString(),
    cooldownEndsAt: cooldownEndsAt.toISOString(),
    bestScore: Math.max(record.bestScore, result.score),
    mastered: record.mastered || result.passed,
    masteredAt: result.passed && !record.mastered ? now.toISOString() : record.masteredAt,
    resetsAt: newResetsAt,
    attemptHistory: [...record.attemptHistory, historyEntry].slice(-50), // Keep last 50
  };
}

/**
 * Get current cooldown status for display.
 */
export function getCooldownStatus(
  record: AttemptRecord,
  config: Partial<CooldownConfig> = {}
): CooldownStatus {
  const fullConfig = { ...DEFAULT_COOLDOWN_CONFIG, ...config };
  const now = new Date();

  // Check if reset time has passed
  const resetTime = new Date(record.resetsAt);
  let attemptsToday = record.attemptsToday;
  let resetsAt = record.resetsAt;

  if (now >= resetTime) {
    attemptsToday = 0;
    resetsAt = calculateResetTime(fullConfig.dailyResetHour);
  }

  // Check cooldown
  let inCooldown = false;
  let cooldownEndsAt: string | null = null;
  let remainingMs = 0;

  if (record.cooldownEndsAt) {
    const cooldownEnd = new Date(record.cooldownEndsAt);
    if (now < cooldownEnd) {
      inCooldown = true;
      cooldownEndsAt = record.cooldownEndsAt;
      remainingMs = cooldownEnd.getTime() - now.getTime();
    }
  }

  const timeUntilResetMs = new Date(resetsAt).getTime() - now.getTime();

  return {
    inCooldown,
    cooldownEndsAt,
    remainingMs,
    remainingFormatted: formatDuration(remainingMs),
    attemptsUsed: attemptsToday,
    attemptsRemaining: Math.max(0, record.maxAttemptsPerDay - attemptsToday),
    resetsAt,
    timeUntilResetMs,
  };
}

/**
 * Format duration in ms to human-readable string.
 */
function formatDuration(ms: number): string {
  if (ms <= 0) return '0m';

  const hours = Math.floor(ms / (60 * 60 * 1000));
  const minutes = Math.floor((ms % (60 * 60 * 1000)) / (60 * 1000));

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

/**
 * Get remaining attempts for today.
 */
export function getRemainingAttempts(record: AttemptRecord): number {
  const now = new Date();
  const resetTime = new Date(record.resetsAt);

  if (now >= resetTime) {
    return record.maxAttemptsPerDay; // New day
  }

  return Math.max(0, record.maxAttemptsPerDay - record.attemptsToday);
}

/**
 * Serialize attempt record for persistence.
 */
export function serializeAttemptRecord(record: AttemptRecord): string {
  return JSON.stringify(record);
}

/**
 * Deserialize attempt record from persistence.
 */
export function deserializeAttemptRecord(json: string): AttemptRecord | null {
  try {
    return JSON.parse(json) as AttemptRecord;
  } catch {
    return null;
  }
}

/**
 * Storage key for attempt records.
 */
export function getAttemptRecordKey(contentId: string, humanId: string): string {
  return `attempt-record:${humanId}:${contentId}`;
}
