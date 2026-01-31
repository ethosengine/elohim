import { Injectable, Optional, OnDestroy } from '@angular/core';

// @coverage: 62.7% (2026-02-04)

import { map, tap, switchMap, take, takeUntil } from 'rxjs/operators';

import { BehaviorSubject, Observable, of, Subject } from 'rxjs';

import { SessionHumanService } from '../../imagodei/services/session-human.service';
import {
  AccessLevel,
  ContentAccessMetadata,
  AccessCheckResult,
} from '../../lamad/models/content-access.model';
import {
  Agent,
  AgentProgress,
  FrontierItem,
  MasteryLevel,
  MASTERY_LEVEL_VALUES,
} from '../models/agent.model';

import { DataLoaderService } from './data-loader.service';

// Models from elohim (local)

// Models from lamad pillar (content-specific access control)

// Services from imagodei pillar (identity)
// Using relative import for now; will update to @app/imagodei after full migration

/**
 * AgentService - Manages the current agent (session or authenticated).
 *
 * Architecture:
 * - Session users: Delegated to SessionHumanService
 * - Authenticated users: Holochain conductor (future)
 *
 * Holochain migration:
 * - Agent profile comes from conductor's agent info
 * - Progress lives on private source chain
 * - Attestations are verifiable credentials
 *
 * MVP behavior:
 * - Session users get temporary identity via SessionHumanService
 * - Progress stored in localStorage (session-scoped)
 * - Attestations tracked in session
 */
@Injectable({ providedIn: 'root' })
export class AgentService implements OnDestroy {
  private readonly destroy$ = new Subject<void>();
  private readonly agentSubject = new BehaviorSubject<Agent | null>(null);
  readonly agent$ = this.agentSubject.asObservable();

  // Progress cache (keyed by pathId)
  private readonly progressCache = new Map<string, AgentProgress>();

  // Attestations set
  private readonly attestations = new Set<string>();

  constructor(
    private readonly dataLoader: DataLoaderService,
    @Optional() private readonly sessionHumanService: SessionHumanService | null
  ) {
    this.initializeAgent();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Initialize agent based on session context.
   * Creates agent from SessionHumanService or anonymous fallback.
   */
  private initializeAgent(): void {
    if (this.sessionHumanService) {
      // Create agent from session
      this.sessionHumanService.session$.pipe(takeUntil(this.destroy$)).subscribe(session => {
        if (session) {
          const agent: Agent = {
            id: session.sessionId,
            displayName: session.displayName,
            type: 'human',
            visibility: 'private',
            createdAt: session.createdAt,
            updatedAt: session.lastActiveAt,
          };
          this.agentSubject.next(agent);
        }
      });
    } else {
      // No session service - create anonymous agent
      const anonymousAgent: Agent = {
        id: `anon-${Date.now()}`,
        displayName: 'Anonymous',
        type: 'human',
        visibility: 'private',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      this.agentSubject.next(anonymousAgent);
    }
  }

  /**
   * Get the current agent.
   */
  getCurrentAgent(): Observable<Agent | null> {
    return this.agent$.pipe(take(1));
  }

  /**
   * Get the current agent synchronously.
   * @deprecated Use getCurrentAgent() observable instead
   */
  getAgent(): Agent | null {
    return this.agentSubject.value;
  }

  /**
   * Get all progress records for the current agent.
   * Scans localStorage for all progress entries.
   */
  getAgentProgress(): Observable<AgentProgress[]> {
    const progress: AgentProgress[] = [];
    const agentId = this.getCurrentAgentId();

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith(`lamad-progress-${agentId}-`)) {
        try {
          const data = localStorage.getItem(key);
          if (data) {
            progress.push(JSON.parse(data) as AgentProgress);
          }
        } catch {
          // Skip malformed entries
        }
      }
    }

    return of(progress);
  }

  /**
   * Get the current agent ID (synchronous, for cache keys).
   */
  getCurrentAgentId(): string {
    if (this.sessionHumanService) {
      return this.sessionHumanService.getSessionId();
    }
    return this.agentSubject.value?.id ?? 'anonymous';
  }

  /**
   * Check if user is a session user (vs authenticated Holochain user).
   */
  isSessionUser(): boolean {
    return this.sessionHumanService !== null;
  }

  /**
   * Get the current access level.
   */
  getAccessLevel(): AccessLevel {
    if (this.sessionHumanService) {
      return this.sessionHumanService.getAccessLevel();
    }
    // Future: check Holochain attestations
    return 'visitor';
  }

  /**
   * Check if user can access content.
   */
  checkContentAccess(accessMetadata?: ContentAccessMetadata): AccessCheckResult {
    if (this.sessionHumanService) {
      return this.sessionHumanService.checkContentAccess(accessMetadata);
    }
    // Default: allow (no access metadata = open)
    return { canAccess: true };
  }

  /**
   * Get progress for a specific path.
   * Checks localStorage first (prototype writes go here),
   * then falls back to JSON file.
   */
  getProgressForPath(pathId: string): Observable<AgentProgress | null> {
    // Check cache first
    if (this.progressCache.has(pathId)) {
      return of(this.progressCache.get(pathId)!);
    }

    const agentId = this.getCurrentAgentId();

    // Check localStorage (prototype writes go here)
    const localProgress = this.dataLoader.getLocalProgress(agentId, pathId);
    if (localProgress) {
      this.progressCache.set(pathId, localProgress);
      localProgress.attestationsEarned.forEach(a => this.attestations.add(a));
      return of(localProgress);
    }

    // Fall back to JSON file
    return this.dataLoader.getAgentProgress(agentId, pathId).pipe(
      tap(progress => {
        if (progress) {
          this.progressCache.set(pathId, progress);
          progress.attestationsEarned.forEach(a => this.attestations.add(a));
        }
      })
    );
  }

  /**
   * Mark a step as completed.
   *
   * @param pathId The learning path ID
   * @param stepIndex The step index to mark complete
   * @param resourceId Optional content resourceId - if provided, also tracks global content completion
   */
  completeStep(pathId: string, stepIndex: number, resourceId?: string): Observable<void> {
    return this.getProgressForPath(pathId).pipe(
      switchMap(existingProgress => {
        const now = new Date().toISOString();
        const agentId = this.getCurrentAgentId();

        const progress: AgentProgress = existingProgress ?? {
          agentId,
          pathId,
          currentStepIndex: 0,
          completedStepIndices: [],
          startedAt: now,
          lastActivityAt: now,
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: [],
        };

        // Check if this is starting a new path
        const isNewPath = !existingProgress;

        if (!progress.completedStepIndices.includes(stepIndex)) {
          progress.completedStepIndices.push(stepIndex);
          progress.completedStepIndices.sort((a, b) => a - b);
        }

        progress.currentStepIndex = Math.max(progress.currentStepIndex, stepIndex + 1);
        progress.lastActivityAt = now;

        this.progressCache.set(pathId, progress);

        // Record activity in session
        if (this.sessionHumanService) {
          if (isNewPath) {
            this.sessionHumanService.recordPathStarted(pathId);
          }
          this.sessionHumanService.recordStepCompleted(pathId, stepIndex);
        }

        // Track content completion globally if resourceId provided
        if (resourceId) {
          return this.dataLoader
            .saveAgentProgress(progress)
            .pipe(switchMap(() => this.completeContentNode(resourceId, agentId)));
        }

        return this.dataLoader.saveAgentProgress(progress);
      })
    );
  }

  /**
   * Update affinity for a step.
   * Delta can be positive (engaged) or negative (disengaged).
   * Clamped to 0.0-1.0 range.
   */
  updateAffinity(pathId: string, stepIndex: number, delta: number): Observable<void> {
    return this.getProgressForPath(pathId).pipe(
      switchMap(existingProgress => {
        if (!existingProgress) {
          // Can't update affinity without progress
          return of(undefined);
        }

        const current = existingProgress.stepAffinity[stepIndex] ?? 0;
        existingProgress.stepAffinity[stepIndex] = Math.max(0, Math.min(1, current + delta));
        existingProgress.lastActivityAt = new Date().toISOString();

        this.progressCache.set(pathId, existingProgress);
        return this.dataLoader.saveAgentProgress(existingProgress);
      })
    );
  }

  /**
   * Save notes for a specific step.
   */
  saveStepNotes(pathId: string, stepIndex: number, notes: string): Observable<void> {
    return this.getProgressForPath(pathId).pipe(
      switchMap(existingProgress => {
        const now = new Date().toISOString();
        const agentId = this.getCurrentAgentId();

        const progress: AgentProgress = existingProgress ?? {
          agentId,
          pathId,
          currentStepIndex: 0,
          completedStepIndices: [],
          startedAt: now,
          lastActivityAt: now,
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: [],
        };

        progress.stepNotes[stepIndex] = notes;
        progress.lastActivityAt = now;

        this.progressCache.set(pathId, progress);

        // Record notes saved in session (triggers upgrade prompt)
        if (this.sessionHumanService) {
          this.sessionHumanService.recordNotesSaved(pathId, stepIndex);
        }

        return this.dataLoader.saveAgentProgress(progress);
      })
    );
  }

  /**
   * Save reflection responses for a step.
   */
  saveReflectionResponses(
    pathId: string,
    stepIndex: number,
    responses: string[]
  ): Observable<void> {
    return this.getProgressForPath(pathId).pipe(
      switchMap(existingProgress => {
        if (!existingProgress) {
          return of(undefined);
        }

        existingProgress.reflectionResponses[stepIndex] = responses;
        existingProgress.lastActivityAt = new Date().toISOString();

        this.progressCache.set(pathId, existingProgress);
        return this.dataLoader.saveAgentProgress(existingProgress);
      })
    );
  }

  /**
   * Grant an attestation to the current agent.
   */
  grantAttestation(attestationId: string, _earnedVia: string): void {
    this.attestations.add(attestationId);
  }

  /**
   * Check if agent has a specific attestation.
   */
  hasAttestation(attestationId: string): boolean {
    return this.attestations.has(attestationId);
  }

  /**
   * Get all attestations.
   */
  getAttestations(): string[] {
    return Array.from(this.attestations);
  }

  // =========================================================================
  // SHARED CONTENT COMPLETION TRACKING (Khan Academy-style)
  // =========================================================================

  /**
   * Track content completion globally across all paths.
   *
   * Uses special pathId '__global__' to store cross-path completion data.
   * When content is completed in any path, it's marked here and will show
   * as completed in ALL paths that reference the same content.
   *
   * @param contentId The resourceId of the completed content
   * @param agentId Optional agent ID (defaults to current agent)
   */
  completeContentNode(contentId: string, agentId?: string): Observable<void> {
    const targetAgentId = agentId ?? this.getCurrentAgentId();

    return this.getProgressForPath('__global__').pipe(
      switchMap(existingProgress => {
        const now = new Date().toISOString();

        const progress: AgentProgress = existingProgress ?? {
          agentId: targetAgentId,
          pathId: '__global__',
          currentStepIndex: 0,
          completedStepIndices: [],
          startedAt: now,
          lastActivityAt: now,
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: [],
          completedContentIds: [],
        };

        // Add to completed content (avoid duplicates)
        progress.completedContentIds ??= [];
        if (!progress.completedContentIds.includes(contentId)) {
          progress.completedContentIds.push(contentId);
        }
        progress.lastActivityAt = now;

        this.progressCache.set('__global__', progress);
        return this.dataLoader.saveAgentProgress(progress);
      })
    );
  }

  /**
   * Check if content has been completed in any path.
   *
   * @param contentId The resourceId to check
   * @param agentId Optional agent ID (defaults to current agent)
   */
  isContentCompleted(contentId: string, _agentId?: string): Observable<boolean> {
    // Note: agentId parameter is for future multi-agent support
    return this.getProgressForPath('__global__').pipe(
      map(progress => {
        if (!progress?.completedContentIds) {
          return false;
        }
        return progress.completedContentIds.includes(contentId);
      })
    );
  }

  /**
   * Get all completed content IDs across all paths.
   *
   * Returns as a Set for efficient O(1) lookup in PathService calculations.
   *
   * @param agentId Optional agent ID (defaults to current agent)
   */
  getCompletedContentIds(_agentId?: string): Observable<Set<string>> {
    // Note: agentId parameter is for future multi-agent support
    return this.getProgressForPath('__global__').pipe(
      map(progress => {
        if (!progress?.completedContentIds) {
          return new Set<string>();
        }
        return new Set(progress.completedContentIds);
      })
    );
  }

  /**
   * Get mastery level for a specific content node.
   *
   * @param contentId The content resource ID
   * @param agentId Optional agent ID (defaults to current agent)
   */
  getContentMastery(contentId: string, _agentId?: string): Observable<MasteryLevel> {
    return this.getProgressForPath('__global__').pipe(
      map(progress => {
        if (!progress?.contentMastery) {
          return 'not_started' as MasteryLevel;
        }
        return progress.contentMastery[contentId] ?? 'not_started';
      })
    );
  }

  /**
   * Get all content mastery levels as a Map.
   *
   * @param agentId Optional agent ID (defaults to current agent)
   */
  getAllContentMastery(_agentId?: string): Observable<Map<string, MasteryLevel>> {
    return this.getProgressForPath('__global__').pipe(
      map(progress => {
        if (!progress?.contentMastery) {
          return new Map<string, MasteryLevel>();
        }
        return new Map(Object.entries(progress.contentMastery));
      })
    );
  }

  /**
   * Update mastery level for a content node.
   *
   * Mastery only increases, never decreases (ratchet behavior).
   * This tracks progression through: seen → practiced → applied → mastered
   *
   * @param contentId The content resource ID
   * @param level The new mastery level
   * @param agentId Optional agent ID (defaults to current agent)
   */
  updateContentMastery(contentId: string, level: MasteryLevel, agentId?: string): Observable<void> {
    const targetAgentId = agentId ?? this.getCurrentAgentId();

    return this.getProgressForPath('__global__').pipe(
      switchMap(existingProgress => {
        const now = new Date().toISOString();

        const progress: AgentProgress = existingProgress ?? {
          agentId: targetAgentId,
          pathId: '__global__',
          currentStepIndex: 0,
          completedStepIndices: [],
          startedAt: now,
          lastActivityAt: now,
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: [],
          completedContentIds: [],
          contentMastery: {},
        };

        // Initialize if missing
        progress.contentMastery ??= {};

        // Ratchet behavior: only increase mastery level
        const currentLevel = progress.contentMastery[contentId] ?? 'not_started';
        if (MASTERY_LEVEL_VALUES[level] > MASTERY_LEVEL_VALUES[currentLevel]) {
          progress.contentMastery[contentId] = level;
        }

        // Also mark as completed if at 'apply' or above
        if (MASTERY_LEVEL_VALUES[level] >= MASTERY_LEVEL_VALUES['apply']) {
          progress.completedContentIds ??= [];
          if (!progress.completedContentIds.includes(contentId)) {
            progress.completedContentIds.push(contentId);
          }
        }

        progress.lastActivityAt = now;

        this.progressCache.set('__global__', progress);
        return this.dataLoader.saveAgentProgress(progress);
      })
    );
  }

  /**
   * Mark content as "seen" (viewed but not yet practiced).
   * Convenience method for the most common mastery update.
   */
  markContentSeen(contentId: string, agentId?: string): Observable<void> {
    return this.updateContentMastery(contentId, 'seen', agentId);
  }

  /**
   * Get the learning frontier - paths with active progress.
   * Returns the "resume" points for the learner dashboard.
   */
  getLearningFrontier(): Observable<FrontierItem[]> {
    // In prototype, scan localStorage for progress entries
    const frontier: FrontierItem[] = [];
    const agentId = this.getCurrentAgentId();

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith(`lamad-progress-${agentId}-`)) {
        try {
          const data = localStorage.getItem(key);
          if (data) {
            const progress = JSON.parse(data) as AgentProgress;
            if (!progress.completedAt) {
              frontier.push({
                pathId: progress.pathId,
                nextStepIndex: progress.currentStepIndex,
                lastActivity: progress.lastActivityAt,
              });
            }
          }
        } catch {
          // Skip malformed entries
        }
      }
    }

    // Sort by most recent activity
    frontier.sort(
      (a, b) => new Date(b.lastActivity).getTime() - new Date(a.lastActivity).getTime()
    );

    return of(frontier);
  }

  /**
   * Clear progress cache - useful for testing.
   */
  clearProgressCache(): void {
    this.progressCache.clear();
  }

  // =========================================================================
  // LEARNING ANALYTICS
  // =========================================================================

  /**
   * Get comprehensive learning analytics for the current agent.
   *
   * Returns metrics useful for dashboard displays:
   * - Overall progress statistics
   * - Learning patterns and streaks
   * - Path activity summary
   *
   * This aggregates data from localStorage progress records.
   */
  getLearningAnalytics(): Observable<{
    totalPathsStarted: number;
    totalPathsCompleted: number;
    totalContentNodesCompleted: number;
    totalStepsCompleted: number;
    totalLearningTime: number;
    lastActivityDate: string;
    firstActivityDate: string;
    currentStreak: number;
    longestStreak: number;
    mostActivePathId: string | null;
    mostActivePathTitle?: string;
    mostRecentPathId: string | null;
    mostRecentPathTitle?: string;
    averageAffinity: number;
    highAffinityPaths: string[];
    totalAttestationsEarned: number;
    attestationIds: string[];
  }> {
    return this.getAgentProgress().pipe(
      map(progressRecords => this.buildLearningAnalytics(progressRecords))
    );
  }

  private buildLearningAnalytics(progressRecords: AgentProgress[]): any {
    const pathProgress = progressRecords.filter(p => p.pathId !== '__global__');
    const globalProgress = progressRecords.find(p => p.pathId === '__global__');

    const basicCounts = this.calculateBasicCounts(pathProgress, globalProgress);
    const dateMetrics = this.calculateDateMetrics(pathProgress);
    const pathMetrics = this.calculatePathMetrics(pathProgress);
    const affinityMetrics = this.calculateAffinityMetrics(pathProgress);
    const attestationMetrics = this.calculateAttestationMetrics(pathProgress);

    return {
      ...basicCounts,
      ...dateMetrics,
      ...pathMetrics,
      ...affinityMetrics,
      ...attestationMetrics,
    };
  }

  private calculateBasicCounts(
    pathProgress: AgentProgress[],
    globalProgress: AgentProgress | undefined
  ): {
    totalPathsStarted: number;
    totalPathsCompleted: number;
    totalContentNodesCompleted: number;
    totalStepsCompleted: number;
  } {
    return {
      totalPathsStarted: pathProgress.length,
      totalPathsCompleted: pathProgress.filter(p => p.completedAt).length,
      totalContentNodesCompleted: globalProgress?.completedContentIds?.length ?? 0,
      totalStepsCompleted: pathProgress.reduce((sum, p) => sum + p.completedStepIndices.length, 0),
    };
  }

  private calculateDateMetrics(pathProgress: AgentProgress[]): {
    firstActivityDate: string;
    lastActivityDate: string;
    totalLearningTime: number;
    currentStreak: number;
    longestStreak: number;
  } {
    let firstActivityDate = '';
    let lastActivityDate = '';

    if (pathProgress.length > 0) {
      const startDates = pathProgress
        .map(p => new Date(p.startedAt).getTime())
        .filter(d => !isNaN(d));
      const endDates = pathProgress
        .map(p => new Date(p.lastActivityAt).getTime())
        .filter(d => !isNaN(d));

      if (startDates.length > 0)
        firstActivityDate = new Date(Math.min(...startDates)).toISOString();
      if (endDates.length > 0) lastActivityDate = new Date(Math.max(...endDates)).toISOString();
    }

    const totalLearningTime =
      firstActivityDate && lastActivityDate
        ? Math.floor(
            (new Date(lastActivityDate).getTime() - new Date(firstActivityDate).getTime()) /
              (1000 * 60 * 60 * 24)
          )
        : 0;

    const activityDates = Array.from(
      new Set(pathProgress.map(p => new Date(p.lastActivityAt).toISOString().split('T')[0]))
    );

    return {
      firstActivityDate,
      lastActivityDate,
      totalLearningTime,
      currentStreak: this.calculateCurrentStreak(activityDates),
      longestStreak: this.calculateLongestStreak(activityDates),
    };
  }

  private calculatePathMetrics(pathProgress: AgentProgress[]): {
    mostActivePathId: string | null;
    mostRecentPathId: string | null;
  } {
    let mostActivePathId: string | null = null;
    let maxSteps = 0;
    for (const p of pathProgress) {
      if (p.completedStepIndices.length > maxSteps) {
        maxSteps = p.completedStepIndices.length;
        mostActivePathId = p.pathId;
      }
    }

    const mostRecentPathId =
      pathProgress.length > 0
        ? pathProgress.reduce(
            (latest, p) =>
              new Date(p.lastActivityAt) > new Date(latest.lastActivityAt) ? p : latest,
            pathProgress[0]
          ).pathId
        : null;

    return { mostActivePathId, mostRecentPathId };
  }

  private calculateAffinityMetrics(pathProgress: AgentProgress[]): {
    averageAffinity: number;
    highAffinityPaths: string[];
  } {
    let totalAffinity = 0;
    let affinityCount = 0;
    const pathAffinities = new Map<string, { sum: number; count: number }>();

    for (const p of pathProgress) {
      const affinityValues = Object.values(p.stepAffinity);
      for (const affinity of affinityValues) {
        totalAffinity += affinity;
        affinityCount++;
      }
      if (affinityValues.length > 0) {
        pathAffinities.set(p.pathId, {
          sum: affinityValues.reduce((sum, a) => sum + a, 0),
          count: affinityValues.length,
        });
      }
    }

    const highAffinityPaths: string[] = [];
    for (const [pathId, { sum, count }] of pathAffinities) {
      if (sum / count > 0.7) highAffinityPaths.push(pathId);
    }

    return {
      averageAffinity: affinityCount > 0 ? totalAffinity / affinityCount : 0,
      highAffinityPaths,
    };
  }

  private calculateAttestationMetrics(pathProgress: AgentProgress[]): {
    totalAttestationsEarned: number;
    attestationIds: string[];
  } {
    const allAttestations = new Set<string>();
    for (const p of pathProgress) {
      for (const att of p.attestationsEarned) {
        allAttestations.add(att);
      }
    }
    return {
      totalAttestationsEarned: allAttestations.size,
      attestationIds: Array.from(allAttestations),
    };
  }

  /**
   * Calculate current learning streak (consecutive days with activity).
   * A streak is broken if there's a gap > 1 day.
   */
  private calculateCurrentStreak(activityDates: string[]): number {
    if (activityDates.length === 0) return 0;

    // Sort dates descending (most recent first)
    const sorted = activityDates.map(d => new Date(d)).sort((a, b) => b.getTime() - a.getTime());

    const today = new Date();
    today.setHours(0, 0, 0, 0);

    let streak = 0;
    let currentDate = today;

    for (const activityDate of sorted) {
      activityDate.setHours(0, 0, 0, 0);

      const daysDiff = Math.floor(
        (currentDate.getTime() - activityDate.getTime()) / (1000 * 60 * 60 * 24)
      );

      if (daysDiff === 0 || daysDiff === 1) {
        streak++;
        currentDate = activityDate;
      } else {
        break;
      }
    }

    return streak;
  }

  /**
   * Calculate longest learning streak from activity dates.
   */
  private calculateLongestStreak(activityDates: string[]): number {
    if (activityDates.length === 0) return 0;

    // Sort dates ascending
    const sorted = activityDates.map(d => new Date(d)).sort((a, b) => a.getTime() - b.getTime());

    let longestStreak = 1;
    let currentStreak = 1;

    for (let i = 1; i < sorted.length; i++) {
      const prevDate = sorted[i - 1];
      const currDate = sorted[i];

      const daysDiff = Math.floor(
        (currDate.getTime() - prevDate.getTime()) / (1000 * 60 * 60 * 24)
      );

      if (daysDiff === 1) {
        currentStreak++;
        longestStreak = Math.max(longestStreak, currentStreak);
      } else {
        currentStreak = 1;
      }
    }

    return longestStreak;
  }
}
