import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable, of, forkJoin } from 'rxjs';
import { catchError, map, shareReplay } from 'rxjs/operators';
import { LearningPath, PathIndex } from '../models/learning-path.model';
import { ContentNode, ContentGraph, ContentGraphMetadata } from '../models/content-node.model';
import { Agent, AgentProgress } from '../models/agent.model';
import { ContentAttestation } from '../models/content-attestation.model';
import { KnowledgeMapIndex, KnowledgeMap } from '../models/knowledge-map.model';
import { PathExtensionIndex, PathExtension } from '../models/path-extension.model';

// Assessment types (inline until models are expanded)
export interface AssessmentIndex {
  lastUpdated: string;
  totalCount: number;
  assessments: AssessmentIndexEntry[];
}

export interface AssessmentIndexEntry {
  id: string;
  title: string;
  domain: string;
  instrumentType: string;
  estimatedTime: string;
}

// Governance types (inline until models are expanded)
export interface GovernanceIndex {
  lastUpdated: string;
  challengeCount: number;
  proposalCount: number;
  precedentCount: number;
  discussionCount: number;
}

export interface ChallengeRecord {
  id: string;
  entityType: string;
  entityId: string;
  challenger: { agentId: string; displayName: string; standing: string };
  grounds: string;
  description: string;
  status: string;
  filedAt: string;
  slaDeadline?: string;
  assignedElohim?: string;
  resolution?: {
    outcome: string;
    reasoning: string;
    decidedBy: string;
    decidedAt: string;
  };
}

export interface ProposalRecord {
  id: string;
  title: string;
  proposalType: string;
  description: string;
  proposer: { agentId: string; displayName: string };
  status: string;
  phase: string;
  createdAt: string;
  votingConfig?: {
    mechanism: string;
    quorum: number;
    passageThreshold: number;
  };
  currentVotes?: Record<string, number>;
  outcome?: {
    decision: string;
    reasoning: string;
  };
}

export interface PrecedentRecord {
  id: string;
  title: string;
  summary: string;
  fullReasoning: string;
  binding: string;
  scope: { entityTypes: string[]; categories?: string[]; roles?: string[] };
  citations: number;
  status: string;
}

export interface DiscussionRecord {
  id: string;
  entityType: string;
  entityId: string;
  category: string;
  title: string;
  messages: Array<{
    id: string;
    authorId: string;
    authorName: string;
    content: string;
    createdAt: string;
  }>;
  status: string;
  messageCount: number;
}

export interface GovernanceStateRecord {
  entityType: string;
  entityId: string;
  status: string;
  statusBasis: {
    method: string;
    reasoning: string;
    deciderId: string;
    deciderType: string;
    decidedAt: string;
  };
  labels: Array<{ labelType: string; severity: string; appliedBy: string }>;
  activeChallenges: string[];
  lastUpdated: string;
}

/**
 * DataLoaderService - Loads data from JSON files (prototype) or Holochain (production).
 *
 * This service is the ONLY place that knows about the data source.
 * All other services depend on this abstraction.
 *
 * Holochain migration:
 * - Prototype: HttpClient fetches from /assets/lamad-data/
 * - Holochain: Replace with HolochainService calls to conductor
 *
 * File structure expected:
 * /assets/lamad-data/
 *   paths/
 *     index.json         <- PathIndex
 *     {pathId}.json      <- LearningPath
 *   content/
 *     index.json         <- ContentIndex (metadata only)
 *     {resourceId}.json  <- ContentNode
 *   agents/
 *     index.json         <- Agent profiles
 *     {agentId}.json     <- Agent profile
 *   progress/
 *     {agentId}/
 *       {pathId}.json    <- AgentProgress
 *   attestations/
 *     index.json         <- ContentAttestation records
 *   knowledge-maps/
 *     index.json         <- KnowledgeMapIndex
 *     {mapId}.json       <- KnowledgeMap
 *   extensions/
 *     index.json         <- PathExtensionIndex
 *     {extensionId}.json <- PathExtension
 */
@Injectable({ providedIn: 'root' })
export class DataLoaderService {
  private readonly basePath = '/assets/lamad-data';

  // Caches to prevent redundant HTTP calls (shareReplay pattern)
  private readonly pathCache = new Map<string, Observable<LearningPath>>();
  private readonly contentCache = new Map<string, Observable<ContentNode>>();
  private attestationCache$: Observable<ContentAttestation[]> | null = null;
  private readonly attestationsByContentCache = new Map<string, ContentAttestation[]>();
  private graphCache$: Observable<ContentGraph> | null = null;

  constructor(private readonly http: HttpClient) {}

  /**
   * Load a LearningPath by ID.
   * Does NOT load the content for each step (lazy loading).
   */
  getPath(pathId: string): Observable<LearningPath> {
    if (!this.pathCache.has(pathId)) {
      const request = this.http.get<LearningPath>(
        `${this.basePath}/paths/${pathId}.json`
      ).pipe(
        shareReplay(1),
        catchError(err => {
          console.error(`[DataLoaderService] Failed to load path: ${pathId}`, err);
          throw new Error(`Path not found: ${pathId}`);
        })
      );
      this.pathCache.set(pathId, request);
    }
    return this.pathCache.get(pathId)!;
  }

  /**
   * Load a ContentNode by ID.
   * This is the only way to get content - enforces lazy loading.
   */
  getContent(resourceId: string): Observable<ContentNode> {
    if (!this.contentCache.has(resourceId)) {
      const request = this.http.get<ContentNode>(
        `${this.basePath}/content/${resourceId}.json`
      ).pipe(
        shareReplay(1),
        catchError(err => {
          console.error(`[DataLoaderService] Failed to load content: ${resourceId}`, err);
          throw new Error(`Content not found: ${resourceId}`);
        })
      );
      this.contentCache.set(resourceId, request);
    }
    return this.contentCache.get(resourceId)!;
  }

  /**
   * Load the content index for search/discovery.
   * Returns metadata only, not full content.
   */
  getContentIndex(): Observable<any> {
    return this.http.get(`${this.basePath}/content/index.json`).pipe(
      catchError(err => {
        console.error('[DataLoaderService] Failed to load content index', err);
        return of({ nodes: [], lastUpdated: new Date().toISOString() });
      })
    );
  }

  /**
   * Load the path index for discovery.
   */
  getPathIndex(): Observable<PathIndex> {
    return this.http.get<PathIndex>(`${this.basePath}/paths/index.json`).pipe(
      catchError(err => {
        console.error('[DataLoaderService] Failed to load path index', err);
        return of({ paths: [], totalCount: 0, lastUpdated: new Date().toISOString() });
      })
    );
  }

  /**
   * Load agent profile.
   */
  getAgent(agentId: string): Observable<Agent | null> {
    return this.http.get<Agent>(`${this.basePath}/agents/${agentId}.json`).pipe(
      catchError(() => of(null))  // Not authenticated or profile missing
    );
  }

  /**
   * Load agent progress for a specific path.
   * In Holochain, this reads from the agent's private source chain.
   */
  getAgentProgress(agentId: string, pathId: string): Observable<AgentProgress | null> {
    return this.http.get<AgentProgress>(
      `${this.basePath}/progress/${agentId}/${pathId}.json`
    ).pipe(
      catchError(() => of(null))  // No progress yet is not an error
    );
  }

  /**
   * Save agent progress.
   * In prototype: Updates localStorage (JSON files are read-only).
   * In Holochain: Writes to private source chain.
   */
  saveAgentProgress(progress: AgentProgress): Observable<void> {
    const key = `lamad-progress-${progress.agentId}-${progress.pathId}`;
    try {
      localStorage.setItem(key, JSON.stringify(progress));
    } catch (err) {
      console.error('[DataLoaderService] Failed to save progress to localStorage', err);
    }
    return of(undefined);
  }

  /**
   * Load progress from localStorage (prototype fallback).
   */
  getLocalProgress(agentId: string, pathId: string): AgentProgress | null {
    const key = `lamad-progress-${agentId}-${pathId}`;
    const data = localStorage.getItem(key);
    if (data) {
      try {
        return JSON.parse(data) as AgentProgress;
      } catch {
        return null;
      }
    }
    return null;
  }

  /**
   * Clear all caches - useful for testing or after auth changes.
   */
  clearCache(): void {
    this.pathCache.clear();
    this.contentCache.clear();
    this.attestationCache$ = null;
    this.attestationsByContentCache.clear();
    this.graphCache$ = null;
  }

  // =========================================================================
  // Attestation Loading (Bidirectional Trust Model)
  // =========================================================================

  /**
   * Load all content attestations.
   * Returns the full attestation index.
   */
  getAttestations(): Observable<ContentAttestation[]> {
    if (!this.attestationCache$) {
      this.attestationCache$ = this.http.get<{ attestations: ContentAttestation[] }>(
        `${this.basePath}/attestations/index.json`
      ).pipe(
        map(response => response.attestations || []),
        shareReplay(1),
        catchError(err => {
          console.error('[DataLoaderService] Failed to load attestations', err);
          return of([]);
        })
      );
    }
    return this.attestationCache$;
  }

  /**
   * Get attestations for a specific content node.
   * Filters from the attestation index.
   */
  getAttestationsForContent(contentId: string): Observable<ContentAttestation[]> {
    // Check local cache first
    if (this.attestationsByContentCache.has(contentId)) {
      return of(this.attestationsByContentCache.get(contentId)!);
    }

    return this.getAttestations().pipe(
      map(attestations => {
        const filtered = attestations.filter(att => att.contentId === contentId);
        this.attestationsByContentCache.set(contentId, filtered);
        return filtered;
      })
    );
  }

  /**
   * Get all active attestations (not revoked or expired).
   */
  getActiveAttestations(): Observable<ContentAttestation[]> {
    return this.getAttestations().pipe(
      map(attestations => attestations.filter(att => att.status === 'active'))
    );
  }

  /**
   * Load the agent index (all known agents).
   */
  getAgentIndex(): Observable<{ agents: Agent[] }> {
    return this.http.get<{ agents: Agent[] }>(
      `${this.basePath}/agents/index.json`
    ).pipe(
      catchError(err => {
        console.error('[DataLoaderService] Failed to load agent index', err);
        return of({ agents: [] });
      })
    );
  }

  // =========================================================================
  // Knowledge Map Loading
  // =========================================================================

  /**
   * Load the knowledge map index.
   */
  getKnowledgeMapIndex(): Observable<KnowledgeMapIndex> {
    return this.http.get<KnowledgeMapIndex>(
      `${this.basePath}/knowledge-maps/index.json`
    ).pipe(
      catchError(err => {
        console.error('[DataLoaderService] Failed to load knowledge map index', err);
        return of({ maps: [], totalCount: 0, lastUpdated: new Date().toISOString() });
      })
    );
  }

  /**
   * Load a specific knowledge map.
   */
  getKnowledgeMap(mapId: string): Observable<KnowledgeMap | null> {
    return this.http.get<KnowledgeMap>(
      `${this.basePath}/knowledge-maps/${mapId}.json`
    ).pipe(
      catchError(err => {
        console.error(`[DataLoaderService] Failed to load knowledge map: ${mapId}`, err);
        return of(null);
      })
    );
  }

  // =========================================================================
  // Path Extension Loading
  // =========================================================================

  /**
   * Load the path extension index.
   */
  getPathExtensionIndex(): Observable<PathExtensionIndex> {
    return this.http.get<PathExtensionIndex>(
      `${this.basePath}/extensions/index.json`
    ).pipe(
      catchError(err => {
        console.error('[DataLoaderService] Failed to load extension index', err);
        return of({ extensions: [], totalCount: 0, lastUpdated: new Date().toISOString() });
      })
    );
  }

  /**
   * Load a specific path extension.
   */
  getPathExtension(extensionId: string): Observable<PathExtension | null> {
    return this.http.get<PathExtension>(
      `${this.basePath}/extensions/${extensionId}.json`
    ).pipe(
      catchError(err => {
        console.error(`[DataLoaderService] Failed to load extension: ${extensionId}`, err);
        return of(null);
      })
    );
  }

  /**
   * Get extensions for a specific base path.
   */
  getExtensionsForPath(pathId: string): Observable<PathExtension[]> {
    return this.getPathExtensionIndex().pipe(
      map(index => {
        // Filter by basePathId and return just the IDs
        return index.extensions.filter(e => e.basePathId === pathId);
      }),
      // For full extension data, we'd need to load each one
      // For now, just return the index entries cast appropriately
      map(entries => entries as unknown as PathExtension[])
    );
  }

  // =========================================================================
  // Graph Loading (for Exploration Service)
  // =========================================================================

  /**
   * Load the full content graph for exploration.
   * This builds the graph from the content index and relationships.
   *
   * Note: This is a heavier operation than lazy loading individual nodes.
   * Use only for graph exploration features.
   */
  getGraph(): Observable<ContentGraph> {
    if (!this.graphCache$) {
      this.graphCache$ = forkJoin({
        overview: this.http.get<ContentGraphMetadata>(`${this.basePath}/graph/overview.json`),
        index: this.getContentIndex(),
        relationships: this.http.get<{ relationships: Array<{ id: string; source: string; target: string; type: string }> }>(
          `${this.basePath}/graph/relationships.json`
        )
      }).pipe(
        map(({ overview, index, relationships }) => this.buildContentGraph(overview, index, relationships)),
        shareReplay(1),
        catchError(err => {
          console.error('[DataLoaderService] Failed to load graph', err);
          return of(this.createEmptyGraph());
        })
      );
    }
    return this.graphCache$;
  }

  /**
   * Build ContentGraph structure from raw data.
   */
  private buildContentGraph(
    metadata: ContentGraphMetadata,
    contentIndex: { nodes?: ContentNode[] },
    relationshipData: { relationships: Array<{ id: string; source: string; target: string; type: string }> }
  ): ContentGraph {
    const nodes = new Map<string, ContentNode>();
    const nodesByType = new Map<string, Set<string>>();
    const nodesByTag = new Map<string, Set<string>>();
    const nodesByCategory = new Map<string, Set<string>>();
    const adjacency = new Map<string, Set<string>>();
    const reverseAdjacency = new Map<string, Set<string>>();
    const relationshipsMap = new Map<string, any>();

    // Build nodes map from index
    for (const node of contentIndex.nodes || []) {
      nodes.set(node.id, node);

      // Track by type
      if (!nodesByType.has(node.contentType)) {
        nodesByType.set(node.contentType, new Set());
      }
      nodesByType.get(node.contentType)!.add(node.id);

      // Track by tags
      for (const tag of node.tags || []) {
        if (!nodesByTag.has(tag)) {
          nodesByTag.set(tag, new Set());
        }
        nodesByTag.get(tag)!.add(node.id);
      }

      // Track by category
      const category = (node.metadata as any)?.category || 'uncategorized';
      if (!nodesByCategory.has(category)) {
        nodesByCategory.set(category, new Set());
      }
      nodesByCategory.get(category)!.add(node.id);

      // Initialize adjacency sets
      adjacency.set(node.id, new Set());
      reverseAdjacency.set(node.id, new Set());
    }

    // Build relationships and adjacency
    for (const rel of relationshipData.relationships || []) {
      const relId = rel.id || `${rel.source}-${rel.target}`;
      relationshipsMap.set(relId, {
        id: relId,
        sourceId: rel.source,
        targetId: rel.target,
        type: rel.type
      });

      // Update adjacency lists
      if (adjacency.has(rel.source)) {
        adjacency.get(rel.source)!.add(rel.target);
      }
      if (reverseAdjacency.has(rel.target)) {
        reverseAdjacency.get(rel.target)!.add(rel.source);
      }

      // Also update node.relatedNodeIds for compatibility
      const sourceNode = nodes.get(rel.source);
      const targetNode = nodes.get(rel.target);
      if (sourceNode && !sourceNode.relatedNodeIds.includes(rel.target)) {
        sourceNode.relatedNodeIds.push(rel.target);
      }
      if (targetNode && !targetNode.relatedNodeIds.includes(rel.source)) {
        targetNode.relatedNodeIds.push(rel.source);
      }
    }

    return {
      nodes,
      relationships: relationshipsMap,
      nodesByType,
      nodesByTag,
      nodesByCategory,
      adjacency,
      reverseAdjacency,
      metadata
    };
  }

  /**
   * Create empty ContentGraph for error fallback.
   */
  private createEmptyGraph(): ContentGraph {
    return {
      nodes: new Map<string, ContentNode>(),
      relationships: new Map<string, any>(),
      nodesByType: new Map<string, Set<string>>(),
      nodesByTag: new Map<string, Set<string>>(),
      nodesByCategory: new Map<string, Set<string>>(),
      adjacency: new Map<string, Set<string>>(),
      reverseAdjacency: new Map<string, Set<string>>(),
      metadata: {
        nodeCount: 0,
        relationshipCount: 0,
        lastUpdated: new Date().toISOString(),
        version: '1.0.0'
      }
    };
  }

  // =========================================================================
  // Assessment Loading
  // =========================================================================

  /**
   * Load the assessment index.
   */
  getAssessmentIndex(): Observable<AssessmentIndex> {
    return this.http.get<AssessmentIndex>(
      `${this.basePath}/assessments/index.json`
    ).pipe(
      catchError(err => {
        console.error('[DataLoaderService] Failed to load assessment index', err);
        return of({ assessments: [], totalCount: 0, lastUpdated: new Date().toISOString() });
      })
    );
  }

  /**
   * Load a specific assessment instrument.
   * Assessments are also stored as content nodes, so this uses the content loader.
   */
  getAssessment(assessmentId: string): Observable<ContentNode | null> {
    return this.getContent(assessmentId).pipe(
      catchError(err => {
        console.error(`[DataLoaderService] Failed to load assessment: ${assessmentId}`, err);
        return of(null);
      })
    );
  }

  /**
   * Get assessments by domain (values, attachment, strengths, etc.).
   */
  getAssessmentsByDomain(domain: string): Observable<AssessmentIndexEntry[]> {
    return this.getAssessmentIndex().pipe(
      map(index => index.assessments.filter(a => a.domain === domain))
    );
  }

  // =========================================================================
  // Governance Loading
  // =========================================================================

  /**
   * Load the governance index (counts and metadata).
   */
  getGovernanceIndex(): Observable<GovernanceIndex> {
    return this.http.get<GovernanceIndex>(
      `${this.basePath}/governance/index.json`
    ).pipe(
      catchError(err => {
        console.error('[DataLoaderService] Failed to load governance index', err);
        return of({
          lastUpdated: new Date().toISOString(),
          challengeCount: 0,
          proposalCount: 0,
          precedentCount: 0,
          discussionCount: 0
        });
      })
    );
  }

  /**
   * Load all challenges.
   */
  getChallenges(): Observable<ChallengeRecord[]> {
    return this.http.get<{ challenges: ChallengeRecord[] }>(
      `${this.basePath}/governance/challenges.json`
    ).pipe(
      map(response => response.challenges || []),
      catchError(err => {
        console.error('[DataLoaderService] Failed to load challenges', err);
        return of([]);
      })
    );
  }

  /**
   * Get challenges for a specific entity.
   */
  getChallengesForEntity(entityType: string, entityId: string): Observable<ChallengeRecord[]> {
    return this.getChallenges().pipe(
      map(challenges => challenges.filter(
        c => c.entityType === entityType && c.entityId === entityId
      ))
    );
  }

  /**
   * Load all proposals.
   */
  getProposals(): Observable<ProposalRecord[]> {
    return this.http.get<{ proposals: ProposalRecord[] }>(
      `${this.basePath}/governance/proposals.json`
    ).pipe(
      map(response => response.proposals || []),
      catchError(err => {
        console.error('[DataLoaderService] Failed to load proposals', err);
        return of([]);
      })
    );
  }

  /**
   * Get proposals by status (voting, discussion, decided).
   */
  getProposalsByStatus(status: string): Observable<ProposalRecord[]> {
    return this.getProposals().pipe(
      map(proposals => proposals.filter(p => p.status === status))
    );
  }

  /**
   * Load all precedents.
   */
  getPrecedents(): Observable<PrecedentRecord[]> {
    return this.http.get<{ precedents: PrecedentRecord[] }>(
      `${this.basePath}/governance/precedents.json`
    ).pipe(
      map(response => response.precedents || []),
      catchError(err => {
        console.error('[DataLoaderService] Failed to load precedents', err);
        return of([]);
      })
    );
  }

  /**
   * Get precedents by binding level (constitutional, binding-network, binding-local, persuasive).
   */
  getPrecedentsByBinding(binding: string): Observable<PrecedentRecord[]> {
    return this.getPrecedents().pipe(
      map(precedents => precedents.filter(p => p.binding === binding))
    );
  }

  /**
   * Load all discussion threads.
   */
  getDiscussions(): Observable<DiscussionRecord[]> {
    return this.http.get<{ discussions: DiscussionRecord[] }>(
      `${this.basePath}/governance/discussions.json`
    ).pipe(
      map(response => response.discussions || []),
      catchError(err => {
        console.error('[DataLoaderService] Failed to load discussions', err);
        return of([]);
      })
    );
  }

  /**
   * Get discussions for a specific entity.
   */
  getDiscussionsForEntity(entityType: string, entityId: string): Observable<DiscussionRecord[]> {
    return this.getDiscussions().pipe(
      map(discussions => discussions.filter(
        d => d.entityType === entityType && d.entityId === entityId
      ))
    );
  }

  /**
   * Load governance state for a specific entity.
   */
  getGovernanceState(entityType: string, entityId: string): Observable<GovernanceStateRecord | null> {
    return this.http.get<GovernanceStateRecord>(
      `${this.basePath}/governance/state-${entityType}-${entityId}.json`
    ).pipe(
      catchError(err => {
        // Not all entities have governance state files - that's expected
        return of(null);
      })
    );
  }
}
