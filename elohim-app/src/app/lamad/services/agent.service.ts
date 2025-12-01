import { Injectable, Optional } from '@angular/core';
import { BehaviorSubject, Observable, of } from 'rxjs';
import { map, tap, switchMap, take } from 'rxjs/operators';
import { DataLoaderService } from './data-loader.service';
import { SessionHumanService } from './session-human.service';
import { Agent, AgentProgress, FrontierItem } from '../models/agent.model';
import { AccessLevel, ContentAccessMetadata, AccessCheckResult } from '../models/content-access.model';

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
export class AgentService {
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

  /**
   * Initialize agent based on context.
   * For MVP, creates a session-based agent.
   */
  private initializeAgent(): void {
    if (this.sessionHumanService) {
      // Create agent from session
      this.sessionHumanService.session$.subscribe(session => {
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
      // Fallback to loading from data (legacy behavior)
      this.loadCurrentAgent();
    }
  }

  /**
   * Load the current agent's profile from data files.
   * Legacy behavior - used when no session service available.
   */
  private loadCurrentAgent(): void {
    // Legacy fallback - load from JSON
    this.dataLoader.getAgent('agent-matthew').subscribe(agent => {
      this.agentSubject.next(agent);
    });
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

        const progress: AgentProgress = existingProgress || {
          agentId,
          pathId,
          currentStepIndex: 0,
          completedStepIndices: [],
          startedAt: now,
          lastActivityAt: now,
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: []
        };

        // Check if this is starting a new path
        const isNewPath = !existingProgress;

        if (!progress.completedStepIndices.includes(stepIndex)) {
          progress.completedStepIndices.push(stepIndex);
          progress.completedStepIndices.sort((a, b) => a - b);
        }

        progress.currentStepIndex = Math.max(
          progress.currentStepIndex,
          stepIndex + 1
        );
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
          return this.dataLoader.saveAgentProgress(progress).pipe(
            switchMap(() => this.completeContentNode(resourceId, agentId))
          );
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

        const progress: AgentProgress = existingProgress || {
          agentId,
          pathId,
          currentStepIndex: 0,
          completedStepIndices: [],
          startedAt: now,
          lastActivityAt: now,
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: []
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
  saveReflectionResponses(pathId: string, stepIndex: number, responses: string[]): Observable<void> {
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
  grantAttestation(attestationId: string, earnedVia: string): void {
    this.attestations.add(attestationId);
    console.log(`[AgentService] Attestation granted: ${attestationId} via ${earnedVia}`);
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

        const progress: AgentProgress = existingProgress || {
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
          completedContentIds: []
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
                lastActivity: progress.lastActivityAt
              });
            }
          }
        } catch {
          // Skip malformed entries
        }
      }
    }

    // Sort by most recent activity
    frontier.sort((a, b) =>
      new Date(b.lastActivity).getTime() - new Date(a.lastActivity).getTime()
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
    // Overall progress
    totalPathsStarted: number;
    totalPathsCompleted: number;
    totalContentNodesCompleted: number;
    totalStepsCompleted: number;

    // Engagement metrics
    totalLearningTime: number; // Days between first and last activity
    lastActivityDate: string;
    firstActivityDate: string;
    currentStreak: number; // Days with consecutive activity
    longestStreak: number;

    // Path breakdown
    mostActivePathId: string | null;
    mostActivePathTitle?: string;
    mostRecentPathId: string | null;
    mostRecentPathTitle?: string;

    // Affinity insights
    averageAffinity: number; // Average across all steps with affinity
    highAffinityPaths: string[]; // Paths with avg affinity > 0.7

    // Attestations
    totalAttestationsEarned: number;
    attestationIds: string[];
  }> {
    const progress = this.getAgentProgress();

    return progress.pipe(
      map(progressRecords => {
        // Filter out __global__ progress
        const pathProgress = progressRecords.filter(p => p.pathId !== '__global__');
        const globalProgress = progressRecords.find(p => p.pathId === '__global__');

        // Basic counts
        const totalPathsStarted = pathProgress.length;
        const totalPathsCompleted = pathProgress.filter(p => p.completedAt).length;
        const totalContentNodesCompleted = globalProgress?.completedContentIds?.length ?? 0;
        const totalStepsCompleted = pathProgress.reduce(
          (sum, p) => sum + p.completedStepIndices.length,
          0
        );

        // Time analysis
        let firstActivityDate = '';
        let lastActivityDate = '';
        if (pathProgress.length > 0) {
          const dates = pathProgress
            .map(p => new Date(p.startedAt).getTime())
            .filter(d => !isNaN(d));

          if (dates.length > 0) {
            firstActivityDate = new Date(Math.min(...dates)).toISOString();
          }

          const lastDates = pathProgress
            .map(p => new Date(p.lastActivityAt).getTime())
            .filter(d => !isNaN(d));

          if (lastDates.length > 0) {
            lastActivityDate = new Date(Math.max(...lastDates)).toISOString();
          }
        }

        const totalLearningTime = firstActivityDate && lastActivityDate
          ? Math.floor(
              (new Date(lastActivityDate).getTime() - new Date(firstActivityDate).getTime()) /
              (1000 * 60 * 60 * 24)
            )
          : 0;

        // Streak calculation (simplified - counts distinct activity days)
        const activityDates = new Set(
          pathProgress
            .map(p => new Date(p.lastActivityAt).toISOString().split('T')[0])
        );
        const currentStreak = this.calculateCurrentStreak(Array.from(activityDates));
        const longestStreak = this.calculateLongestStreak(Array.from(activityDates));

        // Most active path (by total steps completed)
        let mostActivePathId: string | null = null;
        let maxSteps = 0;
        for (const p of pathProgress) {
          if (p.completedStepIndices.length > maxSteps) {
            maxSteps = p.completedStepIndices.length;
            mostActivePathId = p.pathId;
          }
        }

        // Most recent path
        const mostRecentPathId = pathProgress.length > 0
          ? pathProgress.reduce((latest, p) =>
              new Date(p.lastActivityAt) > new Date(latest.lastActivityAt) ? p : latest,
              pathProgress[0]
            ).pathId
          : null;

        // Affinity analysis
        let totalAffinity = 0;
        let affinityCount = 0;
        const pathAffinities = new Map<string, { sum: number; count: number }>();

        for (const p of pathProgress) {
          const affinityValues = Object.values(p.stepAffinity);
          for (const affinity of affinityValues) {
            totalAffinity += affinity;
            affinityCount++;
          }

          // Track per-path affinity
          if (affinityValues.length > 0) {
            const pathSum = affinityValues.reduce((sum, a) => sum + a, 0);
            pathAffinities.set(p.pathId, {
              sum: pathSum,
              count: affinityValues.length
            });
          }
        }

        const averageAffinity = affinityCount > 0 ? totalAffinity / affinityCount : 0;

        // High affinity paths (avg > 0.7)
        const highAffinityPaths: string[] = [];
        for (const [pathId, { sum, count }] of pathAffinities) {
          const avg = sum / count;
          if (avg > 0.7) {
            highAffinityPaths.push(pathId);
          }
        }

        // Attestations
        const allAttestations = new Set<string>();
        for (const p of pathProgress) {
          for (const att of p.attestationsEarned) {
            allAttestations.add(att);
          }
        }

        return {
          totalPathsStarted,
          totalPathsCompleted,
          totalContentNodesCompleted,
          totalStepsCompleted,
          totalLearningTime,
          lastActivityDate,
          firstActivityDate,
          currentStreak,
          longestStreak,
          mostActivePathId,
          mostRecentPathId,
          averageAffinity,
          highAffinityPaths,
          totalAttestationsEarned: allAttestations.size,
          attestationIds: Array.from(allAttestations)
        };
      })
    );
  }

  /**
   * Calculate current learning streak (consecutive days with activity).
   * A streak is broken if there's a gap > 1 day.
   */
  private calculateCurrentStreak(activityDates: string[]): number {
    if (activityDates.length === 0) return 0;

    // Sort dates descending (most recent first)
    const sorted = activityDates
      .map(d => new Date(d))
      .sort((a, b) => b.getTime() - a.getTime());

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
    const sorted = activityDates
      .map(d => new Date(d))
      .sort((a, b) => a.getTime() - b.getTime());

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
