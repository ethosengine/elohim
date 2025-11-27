import { Injectable } from '@angular/core';
import { Observable, forkJoin } from 'rxjs';
import { map, switchMap } from 'rxjs/operators';
import { DataLoaderService } from './data-loader.service';
import { AgentService } from './agent.service';
import { LearningPath, PathStep, PathStepView, PathIndex } from '../models/learning-path.model';
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
    private dataLoader: DataLoaderService,
    private agentService: AgentService
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
    } else {
      // No progress: can only access step 0
      if (stepIndex > 0) {
        return {
          accessible: false,
          reason: 'Start from the beginning'
        };
      }
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
}
