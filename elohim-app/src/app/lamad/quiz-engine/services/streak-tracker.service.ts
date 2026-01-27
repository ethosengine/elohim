import { Injectable } from '@angular/core';

import { BehaviorSubject, Observable } from 'rxjs';

import {
  StreakState,
  StreakProgress,
  StreakConfig,
  createStreakState,
  recordAnswer,
  isStreakAchieved,
  isStreakComplete,
  getStreakProgress,
  resetStreak,
  serializeStreakState,
  deserializeStreakState,
} from '../models/streak-state.model';

/**
 * StreakTrackerService - Manages streak-based attestation tracking.
 *
 * Tracks consecutive correct answers for inline quizzes where
 * learners must get 3-5 questions correct in a row to earn
 * the "practiced" attestation.
 *
 * Features:
 * - Real-time streak progress observable
 * - Persistence across page reloads
 * - Configurable target streak
 * - Achievement detection with callbacks
 *
 * @example
 * ```typescript
 * const tracker = inject(StreakTrackerService);
 *
 * // Start tracking for a content node
 * tracker.startTracking('concept-123', 'human-456');
 *
 * // Record answers
 * tracker.recordAnswer('concept-123', 'q-1', true);  // correct
 * tracker.recordAnswer('concept-123', 'q-2', true);  // correct
 * tracker.recordAnswer('concept-123', 'q-3', true);  // achieved!
 *
 * // Check progress
 * const progress = tracker.getProgress('concept-123');
 * console.log(progress.achieved); // true
 * ```
 */
@Injectable({
  providedIn: 'root',
})
export class StreakTrackerService {
  /** Active streak states by content ID */
  private readonly streaks = new Map<string, StreakState>();

  /** Observable states for UI binding */
  private readonly streakSubjects = new Map<string, BehaviorSubject<StreakState>>();

  /** Achievement callbacks */
  private readonly achievementCallbacks = new Map<string, () => void>();

  /** Local storage key prefix */
  private readonly STORAGE_PREFIX = 'streak:';

  // ═══════════════════════════════════════════════════════════════════════════
  // Streak Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Start tracking streak for a content node.
   *
   * @param contentId - Content node ID
   * @param humanId - Human taking the quiz
   * @param config - Optional streak configuration
   * @returns Initial streak state
   */
  startTracking(
    contentId: string,
    humanId: string,
    config: Partial<StreakConfig> = {}
  ): StreakState {
    // Check for existing in-progress streak
    const existing = this.getState(contentId);
    if (existing && !isStreakComplete(existing)) {
      return existing;
    }

    // Create new streak state
    const state = createStreakState(contentId, humanId, config);
    this.setState(contentId, state);

    return state;
  }

  /**
   * Record an answer and update streak.
   *
   * @param contentId - Content node ID
   * @param questionId - Question that was answered
   * @param correct - Whether the answer was correct
   * @returns Updated streak state
   */
  recordAnswer(contentId: string, questionId: string, correct: boolean): StreakState | null {
    const current = this.getState(contentId);
    if (!current) {
      console.warn(`No streak tracking for ${contentId}`);
      return null;
    }

    if (isStreakComplete(current)) {
      return current; // Already complete
    }

    // Update streak
    const updated = recordAnswer(current, questionId, correct);
    this.setState(contentId, updated);

    // Check for achievement
    if (updated.achieved && !current.achieved) {
      this.onAchievement(contentId);
    }

    return updated;
  }

  /**
   * Get current streak state.
   */
  getState(contentId: string): StreakState | null {
    // Try memory first
    let state = this.streaks.get(contentId);

    // Try local storage
    if (!state) {
      const loaded = this.loadFromStorage(contentId);
      if (loaded) {
        state = loaded;
        this.streaks.set(contentId, loaded);
      }
    }

    return state ?? null;
  }

  /**
   * Get observable streak state for UI binding.
   */
  getState$(contentId: string): Observable<StreakState | null> {
    const subject = this.streakSubjects.get(contentId);
    if (subject) {
      return subject.asObservable();
    }

    // Create new subject if state exists
    const state = this.getState(contentId);
    if (state) {
      const newSubject = new BehaviorSubject<StreakState>(state);
      this.streakSubjects.set(contentId, newSubject);
      return newSubject.asObservable();
    }

    // Return empty observable
    return new BehaviorSubject<StreakState | null>(null).asObservable();
  }

  /**
   * Get streak progress for UI display.
   */
  getProgress(contentId: string): StreakProgress | null {
    const state = this.getState(contentId);
    return state ? getStreakProgress(state) : null;
  }

  /**
   * Check if streak target has been achieved.
   */
  isAchieved(contentId: string): boolean {
    const state = this.getState(contentId);
    return state ? isStreakAchieved(state) : false;
  }

  /**
   * Check if streak tracking is complete (achieved or max reached).
   */
  isComplete(contentId: string): boolean {
    const state = this.getState(contentId);
    return state ? isStreakComplete(state) : false;
  }

  /**
   * Reset streak for retry.
   */
  reset(contentId: string): StreakState | null {
    const current = this.getState(contentId);
    if (!current) return null;

    const reset = resetStreak(current);
    this.setState(contentId, reset);

    return reset;
  }

  /**
   * Register callback for streak achievement.
   */
  onAchieved(contentId: string, callback: () => void): void {
    this.achievementCallbacks.set(contentId, callback);
  }

  /**
   * Remove achievement callback.
   */
  offAchieved(contentId: string): void {
    this.achievementCallbacks.delete(contentId);
  }

  /**
   * Clear streak tracking for a content node.
   */
  clear(contentId: string): void {
    this.streaks.delete(contentId);
    this.streakSubjects.get(contentId)?.complete();
    this.streakSubjects.delete(contentId);
    this.achievementCallbacks.delete(contentId);
    this.removeFromStorage(contentId);
  }

  /**
   * Clear all streak tracking.
   */
  clearAll(): void {
    for (const contentId of this.streaks.keys()) {
      this.clear(contentId);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Persistence
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Save streak state to local storage.
   */
  saveToStorage(contentId: string): void {
    const state = this.streaks.get(contentId);
    if (state) {
      try {
        localStorage.setItem(this.STORAGE_PREFIX + contentId, serializeStreakState(state));
      } catch (e) {
        console.warn('Failed to save streak state:', e);
      }
    }
  }

  /**
   * Load streak state from local storage.
   */
  private loadFromStorage(contentId: string): StreakState | null {
    try {
      const json = localStorage.getItem(this.STORAGE_PREFIX + contentId);
      if (json) {
        return deserializeStreakState(json);
      }
    } catch (e) {
      console.warn('Failed to load streak state:', e);
    }
    return null;
  }

  /**
   * Remove streak state from local storage.
   */
  private removeFromStorage(contentId: string): void {
    try {
      localStorage.removeItem(this.STORAGE_PREFIX + contentId);
    } catch (e) {
      console.warn('Failed to remove streak state:', e);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private setState(contentId: string, state: StreakState): void {
    this.streaks.set(contentId, state);

    // Update subject
    let subject = this.streakSubjects.get(contentId);
    if (!subject) {
      subject = new BehaviorSubject(state);
      this.streakSubjects.set(contentId, subject);
    } else {
      subject.next(state);
    }

    // Persist
    this.saveToStorage(contentId);
  }

  private onAchievement(contentId: string): void {
    const callback = this.achievementCallbacks.get(contentId);
    if (callback) {
      callback();
    }
  }
}
