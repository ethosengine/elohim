import { Injectable } from '@angular/core';
import { Observable, forkJoin, of } from 'rxjs';
import { map, switchMap } from 'rxjs/operators';
import { DataLoaderService } from './data-loader.service';
import { AgentService } from './agent.service';
import { LearningPath, PathStep, PathStepView, PathIndex, PathChapter } from '../models/learning-path.model';
import { ContentNode } from '../models/content-node.model';
import { AgentProgress } from '../models/agent.model';

/**
 * Access check result for fog-of-war system.
 */
export interface AccessCheckResult {
  accessible: boolean;
  reason?: string;
}

/**
 * PathService - Manages learning path navigation and implements fog-of-war access control.
 *
 * Key responsibilities:
 * - Load path metadata (without step content - lazy loading)
 * - Load specific steps with resolved content
 * - Enforce fog-of-war access rules
 * - Compose PathStepView with navigation context
 */
@Injectable({ providedIn: 'root' })
export class PathService {
  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly agentService: AgentService
  ) {}

  /**
   * Get path metadata (does NOT load step content).
   * Use this for path overview pages.
   */
  getPath(pathId: string): Observable<LearningPath> {
    return this.dataLoader.getPath(pathId);
  }

  /**
   * Get specific step WITH resolved content (lazy loading).
   * This is the primary navigation method.
   */
  getPathStep(pathId: string, stepIndex: number): Observable<PathStepView> {
    return this.dataLoader.getPath(pathId).pipe(
      switchMap(path => {
        // Validate step index
        if (stepIndex < 0 || stepIndex >= path.steps.length) {
          throw new Error(`Step index ${stepIndex} out of range for path ${pathId}`);
        }

        const step = path.steps[stepIndex];

        // Load content for THIS step only (lazy loading)
        return forkJoin({
          content: this.dataLoader.getContent(step.resourceId),
          progress: this.agentService.getProgressForPath(pathId)
        }).pipe(
          map(({ content, progress }) =>
            this.composeStepView(path, step, stepIndex, content, progress)
          )
        );
      })
    );
  }

  /**
   * Get path catalog for discovery.
   */
  listPaths(): Observable<PathIndex> {
    return this.dataLoader.getPathIndex();
  }

  /**
   * Check if a step is accessible (fog-of-war rules).
   *
   * Rules:
   * 1. Check attestation requirements
   * 2. Check sequential progression (can access completed, current, or +1 step)
   */
  isStepAccessible(
    path: LearningPath,
    stepIndex: number,
    progress: AgentProgress | null,
    attestations: string[]
  ): AccessCheckResult {
    // Validate step index
    if (stepIndex < 0 || stepIndex >= path.steps.length) {
      return {
        accessible: false,
        reason: 'Invalid step index'
      };
    }

    const step = path.steps[stepIndex];

    // Check attestation requirement
    if (step.attestationRequired) {
      if (!attestations.includes(step.attestationRequired)) {
        return {
          accessible: false,
          reason: `Requires attestation: ${step.attestationRequired}`
        };
      }
    }

    // Check sequential progression
    // Can access: any completed step, current step, or ONE step ahead
    if (progress) {
      const maxCompleted = progress.completedStepIndices.length > 0
        ? Math.max(...progress.completedStepIndices)
        : -1;
      const maxAccessible = maxCompleted + 2; // Completed + current + 1 ahead

      if (stepIndex > maxAccessible) {
        return {
          accessible: false,
          reason: 'Complete previous steps first'
        };
      }
    } else if (stepIndex > 0) {
      // No progress: can only access step 0
      return {
        accessible: false,
        reason: 'Start from the beginning'
      };
    }

    return { accessible: true };
  }

  /**
   * Check accessibility for a step using current agent state.
   */
  checkStepAccess(pathId: string, stepIndex: number): Observable<AccessCheckResult> {
    return forkJoin({
      path: this.getPath(pathId),
      progress: this.agentService.getProgressForPath(pathId)
    }).pipe(
      map(({ path, progress }) => {
        const attestations = this.agentService.getAttestations();
        return this.isStepAccessible(path, stepIndex, progress, attestations);
      })
    );
  }

  /**
   * Get all accessible step indices for a path.
   * Useful for path overview showing fog-of-war state.
   */
  getAccessibleSteps(pathId: string): Observable<number[]> {
    return forkJoin({
      path: this.getPath(pathId),
      progress: this.agentService.getProgressForPath(pathId)
    }).pipe(
      map(({ path, progress }) => {
        const attestations = this.agentService.getAttestations();
        const accessible: number[] = [];

        for (let i = 0; i < path.steps.length; i++) {
          const check = this.isStepAccessible(path, i, progress, attestations);
          if (check.accessible) {
            accessible.push(i);
          }
        }

        return accessible;
      })
    );
  }

  /**
   * Compose a PathStepView from its constituent parts.
   * Adds navigation context and progress information.
   */
  private composeStepView(
    path: LearningPath,
    step: PathStep,
    stepIndex: number,
    content: ContentNode,
    progress: AgentProgress | null
  ): PathStepView {
    return {
      step,
      content,

      // Navigation context
      hasPrevious: stepIndex > 0,
      hasNext: stepIndex < path.steps.length - 1,
      previousStepIndex: stepIndex > 0 ? stepIndex - 1 : undefined,
      nextStepIndex: stepIndex < path.steps.length - 1 ? stepIndex + 1 : undefined,

      // Progress for authenticated user
      isCompleted: progress?.completedStepIndices.includes(stepIndex) ?? false,
      affinity: progress?.stepAffinity[stepIndex],
      notes: progress?.stepNotes[stepIndex]
    };
  }

  /**
   * Get the total number of steps in a path.
   */
  getStepCount(pathId: string): Observable<number> {
    return this.getPath(pathId).pipe(
      map(path => path.steps.length)
    );
  }

  /**
   * Calculate completion percentage for a path.
   */
  getCompletionPercentage(pathId: string): Observable<number> {
    return forkJoin({
      path: this.getPath(pathId),
      progress: this.agentService.getProgressForPath(pathId)
    }).pipe(
      map(({ path, progress }) => {
        if (!progress || path.steps.length === 0) {
          return 0;
        }

        // Only count non-optional steps for completion percentage
        const requiredSteps = path.steps.filter(s => !s.optional);
        const completedRequired = requiredSteps.filter(
          (s, i) => progress.completedStepIndices.includes(path.steps.indexOf(s))
        );

        if (requiredSteps.length === 0) {
          return 100;
        }

        return Math.round((completedRequired.length / requiredSteps.length) * 100);
      })
    );
  }

  /**
   * Check if a path is fully completed.
   */
  isPathCompleted(pathId: string): Observable<boolean> {
    return this.getCompletionPercentage(pathId).pipe(
      map(percentage => percentage === 100)
    );
  }

  // =========================================================================
  // CROSS-PATH COMPLETION TRACKING (Khan Academy-style)
  // =========================================================================

  /**
   * Get path completion based on unique content, not just steps.
   *
   * This enables Khan Academy-style shared completion:
   * - If "1st Grade Math" is 100% complete (80 unique content nodes)
   * - And "Early Math Review" has 40 unique content nodes (20 shared)
   * - Then "Early Math Review" shows ~50% completion (20 shared / 40 total)
   *
   * Returns both traditional step-based and content-based completion metrics.
   */
  getPathCompletionByContent(
    pathId: string,
    agentId?: string
  ): Observable<{
    totalSteps: number;
    completedSteps: number;
    totalUniqueContent: number;
    completedUniqueContent: number;
    contentCompletionPercentage: number;
    stepCompletionPercentage: number;
    sharedContentCompleted: number; // Completed via other paths
  }> {
    return forkJoin({
      path: this.getPath(pathId),
      progress: this.agentService.getProgressForPath(pathId),
      completedContentIds: this.agentService.getCompletedContentIds(agentId)
    }).pipe(
      map(({ path, progress, completedContentIds }) => {
        const totalSteps = path.steps.length;
        const completedSteps = progress?.completedStepIndices.length ?? 0;

        // Extract unique content IDs from this path
        const pathContentIds = new Set(
          path.steps.map(step => step.resourceId)
        );
        const totalUniqueContent = pathContentIds.size;

        // Calculate how many unique content nodes are completed
        let completedUniqueContent = 0;
        let sharedContentCompleted = 0;

        for (const contentId of pathContentIds) {
          if (completedContentIds.has(contentId)) {
            completedUniqueContent++;

            // Check if completed in THIS path vs other paths
            const completedInThisPath = progress?.completedStepIndices.some(
              stepIndex => path.steps[stepIndex]?.resourceId === contentId
            );
            if (!completedInThisPath) {
              sharedContentCompleted++;
            }
          }
        }

        const contentCompletionPercentage = totalUniqueContent > 0
          ? Math.round((completedUniqueContent / totalUniqueContent) * 100)
          : 0;

        const stepCompletionPercentage = totalSteps > 0
          ? Math.round((completedSteps / totalSteps) * 100)
          : 0;

        return {
          totalSteps,
          completedSteps,
          totalUniqueContent,
          completedUniqueContent,
          contentCompletionPercentage,
          stepCompletionPercentage,
          sharedContentCompleted
        };
      })
    );
  }

  /**
   * Get a step with global completion status.
   *
   * Unlike composeStepView (which only checks THIS path's progress),
   * this checks if the content has been completed in ANY path.
   *
   * Useful for showing "Already mastered in X" badges in UI.
   */
  getStepWithCompletionStatus(
    pathId: string,
    stepIndex: number,
    agentId?: string
  ): Observable<PathStepView & {
    isCompletedGlobally: boolean;
    completedInOtherPath: boolean;
  }> {
    return this.getPath(pathId).pipe(
      switchMap(path => {
        const resourceId = path.steps[stepIndex].resourceId;

        return forkJoin({
          stepView: this.getPathStep(pathId, stepIndex),
          isCompletedGlobally: this.agentService.isContentCompleted(resourceId, agentId)
        }).pipe(
          map(({ stepView, isCompletedGlobally }) => {
            const completedInThisPath = stepView.isCompleted;
            const completedInOtherPath = isCompletedGlobally && !completedInThisPath;

            return {
              ...stepView,
              isCompletedGlobally,
              completedInOtherPath
            };
          })
        );
      })
    );
  }

  /**
   * Get all steps for a path with global completion status.
   *
   * Batch version for performance - loads all steps at once.
   * Use this for path overview pages that show all steps.
   */
  getAllStepsWithCompletionStatus(
    pathId: string,
    agentId?: string
  ): Observable<Array<PathStepView & {
    isCompletedGlobally: boolean;
    completedInOtherPath: boolean;
  }>> {
    return forkJoin({
      path: this.getPath(pathId),
      progress: this.agentService.getProgressForPath(pathId),
      completedContentIds: this.agentService.getCompletedContentIds(agentId)
    }).pipe(
      switchMap(({ path, progress, completedContentIds }) => {
        // Load all content nodes for this path
        const contentLoads = path.steps.map(step =>
          this.dataLoader.getContent(step.resourceId)
        );

        return forkJoin(contentLoads).pipe(
          map(contentNodes => {
            return path.steps.map((step, index) => {
              const content = contentNodes[index];
              const isCompletedInThisPath = progress?.completedStepIndices.includes(index) ?? false;
              const isCompletedGlobally = completedContentIds.has(step.resourceId);
              const completedInOtherPath = isCompletedGlobally && !isCompletedInThisPath;

              const baseView = this.composeStepView(path, step, index, content, progress);

              return {
                ...baseView,
                isCompletedGlobally,
                completedInOtherPath
              };
            });
          })
        );
      })
    );
  }

  /**
   * Get concept mastery progress for a path.
   *
   * Aggregates progress by unique concepts found in the path steps.
   * Concepts are identified by the `sharedConcepts` field on steps.
   */
  getConceptProgressForPath(pathId: string): Observable<Array<{
    conceptId: string;
    title: string;
    totalSteps: number;
    completedSteps: number;
    completionPercentage: number;
  }>> {
    return forkJoin({
      path: this.getPath(pathId),
      progress: this.agentService.getProgressForPath(pathId)
    }).pipe(
      switchMap(({ path, progress }) => {
        // Map to store concept ID -> IDs of steps that teach this concept
        const conceptMap = new Map<string, number[]>();
        
        // 1. Collect all concepts and which steps they appear in
        path.steps.forEach((step: any, stepIndex) => {
          if (step.sharedConcepts && Array.isArray(step.sharedConcepts)) {
            step.sharedConcepts.forEach((conceptId: string) => {
              if (!conceptMap.has(conceptId)) {
                conceptMap.set(conceptId, []);
              }
              conceptMap.get(conceptId)?.push(stepIndex);
            });
          }
        });

        const uniqueConceptIds = Array.from(conceptMap.keys());
        
        if (uniqueConceptIds.length === 0) {
          return of([]);
        }

        // 2. Load content nodes for concepts to get titles
        const contentObservables = uniqueConceptIds.map(id =>
          this.dataLoader.getContent(id).pipe(
            // Handle error gracefully if a concept node is missing
            map(node => ({ id, title: node?.title ?? id }))
          )
        );

        return forkJoin(contentObservables).pipe(
          map(conceptNodes => {
            // 3. Aggregate progress
            return conceptNodes.map(node => {
              const stepIndices = conceptMap.get(node.id) ?? [];
              const totalSteps = stepIndices.length;
              
              // Calculate completed steps for this concept
              let completedSteps = 0;
              if (progress) {
                completedSteps = stepIndices.filter(idx => 
                  progress.completedStepIndices.includes(idx)
                ).length;
              }

              const completionPercentage = totalSteps > 0 
                ? Math.round((completedSteps / totalSteps) * 100) 
                : 0;

              return {
                conceptId: node.id,
                title: node.title,
                totalSteps,
                completedSteps,
                completionPercentage
              };
            }).sort((a, b) => b.totalSteps - a.totalSteps); // Sort by prevalence (most common concepts first)
          })
        );
      })
    );
  }

  // =========================================================================
  // BULK LOADING & CHAPTER NAVIGATION
  // =========================================================================

  /**
   * Get multiple steps at once (bulk loading).
   *
   * Use for:
   * - Prefetching next N steps while user reads current step
   * - Loading entire chapter for offline reading
   * - Populating step list UI efficiently
   *
   * @param pathId The learning path ID
   * @param startIndex Start index (inclusive)
   * @param count Number of steps to load
   */
  getBulkSteps(
    pathId: string,
    startIndex: number,
    count: number
  ): Observable<PathStepView[]> {
    return forkJoin({
      path: this.getPath(pathId),
      progress: this.agentService.getProgressForPath(pathId)
    }).pipe(
      switchMap(({ path, progress }) => {
        // Validate range
        const endIndex = Math.min(startIndex + count, path.steps.length);
        if (startIndex < 0 || startIndex >= path.steps.length) {
          return of([]);
        }

        // Extract steps in range
        const stepsToLoad = path.steps.slice(startIndex, endIndex);

        // Load content for all steps in parallel
        const contentLoads = stepsToLoad.map(step =>
          this.dataLoader.getContent(step.resourceId)
        );

        return forkJoin(contentLoads).pipe(
          map(contentNodes => {
            return stepsToLoad.map((step, offsetIndex) => {
              const actualIndex = startIndex + offsetIndex;
              const content = contentNodes[offsetIndex];
              return this.composeStepView(path, step, actualIndex, content, progress);
            });
          })
        );
      })
    );
  }

  /**
   * Get the next N steps from current position.
   *
   * Useful for:
   * - Prefetching while user reads current step
   * - "What's coming next?" preview sections
   * - Preloading for smoother navigation
   *
   * @param pathId The learning path ID
   * @param currentIndex Current step index
   * @param n Number of steps to fetch (default: 3)
   */
  getNextNSteps(
    pathId: string,
    currentIndex: number,
    n: number = 3
  ): Observable<PathStepView[]> {
    return this.getBulkSteps(pathId, currentIndex + 1, n);
  }

  /**
   * Get all steps in a specific chapter.
   *
   * Only works for paths that use chapter structure.
   * Returns empty array if path has no chapters or chapter not found.
   *
   * @param pathId The learning path ID
   * @param chapterId The chapter ID
   */
  getChapterSteps(
    pathId: string,
    chapterId: string
  ): Observable<PathStepView[]> {
    return this.getPath(pathId).pipe(
      switchMap(path => {
        // Check if path uses chapters
        if (!path.chapters || path.chapters.length === 0) {
          return of([]);
        }

        // Find the chapter
        const chapter = path.chapters.find(ch => ch.id === chapterId);
        if (!chapter) {
          return of([]);
        }

        // Calculate absolute step indices for this chapter
        let absoluteStartIndex = 0;
        for (const ch of path.chapters) {
          if (ch.id === chapterId) {
            break;
          }
          absoluteStartIndex += ch.steps.length;
        }

        // Load all steps in this chapter
        return this.getBulkSteps(pathId, absoluteStartIndex, chapter.steps.length);
      })
    );
  }

  /**
   * Get the next chapter in a path.
   *
   * Returns null if:
   * - Path has no chapters
   * - Current chapter is the last one
   * - Current chapter not found
   *
   * @param pathId The learning path ID
   * @param currentChapterId The current chapter ID
   */
  getNextChapter(
    pathId: string,
    currentChapterId: string
  ): Observable<PathStepView[] | null> {
    return this.getPath(pathId).pipe(
      switchMap(path => {
        // Check if path uses chapters
        if (!path.chapters || path.chapters.length === 0) {
          return of(null);
        }

        // Find current chapter index
        const currentChapterIndex = path.chapters.findIndex(
          ch => ch.id === currentChapterId
        );

        if (currentChapterIndex === -1) {
          return of(null);
        }

        // Check if there's a next chapter
        if (currentChapterIndex >= path.chapters.length - 1) {
          return of(null);
        }

        // Get next chapter
        const nextChapter = path.chapters[currentChapterIndex + 1];
        return this.getChapterSteps(pathId, nextChapter.id);
      })
    );
  }

  /**
   * Get the first step of a chapter.
   *
   * Useful for "Start Chapter" buttons in UI.
   *
   * @param pathId The learning path ID
   * @param chapterId The chapter ID
   */
  getChapterFirstStep(
    pathId: string,
    chapterId: string
  ): Observable<PathStepView | null> {
    return this.getChapterSteps(pathId, chapterId).pipe(
      map(steps => steps.length > 0 ? steps[0] : null)
    );
  }

  /**
   * Get chapter summaries for path overview.
   *
   * Includes progress information if user has started the path.
   * Returns STEP-based completion (legacy behavior).
   *
   * @param pathId The learning path ID
   */
  getChapterSummaries(pathId: string): Observable<Array<{
    chapter: PathChapter;
    completedSteps: number;
    totalSteps: number;
    isComplete: boolean;
    completionPercentage: number;
  }>> {
    return forkJoin({
      path: this.getPath(pathId),
      progress: this.agentService.getProgressForPath(pathId)
    }).pipe(
      map(({ path, progress }) => {
        // Path must use chapters
        if (!path.chapters || path.chapters.length === 0) {
          return [];
        }

        let absoluteStepIndex = 0;
        return path.chapters.map(chapter => {
          const totalSteps = chapter.steps.length;

          // Calculate completed steps in this chapter
          let completedSteps = 0;
          if (progress) {
            for (let i = 0; i < totalSteps; i++) {
              const stepIndex = absoluteStepIndex + i;
              if (progress.completedStepIndices.includes(stepIndex)) {
                completedSteps++;
              }
            }
          }

          absoluteStepIndex += totalSteps;

          const isComplete = completedSteps === totalSteps;
          const completionPercentage = totalSteps > 0
            ? Math.round((completedSteps / totalSteps) * 100)
            : 0;

          return {
            chapter,
            completedSteps,
            totalSteps,
            isComplete,
            completionPercentage
          };
        });
      })
    );
  }

  /**
   * Get chapter summaries with CONTENT-based completion (Khan Academy-style).
   *
   * Shows unique content mastery at the chapter level, including shared content
   * completed in other paths. This enables "at-a-glance" visibility of which
   * scaffolding concepts are already mastered.
   *
   * Use this for chapter overview pages to show shared content completion.
   *
   * @param pathId The learning path ID
   * @param agentId Optional agent ID (defaults to current agent)
   */
  getChapterSummariesWithContent(
    pathId: string,
    agentId?: string
  ): Observable<Array<{
    chapter: PathChapter;
    // Step-based metrics (traditional)
    completedSteps: number;
    totalSteps: number;
    stepCompletionPercentage: number;
    // Content-based metrics (Khan Academy-style)
    totalUniqueContent: number;
    completedUniqueContent: number;
    contentCompletionPercentage: number;
    sharedContentCompleted: number;  // Completed via other paths
    isComplete: boolean;
  }>> {
    return forkJoin({
      path: this.getPath(pathId),
      progress: this.agentService.getProgressForPath(pathId),
      completedContentIds: this.agentService.getCompletedContentIds(agentId)
    }).pipe(
      map(({ path, progress, completedContentIds }) => {
        // Path must use chapters
        if (!path.chapters || path.chapters.length === 0) {
          return [];
        }

        let absoluteStepIndex = 0;
        return path.chapters.map(chapter => {
          const totalSteps = chapter.steps.length;

          // Extract unique content IDs from this chapter
          const chapterContentIds = new Set(
            chapter.steps.map(step => step.resourceId)
          );
          const totalUniqueContent = chapterContentIds.size;

          // Calculate step-based completion
          let completedSteps = 0;
          if (progress) {
            for (let i = 0; i < totalSteps; i++) {
              const stepIndex = absoluteStepIndex + i;
              if (progress.completedStepIndices.includes(stepIndex)) {
                completedSteps++;
              }
            }
          }

          // Calculate content-based completion
          let completedUniqueContent = 0;
          let sharedContentCompleted = 0;

          for (const contentId of chapterContentIds) {
            if (completedContentIds.has(contentId)) {
              completedUniqueContent++;

              // Check if completed in THIS chapter vs other paths/chapters
              let completedInThisChapter = false;
              if (progress) {
                for (let i = 0; i < totalSteps; i++) {
                  const stepIndex = absoluteStepIndex + i;
                  if (
                    progress.completedStepIndices.includes(stepIndex) &&
                    chapter.steps[i]?.resourceId === contentId
                  ) {
                    completedInThisChapter = true;
                    break;
                  }
                }
              }

              if (!completedInThisChapter) {
                sharedContentCompleted++;
              }
            }
          }

          absoluteStepIndex += totalSteps;

          const stepCompletionPercentage = totalSteps > 0
            ? Math.round((completedSteps / totalSteps) * 100)
            : 0;

          const contentCompletionPercentage = totalUniqueContent > 0
            ? Math.round((completedUniqueContent / totalUniqueContent) * 100)
            : 0;

          // Chapter is complete when all unique content is mastered
          const isComplete = completedUniqueContent === totalUniqueContent;

          return {
            chapter,
            completedSteps,
            totalSteps,
            stepCompletionPercentage,
            totalUniqueContent,
            completedUniqueContent,
            contentCompletionPercentage,
            sharedContentCompleted,
            isComplete
          };
        });
      })
    );
  }
}
