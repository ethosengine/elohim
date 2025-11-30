import { Injectable, Optional } from '@angular/core';
import { Observable, of, forkJoin, combineLatest } from 'rxjs';
import { map, switchMap, catchError } from 'rxjs/operators';

import { DataLoaderService } from './data-loader.service';
import { PathService } from './path.service';
import { SessionHumanService } from './session-human.service';
import { AffinityTrackingService } from './affinity-tracking.service';
import { AgentService } from './agent.service';

import {
  HumanProfile,
  JourneyStats,
  CurrentFocus,
  DevelopedCapability,
  TimelineEvent,
  TimelineEventType,
  ContentEngagement,
  NoteWithContext,
  ResumePoint,
  PathWithProgress,
  PathsOverview,
  ProfileSummaryCompact,
} from '../models/profile.model';
import { SessionPathProgress, SessionActivity } from '../models/session-human.model';

/**
 * ProfileService - Human-Centered Identity View
 *
 * Aligned with Imago Dei Framework:
 * - imagodei-core: Stable identity center (getProfile)
 * - imagodei-experience: Learning and transformation (getTimeline)
 * - imagodei-gifts: Developed capabilities (getCapabilities)
 * - imagodei-synthesis: Growth and meaning-making (getTopEngagedContent)
 *
 * This service aggregates data from multiple sources to provide
 * a narrative view of the human's journey, not just consumption metrics.
 *
 * Holochain migration:
 * - Profile data derives from agent's source chain
 * - Timeline reconstructed from zome calls
 * - Capabilities come from attestation entries
 */
@Injectable({ providedIn: 'root' })
export class ProfileService {
  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly pathService: PathService,
    private readonly affinityService: AffinityTrackingService,
    private readonly agentService: AgentService,
    @Optional() private readonly sessionHumanService: SessionHumanService | null
  ) {}

  // =========================================================================
  // Core Profile (imagodei-core)
  // =========================================================================

  /**
   * Get the complete human profile.
   * This is the stable center of identity combined with growth indicators.
   */
  getProfile(): Observable<HumanProfile> {
    return combineLatest([
      this.getJourneyStats(),
      this.getCurrentFocus(),
      this.getDevelopedCapabilities(),
    ]).pipe(
      map(([journeyStats, currentFocus, developedCapabilities]) => {
        const session = this.sessionHumanService?.getSession();
        const agent = this.agentService.getAgent();

        return {
          id: session?.sessionId ?? agent?.id ?? 'unknown',
          displayName: session?.displayName ?? agent?.displayName ?? 'Traveler',
          isSessionBased: !!session,
          journeyStartedAt: session?.createdAt ?? agent?.createdAt ?? new Date().toISOString(),
          lastActiveAt: session?.lastActiveAt ?? agent?.updatedAt ?? new Date().toISOString(),
          journeyStats,
          currentFocus,
          developedCapabilities,
        };
      })
    );
  }

  /**
   * Get compact profile summary for headers/cards.
   */
  getProfileSummary(): Observable<ProfileSummaryCompact> {
    return this.getProfile().pipe(
      map(profile => ({
        displayName: profile.displayName,
        isSessionBased: profile.isSessionBased,
        journeysCompleted: profile.journeyStats.journeysCompleted,
        capabilitiesEarned: profile.developedCapabilities.length,
        currentFocusTitle: profile.currentFocus[0]?.pathTitle,
        currentFocusProgress: profile.currentFocus[0]?.progressPercent,
      }))
    );
  }

  // =========================================================================
  // Journey Statistics
  // =========================================================================

  /**
   * Get aggregate journey statistics.
   * Transforms raw stats into growth-oriented metrics.
   */
  getJourneyStats(): Observable<JourneyStats> {
    const session = this.sessionHumanService?.getSession();

    if (session) {
      // Session-based: derive from session stats
      const affinityData = this.affinityService['affinitySubject'].value;
      const meaningfulCount = Object.values(affinityData.affinity)
        .filter(v => v > 0.5).length;

      return of({
        territoryExplored: session.stats.nodesViewed,
        journeysStarted: session.stats.pathsStarted,
        journeysCompleted: session.stats.pathsCompleted,
        stepsCompleted: session.stats.stepsCompleted,
        meaningfulEncounters: meaningfulCount,
        timeInvested: session.stats.totalSessionTime,
        sessionsCount: session.stats.sessionCount,
      });
    }

    // Holochain-based: derive from agent progress
    return this.agentService.getAgentProgress().pipe(
      map(progressRecords => {
        let stepsCompleted = 0;
        let journeysCompleted = 0;
        const journeysStarted = progressRecords.length;

        progressRecords.forEach(progress => {
          stepsCompleted += progress.completedStepIndices.length;
          // A path is completed if we have progress and all required steps done
          // (simplified: assume completed if current step is last)
          if (progress.completedStepIndices.length > 0) {
            // This would need path data to accurately determine completion
            // For now, count any path with completed steps
          }
        });

        return {
          territoryExplored: Object.keys(this.affinityService['affinitySubject'].value.affinity).length,
          journeysStarted,
          journeysCompleted,
          stepsCompleted,
          meaningfulEncounters: 0, // Would need to count high-affinity nodes
          timeInvested: 0, // Not tracked in current model
          sessionsCount: 1,
        };
      })
    );
  }

  // =========================================================================
  // Current Focus (imagodei-experience)
  // =========================================================================

  /**
   * Get paths the human is currently working on.
   * Sorted by most recent activity.
   */
  getCurrentFocus(): Observable<CurrentFocus[]> {
    const pathProgress = this.sessionHumanService?.getAllPathProgress() ?? [];

    if (pathProgress.length === 0) {
      return of([]);
    }

    // Get path metadata for each in-progress path
    const pathRequests = pathProgress
      .filter(p => !p.completedAt) // Only incomplete paths
      .map(progress =>
        this.pathService.getPath(progress.pathId).pipe(
          map((path): CurrentFocus => ({
            pathId: progress.pathId,
            pathTitle: path.title,
            currentStepIndex: progress.currentStepIndex,
            totalSteps: path.steps.length,
            progressPercent: Math.round((progress.completedStepIndices.length / path.steps.length) * 100),
            lastActiveAt: progress.lastActivityAt,
            nextStepTitle: path.steps[progress.currentStepIndex]?.stepTitle,
            nextStepNarrative: path.steps[progress.currentStepIndex]?.stepNarrative,
          })),
          catchError(() => of(null as CurrentFocus | null))
        )
      );

    if (pathRequests.length === 0) {
      return of([]);
    }

    return forkJoin(pathRequests).pipe(
      map(results =>
        results
          .filter((r): r is CurrentFocus => r !== null)
          .sort((a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime())
      )
    );
  }

  // =========================================================================
  // Developed Capabilities (imagodei-gifts)
  // =========================================================================

  /**
   * Get capabilities developed through learning.
   * Derived from attestations earned.
   */
  getDevelopedCapabilities(): Observable<DevelopedCapability[]> {
    const attestations = this.agentService.getAttestations();

    // For MVP, return basic capability info from attestation IDs
    // In full implementation, this would load attestation metadata
    return of(
      attestations.map(attestationId => ({
        id: attestationId,
        name: this.formatAttestationName(attestationId),
        description: `Earned through completing learning path`,
        earnedAt: new Date().toISOString(), // Would come from attestation metadata
        level: 'learning' as const,
        icon: '🏅',
      }))
    );
  }

  /**
   * Format attestation ID into human-readable name.
   */
  private formatAttestationName(id: string): string {
    return id
      .replace(/-/g, ' ')
      .replace(/\b\w/g, l => l.toUpperCase());
  }

  // =========================================================================
  // Learning Timeline (imagodei-experience)
  // =========================================================================

  /**
   * Get chronological timeline of significant learning events.
   * These are transformation points, not just activity logs.
   */
  getTimeline(limit: number = 50): Observable<TimelineEvent[]> {
    const activities = this.sessionHumanService?.getActivityHistory() ?? [];

    // Transform activities into timeline events
    const events: TimelineEvent[] = activities
      .map(activity => this.activityToTimelineEvent(activity))
      .filter((e): e is TimelineEvent => e !== null)
      .slice(-limit)
      .reverse(); // Most recent first

    return of(events);
  }

  /**
   * Convert a session activity to a timeline event.
   * Only significant activities become events.
   */
  private activityToTimelineEvent(activity: SessionActivity): TimelineEvent | null {
    const timestamp = activity.timestamp;
    const resourceId = activity.resourceId;

    switch (activity.type) {
      case 'path-start':
        return {
          id: `${activity.type}-${resourceId}-${timestamp}`,
          type: 'journey_started' as TimelineEventType,
          timestamp,
          title: 'Started a new learning journey',
          resourceId,
          resourceType: 'path',
          significance: 'milestone',
        };

      case 'path-complete':
        return {
          id: `${activity.type}-${resourceId}-${timestamp}`,
          type: 'journey_completed' as TimelineEventType,
          timestamp,
          title: 'Completed a learning journey',
          resourceId,
          resourceType: 'path',
          significance: 'milestone',
        };

      case 'step-complete':
        return {
          id: `${activity.type}-${resourceId}-${timestamp}`,
          type: 'step_completed' as TimelineEventType,
          timestamp,
          title: 'Completed a step',
          description: `Step ${activity.metadata?.['stepIndex']}`,
          resourceId,
          resourceType: 'path',
          significance: 'progress',
        };

      case 'affinity':
        // Only create event for meaningful affinity (> 0.5)
        const affinityValue = activity.metadata?.['value'];
        if (typeof affinityValue === 'number' && affinityValue > 0.5) {
          return {
            id: `${activity.type}-${resourceId}-${timestamp}`,
            type: 'meaningful_encounter' as TimelineEventType,
            timestamp,
            title: 'Found resonant content',
            resourceId,
            resourceType: 'content',
            significance: 'progress',
          };
        }
        return null;

      case 'view':
        // First view is significant
        // (Would need to check if it's actually first view)
        return null;

      default:
        return null;
    }
  }

  // =========================================================================
  // Content Engagement (imagodei-synthesis)
  // =========================================================================

  /**
   * Get content with highest engagement (affinity).
   * High affinity indicates resonance and meaning-making.
   */
  getTopEngagedContent(limit: number = 10): Observable<ContentEngagement[]> {
    const affinityData = this.affinityService['affinitySubject'].value.affinity;

    // Sort by affinity descending
    const sorted = Object.entries(affinityData)
      .filter(([_, affinity]) => affinity > 0)
      .sort((a, b) => b[1] - a[1])
      .slice(0, limit);

    if (sorted.length === 0) {
      return of([]);
    }

    // Load content metadata for top engaged nodes
    const contentRequests = sorted.map(([nodeId, affinity]) =>
      this.dataLoader.getContent(nodeId).pipe(
        map((content): ContentEngagement => ({
          nodeId,
          title: content.title,
          contentType: content.contentType,
          affinity,
          viewCount: 1, // Would need view tracking
          lastViewedAt: new Date().toISOString(), // Would need activity lookup
          hasNotes: false, // Would need notes lookup
          containingPaths: [] as string[], // Would need reverse lookup
        })),
        catchError(() => of(null as ContentEngagement | null))
      )
    );

    return forkJoin(contentRequests).pipe(
      map(results =>
        results.filter((r): r is ContentEngagement => r !== null)
      )
    );
  }

  // =========================================================================
  // Notes (imagodei-synthesis)
  // =========================================================================

  /**
   * Get all personal notes with context.
   * Notes are meaning-making artifacts.
   */
  getAllNotes(): Observable<NoteWithContext[]> {
    const pathProgress = this.sessionHumanService?.getAllPathProgress() ?? [];
    const notes: NoteWithContext[] = [];

    // Collect notes from path progress
    pathProgress.forEach(progress => {
      Object.entries(progress.stepNotes ?? {}).forEach(([stepIndex, content]) => {
        if (content) {
          notes.push({
            id: `${progress.pathId}-step-${stepIndex}`,
            content,
            createdAt: progress.lastActivityAt,
            updatedAt: progress.lastActivityAt,
            context: {
              type: 'path_step',
              pathId: progress.pathId,
              stepIndex: parseInt(stepIndex, 10),
            },
          });
        }
      });
    });

    // Enrich with path/step titles
    if (notes.length === 0) {
      return of([]);
    }

    const enrichmentRequests = notes.map(note =>
      this.pathService.getPath(note.context.pathId!).pipe(
        map(path => {
          const stepIndex = note.context.stepIndex!;
          return {
            ...note,
            context: {
              ...note.context,
              pathTitle: path.title,
              stepTitle: path.steps[stepIndex]?.stepTitle,
            },
          };
        }),
        catchError(() => of(note))
      )
    );

    return forkJoin(enrichmentRequests);
  }

  // =========================================================================
  // Resume Point (Smart Continuation)
  // =========================================================================

  /**
   * Get smart suggestion for where to continue.
   * Honors the human's ongoing journey.
   */
  getResumePoint(): Observable<ResumePoint | null> {
    return this.getCurrentFocus().pipe(
      map(focus => {
        if (focus.length > 0) {
          // Continue most recent path
          const mostRecent = focus[0];
          const daysSince = Math.floor(
            (Date.now() - new Date(mostRecent.lastActiveAt).getTime()) / (1000 * 60 * 60 * 24)
          );

          return {
            type: 'continue_path' as const,
            title: `Continue: ${mostRecent.pathTitle}`,
            reason: daysSince === 0
              ? 'Pick up where you left off'
              : `Return to your journey after ${daysSince} day${daysSince === 1 ? '' : 's'}`,
            pathId: mostRecent.pathId,
            stepIndex: mostRecent.currentStepIndex,
            daysSinceActive: daysSince,
          };
        }

        // No active paths - suggest exploration
        return {
          type: 'explore_new' as const,
          title: 'Begin Your Journey',
          reason: 'Choose a learning path to start exploring',
          daysSinceActive: 0,
        };
      })
    );
  }

  // =========================================================================
  // Paths Overview
  // =========================================================================

  /**
   * Get organized view of all paths relevant to this human.
   */
  getPathsOverview(): Observable<PathsOverview> {
    return this.dataLoader.getPathIndex().pipe(
      switchMap(index => {
        const pathProgress = this.sessionHumanService?.getAllPathProgress() ?? [];
        const progressMap = new Map<string, SessionPathProgress>();
        pathProgress.forEach(p => progressMap.set(p.pathId, p));

        // Categorize paths
        const inProgressIds = pathProgress
          .filter(p => !p.completedAt)
          .map(p => p.pathId);

        const completedIds = pathProgress
          .filter(p => p.completedAt)
          .map(p => p.pathId);

        // Load full path data for each category
        const inProgressRequests = inProgressIds.map(id =>
          this.enrichPathWithProgress(id, progressMap.get(id)!)
        );

        const completedRequests = completedIds.map(id =>
          this.enrichPathWithProgress(id, progressMap.get(id)!)
        );

        // Suggested: paths not yet started (limit to 5)
        const suggestedEntries = index.paths
          .filter(p => !progressMap.has(p.id))
          .slice(0, 5);

        const suggestedRequests = suggestedEntries.map(entry =>
          this.pathService.getPath(entry.id).pipe(
            map(path => this.pathToPathWithProgress(path, null)),
            catchError(() => of(null))
          )
        );

        return forkJoin({
          inProgress: inProgressRequests.length > 0 ? forkJoin(inProgressRequests) : of([]),
          completed: completedRequests.length > 0 ? forkJoin(completedRequests) : of([]),
          suggested: suggestedRequests.length > 0 ? forkJoin(suggestedRequests) : of([]),
        });
      }),
      map(({ inProgress, completed, suggested }) => ({
        inProgress: inProgress.filter((p): p is PathWithProgress => p !== null),
        completed: completed.filter((p): p is PathWithProgress => p !== null),
        suggested: suggested.filter((p): p is PathWithProgress => p !== null),
      }))
    );
  }

  /**
   * Enrich a path ID with progress data.
   */
  private enrichPathWithProgress(
    pathId: string,
    progress: SessionPathProgress
  ): Observable<PathWithProgress | null> {
    return this.pathService.getPath(pathId).pipe(
      map(path => this.pathToPathWithProgress(path, progress)),
      catchError(() => of(null))
    );
  }

  /**
   * Convert path and progress to PathWithProgress.
   */
  private pathToPathWithProgress(
    path: { id: string; title: string; description: string; difficulty: 'beginner' | 'intermediate' | 'advanced'; steps: unknown[]; attestationsGranted?: string[] },
    progress: SessionPathProgress | null
  ): PathWithProgress {
    const totalSteps = path.steps.length;
    const completedSteps = progress?.completedStepIndices.length ?? 0;

    return {
      pathId: path.id,
      title: path.title,
      description: path.description,
      difficulty: path.difficulty,
      totalSteps,
      completedSteps,
      currentStepIndex: progress?.currentStepIndex ?? 0,
      progressPercent: totalSteps > 0 ? Math.round((completedSteps / totalSteps) * 100) : 0,
      startedAt: progress?.startedAt,
      completedAt: progress?.completedAt,
      attestationsGranted: path.attestationsGranted,
    };
  }
}
