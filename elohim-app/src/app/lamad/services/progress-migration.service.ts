import { Injectable } from '@angular/core';
import { Observable, forkJoin, of } from 'rxjs';
import { map, switchMap, catchError } from 'rxjs/operators';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { AgentProgress } from '../models';

/**
 * ProgressMigrationService - Migrates existing progress data to support cross-path completion.
 *
 * Background:
 * - Phase 1: Progress tracked per-path with completedStepIndices
 * - Phase 2: Added global content completion tracking (Khan Academy-style)
 *
 * This migration:
 * 1. Scans all existing progress records in localStorage
 * 2. Extracts resourceIds from completed steps
 * 3. Populates __global__ progress record with completedContentIds
 *
 * Run this ONCE after deploying the cross-path completion feature.
 */
@Injectable({ providedIn: 'root' })
export class ProgressMigrationService {
  constructor(private readonly dataLoader: DataLoaderService) {}

  /**
   * Migrate all progress records for all agents in localStorage.
   *
   * Returns statistics about the migration:
   * - agentsMigrated: Number of unique agents processed
   * - pathsMigrated: Number of path progress records processed
   * - contentNodesMigrated: Number of unique content IDs added to global tracking
   * - errors: Any errors encountered during migration
   */
  migrateAllProgress(): Observable<{
    agentsMigrated: number;
    pathsMigrated: number;
    contentNodesMigrated: number;
    errors: string[];
  }> {
    const progressRecords = this.scanLocalStorageProgress();
    const errors: string[] = [];

    if (progressRecords.length === 0) {
      return of({
        agentsMigrated: 0,
        pathsMigrated: 0,
        contentNodesMigrated: 0,
        errors: ['No progress records found in localStorage']
      });
    }

    // Group by agent ID
    const agentGroups = new Map<string, AgentProgress[]>();
    for (const progress of progressRecords) {
      if (!agentGroups.has(progress.agentId)) {
        agentGroups.set(progress.agentId, []);
      }
      agentGroups.get(progress.agentId)!.push(progress);
    }

    // Migrate each agent's progress
    const migrations = Array.from(agentGroups.entries()).map(([agentId, progressList]) =>
      this.migrateAgentProgress(agentId, progressList).pipe(
        catchError(error => {
          errors.push(`Error migrating agent ${agentId}: ${error.message}`);
          return of({ contentNodesMigrated: 0, pathsMigrated: 0 });
        })
      )
    );

    return forkJoin(migrations).pipe(
      map(results => {
        const totalContentNodes = results.reduce((sum, r) => sum + r.contentNodesMigrated, 0);
        const totalPaths = results.reduce((sum, r) => sum + r.pathsMigrated, 0);

        return {
          agentsMigrated: agentGroups.size,
          pathsMigrated: totalPaths,
          contentNodesMigrated: totalContentNodes,
          errors
        };
      })
    );
  }

  /**
   * Migrate progress for a single agent.
   */
  private migrateAgentProgress(
    agentId: string,
    progressList: AgentProgress[]
  ): Observable<{ contentNodesMigrated: number; pathsMigrated: number }> {
    // Filter out __global__ progress (skip if already exists)
    const pathProgressList = progressList.filter(p => p.pathId !== '__global__');

    if (pathProgressList.length === 0) {
      return of({ contentNodesMigrated: 0, pathsMigrated: 0 });
    }

    // Load path metadata for each progress record to get resourceIds
    const pathLoads = pathProgressList.map(progress =>
      this.dataLoader.getPath(progress.pathId).pipe(
        map(path => ({ progress, path })),
        catchError(error => {
          console.warn(`Could not load path ${progress.pathId}:`, error);
          return of(null);
        })
      )
    );

    return forkJoin(pathLoads).pipe(
      switchMap(results => {
        // Extract all completed content IDs
        const completedContentIds = new Set<string>();

        for (const result of results) {
          if (!result) continue;

          const { progress, path } = result;

          // For each completed step, get its resourceId
          for (const stepIndex of progress.completedStepIndices) {
            if (stepIndex >= 0 && stepIndex < path.steps.length) {
              const resourceId = path.steps[stepIndex].resourceId;
              completedContentIds.add(resourceId);
            }
          }
        }

        // Create or update __global__ progress record
        return this.createGlobalProgress(
          agentId,
          Array.from(completedContentIds)
        ).pipe(
          map(() => ({
            contentNodesMigrated: completedContentIds.size,
            pathsMigrated: pathProgressList.length
          }))
        );
      })
    );
  }

  /**
   * Create or update the __global__ progress record with completed content IDs.
   */
  private createGlobalProgress(
    agentId: string,
    contentIds: string[]
  ): Observable<void> {
    // Check if __global__ progress already exists
    const existingProgress = this.dataLoader.getLocalProgress(agentId, '__global__');

    const now = new Date().toISOString();

    const globalProgress: AgentProgress = existingProgress ?? {
      agentId,
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

    // Merge content IDs (avoid duplicates)
    const existingIds = new Set(globalProgress.completedContentIds ?? []);
    for (const id of contentIds) {
      existingIds.add(id);
    }

    globalProgress.completedContentIds = Array.from(existingIds);
    globalProgress.lastActivityAt = now;

    // Save to localStorage
    return this.dataLoader.saveAgentProgress(globalProgress);
  }

  /**
   * Scan localStorage for all progress records.
   * Returns array of AgentProgress objects.
   */
  private scanLocalStorageProgress(): AgentProgress[] {
    const progressRecords: AgentProgress[] = [];

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith('lamad-progress-')) {
        try {
          const data = localStorage.getItem(key);
          if (data) {
            const progress = JSON.parse(data) as AgentProgress;
            progressRecords.push(progress);
          }
        } catch (error) {
          console.warn(`Malformed progress data at key ${key}:`, error);
        }
      }
    }

    return progressRecords;
  }

  /**
   * Dry run - preview what would be migrated without making changes.
   *
   * Useful for debugging and verification before running the actual migration.
   */
  previewMigration(): Observable<{
    agents: Array<{
      agentId: string;
      pathCount: number;
      estimatedContentNodes: number;
    }>;
    totalAgents: number;
    totalPaths: number;
    estimatedContentNodes: number;
  }> {
    const progressRecords = this.scanLocalStorageProgress();

    // Group by agent ID
    const agentGroups = new Map<string, AgentProgress[]>();
    for (const progress of progressRecords) {
      if (progress.pathId === '__global__') continue; // Skip existing global records

      if (!agentGroups.has(progress.agentId)) {
        agentGroups.set(progress.agentId, []);
      }
      agentGroups.get(progress.agentId)!.push(progress);
    }

    const agents = Array.from(agentGroups.entries()).map(([agentId, progressList]) => {
      // Estimate unique content nodes (may have duplicates across paths)
      const estimatedContentNodes = progressList.reduce(
        (sum, p) => sum + p.completedStepIndices.length,
        0
      );

      return {
        agentId,
        pathCount: progressList.length,
        estimatedContentNodes
      };
    });

    const totalPaths = agents.reduce((sum, a) => sum + a.pathCount, 0);
    const estimatedContentNodes = agents.reduce((sum, a) => sum + a.estimatedContentNodes, 0);

    return of({
      agents,
      totalAgents: agents.length,
      totalPaths,
      estimatedContentNodes
    });
  }

  /**
   * Verify migration completed successfully.
   *
   * Checks that all agents with path progress also have __global__ progress.
   */
  verifyMigration(): Observable<{
    valid: boolean;
    agentsWithProgress: number;
    agentsWithGlobalProgress: number;
    missingGlobalProgress: string[];
  }> {
    const progressRecords = this.scanLocalStorageProgress();

    const agentsWithProgress = new Set<string>();
    const agentsWithGlobalProgress = new Set<string>();

    for (const progress of progressRecords) {
      agentsWithProgress.add(progress.agentId);

      if (progress.pathId === '__global__') {
        agentsWithGlobalProgress.add(progress.agentId);
      }
    }

    const missingGlobalProgress = Array.from(agentsWithProgress).filter(
      agentId => !agentsWithGlobalProgress.has(agentId)
    );

    return of({
      valid: missingGlobalProgress.length === 0,
      agentsWithProgress: agentsWithProgress.size,
      agentsWithGlobalProgress: agentsWithGlobalProgress.size,
      missingGlobalProgress
    });
  }
}
