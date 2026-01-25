/**
 * Holochain Content Service
 *
 * Provides Holochain zome call wrappers.
 * Uses the HolochainClientService for WebSocket communication.
 *
 * DEPRECATION NOTICE:
 * Content methods (getContent, batchGetContent, getPathIndex, etc.) are DEPRECATED.
 * Content is now served from doorway projection (SQLite), not DHT.
 * Use ContentService instead for all content operations.
 *
 * This service remains active for AGENT-CENTRIC DATA ONLY:
 * - Identity and agent profiles
 * - Attestations (trust claims about humans/content)
 * - Points and participation metrics
 * - Consent relationships
 * - Governance operations
 *
 * @see ContentService for content operations (paths, content nodes, search)
 * @see HolochainClientService for connection management
 */

import { Injectable, computed, signal, inject } from '@angular/core';

import { map, catchError, shareReplay, switchMap, tap, debounceTime, buffer } from 'rxjs/operators';

import { Observable, of, from, defer, Subject, BehaviorSubject } from 'rxjs';

import {
  ContentNode,
  ContentType,
  ContentFormat,
  ContentMetadata,
} from '../../lamad/models/content-node.model';

import { CustodianSelectionService } from './custodian-selection.service';
import { HolochainClientService } from './holochain-client.service';

// =============================================================================
// Holochain Content Types (match Rust DNA structures)
// =============================================================================

/**
 * Content entry as stored in Holochain
 * Matches Content struct in integrity zome (extended schema)
 */
export interface HolochainContentEntry {
  id: string;
  contentType: string;
  title: string;
  description: string;
  content: string;
  contentFormat: string;
  tags: string[];
  sourcePath: string | null;
  relatedNodeIds: string[];
  authorId: string | null;
  reach: string;
  trustScore: number;
  metadata: unknown; // Parsed JSON object from Rust API
  createdAt: string;
  updatedAt: string;
}

/**
 * Output from content retrieval zome calls
 */
export interface HolochainContentOutput {
  actionHash: Uint8Array;
  entryHash: Uint8Array;
  content: HolochainContentEntry;
}

/**
 * Content statistics from get_content_stats
 */
export interface HolochainContentStats {
  totalCount: number;
  byType: Record<string, number>;
}

/**
 * Query input for content by type
 *
 * TODO: [HOLOCHAIN-ZOME] Uses snake_case because this is a Holochain zome payload.
 * Zomes are Rust and expect snake_case field names. This cannot be changed without
 * updating the Rust zome and running a DNA migration.
 */
export interface QueryByTypeInput {
  content_type: string;
  limit?: number;
}

/**
 * Query input for content by ID
 */
export interface QueryByIdInput {
  id: string;
}

/**
 * Input for batch content retrieval
 */
export interface BatchGetContentInput {
  ids: string[];
}

/**
 * Output from batch content retrieval
 */
export interface BatchGetContentOutput {
  found: HolochainContentOutput[];
  notFound: string[];
}

/**
 * Input for paginated content query by type
 *
 * TODO: [HOLOCHAIN-ZOME] Uses snake_case because this is a Holochain zome payload.
 * Zomes are Rust and expect snake_case field names. This cannot be changed without
 * updating the Rust zome and running a DNA migration.
 */
export interface PaginatedByTypeInput {
  content_type: string;
  page_size: number;
  offset: number;
}

/**
 * Input for paginated content query by tag
 *
 * TODO: [HOLOCHAIN-ZOME] Uses snake_case because this is a Holochain zome payload.
 * Zomes are Rust and expect snake_case field names. This cannot be changed without
 * updating the Rust zome and running a DNA migration.
 */
export interface PaginatedByTagInput {
  tag: string;
  page_size: number;
  offset: number;
}

/**
 * Output for paginated content queries
 */
export interface PaginatedContentOutput {
  items: HolochainContentOutput[];
  totalCount: number;
  offset: number;
  hasMore: boolean;
}

// =============================================================================
// Holochain Learning Path Types (match Rust DNA structures)
// =============================================================================

/**
 * Path index entry from get_all_paths
 */
export interface HolochainPathIndexEntry {
  id: string;
  title: string;
  description: string;
  difficulty: string;
  estimatedDuration: string | null;
  stepCount: number;
  tags: string[];
}

/**
 * Path index output from get_all_paths
 */
export interface HolochainPathIndex {
  paths: HolochainPathIndexEntry[];
  totalCount: number;
  lastUpdated: string;
}

/**
 * Path step from Holochain
 */
export interface HolochainPathStep {
  id: string;
  pathId: string;
  orderIndex: number;
  stepType: string;
  resourceId: string;
  stepTitle: string | null;
  stepNarrative: string | null;
  isOptional: boolean;
}

/**
 * Learning path entry from Holochain
 */
export interface HolochainLearningPath {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose: string | null;
  createdBy: string;
  difficulty: string;
  estimatedDuration: string | null;
  visibility: string;
  pathType: string;
  tags: string[];
  /** Extensible metadata object (stores chapters for hierarchical paths) */
  metadata: unknown;
  createdAt: string;
  updatedAt: string;
}

/**
 * Path with steps output
 */
export interface HolochainPathWithSteps {
  actionHash: Uint8Array;
  path: HolochainLearningPath;
  steps: {
    actionHash: Uint8Array;
    step: HolochainPathStep;
  }[];
}

/**
 * Lightweight path overview (no step content, just counts)
 * Use for: path listings, path-overview page, initial navigation
 */
export interface HolochainPathOverview {
  actionHash: Uint8Array;
  path: HolochainLearningPath;
  stepCount: number;
}

// =============================================================================
// Holochain Agent Types (match Rust DNA structures)
// =============================================================================

/**
 * Agent entry as stored in Holochain
 * Matches Agent struct in integrity zome
 */
export interface HolochainAgentEntry {
  id: string;
  agentType: string; // human, organization, ai-agent, elohim
  displayName: string;
  bio: string | null;
  avatar: string | null;
  affinities: string[];
  visibility: string; // public, connections, private
  location: string | null;
  holochainAgentKey: string | null;
  did: string | null;
  activityPubType: string | null;
  createdAt: string;
  updatedAt: string;
}

/**
 * Output from agent retrieval zome calls
 */
export interface HolochainAgentOutput {
  action_hash: Uint8Array;
  agent: HolochainAgentEntry;
}

/**
 * Input for querying agents
 */
export interface QueryAgentsInput {
  agent_type?: string;
  affinities?: string[];
  limit?: number;
}

// =============================================================================
// Holochain Attestation Types (match Rust DNA structures)
// =============================================================================

/**
 * Attestation entry as stored in Holochain
 * Matches Attestation struct in integrity zome
 */
export interface HolochainAttestationEntry {
  id: string;
  agentId: string;
  category: string; // domain-mastery, path-completion, role-credential, achievement
  attestationType: string;
  displayName: string;
  description: string;
  iconUrl: string | null;
  tier: string | null; // bronze, silver, gold, platinum
  earnedVia: unknown;
  issuedAt: string;
  issuedBy: string;
  expiresAt: string | null;
  proof: string | null;
}

/**
 * Output from attestation retrieval zome calls
 */
export interface HolochainAttestationOutput {
  actionHash: Uint8Array;
  attestation: HolochainAttestationEntry;
}

/**
 * Input for querying attestations
 */
export interface QueryAttestationsInput {
  agentId?: string;
  category?: string;
  limit?: number;
}

// =============================================================================
// Holochain Content Attestation Types (Trust claims about content)
// =============================================================================

/**
 * ContentAttestation entry as stored in Holochain
 * Matches ContentAttestation struct in integrity zome
 *
 * Different from HolochainAttestationEntry which is for agents:
 * - HolochainAttestationEntry = credentials granted to AGENTS
 * - HolochainContentAttestationEntry = trust claims about CONTENT
 */
export interface HolochainContentAttestationEntry {
  id: string;
  contentId: string;
  attestationType: string; // author-verified, steward-approved, etc.
  reachGranted: string; // private, local, community, commons
  grantedBy: unknown; // AttestationGrantor parsed
  grantedAt: string;
  expiresAt: string | null;
  status: string; // active, expired, revoked, superseded
  revocation: unknown | null; // AttestationRevocation if revoked
  evidence: unknown | null; // AttestationEvidence
  scope: unknown | null; // AttestationScope (optional)
  metadata: unknown;
  createdAt: string;
  updatedAt: string;
  schemaVersion: number;
  validationStatus: string;
}

/**
 * Output from content attestation retrieval zome calls
 */
export interface HolochainContentAttestationOutput {
  actionHash: Uint8Array;
  entryHash: Uint8Array;
  contentAttestation: HolochainContentAttestationEntry;
}

/**
 * Input for creating a content attestation
 */
export interface CreateContentAttestationInput {
  id?: string;
  contentId: string;
  attestationType: string;
  reachGranted: string;
  grantedByJson: string;
  expiresAt?: string;
  evidenceJson?: string;
  scopeJson?: string;
  metadataJson?: string;
}

/**
 * Input for querying content attestations
 */
export interface QueryContentAttestationsInput {
  contentId?: string;
  attestationType?: string;
  reachGranted?: string;
  status?: string;
  limit?: number;
}

/**
 * Input for updating a content attestation
 */
export interface UpdateContentAttestationInput {
  id: string;
  status?: string;
  revocation_json?: string;
  metadata_json?: string;
}

/**
 * Input for revoking a content attestation
 */
export interface RevokeContentAttestationInput {
  id: string;
  revoked_by: string;
  reason: string;
  appealable: boolean;
}

// =============================================================================
// Holochain Relationship/Graph Types (match Rust DNA structures)
// =============================================================================

/**
 * Relationship entry as stored in Holochain
 * Matches Relationship struct in integrity zome
 */
export interface HolochainRelationshipEntry {
  id: string;
  sourceId: string;
  targetId: string;
  relationshipType: string; // RELATES_TO, CONTAINS, DEPENDS_ON, IMPLEMENTS, REFERENCES
  confidence: number; // 0.0 - 1.0
  inferenceSource: string; // explicit, path, tag, semantic
  metadata: unknown | null;
  createdAt: string;
}

/**
 * Output from relationship retrieval zome calls
 */
export interface HolochainRelationshipOutput {
  actionHash: Uint8Array;
  relationship: HolochainRelationshipEntry;
}

/**
 * Input for querying relationships
 *
 * TODO: [HOLOCHAIN-ZOME] Uses snake_case because this is a Holochain zome payload.
 * Zomes are Rust and expect snake_case field names. This cannot be changed without
 * updating the Rust zome and running a DNA migration.
 */
export interface GetRelationshipsInput {
  content_id: string; // snake_case for zome
  direction: string; // outgoing, incoming, both
}

/**
 * Input for querying related content / content graph
 */
export interface QueryRelatedContentInput {
  content_id: string;
  relationship_types?: string[];
  depth?: number;
}

/**
 * Content graph node for tree traversal
 */
export interface HolochainContentGraphNode {
  content: HolochainContentOutput;
  relationshipType: string;
  confidence: number;
  children: HolochainContentGraphNode[];
}

/**
 * Content graph output from get_content_graph
 */
export interface HolochainContentGraph {
  root: HolochainContentOutput | null;
  related: HolochainContentGraphNode[];
  totalNodes: number;
}

// =============================================================================
// Holochain KnowledgeMap Types (match Rust DNA structures)
// =============================================================================

/**
 * KnowledgeMap entry as stored in Holochain
 */
export interface HolochainKnowledgeMapEntry {
  id: string;
  mapType: string; // domain, self, person, collective
  ownerId: string;
  title: string;
  description: string | null;
  subjectType: string;
  subjectId: string;
  subjectName: string;
  visibility: string;
  sharedWith: unknown;
  nodes: unknown;
  pathIds: unknown;
  overallAffinity: number;
  contentGraphId: string | null;
  masteryLevels: unknown;
  goals: unknown;
  createdAt: string;
  updatedAt: string;
  metadata: unknown;
}

/**
 * Output from knowledge map retrieval
 */
export interface HolochainKnowledgeMapOutput {
  actionHash: Uint8Array;
  knowledgeMap: HolochainKnowledgeMapEntry;
}

/**
 * Input for querying knowledge maps
 */
export interface QueryKnowledgeMapsInput {
  ownerId?: string;
  mapType?: string;
  limit?: number;
}

// =============================================================================
// Holochain PathExtension Types (match Rust DNA structures)
// =============================================================================

/**
 * PathExtension entry as stored in Holochain
 */
export interface HolochainPathExtensionEntry {
  id: string;
  basePathId: string;
  basePathVersion: string;
  extendedBy: string;
  title: string;
  description: string | null;
  insertions: unknown;
  annotations: unknown;
  reorderings: unknown;
  exclusions: unknown;
  visibility: string;
  sharedWith: unknown;
  forkedFrom: string | null;
  forks: unknown;
  upstreamProposal: unknown | null;
  stats: unknown;
  createdAt: string;
  updatedAt: string;
}

/**
 * Output from path extension retrieval
 */
export interface HolochainPathExtensionOutput {
  actionHash: Uint8Array;
  pathExtension: HolochainPathExtensionEntry;
}

/**
 * Input for querying path extensions
 */
export interface QueryPathExtensionsInput {
  basePathId?: string;
  extendedBy?: string;
  limit?: number;
}

// =============================================================================
// Holochain Governance Types (match Rust DNA structures)
// =============================================================================

/**
 * Challenge entry as stored in Holochain
 */
export interface HolochainChallengeEntry {
  id: string;
  entityType: string;
  entityId: string;
  challengerId: string;
  challengerName: string;
  challengerStanding: string;
  grounds: string;
  description: string;
  evidence: unknown;
  status: string;
  filedAt: string;
  acknowledgedAt: string | null;
  slaDeadline: string | null;
  assignedElohim: string | null;
  priority: string;
  resolution: unknown | null;
  createdAt: string;
  updatedAt: string;
  metadata: unknown;
}

/**
 * Output from challenge retrieval
 */
export interface HolochainChallengeOutput {
  actionHash: Uint8Array;
  challenge: HolochainChallengeEntry;
}

/**
 * Input for querying challenges
 */
export interface QueryChallengesInput {
  entityType?: string;
  entityId?: string;
  challengerId?: string;
  status?: string;
  limit?: number;
}

/**
 * Proposal entry as stored in Holochain
 */
export interface HolochainProposalEntry {
  id: string;
  title: string;
  proposalType: string;
  description: string;
  proposerId: string;
  proposerName: string;
  rationale: string;
  status: string;
  phase: string;
  amendments: unknown;
  votingConfig: unknown;
  currentVotes: unknown;
  outcome: unknown | null;
  relatedEntityType: string | null;
  relatedEntityId: string | null;
  createdAt: string;
  updatedAt: string;
  metadata: unknown;
}

/**
 * Output from proposal retrieval
 */
export interface HolochainProposalOutput {
  action_hash: Uint8Array;
  proposal: HolochainProposalEntry;
}

/**
 * Input for querying proposals
 */
export interface QueryProposalsInput {
  proposal_type?: string;
  proposer_id?: string;
  status?: string;
  limit?: number;
}

/**
 * Precedent entry as stored in Holochain
 */
export interface HolochainPrecedentEntry {
  id: string;
  title: string;
  summary: string;
  fullReasoning: string;
  binding: string;
  scope: unknown;
  citations: number;
  status: string;
  establishedBy: string;
  establishedAt: string;
  supersededBy: string | null;
  createdAt: string;
  updatedAt: string;
  metadata: unknown;
}

/**
 * Output from precedent retrieval
 */
export interface HolochainPrecedentOutput {
  actionHash: Uint8Array;
  precedent: HolochainPrecedentEntry;
}

/**
 * Input for querying precedents
 */
export interface QueryPrecedentsInput {
  status?: string;
  binding?: string;
  limit?: number;
}

/**
 * Discussion entry as stored in Holochain
 */
export interface HolochainDiscussionEntry {
  id: string;
  entityType: string;
  entityId: string;
  category: string;
  title: string;
  messages: unknown;
  status: string;
  messageCount: number;
  lastActivityAt: string;
  createdAt: string;
  updatedAt: string;
  metadata: unknown;
}

/**
 * Output from discussion retrieval
 */
export interface HolochainDiscussionOutput {
  actionHash: Uint8Array;
  discussion: HolochainDiscussionEntry;
}

/**
 * Input for querying discussions
 */
export interface QueryDiscussionsInput {
  entityType?: string;
  entityId?: string;
  category?: string;
  status?: string;
  limit?: number;
}

/**
 * GovernanceState entry as stored in Holochain
 */
export interface HolochainGovernanceStateEntry {
  id: string;
  entityType: string;
  entityId: string;
  status: string;
  statusBasis: unknown;
  labels: unknown;
  activeChallenges: unknown;
  activeProposals: unknown;
  precedentIds: unknown;
  lastUpdated: string;
  createdAt: string;
  updatedAt: string;
  metadata: unknown;
}

/**
 * Output from governance state retrieval
 */
export interface HolochainGovernanceStateOutput {
  actionHash: Uint8Array;
  governanceState: HolochainGovernanceStateEntry;
}

/**
 * Input for getting governance state
 */
export interface GetGovernanceStateInput {
  entityType: string;
  entityId: string;
}

/**
 * Input for querying governance states
 */
export interface QueryGovernanceStatesInput {
  status?: string;
  limit?: number;
}

// =============================================================================
// Service Implementation
// =============================================================================

@Injectable({
  providedIn: 'root',
})
export class HolochainContentService {
  /**
   * Whether Holochain content service is available.
   *
   * Starts as false, set to true after successful testAvailability() call.
   * The HolochainClientService now properly discovers existing app interfaces
   * and authorizes signing credentials, enabling zome calls from the browser.
   */
  private readonly availableSignal = signal(false);
  readonly available = this.availableSignal.asReadonly();

  /** Computed: true when Holochain client is connected AND available */
  readonly ready = computed(() => this.available() && this.holochainClient.isConnected());

  /** Cache for content by ID */
  private readonly contentCache = new Map<string, Observable<ContentNode | null>>();

  /** Cache for stats (refreshed periodically) */
  private statsCache$: Observable<HolochainContentStats> | null = null;

  // Request coalescing for batch loading
  private readonly pendingBatchRequests = new Map<
    string,
    { resolve: (content: ContentNode | null) => void; reject: (err: any) => void }[]
  >();
  private batchRequestTimer: ReturnType<typeof setTimeout> | null = null;
  private readonly BATCH_DEBOUNCE_MS = 50; // Collect requests for 50ms before batching

  // Custodian selection service for CDN-like content serving
  private readonly custodianSelection = inject(CustodianSelectionService);

  constructor(private readonly holochainClient: HolochainClientService) {}

  /**
   * Get content by ID with request coalescing.
   *
   * Multiple rapid requests for different IDs will be batched into a single
   * zome call, reducing network round-trips.
   *
   * @param resourceId Content ID to fetch
   * @returns Promise resolving to ContentNode or null
   */
  async getContentCoalesced(resourceId: string): Promise<ContentNode | null> {
    if (!this.isAvailable()) {
      return null;
    }

    // Check cache first
    if (this.contentCache.has(resourceId)) {
      try {
        return (await this.contentCache.get(resourceId)!.toPromise()) ?? null;
      } catch {
        // Cache error, continue to fetch
      }
    }

    // Add to pending batch
    return new Promise((resolve, reject) => {
      if (!this.pendingBatchRequests.has(resourceId)) {
        this.pendingBatchRequests.set(resourceId, []);
      }
      this.pendingBatchRequests.get(resourceId)!.push({ resolve, reject });

      // Schedule batch execution
      this.scheduleBatchExecution();
    });
  }

  /**
   * Schedule batch execution after debounce period.
   */
  private scheduleBatchExecution(): void {
    if (this.batchRequestTimer) {
      return; // Already scheduled
    }

    this.batchRequestTimer = setTimeout(() => {
      this.executeBatchRequest();
    }, this.BATCH_DEBOUNCE_MS);
  }

  /**
   * Execute the batched request.
   */
  private async executeBatchRequest(): Promise<void> {
    this.batchRequestTimer = null;

    const ids = Array.from(this.pendingBatchRequests.keys());
    if (ids.length === 0) {
      return;
    }

    // Copy and clear pending requests
    const requests = new Map(this.pendingBatchRequests);
    this.pendingBatchRequests.clear();

    try {
      const { found, notFound } = await this.batchGetContent(ids);

      // Resolve found content
      for (const [id, content] of found) {
        const handlers = requests.get(id);
        if (handlers) {
          handlers.forEach(h => h.resolve(content));
        }
      }

      // Resolve not found as null
      for (const id of notFound) {
        const handlers = requests.get(id);
        if (handlers) {
          handlers.forEach(h => h.resolve(null));
        }
      }
    } catch (err) {
      // Reject all pending requests
      for (const handlers of requests.values()) {
        handlers.forEach(h => h.reject(err));
      }
    }
  }

  /**
   * Check if Holochain content is available for use.
   *
   * This returns false until the app interface proxy is implemented.
   * DataLoaderService should check this before delegating to this service.
   */
  isAvailable(): boolean {
    return this.availableSignal();
  }

  /**
   * Get content by ID from Holochain.
   *
   * @deprecated Use ContentService.getContent() instead.
   * Content is now served from doorway projection (SQLite), not DHT.
   *
   * Returns null if content not found or service unavailable.
   * Uses caching to avoid redundant zome calls.
   */
  getContent(resourceId: string): Observable<ContentNode | null> {
    if (!this.isAvailable()) {
      return of(null);
    }

    if (!this.contentCache.has(resourceId)) {
      const request = defer(() => from(this.fetchContentById(resourceId))).pipe(
        shareReplay(1),
        catchError(err => {
          console.warn(`[HolochainContent] Failed to fetch "${resourceId}":`, err);
          return of(null);
        })
      );

      this.contentCache.set(resourceId, request);
    }

    return this.contentCache.get(resourceId)!;
  }

  /**
   * Batch get multiple content items by IDs in a single zome call.
   *
   * @deprecated Use ContentService.batchGetContent() instead.
   * Content is now served from doorway projection (SQLite), not DHT.
   *
   * This is more efficient than calling getContent() multiple times
   * as it reduces network round-trips.
   *
   * @param ids Array of content IDs to fetch
   * @returns Map of id → ContentNode for found items, plus list of not found IDs
   */
  async batchGetContent(
    ids: string[]
  ): Promise<{ found: Map<string, ContentNode>; notFound: string[] }> {
    if (!this.isAvailable() || ids.length === 0) {
      return { found: new Map(), notFound: ids };
    }

    // Filter out IDs already in cache
    const uncachedIds: string[] = [];
    const found = new Map<string, ContentNode>();

    for (const id of ids) {
      if (this.contentCache.has(id)) {
        // Get from cache synchronously if possible
        const cached = this.contentCache.get(id);
        if (cached) {
          // Note: This is async but we'll handle it
          uncachedIds.push(id); // Still include for batch, cache will be used
        }
      } else {
        uncachedIds.push(id);
      }
    }

    if (uncachedIds.length === 0) {
      // All items were in cache, resolve them
      for (const id of ids) {
        const cached = this.contentCache.get(id);
        if (cached) {
          try {
            const content = await cached.toPromise();
            if (content) {
              found.set(id, content);
            }
          } catch {
            // Ignore cache errors
          }
        }
      }
      return { found, notFound: [] };
    }

    try {
      const result = await this.holochainClient.callZome<BatchGetContentOutput>({
        zomeName: 'content_store',
        fnName: 'batch_get_content_by_ids',
        payload: { ids: uncachedIds } as BatchGetContentInput,
      });

      if (!result.success || !result.data) {
        console.warn('[HolochainContent] Batch get failed:', result.error);
        return { found: new Map(), notFound: ids };
      }

      // Transform and cache results
      for (const output of result.data.found) {
        const content = this.transformToContentNode(output);
        found.set(content.id, content);

        // Update cache with shareReplay observable
        this.contentCache.set(content.id, of(content).pipe(shareReplay(1)));
      }

      return { found, notFound: result.data.notFound };
    } catch (err) {
      console.warn('[HolochainContent] Batch get error:', err);
      return { found: new Map(), notFound: ids };
    }
  }

  /**
   * Prefetch content for related nodes (call in background after loading main content).
   *
   * This proactively loads related content into the cache so it's ready
   * when the user navigates to it.
   *
   * @param relatedNodeIds Array of related content IDs to prefetch
   */
  prefetchRelatedContent(relatedNodeIds: string[]): void {
    if (!this.isAvailable() || relatedNodeIds.length === 0) {
      return;
    }

    // Filter to only uncached IDs
    const uncachedIds = relatedNodeIds.filter(id => !this.contentCache.has(id));

    if (uncachedIds.length === 0) {
      return;
    }

    // Fire and forget - don't await, don't block UI
    this.batchGetContent(uncachedIds).catch(err => {
      console.debug('[HolochainContent] Prefetch error (non-critical):', err);
    });
  }

  /**
   * Get content by type from Holochain.
   *
   * @deprecated Use ContentService.queryContent({contentType}) instead.
   * Content is now served from doorway projection (SQLite), not DHT.
   *
   * Returns empty array if service unavailable.
   */
  getContentByType(contentType: string, limit = 100): Observable<ContentNode[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.fetchContentByType(contentType, limit))).pipe(
      catchError(err => {
        console.warn(`[HolochainContent] Failed to fetch type "${contentType}":`, err);
        return of([]);
      })
    );
  }

  /**
   * Get content by type with pagination support.
   *
   * Use this for large datasets where you want to load content in pages.
   *
   * @param contentType The content type to filter by
   * @param pageSize Number of items per page (max 100)
   * @param offset Number of items to skip (for pagination)
   * @returns Paginated result with items, total count, and has_more flag
   */
  async getContentByTypePaginated(
    contentType: string,
    pageSize = 20,
    offset = 0
  ): Promise<{ items: ContentNode[]; totalCount: number; offset: number; hasMore: boolean }> {
    if (!this.isAvailable()) {
      return { items: [], totalCount: 0, offset, hasMore: false };
    }

    try {
      const result = await this.holochainClient.callZome<PaginatedContentOutput>({
        zomeName: 'content_store',
        fnName: 'get_content_by_type_paginated',
        payload: {
          content_type: contentType,
          page_size: Math.min(pageSize, 100),
          offset,
        } as PaginatedByTypeInput,
      });

      if (!result.success || !result.data) {
        console.warn('[HolochainContent] Paginated query failed:', result.error);
        return { items: [], totalCount: 0, offset, hasMore: false };
      }

      const items = result.data.items.map(output => this.transformToContentNode(output));

      // Cache the items
      for (const item of items) {
        this.contentCache.set(item.id, of(item).pipe(shareReplay(1)));
      }

      return {
        items,
        totalCount: result.data.totalCount,
        offset: result.data.offset,
        hasMore: result.data.hasMore,
      };
    } catch (err) {
      console.warn('[HolochainContent] Paginated query error:', err);
      return { items: [], totalCount: 0, offset, hasMore: false };
    }
  }

  /**
   * Get content by tag with pagination support.
   *
   * @param tag The tag to filter by
   * @param pageSize Number of items per page (max 100)
   * @param offset Number of items to skip
   * @returns Paginated result with items, total count, and has_more flag
   */
  async getContentByTagPaginated(
    tag: string,
    pageSize = 20,
    offset = 0
  ): Promise<{ items: ContentNode[]; totalCount: number; offset: number; hasMore: boolean }> {
    if (!this.isAvailable()) {
      return { items: [], totalCount: 0, offset, hasMore: false };
    }

    try {
      const result = await this.holochainClient.callZome<PaginatedContentOutput>({
        zomeName: 'content_store',
        fnName: 'get_content_by_tag_paginated',
        payload: {
          tag,
          page_size: Math.min(pageSize, 100),
          offset,
        } as PaginatedByTagInput,
      });

      if (!result.success || !result.data) {
        console.warn('[HolochainContent] Paginated tag query failed:', result.error);
        return { items: [], totalCount: 0, offset, hasMore: false };
      }

      const items = result.data.items.map(output => this.transformToContentNode(output));

      // Cache the items
      for (const item of items) {
        this.contentCache.set(item.id, of(item).pipe(shareReplay(1)));
      }

      return {
        items,
        totalCount: result.data.totalCount,
        offset: result.data.offset,
        hasMore: result.data.hasMore,
      };
    } catch (err) {
      console.warn('[HolochainContent] Paginated tag query error:', err);
      return { items: [], totalCount: 0, offset, hasMore: false };
    }
  }

  /**
   * Get content statistics from Holochain.
   *
   * @deprecated Content stats should come from doorway projection.
   *
   * Cached with shareReplay for efficiency.
   */
  getStats(): Observable<HolochainContentStats> {
    if (!this.isAvailable()) {
      return of({ totalCount: 0, byType: {} });
    }

    this.statsCache$ ??= defer(() => from(this.fetchStats())).pipe(
      shareReplay(1),
      catchError(() => of({ totalCount: 0, byType: {} }))
    );

    return this.statsCache$;
  }

  /**
   * Clear all caches (useful after imports or when data changes)
   */
  clearCache(): void {
    this.contentCache.clear();
    this.statsCache$ = null;
  }

  /**
   * Test if Holochain content API is reachable.
   *
   * Attempts a simple zome call to verify connectivity.
   * Updates the available signal based on result.
   */
  async testAvailability(): Promise<boolean> {
    try {
      const result = await this.holochainClient.callZome<HolochainContentStats>({
        zomeName: 'content_store',
        fnName: 'get_content_stats',
        payload: null,
      });

      if (result.success) {
        this.availableSignal.set(true);
        console.log(
          '[HolochainContent] Service available, content count:',
          result.data?.totalCount
        );
        return true;
      }

      this.availableSignal.set(false);
      return false;
    } catch (err) {
      console.warn('[HolochainContent] Availability test failed:', err);
      this.availableSignal.set(false);
      return false;
    }
  }

  // ===========================================================================
  // Learning Path Methods (DEPRECATED - use ContentService)
  // ===========================================================================

  /**
   * Get all learning paths (path index).
   *
   * @deprecated Use ContentService.queryPaths() instead.
   * Paths are now served from doorway projection (SQLite), not DHT.
   */
  async getPathIndex(): Promise<HolochainPathIndex> {
    console.log('[HolochainContent] Calling get_all_paths...');
    const result = await this.holochainClient.callZome<HolochainPathIndex>({
      zomeName: 'content_store',
      fnName: 'get_all_paths',
      payload: null,
    });

    console.log('[HolochainContent] get_all_paths result:', result);

    if (!result.success || !result.data) {
      console.warn('[HolochainContent] get_all_paths failed or empty:', result.error);
      return { paths: [], totalCount: 0, lastUpdated: new Date().toISOString() };
    }

    console.log('[HolochainContent] Found paths:', result.data.totalCount);
    return result.data;
  }

  /**
   * Get a learning path with all its steps.
   *
   * @deprecated Use ContentService.getPath() instead.
   * Paths are now served from doorway projection (SQLite), not DHT.
   */
  async getPathWithSteps(pathId: string): Promise<HolochainPathWithSteps | null> {
    const result = await this.holochainClient.callZome<HolochainPathWithSteps | null>({
      zomeName: 'content_store',
      fnName: 'get_path_with_steps',
      payload: pathId,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Get a lightweight path overview via REST API (cached by Doorway).
   *
   * @deprecated Use ContentService.getPath() instead.
   * Paths are now served from doorway projection (SQLite), not DHT.
   *
   * This is MUCH faster than getPathWithSteps because:
   * - Only counts step links instead of fetching each step record
   * - Uses Doorway's REST cache (15 minute TTL)
   *
   * Use for: path listings, path-overview page, initial navigation
   */
  async getPathOverviewRest(pathId: string): Promise<HolochainPathOverview | null> {
    const result = await this.holochainClient.callZomeRest<HolochainPathOverview | null>({
      zomeName: 'content_store',
      fnName: 'get_path_overview',
      payload: pathId,
    });

    if (!result.success || !result.data) {
      console.log('[HolochainContent] getPathOverviewRest failed:', result.error);
      return null;
    }

    return result.data;
  }

  // ===========================================================================
  // Agent Methods
  // ===========================================================================

  /**
   * Get an agent by ID.
   */
  async getAgentById(agentId: string): Promise<HolochainAgentOutput | null> {
    const result = await this.holochainClient.callZome<HolochainAgentOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_agent_by_id',
      payload: agentId,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Query agents by type or affinities.
   */
  async queryAgents(input: QueryAgentsInput): Promise<HolochainAgentOutput[]> {
    const result = await this.holochainClient.callZome<HolochainAgentOutput[]>({
      zomeName: 'content_store',
      fnName: 'query_agents',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  // ===========================================================================
  // Attestation Methods
  // ===========================================================================

  /**
   * Query attestations by agent ID or category.
   */
  async getAttestations(input: QueryAttestationsInput): Promise<HolochainAttestationOutput[]> {
    const result = await this.holochainClient.callZome<HolochainAttestationOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_attestations',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  // ===========================================================================
  // Content Attestation Methods (Trust claims about content)
  // ===========================================================================

  /**
   * Create a content attestation.
   */
  async createContentAttestation(
    input: CreateContentAttestationInput
  ): Promise<HolochainContentAttestationOutput | null> {
    const result = await this.holochainClient.callZome<HolochainContentAttestationOutput>({
      zomeName: 'content_store',
      fnName: 'create_content_attestation',
      payload: input,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Get content attestation by ID.
   */
  async getContentAttestationById(id: string): Promise<HolochainContentAttestationOutput | null> {
    const result = await this.holochainClient.callZome<HolochainContentAttestationOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_content_attestation_by_id',
      payload: id,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Get all attestations for a specific content node.
   */
  async getAttestationsForContent(contentId: string): Promise<HolochainContentAttestationOutput[]> {
    const result = await this.holochainClient.callZome<HolochainContentAttestationOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_attestations_for_content',
      payload: contentId,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  /**
   * Query content attestations with filters.
   */
  async queryContentAttestations(
    input: QueryContentAttestationsInput
  ): Promise<HolochainContentAttestationOutput[]> {
    const result = await this.holochainClient.callZome<HolochainContentAttestationOutput[]>({
      zomeName: 'content_store',
      fnName: 'query_content_attestations',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  /**
   * Update a content attestation.
   */
  async updateContentAttestation(
    input: UpdateContentAttestationInput
  ): Promise<HolochainContentAttestationOutput | null> {
    const result = await this.holochainClient.callZome<HolochainContentAttestationOutput>({
      zomeName: 'content_store',
      fnName: 'update_content_attestation',
      payload: input,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Revoke a content attestation.
   */
  async revokeContentAttestation(
    input: RevokeContentAttestationInput
  ): Promise<HolochainContentAttestationOutput | null> {
    const result = await this.holochainClient.callZome<HolochainContentAttestationOutput>({
      zomeName: 'content_store',
      fnName: 'revoke_content_attestation',
      payload: input,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  // ===========================================================================
  // Relationship/Graph Methods
  // ===========================================================================

  /**
   * Get relationships for a content node.
   *
   * @deprecated Use ContentService.getRelationships() instead.
   * This method will be removed in a future release.
   */
  async getRelationships(input: GetRelationshipsInput): Promise<HolochainRelationshipOutput[]> {
    const result = await this.holochainClient.callZome<HolochainRelationshipOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_relationships',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  /**
   * Get content graph starting from a root node.
   *
   * @deprecated Use ContentService.getContentGraph() instead.
   * This method will be removed in a future release.
   */
  async getContentGraph(
    contentId: string,
    relationshipTypes?: string[]
  ): Promise<HolochainContentGraph | null> {
    const input: QueryRelatedContentInput = {
      content_id: contentId,
      relationship_types: relationshipTypes,
    };

    const result = await this.holochainClient.callZome<HolochainContentGraph>({
      zomeName: 'content_store',
      fnName: 'get_content_graph',
      payload: input,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  // ===========================================================================
  // KnowledgeMap Methods
  // ===========================================================================

  /**
   * Get a knowledge map by ID.
   *
   * @deprecated Use ContentService.getKnowledgeMap() instead.
   * This method will be removed in a future release.
   */
  async getKnowledgeMapById(id: string): Promise<HolochainKnowledgeMapOutput | null> {
    const result = await this.holochainClient.callZome<HolochainKnowledgeMapOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_knowledge_map_by_id',
      payload: id,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Query knowledge maps.
   *
   * @deprecated Use ContentService.queryKnowledgeMaps() instead.
   * This method will be removed in a future release.
   */
  async queryKnowledgeMaps(input: QueryKnowledgeMapsInput): Promise<HolochainKnowledgeMapOutput[]> {
    const result = await this.holochainClient.callZome<HolochainKnowledgeMapOutput[]>({
      zomeName: 'content_store',
      fnName: 'query_knowledge_maps',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  // ===========================================================================
  // PathExtension Methods
  // ===========================================================================

  /**
   * Get a path extension by ID.
   *
   * @deprecated Use ContentService.getPathExtension() instead.
   * This method will be removed in a future release.
   */
  async getPathExtensionById(id: string): Promise<HolochainPathExtensionOutput | null> {
    const result = await this.holochainClient.callZome<HolochainPathExtensionOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_path_extension_by_id',
      payload: id,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Query path extensions.
   *
   * @deprecated Use ContentService.queryPathExtensions() instead.
   * This method will be removed in a future release.
   */
  async queryPathExtensions(
    input: QueryPathExtensionsInput
  ): Promise<HolochainPathExtensionOutput[]> {
    const result = await this.holochainClient.callZome<HolochainPathExtensionOutput[]>({
      zomeName: 'content_store',
      fnName: 'query_path_extensions',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  // ===========================================================================
  // Governance Methods (Challenge, Proposal, Precedent, Discussion, State)
  // ===========================================================================

  /**
   * Get a challenge by ID.
   */
  async getChallengeById(id: string): Promise<HolochainChallengeOutput | null> {
    const result = await this.holochainClient.callZome<HolochainChallengeOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_challenge_by_id',
      payload: id,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Query challenges.
   */
  async queryChallenges(input: QueryChallengesInput): Promise<HolochainChallengeOutput[]> {
    const result = await this.holochainClient.callZome<HolochainChallengeOutput[]>({
      zomeName: 'content_store',
      fnName: 'query_challenges',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  /**
   * Get a proposal by ID.
   */
  async getProposalById(id: string): Promise<HolochainProposalOutput | null> {
    const result = await this.holochainClient.callZome<HolochainProposalOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_proposal_by_id',
      payload: id,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Query proposals.
   */
  async queryProposals(input: QueryProposalsInput): Promise<HolochainProposalOutput[]> {
    const result = await this.holochainClient.callZome<HolochainProposalOutput[]>({
      zomeName: 'content_store',
      fnName: 'query_proposals',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  /**
   * Get a precedent by ID.
   */
  async getPrecedentById(id: string): Promise<HolochainPrecedentOutput | null> {
    const result = await this.holochainClient.callZome<HolochainPrecedentOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_precedent_by_id',
      payload: id,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Query precedents.
   */
  async queryPrecedents(input: QueryPrecedentsInput): Promise<HolochainPrecedentOutput[]> {
    const result = await this.holochainClient.callZome<HolochainPrecedentOutput[]>({
      zomeName: 'content_store',
      fnName: 'query_precedents',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  /**
   * Get a discussion by ID.
   */
  async getDiscussionById(id: string): Promise<HolochainDiscussionOutput | null> {
    const result = await this.holochainClient.callZome<HolochainDiscussionOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_discussion_by_id',
      payload: id,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Query discussions.
   */
  async queryDiscussions(input: QueryDiscussionsInput): Promise<HolochainDiscussionOutput[]> {
    const result = await this.holochainClient.callZome<HolochainDiscussionOutput[]>({
      zomeName: 'content_store',
      fnName: 'query_discussions',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  /**
   * Get governance state for an entity.
   */
  async getGovernanceState(
    input: GetGovernanceStateInput
  ): Promise<HolochainGovernanceStateOutput | null> {
    const result = await this.holochainClient.callZome<HolochainGovernanceStateOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_governance_state',
      payload: input,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
  }

  /**
   * Query governance states.
   */
  async queryGovernanceStates(
    input: QueryGovernanceStatesInput
  ): Promise<HolochainGovernanceStateOutput[]> {
    const result = await this.holochainClient.callZome<HolochainGovernanceStateOutput[]>({
      zomeName: 'content_store',
      fnName: 'query_governance_states',
      payload: input,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data;
  }

  // ===========================================================================
  // Private Methods - Zome Calls
  // ===========================================================================

  /**
   * Fetch single content by ID from Holochain
   */
  private async fetchContentById(id: string): Promise<ContentNode | null> {
    // Try to select a custodian for CDN-like serving (optional optimization)
    try {
      const custodian = await this.custodianSelection.selectBestCustodian(id);
      if (custodian) {
        console.log(`[HolochainContent] Selected custodian for "${id}":`, {
          custodianId: custodian.custodian.id.slice(0, 12) + '...',
          score: custodian.finalScore.toFixed(1),
        });
        // TODO: When doorway service is ready, fetch from custodian endpoint here
        // const content = await this.fetchFromCustodian(custodian.custodian.endpoint, id);
        // if (content) return content;
      }
    } catch (err) {
      console.debug('[HolochainContent] Custodian selection failed (non-critical):', err);
      // Continue with DHT fallback
    }

    // Fall back to DHT query
    const result = await this.holochainClient.callZome<HolochainContentOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_content_by_id',
      payload: { id } as QueryByIdInput,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return this.transformToContentNode(result.data);
  }

  /**
   * Fetch content by type from Holochain
   */
  private async fetchContentByType(contentType: string, limit: number): Promise<ContentNode[]> {
    const result = await this.holochainClient.callZome<HolochainContentOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_content_by_type',
      payload: { content_type: contentType, limit } as QueryByTypeInput,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data.map(output => this.transformToContentNode(output));
  }

  /**
   * Fetch content statistics from Holochain
   */
  private async fetchStats(): Promise<HolochainContentStats> {
    const result = await this.holochainClient.callZome<HolochainContentStats>({
      zomeName: 'content_store',
      fnName: 'get_content_stats',
      payload: null,
    });

    if (!result.success || !result.data) {
      return { totalCount: 0, byType: {} };
    }

    return result.data;
  }

  // ===========================================================================
  // Transformation - Holochain Entry → ContentNode
  // ===========================================================================

  /**
   * Transform Holochain content output to ContentNode
   *
   * Maps snake_case Rust fields to camelCase TypeScript fields.
   * Uses parsed metadata from Rust API.
   */
  private transformToContentNode(output: HolochainContentOutput): ContentNode {
    const entry = output.content;

    // Use parsed metadata from Rust API
    const metadata: ContentMetadata = (entry.metadata as any) ?? {};

    return {
      id: entry.id,
      contentType: entry.contentType as ContentType,
      title: entry.title,
      description: entry.description,
      content: entry.content,
      contentFormat: entry.contentFormat as ContentFormat,
      tags: entry.tags,
      sourcePath: entry.sourcePath ?? undefined,
      relatedNodeIds: entry.relatedNodeIds,
      metadata,
      authorId: entry.authorId ?? undefined,
      reach: this.mapReachLevel(entry.reach),
      trustScore: entry.trustScore,
      createdAt: entry.createdAt,
      updatedAt: entry.updatedAt,
    };
  }

  /**
   * Map Holochain reach string to ReachLevel type.
   * Uses the ReachLevel values from protocol-core.model.ts:
   * private, invited, local, neighborhood, municipal, bioregional, regional, commons
   */
  private mapReachLevel(reach: string): ContentNode['reach'] {
    // Handle various reach string formats
    const reachMap: Record<string, ContentNode['reach']> = {
      private: 'private',
      invited: 'invited',
      local: 'local',
      neighborhood: 'neighborhood',
      municipal: 'municipal',
      community: 'municipal', // Alias: community → municipal
      bioregional: 'bioregional',
      regional: 'regional',
      federated: 'regional', // Alias: federated → regional
      commons: 'commons',
      public: 'commons', // Alias: public → commons
    };

    return reachMap[reach.toLowerCase()] ?? 'commons';
  }
}
