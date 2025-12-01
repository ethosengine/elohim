import { Injectable } from '@angular/core';
import { Observable, of, throwError, BehaviorSubject } from 'rxjs';
import { map, switchMap, tap } from 'rxjs/operators';

import { PathService } from './path.service';
import { PathStep, LearningPath } from '../models/learning-path.model';
import {
  PathExtension,
  PathStepInsertion,
  PathStepAnnotation,
  PathStepReorder,
  PathStepExclusion,
  PathExtensionIndex,
  PathExtensionIndexEntry,
  ApplyExtensionResult,
  ExtensionWarning,
  AnnotationType,
  ExclusionReason,
  UpstreamProposal,
  CollaborativePath,
  CollaborationType,
  CollaboratorRole,
  PathProposal,
  ProposedChange
} from '../models/path-extension.model';

/**
 * PathExtensionService - Learner-owned mutations to curated paths.
 *
 * Extensions allow personalization without fragmenting curation:
 * - Curators create canonical paths (immutable, versioned)
 * - Learners extend paths for their own use
 * - Extensions can be shared, forked, and merged upstream
 *
 * From the model documentation:
 * "Extensions are layered on top of canonical paths, allowing
 * personalization without modifying the original."
 *
 * Key operations:
 * - Create/manage personal extensions
 * - Apply extensions to resolve effective path
 * - Fork and share extensions
 * - Propose upstream merges
 * - Manage collaborative paths
 */
@Injectable({ providedIn: 'root' })
export class PathExtensionService {
  // In-memory storage (prototype - production uses Holochain)
  private readonly extensions: Map<string, PathExtension> = new Map();
  private readonly collaborativePaths: Map<string, CollaborativePath> = new Map();
  private extensionIndex: PathExtensionIndex | null = null;

  // Current agent's extensions
  private readonly myExtensionsSubject = new BehaviorSubject<PathExtensionIndexEntry[]>([]);
  public readonly myExtensions$ = this.myExtensionsSubject.asObservable();

  // Current agent ID
  private currentAgentId = 'demo-learner';

  constructor(private readonly pathService: PathService) {
    this.initializeDemoExtensions();
  }

  // =========================================================================
  // Extension Discovery
  // =========================================================================

  /**
   * Get all extensions visible to the current agent.
   */
  getExtensionIndex(): Observable<PathExtensionIndex> {
    if (this.extensionIndex) {
      return of(this.extensionIndex);
    }

    const entries: PathExtensionIndexEntry[] = [];

    this.extensions.forEach(ext => {
      if (this.canView(ext)) {
        entries.push(this.toIndexEntry(ext));
      }
    });

    const index: PathExtensionIndex = {
      lastUpdated: new Date().toISOString(),
      totalCount: entries.length,
      extensions: entries
    };

    this.extensionIndex = index;
    return of(index);
  }

  /**
   * Get extensions created by the current agent.
   */
  getMyExtensions(): Observable<PathExtensionIndexEntry[]> {
    return this.getExtensionIndex().pipe(
      map(index => index.extensions.filter(e => e.extendedBy === this.currentAgentId)),
      tap(exts => this.myExtensionsSubject.next(exts))
    );
  }

  /**
   * Get extensions for a specific base path.
   */
  getExtensionsForPath(pathId: string): Observable<PathExtensionIndexEntry[]> {
    return this.getExtensionIndex().pipe(
      map(index => index.extensions.filter(e => e.basePathId === pathId))
    );
  }

  /**
   * Get a specific extension.
   */
  getExtension(extensionId: string): Observable<PathExtension | null> {
    const ext = this.extensions.get(extensionId);

    if (!ext) {
      return of(null);
    }

    if (!this.canView(ext)) {
      return throwError(() => ({
        code: 'UNAUTHORIZED',
        message: 'You do not have permission to view this extension'
      }));
    }

    return of(ext);
  }

  // =========================================================================
  // Extension Creation
  // =========================================================================

  /**
   * Create a new extension for a path.
   */
  createExtension(params: {
    basePathId: string;
    title: string;
    description?: string;
    visibility?: PathExtension['visibility'];
  }): Observable<PathExtension> {
    return this.pathService.getPath(params.basePathId).pipe(
      switchMap(path => {
        if (!path) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Base path not found' }));
        }

        const extensionId = `ext-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`; // NOSONAR - Non-cryptographic extension ID generation
        const now = new Date().toISOString();

        const newExtension: PathExtension = {
          id: extensionId,
          basePathId: params.basePathId,
          basePathVersion: path.version,
          extendedBy: this.currentAgentId,
          title: params.title,
          description: params.description,
          insertions: [],
          annotations: [],
          reorderings: [],
          exclusions: [],
          visibility: params.visibility || 'private',
          createdAt: now,
          updatedAt: now
        };

        this.extensions.set(extensionId, newExtension);
        this.invalidateIndex();

        return of(newExtension);
      })
    );
  }

  /**
   * Fork an existing extension.
   */
  forkExtension(extensionId: string, params?: {
    title?: string;
    description?: string;
  }): Observable<PathExtension> {
    return this.getExtension(extensionId).pipe(
      switchMap(ext => {
        if (!ext) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Extension not found' }));
        }

        const forkedId = `ext-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`; // NOSONAR - Non-cryptographic extension ID generation
        const now = new Date().toISOString();

        const forkedExtension: PathExtension = {
          ...ext,
          id: forkedId,
          extendedBy: this.currentAgentId,
          title: params?.title || `${ext.title} (fork)`,
          description: params?.description || ext.description,
          forkedFrom: extensionId,
          forks: [],
          upstreamProposal: undefined,
          stats: undefined,
          visibility: 'private',
          createdAt: now,
          updatedAt: now
        };

        // Update original's forks list
        ext.forks = ext.forks ?? [];
        ext.forks.push(forkedId);

        // Update stats
        if (ext.stats) {
          ext.stats.forkCount = (ext.stats.forkCount ?? 0) + 1;
        }

        this.extensions.set(forkedId, forkedExtension);
        this.invalidateIndex();

        return of(forkedExtension);
      })
    );
  }

  // =========================================================================
  // Extension Modifications
  // =========================================================================

  /**
   * Add a step insertion to an extension.
   */
  addInsertion(
    extensionId: string,
    afterStepIndex: number,
    steps: PathStep[],
    rationale?: string
  ): Observable<PathStepInsertion> {
    return this.getExtension(extensionId).pipe(
      switchMap(ext => {
        if (!ext) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Extension not found' }));
        }

        if (!this.canEdit(ext)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot edit this extension' }));
        }

        const insertion: PathStepInsertion = {
          id: `ins-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`, // NOSONAR - Non-cryptographic insertion ID generation
          afterStepIndex,
          steps,
          rationale,
          source: {
            type: 'self',
            confidence: 1.0
          }
        };

        ext.insertions.push(insertion);
        ext.updatedAt = new Date().toISOString();
        this.invalidateIndex();

        return of(insertion);
      })
    );
  }

  /**
   * Add an annotation to a step.
   */
  addAnnotation(
    extensionId: string,
    stepIndex: number,
    type: AnnotationType,
    content: string,
    options?: {
      additionalResources?: PathStepAnnotation['additionalResources'];
      personalDifficulty?: PathStepAnnotation['personalDifficulty'];
      actualTime?: string;
    }
  ): Observable<PathStepAnnotation> {
    return this.getExtension(extensionId).pipe(
      switchMap(ext => {
        if (!ext) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Extension not found' }));
        }

        if (!this.canEdit(ext)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot edit this extension' }));
        }

        const annotation: PathStepAnnotation = {
          id: `ann-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`, // NOSONAR - Non-cryptographic annotation ID generation
          stepIndex,
          type,
          content,
          additionalResources: options?.additionalResources,
          personalDifficulty: options?.personalDifficulty,
          actualTime: options?.actualTime,
          createdAt: new Date().toISOString()
        };

        ext.annotations.push(annotation);
        ext.updatedAt = new Date().toISOString();
        this.invalidateIndex();

        return of(annotation);
      })
    );
  }

  /**
   * Reorder a step in an extension.
   */
  addReorder(
    extensionId: string,
    fromIndex: number,
    toIndex: number,
    rationale?: string
  ): Observable<PathStepReorder> {
    return this.getExtension(extensionId).pipe(
      switchMap(ext => {
        if (!ext) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Extension not found' }));
        }

        if (!this.canEdit(ext)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot edit this extension' }));
        }

        const reorder: PathStepReorder = {
          id: `reo-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`, // NOSONAR - Non-cryptographic reorder ID generation
          fromIndex,
          toIndex,
          rationale
        };

        ext.reorderings.push(reorder);
        ext.updatedAt = new Date().toISOString();
        this.invalidateIndex();

        return of(reorder);
      })
    );
  }

  /**
   * Exclude a step from the path.
   */
  addExclusion(
    extensionId: string,
    stepIndex: number,
    reason: ExclusionReason,
    notes?: string
  ): Observable<PathStepExclusion> {
    return this.getExtension(extensionId).pipe(
      switchMap(ext => {
        if (!ext) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Extension not found' }));
        }

        if (!this.canEdit(ext)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot edit this extension' }));
        }

        const exclusion: PathStepExclusion = {
          id: `exc-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`, // NOSONAR - Non-cryptographic exclusion ID generation
          stepIndex,
          reason,
          notes
        };

        ext.exclusions.push(exclusion);
        ext.updatedAt = new Date().toISOString();
        this.invalidateIndex();

        return of(exclusion);
      })
    );
  }

  /**
   * Remove a modification from an extension.
   */
  removeModification(
    extensionId: string,
    modificationId: string
  ): Observable<void> {
    return this.getExtension(extensionId).pipe(
      switchMap(ext => {
        if (!ext) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Extension not found' }));
        }

        if (!this.canEdit(ext)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot edit this extension' }));
        }

        ext.insertions = ext.insertions.filter(i => i.id !== modificationId);
        ext.annotations = ext.annotations.filter(a => a.id !== modificationId);
        ext.reorderings = ext.reorderings.filter(r => r.id !== modificationId);
        ext.exclusions = ext.exclusions.filter(e => e.id !== modificationId);
        ext.updatedAt = new Date().toISOString();
        this.invalidateIndex();

        return of(undefined);
      })
    );
  }

  // =========================================================================
  // Apply Extension
  // =========================================================================

  /**
   * Apply an extension to its base path to get the effective steps.
   * This is the core operation for rendering an extended path.
   */
  applyExtension(extensionId: string): Observable<ApplyExtensionResult> {
    return this.getExtension(extensionId).pipe(
      switchMap(ext => {
        if (!ext) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Extension not found' }));
        }

        return this.pathService.getPath(ext.basePathId).pipe(
          map(path => {
            if (!path) {
              throw { code: 'NOT_FOUND', message: 'Base path not found' };
            }

            return this.resolveExtension(path, ext);
          })
        );
      })
    );
  }

  /**
   * Resolve an extension against its base path.
   */
  private resolveExtension(path: LearningPath, ext: PathExtension): ApplyExtensionResult {
    const warnings: ExtensionWarning[] = [];

    // Check version compatibility
    if (path.version !== ext.basePathVersion) {
      warnings.push({
        type: 'version-mismatch',
        message: `Extension targets version ${ext.basePathVersion}, but path is now ${path.version}`,
        affectedItems: [ext.id]
      });
    }

    // Start with base steps (deep copy)
    let effectiveSteps = JSON.parse(JSON.stringify(path.steps)) as PathStep[];
    const indexMapping = new Map<number, string>();

    // Initialize index mapping
    effectiveSteps.forEach((_, i) => indexMapping.set(i, `base-${i}`));

    // Apply exclusions first (mark steps to skip)
    const excludedIndices = new Set<number>();
    for (const exclusion of ext.exclusions) {
      if (exclusion.stepIndex < effectiveSteps.length) {
        excludedIndices.add(exclusion.stepIndex);
      } else {
        warnings.push({
          type: 'missing-step',
          message: `Exclusion references step ${exclusion.stepIndex} which doesn't exist`,
          affectedItems: [exclusion.id]
        });
      }
    }

    // Apply reorderings
    const reorderMap = new Map<number, number>();
    for (const reorder of ext.reorderings) {
      if (reorder.fromIndex < effectiveSteps.length) {
        reorderMap.set(reorder.fromIndex, reorder.toIndex);
      } else {
        warnings.push({
          type: 'missing-step',
          message: `Reorder references step ${reorder.fromIndex} which doesn't exist`,
          affectedItems: [reorder.id]
        });
      }
    }

    // Build reordered step list
    if (reorderMap.size > 0) {
      const reorderedSteps: PathStep[] = [];
      const reorderedMapping = new Map<number, string>();

      // Get steps in new order
      const usedIndices = new Set<number>();
      for (let i = 0; i < effectiveSteps.length; i++) {
        const targetIndex = reorderMap.get(i);
        if (targetIndex !== undefined) {
          // This step was reordered
          reorderedSteps[targetIndex] = effectiveSteps[i];
          reorderedMapping.set(targetIndex, `base-${i}`);
          usedIndices.add(targetIndex);
        }
      }

      // Fill in non-reordered steps
      let nextFreeIndex = 0;
      for (let i = 0; i < effectiveSteps.length; i++) {
        if (!reorderMap.has(i)) {
          while (usedIndices.has(nextFreeIndex)) nextFreeIndex++;
          reorderedSteps[nextFreeIndex] = effectiveSteps[i];
          reorderedMapping.set(nextFreeIndex, `base-${i}`);
          usedIndices.add(nextFreeIndex);
          nextFreeIndex++;
        }
      }

      effectiveSteps = reorderedSteps.filter(s => s !== undefined);
      reorderedMapping.forEach((v, k) => indexMapping.set(k, v));
    }

    // Apply insertions (sorted by position to handle correctly)
    const sortedInsertions = [...ext.insertions].sort((a, b) => a.afterStepIndex - b.afterStepIndex);
    let insertionOffset = 0;

    for (const insertion of sortedInsertions) {
      const insertAt = insertion.afterStepIndex + 1 + insertionOffset;

      if (insertAt <= effectiveSteps.length) {
        // Insert the steps
        effectiveSteps.splice(insertAt, 0, ...insertion.steps);

        // Update index mapping for inserted steps
        for (let i = 0; i < insertion.steps.length; i++) {
          indexMapping.set(insertAt + i, `insertion-${insertion.id}-${i}`);
        }

        // Shift subsequent mappings
        insertionOffset += insertion.steps.length;
      } else {
        warnings.push({
          type: 'missing-step',
          message: `Insertion after step ${insertion.afterStepIndex} is out of bounds`,
          affectedItems: [insertion.id]
        });
      }
    }

    // Remove excluded steps
    if (excludedIndices.size > 0) {
      effectiveSteps = effectiveSteps.filter((_, i) => {
        const baseRef = indexMapping.get(i);
        if (baseRef?.startsWith('base-')) {
          const baseIndex = parseInt(baseRef.split('-')[1], 10);
          return !excludedIndices.has(baseIndex);
        }
        return true;
      });
    }

    // Build annotations map
    const annotationsMap = new Map<number, PathStepAnnotation[]>();
    for (const annotation of ext.annotations) {
      // Find the effective index for this annotation's step
      for (const [effectiveIdx, baseRef] of indexMapping.entries()) {
        if (baseRef === `base-${annotation.stepIndex}`) {
          const existing = annotationsMap.get(effectiveIdx) ?? [];
          existing.push(annotation);
          annotationsMap.set(effectiveIdx, existing);
          break;
        }
      }
    }

    return {
      effectiveSteps,
      indexMapping,
      annotations: annotationsMap,
      warnings
    };
  }

  // =========================================================================
  // Upstream Proposals
  // =========================================================================

  /**
   * Submit extension modifications as upstream proposal to path maintainers.
   */
  submitUpstreamProposal(extensionId: string): Observable<UpstreamProposal> {
    return this.getExtension(extensionId).pipe(
      switchMap(ext => {
        if (!ext) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Extension not found' }));
        }

        if (!this.canEdit(ext)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot submit proposals for this extension' }));
        }

        const proposal: UpstreamProposal = {
          status: 'submitted',
          submittedAt: new Date().toISOString()
        };

        ext.upstreamProposal = proposal;
        ext.updatedAt = new Date().toISOString();
        this.invalidateIndex();

        // In production: notify path maintainers
        console.log(`Upstream proposal submitted for extension ${extensionId}`);

        return of(proposal);
      })
    );
  }

  // =========================================================================
  // Collaborative Paths
  // =========================================================================

  /**
   * Get collaborative path settings.
   */
  getCollaborativePath(pathId: string): Observable<CollaborativePath | null> {
    return of(this.collaborativePaths.get(pathId) || null);
  }

  /**
   * Make a path collaborative.
   */
  enableCollaboration(
    pathId: string,
    type: CollaborationType,
    settings?: Partial<CollaborativePath['settings']>
  ): Observable<CollaborativePath> {
    return this.pathService.getPath(pathId).pipe(
      switchMap(path => {
        if (!path) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Path not found' }));
        }

        // Only path owner can enable collaboration
        if (path.createdBy !== this.currentAgentId) {
          return throwError(() => ({
            code: 'UNAUTHORIZED',
            message: 'Only path owner can enable collaboration'
          }));
        }

        const collab: CollaborativePath = {
          pathId,
          collaborationType: type,
          roles: new Map([[this.currentAgentId, 'owner']]),
          pendingProposals: [],
          settings: {
            requireApproval: settings?.requireApproval ?? (type === 'review-required'),
            minApprovals: settings?.minApprovals,
            approvers: settings?.approvers || [this.currentAgentId],
            allowAnonymousSuggestions: settings?.allowAnonymousSuggestions ?? false,
            notifyOnChange: settings?.notifyOnChange ?? true
          },
          activityLog: [{
            id: `act-${Date.now()}`,
            type: 'member-joined',
            actorId: this.currentAgentId,
            details: { role: 'owner' },
            timestamp: new Date().toISOString()
          }]
        };

        this.collaborativePaths.set(pathId, collab);

        return of(collab);
      })
    );
  }

  /**
   * Add a collaborator to a path.
   */
  addCollaborator(
    pathId: string,
    agentId: string,
    role: CollaboratorRole
  ): Observable<void> {
    return this.getCollaborativePath(pathId).pipe(
      switchMap(collab => {
        if (!collab) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Collaborative path not found' }));
        }

        const currentRole = collab.roles.get(this.currentAgentId);
        if (!currentRole || (currentRole !== 'owner' && currentRole !== 'editor')) {
          return throwError(() => ({
            code: 'UNAUTHORIZED',
            message: 'Only owners and editors can add collaborators'
          }));
        }

        collab.roles.set(agentId, role);
        collab.activityLog.push({
          id: `act-${Date.now()}`,
          type: 'member-joined',
          actorId: agentId,
          details: { role, addedBy: this.currentAgentId },
          timestamp: new Date().toISOString()
        });

        return of(undefined);
      })
    );
  }

  /**
   * Submit a proposal to a collaborative path.
   */
  submitProposal(
    pathId: string,
    changeType: PathProposal['changeType'],
    change: ProposedChange,
    rationale: string
  ): Observable<PathProposal> {
    return this.getCollaborativePath(pathId).pipe(
      switchMap(collab => {
        if (!collab) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Collaborative path not found' }));
        }

        const role = collab.roles.get(this.currentAgentId);
        if (!role && !collab.settings.allowAnonymousSuggestions) {
          return throwError(() => ({
            code: 'UNAUTHORIZED',
            message: 'You must be a collaborator to submit proposals'
          }));
        }

        const proposal: PathProposal = {
          id: `prop-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`, // NOSONAR - Non-cryptographic proposal ID generation
          proposedBy: this.currentAgentId,
          changeType,
          change,
          rationale,
          status: 'pending',
          votes: new Map(),
          comments: [],
          createdAt: new Date().toISOString()
        };

        collab.pendingProposals.push(proposal);
        collab.activityLog.push({
          id: `act-${Date.now()}`,
          type: 'proposal-created',
          actorId: this.currentAgentId,
          details: { proposalId: proposal.id, changeType },
          timestamp: new Date().toISOString()
        });

        return of(proposal);
      })
    );
  }

  // =========================================================================
  // Helper Methods
  // =========================================================================

  /**
   * Check if current agent can view an extension.
   */
  private canView(ext: PathExtension): boolean {
    if (ext.visibility === 'public') return true;
    if (ext.extendedBy === this.currentAgentId) return true;
    if (ext.visibility === 'shared' && ext.sharedWith?.includes(this.currentAgentId)) return true;
    return false;
  }

  /**
   * Check if current agent can edit an extension.
   */
  private canEdit(ext: PathExtension): boolean {
    return ext.extendedBy === this.currentAgentId;
  }

  /**
   * Convert extension to index entry.
   */
  private toIndexEntry(ext: PathExtension): PathExtensionIndexEntry {
    return {
      id: ext.id,
      basePathId: ext.basePathId,
      basePathTitle: ext.basePathId, // In production: resolve actual title
      title: ext.title,
      description: ext.description,
      extendedBy: ext.extendedBy,
      extenderName: ext.extendedBy, // In production: resolve display name
      visibility: ext.visibility,
      insertionCount: ext.insertions.length,
      annotationCount: ext.annotations.length,
      forkCount: ext.forks?.length ?? 0,
      rating: ext.stats?.averageRating,
      updatedAt: ext.updatedAt
    };
  }

  /**
   * Invalidate cached index.
   */
  private invalidateIndex(): void {
    this.extensionIndex = null;
  }

  /**
   * Set current agent.
   */
  setCurrentAgent(agentId: string): void {
    this.currentAgentId = agentId;
  }

  /**
   * Initialize demo extensions.
   */
  private initializeDemoExtensions(): void {
    const now = new Date().toISOString();

    const demoExtension: PathExtension = {
      id: 'ext-demo-elohim-path',
      basePathId: 'elohim-protocol',
      basePathVersion: '1.0.0',
      extendedBy: 'demo-learner',
      title: 'My Elohim Protocol Notes',
      description: 'Personal annotations and additional resources for the Elohim Protocol path',
      insertions: [],
      annotations: [
        {
          id: 'ann-1',
          stepIndex: 0,
          type: 'insight',
          content: 'The constitutional approach reminds me of Rawlsian veil of ignorance.',
          createdAt: now
        },
        {
          id: 'ann-2',
          stepIndex: 1,
          type: 'question',
          content: 'How does this scale to millions of users?',
          createdAt: now
        }
      ],
      reorderings: [],
      exclusions: [],
      visibility: 'private',
      createdAt: now,
      updatedAt: now
    };

    this.extensions.set(demoExtension.id, demoExtension);
  }
}
