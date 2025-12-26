import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable, interval, map, startWith } from 'rxjs';
import {
  AttemptRecord,
  AttemptCheckResult,
  CooldownStatus,
  CooldownConfig,
  DEFAULT_COOLDOWN_CONFIG,
  createAttemptRecord,
  checkCanAttempt,
  recordAttempt,
  getCooldownStatus,
  getRemainingAttempts,
  serializeAttemptRecord,
  deserializeAttemptRecord,
  getAttemptRecordKey
} from '../models/attempt-record.model';
import type { QuizResult } from '../models/quiz-session.model';

/**
 * AttemptCooldownService - Manages mastery quiz attempt limits and cooldowns.
 *
 * Mastery quizzes are gated to prevent brute-force memorization:
 * - 2 attempts per day (configurable)
 * - 4-hour cooldown between attempts
 * - Daily reset at midnight
 *
 * This creates meaningful spaced repetition where learners must
 * actually learn the material rather than just retrying immediately.
 *
 * @example
 * ```typescript
 * const cooldownService = inject(AttemptCooldownService);
 *
 * // Check if attempt allowed
 * const check = cooldownService.checkAttempt('concept-123', 'human-456');
 * if (check.allowed) {
 *   // Start mastery quiz
 *   const session = await quizService.startMasteryQuiz(...);
 *
 *   // After completion
 *   cooldownService.recordAttempt('concept-123', 'human-456', result);
 * } else {
 *   console.log(check.message); // "Please wait 3h 45m..."
 * }
 *
 * // Subscribe to countdown
 * cooldownService.getCooldownStatus$('concept-123', 'human-456')
 *   .subscribe(status => {
 *     this.remainingTime = status.remainingFormatted;
 *   });
 * ```
 */
@Injectable({
  providedIn: 'root'
})
export class AttemptCooldownService {
  /** Active attempt records by key */
  private readonly records = new Map<string, AttemptRecord>();

  /** Observable states for UI binding */
  private readonly statusSubjects = new Map<string, BehaviorSubject<CooldownStatus>>();

  /** Configuration (can be overridden per-request) */
  private config: CooldownConfig = { ...DEFAULT_COOLDOWN_CONFIG };

  /** Interval for updating countdown displays (1 minute) */
  private readonly UPDATE_INTERVAL = 60 * 1000;

  // ═══════════════════════════════════════════════════════════════════════════
  // Configuration
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Configure global cooldown settings.
   */
  configure(config: Partial<CooldownConfig>): void {
    this.config = { ...DEFAULT_COOLDOWN_CONFIG, ...config };
  }

  /**
   * Get current configuration.
   */
  getConfig(): CooldownConfig {
    return { ...this.config };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Attempt Checking
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if a mastery quiz attempt is allowed.
   *
   * @param contentId - Content node ID
   * @param humanId - Human taking the quiz
   * @param config - Optional config override
   * @returns Attempt check result with allowed status and message
   */
  checkAttempt(
    contentId: string,
    humanId: string,
    config?: Partial<CooldownConfig>
  ): AttemptCheckResult {
    const record = this.getOrCreateRecord(contentId, humanId, config);
    return checkCanAttempt(record, config ?? this.config);
  }

  /**
   * Check if a mastery quiz attempt is allowed (observable).
   * Updates when cooldown expires.
   */
  checkAttempt$(
    contentId: string,
    humanId: string,
    config?: Partial<CooldownConfig>
  ): Observable<AttemptCheckResult> {
    // Update every minute to catch cooldown expirations
    return interval(this.UPDATE_INTERVAL).pipe(
      startWith(0),
      map(() => this.checkAttempt(contentId, humanId, config))
    );
  }

  /**
   * Quick check if attempt is allowed.
   */
  canAttempt(contentId: string, humanId: string): boolean {
    return this.checkAttempt(contentId, humanId).allowed;
  }

  /**
   * Get remaining attempts for today.
   */
  getRemainingAttempts(contentId: string, humanId: string): number {
    const record = this.getRecord(contentId, humanId);
    return record ? getRemainingAttempts(record) : this.config.masteryAttemptsPerDay;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Recording Attempts
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record a mastery quiz attempt.
   *
   * @param contentId - Content node ID
   * @param humanId - Human who took the quiz
   * @param result - Quiz result
   * @param config - Optional config override
   * @returns Updated attempt record
   */
  recordAttempt(
    contentId: string,
    humanId: string,
    result: QuizResult,
    config?: Partial<CooldownConfig>
  ): AttemptRecord {
    const currentRecord = this.getOrCreateRecord(contentId, humanId, config);
    const updatedRecord = recordAttempt(currentRecord, result, config ?? this.config);

    this.setRecord(contentId, humanId, updatedRecord);

    return updatedRecord;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Cooldown Status
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get current cooldown status for display.
   */
  getCooldownStatus(
    contentId: string,
    humanId: string,
    config?: Partial<CooldownConfig>
  ): CooldownStatus {
    const record = this.getOrCreateRecord(contentId, humanId, config);
    return getCooldownStatus(record, config ?? this.config);
  }

  /**
   * Get observable cooldown status.
   * Updates every minute for countdown display.
   */
  getCooldownStatus$(
    contentId: string,
    humanId: string,
    config?: Partial<CooldownConfig>
  ): Observable<CooldownStatus> {
    return interval(this.UPDATE_INTERVAL).pipe(
      startWith(0),
      map(() => this.getCooldownStatus(contentId, humanId, config))
    );
  }

  /**
   * Check if currently in cooldown.
   */
  isInCooldown(contentId: string, humanId: string): boolean {
    return this.getCooldownStatus(contentId, humanId).inCooldown;
  }

  /**
   * Get formatted time remaining in cooldown.
   */
  getTimeRemaining(contentId: string, humanId: string): string {
    return this.getCooldownStatus(contentId, humanId).remainingFormatted;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Record Access
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get attempt record if it exists.
   */
  getRecord(contentId: string, humanId: string): AttemptRecord | null {
    const key = getAttemptRecordKey(contentId, humanId);

    // Try memory first
    let record = this.records.get(key);

    // Try localStorage
    if (!record) {
      const loaded = this.loadFromStorage(key);
      if (loaded) {
        record = loaded;
        this.records.set(key, loaded);
      }
    }

    return record ?? null;
  }

  /**
   * Check if content has been mastered.
   */
  isMastered(contentId: string, humanId: string): boolean {
    return this.getRecord(contentId, humanId)?.mastered ?? false;
  }

  /**
   * Get best score achieved.
   */
  getBestScore(contentId: string, humanId: string): number {
    return this.getRecord(contentId, humanId)?.bestScore ?? 0;
  }

  /**
   * Get attempt history.
   */
  getAttemptHistory(contentId: string, humanId: string): AttemptRecord['attemptHistory'] {
    return this.getRecord(contentId, humanId)?.attemptHistory ?? [];
  }

  /**
   * Get all records for a human (for dashboard display).
   */
  getAllRecordsForHuman(humanId: string): AttemptRecord[] {
    const records: AttemptRecord[] = [];

    // Check memory
    for (const record of this.records.values()) {
      if (record.humanId === humanId) {
        records.push(record);
      }
    }

    // Also check localStorage for any we haven't loaded
    this.loadAllFromStorageForHuman(humanId).forEach(record => {
      if (!records.find(r => r.contentId === record.contentId)) {
        records.push(record);
      }
    });

    return records;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Clearing
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Clear attempt record for a content node.
   * Use with caution - this resets cooldowns and attempts.
   */
  clearRecord(contentId: string, humanId: string): void {
    const key = getAttemptRecordKey(contentId, humanId);
    this.records.delete(key);
    this.statusSubjects.get(key)?.complete();
    this.statusSubjects.delete(key);
    this.removeFromStorage(key);
  }

  /**
   * Clear all records for a human.
   * Use with caution.
   */
  clearAllForHuman(humanId: string): void {
    for (const [key, record] of this.records.entries()) {
      if (record.humanId === humanId) {
        this.records.delete(key);
        this.statusSubjects.get(key)?.complete();
        this.statusSubjects.delete(key);
        this.removeFromStorage(key);
      }
    }
  }

  /**
   * Reset only the cooldown (not attempts) - for testing/admin use.
   */
  resetCooldown(contentId: string, humanId: string): void {
    const record = this.getRecord(contentId, humanId);
    if (record) {
      const updated: AttemptRecord = {
        ...record,
        cooldownEndsAt: null
      };
      this.setRecord(contentId, humanId, updated);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Bulk Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get cooldown status for multiple content nodes.
   * Useful for displaying in path navigator.
   */
  getBulkCooldownStatus(
    contentIds: string[],
    humanId: string
  ): Map<string, CooldownStatus> {
    const results = new Map<string, CooldownStatus>();

    for (const contentId of contentIds) {
      results.set(contentId, this.getCooldownStatus(contentId, humanId));
    }

    return results;
  }

  /**
   * Check which content nodes are mastered.
   */
  getBulkMasteryStatus(
    contentIds: string[],
    humanId: string
  ): Map<string, boolean> {
    const results = new Map<string, boolean>();

    for (const contentId of contentIds) {
      results.set(contentId, this.isMastered(contentId, humanId));
    }

    return results;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private getOrCreateRecord(
    contentId: string,
    humanId: string,
    config?: Partial<CooldownConfig>
  ): AttemptRecord {
    let record = this.getRecord(contentId, humanId);

    if (!record) {
      record = createAttemptRecord(contentId, humanId, config ?? this.config);
      this.setRecord(contentId, humanId, record);
    }

    return record;
  }

  private setRecord(contentId: string, humanId: string, record: AttemptRecord): void {
    const key = getAttemptRecordKey(contentId, humanId);
    this.records.set(key, record);

    // Update subject
    let subject = this.statusSubjects.get(key);
    if (!subject) {
      subject = new BehaviorSubject(getCooldownStatus(record, this.config));
      this.statusSubjects.set(key, subject);
    } else {
      subject.next(getCooldownStatus(record, this.config));
    }

    // Persist
    this.saveToStorage(key, record);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Persistence
  // ═══════════════════════════════════════════════════════════════════════════

  private saveToStorage(key: string, record: AttemptRecord): void {
    try {
      localStorage.setItem(key, serializeAttemptRecord(record));
    } catch (e) {
      console.warn('Failed to save attempt record:', e);
    }
  }

  private loadFromStorage(key: string): AttemptRecord | null {
    try {
      const json = localStorage.getItem(key);
      if (json) {
        return deserializeAttemptRecord(json);
      }
    } catch (e) {
      console.warn('Failed to load attempt record:', e);
    }
    return null;
  }

  private removeFromStorage(key: string): void {
    try {
      localStorage.removeItem(key);
    } catch (e) {
      console.warn('Failed to remove attempt record:', e);
    }
  }

  private loadAllFromStorageForHuman(humanId: string): AttemptRecord[] {
    const records: AttemptRecord[] = [];
    const prefix = `attempt-record:${humanId}:`;

    try {
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key?.startsWith(prefix)) {
          const record = this.loadFromStorage(key);
          if (record) {
            records.push(record);
          }
        }
      }
    } catch (e) {
      console.warn('Failed to load attempt records from storage:', e);
    }

    return records;
  }
}
