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
  private agentSubject = new BehaviorSubject<Agent | null>(null);
  agent$ = this.agentSubject.asObservable();

  // Progress cache (keyed by pathId)
  private progressCache = new Map<string, AgentProgress>();

  // Attestations set
  private attestations = new Set<string>();

  constructor(
    private dataLoader: DataLoaderService,
    @Optional() private sessionHumanService: SessionHumanService | null
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
    return this.agentSubject.value?.id || 'anonymous';
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
   */
  completeStep(pathId: string, stepIndex: number): Observable<void> {
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

        const current = existingProgress.stepAffinity[stepIndex] || 0;
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
}
