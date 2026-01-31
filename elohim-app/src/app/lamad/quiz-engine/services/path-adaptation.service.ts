import { Injectable, inject } from '@angular/core';

import { BehaviorSubject, Observable, map } from 'rxjs';

import { AttemptCooldownService } from './attempt-cooldown.service';
import { QuestionPoolService } from './question-pool.service';
import { StreakTrackerService } from './streak-tracker.service';

import type { QuizResult } from '../models/quiz-session.model';

/**
 * Path Adaptation Types
 *
 * Attestation quizzes are INVISIBLE path signals - they don't show as steps.
 * They influence path behavior through gates, recommendations, and skip-ahead.
 */

/**
 * Gate status for a section boundary.
 * Gates block progress until mastery quiz is passed.
 */
export interface GateStatus {
  /** Section ID this gate protects */
  sectionId: string;

  /** Whether the gate is locked (blocking progress) */
  locked: boolean;

  /** Reason the gate is locked */
  reason?: GateLockReason;

  /** Whether mastery quiz is available (not in cooldown) */
  quizAvailable: boolean;

  /** Cooldown info if in cooldown */
  cooldownEndsAt?: string;

  /** Remaining attempts today */
  remainingAttempts: number;

  /** Best score achieved so far */
  bestScore: number;

  /** Whether section has been mastered */
  mastered: boolean;
}

export type GateLockReason =
  | 'not_attempted' // Haven't taken the mastery quiz yet
  | 'failed' // Failed mastery quiz, can retry
  | 'in_cooldown' // Failed and waiting for cooldown
  | 'max_attempts' // Used all attempts for the day
  | 'prerequisites'; // Previous sections not mastered

/**
 * Recommendation for supplementary content.
 */
export interface ContentRecommendation {
  /** Content ID to recommend */
  contentId: string;

  /** Why this is recommended */
  reason: RecommendationReason;

  /** Confidence score (0-1) */
  confidence: number;

  /** Related quiz performance that triggered this */
  triggerContext?: {
    quizType: 'inline' | 'mastery' | 'practice';
    conceptIds: string[];
    score: number;
  };
}

export type RecommendationReason =
  | 'struggled_with_concept' // Low scores on related questions
  | 'prerequisite_gap' // Missing prerequisite knowledge
  | 'exploration_interest' // User explored related content
  | 'reinforcement' // Good score but could use reinforcement
  | 'advanced_option'; // High score, offer advanced content

/**
 * Skip-ahead eligibility from pre-assessment.
 */
export interface SkipAheadResult {
  /** Whether skip-ahead is available */
  eligible: boolean;

  /** Sections that can be skipped */
  skippableSections: SkippableSection[];

  /** Recommended starting point */
  recommendedStartSection?: string;

  /** Pre-assessment score */
  preAssessmentScore: number;

  /** Per-concept scores */
  conceptScores: Map<string, number>;
}

export interface SkippableSection {
  /** Section ID */
  sectionId: string;

  /** Section title */
  title: string;

  /** Mastery demonstrated (0-1) */
  masteryScore: number;

  /** Concepts in this section */
  conceptIds: string[];

  /** Whether to skip (recommended) */
  recommendSkip: boolean;
}

/**
 * Path adaptation state for a learner on a path.
 */
export interface PathAdaptationState {
  /** Path ID */
  pathId: string;

  /** Human ID */
  humanId: string;

  /** Gate status for each section */
  gates: Map<string, GateStatus>;

  /** Active recommendations */
  recommendations: ContentRecommendation[];

  /** Skip-ahead result (if pre-assessment taken) */
  skipAhead?: SkipAheadResult;

  /** Sections marked as complete */
  completedSections: Set<string>;

  /** Sections skipped via pre-assessment */
  skippedSections: Set<string>;

  /** Current section ID */
  currentSectionId?: string;

  /** Last updated */
  updatedAt: string;
}

/**
 * Configuration for path adaptation.
 */
export interface PathAdaptationConfig {
  /** Minimum score to pass mastery gate (0-1) */
  masteryPassingScore: number;

  /** Minimum score to skip section via pre-assessment (0-1) */
  skipThreshold: number;

  /** Whether to show recommendations */
  enableRecommendations: boolean;

  /** Maximum recommendations to show */
  maxRecommendations: number;

  /** Minimum confidence for recommendations */
  recommendationThreshold: number;

  /** Whether inline quiz completion is required before mastery gate */
  requireInlineBeforeMastery: boolean;
}

const DEFAULT_CONFIG: PathAdaptationConfig = {
  masteryPassingScore: 0.8,
  skipThreshold: 0.85,
  enableRecommendations: true,
  maxRecommendations: 3,
  recommendationThreshold: 0.6,
  requireInlineBeforeMastery: true,
};

/**
 * PathAdaptationService - Manages invisible attestation quiz signals for path behavior.
 *
 * Attestation quizzes (inline, mastery, pre-assessment) are NOT visible path steps.
 * They're triggered contextually and influence path behavior:
 *
 * - **Inline quizzes**: Appear after content, earning "practiced" attestation
 * - **Mastery gates**: Block section transitions until quiz passed
 * - **Pre-assessment**: Enables skip-ahead at path start
 * - **Recommendations**: Surface supplementary content based on performance
 *
 * Discovery assessments (Enneagram, learning style, etc.) ARE visible path steps
 * and are handled as regular content, not by this service.
 *
 * @example
 * ```typescript
 * const adaptation = inject(PathAdaptationService);
 *
 * // Check if learner can proceed to next section
 * const gate = adaptation.getGateStatus(pathId, sectionId, humanId);
 * if (gate.locked) {
 *   // Show mastery quiz modal
 *   this.showMasteryQuiz(sectionId);
 * }
 *
 * // Get recommendations after quiz
 * const recs = adaptation.getRecommendations(pathId, humanId);
 * ```
 */
@Injectable({
  providedIn: 'root',
})
export class PathAdaptationService {
  private readonly cooldownService = inject(AttemptCooldownService);
  private readonly poolService = inject(QuestionPoolService);
  private readonly streakService = inject(StreakTrackerService);

  /** Cached adaptation states by path:human key */
  private readonly states = new Map<string, PathAdaptationState>();

  /** Observable state subjects */
  private readonly stateSubjects = new Map<string, BehaviorSubject<PathAdaptationState>>();

  /** Configuration */
  private config: PathAdaptationConfig = { ...DEFAULT_CONFIG };

  /** Storage key prefix */
  private readonly STORAGE_PREFIX = 'path-adaptation:';

  // ═══════════════════════════════════════════════════════════════════════════
  // Configuration
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Configure path adaptation behavior.
   */
  configure(config: Partial<PathAdaptationConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Get current configuration.
   */
  getConfig(): PathAdaptationConfig {
    return { ...this.config };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Gate Management (Section Boundaries)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get gate status for a section.
   * Gates block progress until mastery quiz is passed.
   */
  getGateStatus(pathId: string, sectionId: string, humanId: string): GateStatus {
    const state = this.getOrCreateState(pathId, humanId);
    const existingGate = state.gates.get(sectionId);

    if (existingGate) {
      // Refresh cooldown status
      return this.refreshGateStatus(existingGate, humanId);
    }

    // Create new gate status
    return this.createGateStatus(sectionId, humanId);
  }

  /**
   * Get observable gate status for UI binding.
   */
  getGateStatus$(pathId: string, sectionId: string, humanId: string): Observable<GateStatus> {
    return this.getState$(pathId, humanId).pipe(
      map(state => {
        const gate = state.gates.get(sectionId);
        return gate
          ? this.refreshGateStatus(gate, humanId)
          : this.createGateStatus(sectionId, humanId);
      })
    );
  }

  /**
   * Check if learner can proceed past a gate.
   */
  canProceed(pathId: string, sectionId: string, humanId: string): boolean {
    const gate = this.getGateStatus(pathId, sectionId, humanId);
    return !gate.locked;
  }

  /**
   * Record mastery quiz result and update gate.
   */
  recordMasteryResult(
    pathId: string,
    sectionId: string,
    humanId: string,
    result: QuizResult
  ): GateStatus {
    // Record in cooldown service
    this.cooldownService.recordAttempt(sectionId, humanId, result);

    // Update gate status
    const state = this.getOrCreateState(pathId, humanId);
    const gate = this.getGateStatus(pathId, sectionId, humanId);

    const updatedGate: GateStatus = {
      ...gate,
      locked: !result.passed,
      reason: result.passed ? undefined : 'failed',
      bestScore: Math.max(gate.bestScore, result.score),
      mastered: result.passed || gate.mastered,
    };

    // Refresh with cooldown info
    const finalGate = this.refreshGateStatus(updatedGate, humanId);
    state.gates.set(sectionId, finalGate);

    // Mark section complete if passed
    if (result.passed) {
      state.completedSections.add(sectionId);
    }

    this.saveState(pathId, humanId, state);

    // Generate recommendations based on result
    if (this.config.enableRecommendations && !result.passed) {
      this.generateRecommendations(pathId, humanId, result);
    }

    return finalGate;
  }

  /**
   * Unlock a gate (admin/testing function).
   */
  unlockGate(pathId: string, sectionId: string, humanId: string): void {
    const state = this.getOrCreateState(pathId, humanId);
    const gate = state.gates.get(sectionId);

    if (gate) {
      state.gates.set(sectionId, {
        ...gate,
        locked: false,
        mastered: true,
      });
      state.completedSections.add(sectionId);
      this.saveState(pathId, humanId, state);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Recommendations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get active recommendations for a learner.
   */
  getRecommendations(pathId: string, humanId: string): ContentRecommendation[] {
    const state = this.getOrCreateState(pathId, humanId);
    return state.recommendations.slice(0, this.config.maxRecommendations);
  }

  /**
   * Get observable recommendations.
   */
  getRecommendations$(pathId: string, humanId: string): Observable<ContentRecommendation[]> {
    return this.getState$(pathId, humanId).pipe(
      map(state => state.recommendations.slice(0, this.config.maxRecommendations))
    );
  }

  /**
   * Dismiss a recommendation.
   */
  dismissRecommendation(pathId: string, humanId: string, contentId: string): void {
    const state = this.getOrCreateState(pathId, humanId);
    state.recommendations = state.recommendations.filter(r => r.contentId !== contentId);
    this.saveState(pathId, humanId, state);
  }

  /**
   * Clear all recommendations.
   */
  clearRecommendations(pathId: string, humanId: string): void {
    const state = this.getOrCreateState(pathId, humanId);
    state.recommendations = [];
    this.saveState(pathId, humanId, state);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Skip-Ahead (Pre-Assessment)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record pre-assessment result and calculate skip-ahead eligibility.
   */
  recordPreAssessmentResult(
    pathId: string,
    humanId: string,
    result: QuizResult,
    sectionMapping: Map<string, { title: string; conceptIds: string[] }>
  ): SkipAheadResult {
    const conceptScores = new Map<string, number>();

    // Build per-concept scores from result
    for (const score of result.contentScores) {
      conceptScores.set(score.contentId, score.averageScore);
    }

    // Determine which sections can be skipped
    const skippableSections: SkippableSection[] = [];
    let recommendedStartSection: string | undefined;

    for (const [sectionId, section] of sectionMapping) {
      // Calculate average score for section's concepts
      let totalScore = 0;
      let count = 0;

      for (const conceptId of section.conceptIds) {
        const score = conceptScores.get(conceptId);
        if (score !== undefined) {
          totalScore += score;
          count++;
        }
      }

      const masteryScore = count > 0 ? totalScore / count : 0;
      const recommendSkip = masteryScore >= this.config.skipThreshold;

      skippableSections.push({
        sectionId,
        title: section.title,
        masteryScore,
        conceptIds: section.conceptIds,
        recommendSkip,
      });

      // Find first section that shouldn't be skipped
      if (!recommendSkip && !recommendedStartSection) {
        recommendedStartSection = sectionId;
      }
    }

    const skipAheadResult: SkipAheadResult = {
      eligible: skippableSections.some(s => s.recommendSkip),
      skippableSections,
      recommendedStartSection,
      preAssessmentScore: result.score,
      conceptScores,
    };

    // Update state
    const state = this.getOrCreateState(pathId, humanId);
    state.skipAhead = skipAheadResult;

    // Mark skipped sections
    for (const section of skippableSections) {
      if (section.recommendSkip) {
        state.skippedSections.add(section.sectionId);
        state.completedSections.add(section.sectionId);

        // Also unlock their gates
        state.gates.set(section.sectionId, {
          sectionId: section.sectionId,
          locked: false,
          quizAvailable: true,
          remainingAttempts: 2,
          bestScore: section.masteryScore,
          mastered: true,
        });
      }
    }

    this.saveState(pathId, humanId, state);

    return skipAheadResult;
  }

  /**
   * Get skip-ahead result if available.
   */
  getSkipAheadResult(pathId: string, humanId: string): SkipAheadResult | undefined {
    const state = this.getOrCreateState(pathId, humanId);
    return state.skipAhead;
  }

  /**
   * Apply skip-ahead selections.
   */
  applySkipAhead(pathId: string, humanId: string, sectionIdsToSkip: string[]): void {
    const state = this.getOrCreateState(pathId, humanId);

    for (const sectionId of sectionIdsToSkip) {
      state.skippedSections.add(sectionId);
      state.completedSections.add(sectionId);

      // Unlock gate
      const existingGate = state.gates.get(sectionId);
      state.gates.set(sectionId, {
        sectionId,
        locked: false,
        quizAvailable: true,
        remainingAttempts: 2,
        bestScore: existingGate?.bestScore ?? 0,
        mastered: true,
      });
    }

    this.saveState(pathId, humanId, state);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Inline Quiz Integration
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if inline quiz is complete for content.
   */
  isInlineComplete(contentId: string): boolean {
    return this.streakService.isAchieved(contentId);
  }

  /**
   * Check if all inline quizzes in a section are complete.
   * Required before mastery gate if configured.
   */
  areSectionInlinesComplete(pathId: string, sectionId: string, contentIds: string[]): boolean {
    if (!this.config.requireInlineBeforeMastery) {
      return true;
    }

    return contentIds.every(id => this.isInlineComplete(id));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // State Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get full adaptation state for a path/human.
   */
  getState(pathId: string, humanId: string): PathAdaptationState {
    return this.getOrCreateState(pathId, humanId);
  }

  /**
   * Get observable adaptation state.
   */
  getState$(pathId: string, humanId: string): Observable<PathAdaptationState> {
    const key = this.getKey(pathId, humanId);

    let subject = this.stateSubjects.get(key);
    if (!subject) {
      const state = this.getOrCreateState(pathId, humanId);
      subject = new BehaviorSubject(state);
      this.stateSubjects.set(key, subject);
    }

    return subject.asObservable();
  }

  /**
   * Clear adaptation state for a path/human.
   */
  clearState(pathId: string, humanId: string): void {
    const key = this.getKey(pathId, humanId);
    this.states.delete(key);
    this.stateSubjects.get(key)?.complete();
    this.stateSubjects.delete(key);
    this.removeFromStorage(key);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private getKey(pathId: string, humanId: string): string {
    return `${pathId}:${humanId}`;
  }

  private getOrCreateState(pathId: string, humanId: string): PathAdaptationState {
    const key = this.getKey(pathId, humanId);

    // Try memory
    const cached = this.states.get(key);
    if (cached) return cached;

    // Try storage
    const loaded = this.loadFromStorage(key);
    if (loaded) {
      this.states.set(key, loaded);
      return loaded;
    }

    // Create new
    const newState: PathAdaptationState = {
      pathId,
      humanId,
      gates: new Map(),
      recommendations: [],
      completedSections: new Set(),
      skippedSections: new Set(),
      updatedAt: new Date().toISOString(),
    };

    this.states.set(key, newState);
    return newState;
  }

  private saveState(pathId: string, humanId: string, state: PathAdaptationState): void {
    const key = this.getKey(pathId, humanId);
    state.updatedAt = new Date().toISOString();

    this.states.set(key, state);

    // Update subject
    const subject = this.stateSubjects.get(key);
    if (subject) {
      subject.next(state);
    }

    // Persist
    this.saveToStorage(key, state);
  }

  private createGateStatus(sectionId: string, humanId: string): GateStatus {
    const cooldownStatus = this.cooldownService.getCooldownStatus(sectionId, humanId);

    return {
      sectionId,
      locked: true,
      reason: 'not_attempted',
      quizAvailable: !cooldownStatus.inCooldown && cooldownStatus.attemptsRemaining > 0,
      cooldownEndsAt: cooldownStatus.cooldownEndsAt ?? undefined,
      remainingAttempts: cooldownStatus.attemptsRemaining,
      bestScore: this.cooldownService.getBestScore(sectionId, humanId),
      mastered: this.cooldownService.isMastered(sectionId, humanId),
    };
  }

  private refreshGateStatus(gate: GateStatus, humanId: string): GateStatus {
    const cooldownStatus = this.cooldownService.getCooldownStatus(gate.sectionId, humanId);

    // Determine lock reason
    let reason: GateLockReason | undefined = gate.reason;
    if (gate.mastered) {
      reason = undefined;
    } else if (cooldownStatus.attemptsRemaining === 0) {
      reason = 'max_attempts';
    } else if (cooldownStatus.inCooldown) {
      reason = 'in_cooldown';
    }

    return {
      ...gate,
      locked: !gate.mastered,
      reason,
      quizAvailable: !cooldownStatus.inCooldown && cooldownStatus.attemptsRemaining > 0,
      cooldownEndsAt: cooldownStatus.cooldownEndsAt ?? undefined,
      remainingAttempts: cooldownStatus.attemptsRemaining,
      bestScore: Math.max(
        gate.bestScore,
        this.cooldownService.getBestScore(gate.sectionId, humanId)
      ),
      mastered: gate.mastered || this.cooldownService.isMastered(gate.sectionId, humanId),
    };
  }

  private generateRecommendations(pathId: string, humanId: string, result: QuizResult): void {
    const state = this.getOrCreateState(pathId, humanId);
    const newRecs: ContentRecommendation[] = [];

    // Find concepts with low scores
    for (const contentScore of result.contentScores) {
      if (contentScore.averageScore < 0.6) {
        // TODO: Look up related/prerequisite content from graph
        // For now, just flag the concept itself
        newRecs.push({
          contentId: contentScore.contentId,
          reason: 'struggled_with_concept',
          confidence: 1 - contentScore.averageScore,
          triggerContext: {
            quizType: result.type === 'mastery' ? 'mastery' : 'practice',
            conceptIds: [contentScore.contentId],
            score: contentScore.averageScore,
          },
        });
      }
    }

    // Filter by threshold and limit
    const filtered = newRecs
      .filter(r => r.confidence >= this.config.recommendationThreshold)
      .sort((a, b) => b.confidence - a.confidence)
      .slice(0, this.config.maxRecommendations);

    // Merge with existing, avoiding duplicates
    const existingIds = new Set(state.recommendations.map(r => r.contentId));
    for (const rec of filtered) {
      if (!existingIds.has(rec.contentId)) {
        state.recommendations.push(rec);
      }
    }

    // Trim to max
    state.recommendations = state.recommendations.slice(0, this.config.maxRecommendations);

    this.saveState(pathId, humanId, state);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Persistence
  // ═══════════════════════════════════════════════════════════════════════════

  private saveToStorage(key: string, state: PathAdaptationState): void {
    try {
      const serializable = {
        ...state,
        gates: Array.from(state.gates.entries()),
        completedSections: Array.from(state.completedSections),
        skippedSections: Array.from(state.skippedSections),
        skipAhead: state.skipAhead
          ? {
              ...state.skipAhead,
              conceptScores: Array.from(state.skipAhead.conceptScores.entries()),
            }
          : undefined,
      };
      localStorage.setItem(this.STORAGE_PREFIX + key, JSON.stringify(serializable));
    } catch {
      // localStorage write failure is non-critical
    }
  }

  private loadFromStorage(key: string): PathAdaptationState | null {
    try {
      const json = localStorage.getItem(this.STORAGE_PREFIX + key);
      if (!json) return null;

      const parsed = JSON.parse(json);
      return {
        ...parsed,
        gates: new Map(parsed.gates),
        completedSections: new Set(parsed.completedSections),
        skippedSections: new Set(parsed.skippedSections),
        skipAhead: parsed.skipAhead
          ? {
              ...parsed.skipAhead,
              conceptScores: new Map(parsed.skipAhead.conceptScores),
            }
          : undefined,
      };
    } catch {
      // localStorage read failure is non-critical
      return null;
    }
  }

  private removeFromStorage(key: string): void {
    try {
      localStorage.removeItem(this.STORAGE_PREFIX + key);
    } catch {
      // localStorage remove failure is non-critical
    }
  }
}
