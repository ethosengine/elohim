/**
 * ContextAssemblyService — Assembles the 6-layer context envelope on-device
 * before any "create" action, then invokes reach negotiation via ElohimAgentService.
 *
 * This is the engine behind "contextual wisdom at the edge." Every post, comment,
 * attestation, or governance vote assembles a rich CreateContext envelope from
 * local data before reaching the network. Trust determines how much assembly
 * is needed — high-trust authors get lightweight pipelines.
 *
 * Pipeline phases:
 *   1. Sync data collection (author ID, attestations, thread context)
 *   2. Async author profile assembly (trust score, reach ceiling, 15-min cache)
 *   3. Trust-adaptive gate (determinePipelineDepth decides which layers to skip)
 *   4. Conditional async layers (constitutional, community norms, graph neighborhood)
 *   5. Invoke reach negotiation + derive birth context
 */

import { inject, Injectable } from '@angular/core';

import { catchError, map, switchMap, take } from 'rxjs/operators';

import { Observable, forkJoin, of, race, timer } from 'rxjs';

import { IdentityService } from '@app/imagodei/services/identity.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { PathContextService } from '@app/lamad/services/path-context.service';
import { RelatedConceptsService } from '@app/lamad/services/related-concepts.service';

import { deriveBirthContext, determinePipelineDepth } from '../models/create-context.model';

import { AgentService } from './agent.service';
import { DataLoaderService } from './data-loader.service';
import { ElohimAgentService } from './elohim-agent.service';
import { GovernanceService } from './governance.service';

import type {
  AuthorProfileLayer,
  CommunityNormsLayer,
  ConstitutionalContextLayer,
  ContentBirthContext,
  ContextLayerName,
  CreateContext,
  CreatePayload,
  GraphNeighborhoodLayer,
  PipelineDepth,
  ReachNegotiationResult,
  ThreadContextLayer,
} from '../models/create-context.model';
import type { ElohimRequest, ElohimResponse } from '../models/elohim-agent.model';
import type { ContentAttestationType } from '@app/lamad/models/content-attestation.model';
import type { ContentReach } from '@app/lamad/models/content-node.model';

// ============================================================================
// Public Types
// ============================================================================

export interface AssemblyOptions {
  /** Timeout for total assembly in ms (default: 500) */
  timeoutMs?: number;
  /** If the action is within a thread, pass threadId here */
  threadId?: string;
  /** Author's intent statement (included in birth context) */
  authorIntent?: string;
}

export interface ContextAssemblyResult {
  createContext: CreateContext;
  negotiationResult: ReachNegotiationResult;
  birthContext: ContentBirthContext;
  /** Whether assembly timed out and returned a partial result */
  timedOut: boolean;
}

// ============================================================================
// Constants
// ============================================================================

const CAPABILITY_REACH_NEGOTIATION = 'reach-negotiation';
const LAYER_AUTHOR_PROFILE: ContextLayerName = 'author-profile';
const LAYER_COMMUNITY_NORMS: ContextLayerName = 'community-norms';
const LAYER_GRAPH_NEIGHBORHOOD: ContextLayerName = 'graph-neighborhood';
const REASON_TIMED_OUT = 'Assembly timed out';
const REASON_FAILED = 'Assembly failed';

/**
 * Trust score weights — replicated from TrustBadgeService.computeTrustScore().
 * Source: trust-badge.service.ts:421-432
 *
 * Formula: min((earnedWeight / totalWeight) * 2, 1)
 */
const ATT_AUTHOR_VERIFIED: ContentAttestationType = 'author-verified';
const ATT_COMMUNITY_ENDORSED: ContentAttestationType = 'community-endorsed';
const ATT_GOVERNANCE_RATIFIED: ContentAttestationType = 'governance-ratified';
const ATT_PEER_REVIEWED: ContentAttestationType = 'peer-reviewed';
const ATT_SAFETY_REVIEWED: ContentAttestationType = 'safety-reviewed';
const ATT_STEWARD_APPROVED: ContentAttestationType = 'steward-approved';

const TRUST_WEIGHTS: Partial<Record<ContentAttestationType, number>> = {
  [ATT_GOVERNANCE_RATIFIED]: 0.5,
  'curriculum-canonical': 0.4,
  [ATT_PEER_REVIEWED]: 0.4,
  [ATT_STEWARD_APPROVED]: 0.3,
  [ATT_SAFETY_REVIEWED]: 0.2,
  'accuracy-verified': 0.2,
  [ATT_COMMUNITY_ENDORSED]: 0.15,
  'accessibility-checked': 0.1,
  'license-cleared': 0.1,
  [ATT_AUTHOR_VERIFIED]: 0.1,
};

const TOTAL_TRUST_WEIGHT = Object.values(TRUST_WEIGHTS).reduce((sum, w) => sum + w, 0);

/**
 * Reach requirements table — inverted from TrustBadgeService.getAttestationsNeededForNextLevel().
 * Source: trust-badge.service.ts:589-598
 *
 * Walk from highest to lowest. Author's ceiling is the highest reach
 * where all required attestations are present.
 */
const REACH_REQUIREMENTS: [ContentReach, ContentAttestationType[]][] = [
  ['commons', [ATT_GOVERNANCE_RATIFIED, ATT_SAFETY_REVIEWED, 'license-cleared']],
  ['regional', [ATT_PEER_REVIEWED, ATT_GOVERNANCE_RATIFIED]],
  ['bioregional', [ATT_STEWARD_APPROVED, ATT_PEER_REVIEWED, ATT_SAFETY_REVIEWED]],
  ['municipal', [ATT_STEWARD_APPROVED, ATT_COMMUNITY_ENDORSED, ATT_SAFETY_REVIEWED]],
  ['neighborhood', [ATT_AUTHOR_VERIFIED, ATT_COMMUNITY_ENDORSED]],
  ['local', [ATT_AUTHOR_VERIFIED]],
  ['invited', [ATT_AUTHOR_VERIFIED]],
];

const DEFAULT_TIMEOUT_MS = 500;

// ============================================================================
// Internal Types
// ============================================================================

interface CacheEntry<T> {
  data: T;
  timestamp: number;
}

interface PipelineTracking {
  layersPopulated: ContextLayerName[];
  layersSkipped: { layer: ContextLayerName; reason: string }[];
  layerTimings: Partial<Record<ContextLayerName, number>>;
  startTime: number;
}

// ============================================================================
// Service
// ============================================================================

@Injectable({ providedIn: 'root' })
export class ContextAssemblyService {
  // Session-long cache for constitutional layer
  private readonly constitutionalCache = new Map<string, CacheEntry<ConstitutionalContextLayer>>();
  // 1-hour TTL for community norms
  private readonly communityNormsCache = new Map<string, CacheEntry<CommunityNormsLayer>>();
  // 15-minute TTL for author profile
  private readonly authorProfileCache = new Map<string, CacheEntry<AuthorProfileLayer>>();

  private static readonly COMMUNITY_NORMS_TTL = 60 * 60 * 1000; // 1 hour
  private static readonly AUTHOR_PROFILE_TTL = 15 * 60 * 1000; // 15 minutes

  private readonly sessionHumanService = inject(SessionHumanService);

  constructor(
    private readonly agentService: AgentService,
    private readonly elohimAgentService: ElohimAgentService,
    private readonly governanceService: GovernanceService,
    private readonly dataLoader: DataLoaderService,
    private readonly relatedConceptsService: RelatedConceptsService,
    private readonly pathContextService: PathContextService,
    private readonly identityService: IdentityService
  ) {}

  // ==========================================================================
  // Public API
  // ==========================================================================

  /**
   * Assemble context envelope from local data and negotiate reach.
   *
   * Returns the full CreateContext, the negotiation result, and the derived
   * ContentBirthContext ready for embedding in the ContentNode.
   */
  assembleAndNegotiate(
    payload: CreatePayload,
    options: AssemblyOptions = {}
  ): Observable<ContextAssemblyResult> {
    const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    const startTime = Date.now();
    const contextId = this.generateId();
    const assembledAt = new Date().toISOString();

    const assembly$ = this.runPipeline(payload, options, contextId, assembledAt, startTime);

    const timeout$ = timer(timeoutMs).pipe(
      map(() => this.buildFallbackResult(payload, contextId, assembledAt, startTime, true))
    );

    return race(assembly$, timeout$).pipe(take(1));
  }

  // ==========================================================================
  // Pipeline
  // ==========================================================================

  private runPipeline(
    payload: CreatePayload,
    options: AssemblyOptions,
    contextId: string,
    assembledAt: string,
    startTime: number
  ): Observable<ContextAssemblyResult> {
    const tracking: PipelineTracking = {
      layersPopulated: [],
      layersSkipped: [],
      layerTimings: {},
      startTime,
    };
    let cacheHits = 0;
    let cacheMisses = 0;

    // Phase 1: Sync — thread context
    const t1 = Date.now();
    const threadContext = this.assembleThreadContext(options.threadId);
    tracking.layerTimings['thread-context'] = Date.now() - t1;
    tracking.layersPopulated.push('thread-context');

    // Payload is always present
    tracking.layersPopulated.push('payload');

    // Phase 2: Async author profile (with cache)
    return this.assembleAuthorProfile(cacheHits, cacheMisses).pipe(
      switchMap(({ authorProfile, hits, misses }) => {
        cacheHits = hits;
        cacheMisses = misses;
        tracking.layerTimings[LAYER_AUTHOR_PROFILE] = Date.now() - startTime;
        tracking.layersPopulated.push(LAYER_AUTHOR_PROFILE);

        // Phase 3: Trust-adaptive gate
        const pipelineDepth = determinePipelineDepth(authorProfile);

        // Phase 4: Conditional async layers
        return this.assembleConditionalLayers(
          payload,
          pipelineDepth,
          tracking,
          cacheHits,
          cacheMisses
        ).pipe(
          map(({ constitutional, communityNorms, graphNeighborhood, hits: h, misses: m }) => {
            cacheHits = h;
            cacheMisses = m;

            const assemblyDurationMs = Date.now() - startTime;
            const createContext: CreateContext = {
              contextId,
              assembledAt,
              assemblyDurationMs,
              constitutional,
              communityNorms,
              authorProfile,
              graphNeighborhood,
              threadContext,
              payload,
              layersPopulated: [...tracking.layersPopulated],
              layersSkipped: [...tracking.layersSkipped],
              assemblyCost: {
                layerTimings: { ...tracking.layerTimings },
                dhtLookups: 0,
                cacheHits,
                cacheMisses,
                totalSizeBytes: 0, // Computed below
              },
            };
            // Approximate size
            createContext.assemblyCost.totalSizeBytes = JSON.stringify(createContext).length;
            return createContext;
          })
        );
      }),
      // Phase 5: Reach negotiation + birth context
      switchMap(createContext =>
        this.invokeReachNegotiation(createContext).pipe(
          map(negotiationResult => {
            const sourceContextHash = this.hashContext(createContext);
            const birthContext = deriveBirthContext(
              createContext,
              negotiationResult,
              sourceContextHash,
              options.authorIntent
            );
            return { createContext, negotiationResult, birthContext, timedOut: false };
          })
        )
      ),
      catchError(() =>
        of(this.buildFallbackResult(payload, contextId, assembledAt, startTime, false))
      )
    );
  }

  // ==========================================================================
  // Layer Assembly — Thread Context (Sync)
  // ==========================================================================

  private assembleThreadContext(threadId?: string): ThreadContextLayer {
    const pathCtx = this.pathContextService.currentContext;
    const activities = this.sessionHumanService.getActivityHistory();

    return {
      pathContext: pathCtx
        ? {
            pathId: pathCtx.pathId,
            pathTitle: pathCtx.pathTitle,
            stepIndex: pathCtx.stepIndex,
            totalSteps: pathCtx.totalSteps,
          }
        : undefined,
      threadId,
      sessionActions: activities.slice(-20).map(a => ({
        actionType: a.type,
        contentType: a.metadata?.['contentType'] as string | undefined,
        timestamp: a.timestamp,
      })),
    };
  }

  // ==========================================================================
  // Layer Assembly — Author Profile (15-min cache)
  // ==========================================================================

  private assembleAuthorProfile(
    cacheHits: number,
    cacheMisses: number
  ): Observable<{ authorProfile: AuthorProfileLayer; hits: number; misses: number }> {
    const authorId = this.agentService.getCurrentAgentId();
    const cacheKey = `author:${authorId}`;

    const cached = this.getCached(
      this.authorProfileCache,
      cacheKey,
      ContextAssemblyService.AUTHOR_PROFILE_TTL
    );
    if (cached) {
      return of({ authorProfile: cached, hits: cacheHits + 1, misses: cacheMisses });
    }

    return this.agentService.getLearningAnalytics().pipe(
      take(1),
      map(analytics => {
        const attestationTypes = this.agentService.getAttestations() as ContentAttestationType[];
        const trustScore = this.computeTrustScore(attestationTypes);
        const reachCeiling = this.deriveReachCeiling(attestationTypes);

        const authorProfile: AuthorProfileLayer = {
          authorId,
          trustIndicators: {
            trustScore,
            activeAttestationTypes: attestationTypes,
            totalContributions: analytics.totalContentNodesCompleted,
            engagementHistory: {
              totalEndorsements: 0,
              totalCitations: 0,
            },
          },
          reachCeiling,
          recentContributions: {
            last30Days: analytics.totalStepsCompleted,
            averageTrustScore: trustScore,
            topContentTypes: [],
          },
          computedAt: new Date().toISOString(),
        };

        this.setCached(this.authorProfileCache, cacheKey, authorProfile);
        return { authorProfile, hits: cacheHits, misses: cacheMisses + 1 };
      }),
      catchError(() => {
        const defaultProfile = this.buildDefaultAuthorProfile(authorId);
        return of({ authorProfile: defaultProfile, hits: cacheHits, misses: cacheMisses + 1 });
      })
    );
  }

  // ==========================================================================
  // Layer Assembly — Conditional Layers (Constitutional, Community, Graph)
  // ==========================================================================

  private assembleConditionalLayers(
    payload: CreatePayload,
    pipelineDepth: PipelineDepth,
    tracking: PipelineTracking,
    cacheHits: number,
    cacheMisses: number
  ): Observable<{
    constitutional: ConstitutionalContextLayer;
    communityNorms: CommunityNormsLayer;
    graphNeighborhood: GraphNeighborhoodLayer;
    hits: number;
    misses: number;
  }> {
    const required = pipelineDepth.requiredLayers;

    // Constitutional — always required
    const constitutional$ = this.assembleConstitutionalLayer(tracking);

    // Community norms — conditional
    const communityNorms$ = required.includes(LAYER_COMMUNITY_NORMS)
      ? this.assembleCommunityNormsLayer(tracking)
      : of({ layer: this.buildDefaultCommunityNorms(), hit: false });

    // Graph neighborhood — conditional
    const graphNeighborhood$ = required.includes(LAYER_GRAPH_NEIGHBORHOOD)
      ? this.assembleGraphNeighborhoodLayer(payload, pipelineDepth, tracking)
      : of(this.buildDefaultGraphNeighborhood());

    return forkJoin({
      constitutional: constitutional$,
      communityNormsResult: communityNorms$,
      graphNeighborhood: graphNeighborhood$,
    }).pipe(
      map(({ constitutional, communityNormsResult, graphNeighborhood }) => {
        let hits = cacheHits;
        let misses = cacheMisses;

        // Track constitutional
        tracking.layersPopulated.push('constitutional');

        // Track community norms
        if (required.includes(LAYER_COMMUNITY_NORMS)) {
          tracking.layersPopulated.push(LAYER_COMMUNITY_NORMS);
          if (communityNormsResult.hit) hits++;
          else misses++;
        } else {
          tracking.layersSkipped.push({
            layer: LAYER_COMMUNITY_NORMS,
            reason: 'Skipped by trust-adaptive pipeline (high/standard trust)',
          });
        }

        // Track graph neighborhood
        if (required.includes(LAYER_GRAPH_NEIGHBORHOOD)) {
          tracking.layersPopulated.push(LAYER_GRAPH_NEIGHBORHOOD);
          misses++; // Never cached
        } else {
          tracking.layersSkipped.push({
            layer: LAYER_GRAPH_NEIGHBORHOOD,
            reason: 'Skipped by trust-adaptive pipeline (high trust)',
          });
        }

        return {
          constitutional,
          communityNorms: communityNormsResult.layer,
          graphNeighborhood,
          hits,
          misses,
        };
      })
    );
  }

  // ==========================================================================
  // Constitutional Layer (session-long cache)
  // ==========================================================================

  private assembleConstitutionalLayer(
    tracking: PipelineTracking
  ): Observable<ConstitutionalContextLayer> {
    const cached = this.getCached(this.constitutionalCache, 'global', Infinity);
    if (cached) {
      tracking.layerTimings['constitutional'] = Date.now() - tracking.startTime;
      return of(cached);
    }

    return forkJoin({
      elohim: this.elohimAgentService
        .selectElohim({ capability: CAPABILITY_REACH_NEGOTIATION })
        .pipe(take(1)),
      precedents: this.governanceService.getConstitutionalPrecedents().pipe(take(1)),
    }).pipe(
      map(({ elohim, precedents }) => {
        const layer: ConstitutionalContextLayer = {
          stackHash: elohim?.constitutionalBinding?.constitutionHash ?? 'unverified',
          lastVerifiedAt: elohim?.constitutionalBinding?.lastVerified ?? new Date().toISOString(),
          applicableLayer: elohim?.layer ?? 'community',
          relevantPrinciples: precedents.slice(0, 5).map((p, i) => ({
            id: p.id ?? `precedent-${i}`,
            name: p.title ?? 'Constitutional principle',
            weight: 1,
          })),
          activeBoundaries: [],
        };

        this.setCached(this.constitutionalCache, 'global', layer);
        tracking.layerTimings['constitutional'] = Date.now() - tracking.startTime;
        return layer;
      }),
      catchError(() => {
        tracking.layerTimings['constitutional'] = Date.now() - tracking.startTime;
        return of(this.buildDefaultConstitutional());
      })
    );
  }

  // ==========================================================================
  // Community Norms Layer (1-hr cache)
  // ==========================================================================

  private assembleCommunityNormsLayer(
    tracking: PipelineTracking
  ): Observable<{ layer: CommunityNormsLayer; hit: boolean }> {
    const communityIds: string[] = this.identityService.profile()?.affinities ?? [];
    const sortedIds = [...communityIds];
    sortedIds.sort((a, b) => a.localeCompare(b));
    const cacheKey = `community:${sortedIds.join(',')}`;

    const cached = this.getCached(
      this.communityNormsCache,
      cacheKey,
      ContextAssemblyService.COMMUNITY_NORMS_TTL
    );
    if (cached) {
      tracking.layerTimings[LAYER_COMMUNITY_NORMS] = Date.now() - tracking.startTime;
      return of({ layer: cached, hit: true });
    }

    return this.governanceService.getActiveProposals().pipe(
      take(1),
      map(proposals => {
        const layer: CommunityNormsLayer = {
          communityIds,
          activeGovernanceContext: proposals.map(p => p.id),
        };

        this.setCached(this.communityNormsCache, cacheKey, layer);
        tracking.layerTimings[LAYER_COMMUNITY_NORMS] = Date.now() - tracking.startTime;
        return { layer, hit: false };
      }),
      catchError(() => {
        tracking.layerTimings[LAYER_COMMUNITY_NORMS] = Date.now() - tracking.startTime;
        return of({ layer: this.buildDefaultCommunityNorms(), hit: false });
      })
    );
  }

  // ==========================================================================
  // Graph Neighborhood Layer (never cached)
  // ==========================================================================

  private assembleGraphNeighborhoodLayer(
    payload: CreatePayload,
    pipelineDepth: PipelineDepth,
    tracking: PipelineTracking
  ): Observable<GraphNeighborhoodLayer> {
    const parentId = payload.parentContentId;

    // If no parent, return minimal neighborhood
    if (!parentId) {
      tracking.layerTimings[LAYER_GRAPH_NEIGHBORHOOD] = Date.now() - tracking.startTime;
      return of(this.buildDefaultGraphNeighborhood());
    }

    const relatedConcepts$ = this.relatedConceptsService
      .getRelatedConcepts(parentId, { limit: pipelineDepth.neighborhoodHops * 5 || 5 })
      .pipe(take(1));

    const parentContent$ = this.dataLoader.getContent(parentId).pipe(take(1));

    const sentiment$ = this.governanceService.getGovernanceSummary().pipe(take(1));

    return forkJoin({
      related: relatedConcepts$,
      parent: parentContent$,
      sentiment: sentiment$,
    }).pipe(
      map(({ related, parent, sentiment }) => {
        const allRelated = [
          ...related.prerequisites,
          ...related.extensions,
          ...related.related,
          ...related.children,
          ...related.parents,
        ];

        const activityLevel = this.deriveActivityLevel(sentiment.activeChallenges);

        const layer: GraphNeighborhoodLayer = {
          relatedNodes: allRelated.slice(0, 10).map(node => ({
            nodeId: node.id,
            title: node.title,
            contentType: node.contentType,
            reach: node.reach ?? 'private',
            trustScore: node.trustScore ?? 0,
            relationshipType: 'RELATES_TO',
            confidence: 0.8,
          })),
          parentContext: {
            nodeId: parent.id,
            reach: parent.reach ?? 'private',
            trustScore: parent.trustScore ?? 0,
            authorId: parent.authorId ?? 'unknown',
            feedbackProfileId: parent.feedbackProfileId,
            activeFlags: (parent.flags ?? []).map(f => ({
              type: f.type,
              reason: f.reason,
            })),
          },
          neighborhoodAttestationSummary: {
            totalAttestations: allRelated.length,
            averageTrustScore:
              allRelated.length > 0
                ? allRelated.reduce((sum, n) => sum + (n.trustScore ?? 0), 0) / allRelated.length
                : 0,
            dominantReach: parent.reach ?? 'private',
          },
          communitySentiment: {
            recentActivityLevel: activityLevel,
            recentFlagRate: sentiment.activeChallenges / Math.max(sentiment.votingProposals, 1),
          },
        };

        tracking.layerTimings[LAYER_GRAPH_NEIGHBORHOOD] = Date.now() - tracking.startTime;
        return layer;
      }),
      catchError(() => {
        tracking.layerTimings[LAYER_GRAPH_NEIGHBORHOOD] = Date.now() - tracking.startTime;
        return of(this.buildDefaultGraphNeighborhood());
      })
    );
  }

  // ==========================================================================
  // Reach Negotiation
  // ==========================================================================

  private invokeReachNegotiation(createContext: CreateContext): Observable<ReachNegotiationResult> {
    const request: ElohimRequest = {
      requestId: this.generateId(),
      targetElohimId: 'auto',
      capability: CAPABILITY_REACH_NEGOTIATION,
      params: { type: CAPABILITY_REACH_NEGOTIATION, createContext },
      requesterId: this.agentService.getCurrentAgentId(),
      priority: 'normal',
      requestedAt: new Date().toISOString(),
    };

    return this.elohimAgentService.invoke(request).pipe(
      take(1),
      map((response: ElohimResponse) => {
        if (response.status === 'fulfilled' && response.payload) {
          return response.payload as ReachNegotiationResult;
        }
        return this.buildDefaultNegotiationResult(createContext);
      }),
      catchError(() => of(this.buildDefaultNegotiationResult(createContext)))
    );
  }

  // ==========================================================================
  // Trust Score Computation
  // ==========================================================================

  /** Compute trust score from attestation types. Mirrors TrustBadgeService. */
  private computeTrustScore(attestationTypes: ContentAttestationType[]): number {
    if (attestationTypes.length === 0) return 0;

    let earnedWeight = 0;
    for (const type of attestationTypes) {
      earnedWeight += TRUST_WEIGHTS[type] ?? 0;
    }
    return Math.min((earnedWeight / TOTAL_TRUST_WEIGHT) * 2, 1);
  }

  /** Derive reach ceiling from attestation types. Inverts getAttestationsNeededForNextLevel. */
  private deriveReachCeiling(attestationTypes: ContentAttestationType[]): ContentReach {
    for (const [reach, required] of REACH_REQUIREMENTS) {
      if (required.every(att => attestationTypes.includes(att))) {
        return reach;
      }
    }
    return 'private';
  }

  // ==========================================================================
  // Helpers
  // ==========================================================================

  private deriveActivityLevel(activeChallenges: number): 'quiet' | 'normal' | 'active' | 'heated' {
    if (activeChallenges > 3) return 'heated';
    if (activeChallenges > 0) return 'active';
    return 'normal';
  }

  // ==========================================================================
  // Cache Helpers
  // ==========================================================================

  private getCached<T>(cache: Map<string, CacheEntry<T>>, key: string, ttlMs: number): T | null {
    const entry = cache.get(key);
    if (!entry) return null;
    if (ttlMs !== Infinity && Date.now() - entry.timestamp > ttlMs) {
      cache.delete(key);
      return null;
    }
    return entry.data;
  }

  private setCached<T>(cache: Map<string, CacheEntry<T>>, key: string, data: T): void {
    cache.set(key, { data, timestamp: Date.now() });
  }

  // ==========================================================================
  // Default / Fallback Builders
  // ==========================================================================

  private buildDefaultConstitutional(): ConstitutionalContextLayer {
    return {
      stackHash: 'unverified',
      lastVerifiedAt: new Date().toISOString(),
      applicableLayer: 'community',
      relevantPrinciples: [],
      activeBoundaries: [],
    };
  }

  private buildDefaultCommunityNorms(): CommunityNormsLayer {
    return { communityIds: [] };
  }

  private buildDefaultAuthorProfile(authorId: string): AuthorProfileLayer {
    return {
      authorId,
      trustIndicators: {
        trustScore: 0,
        activeAttestationTypes: [],
        totalContributions: 0,
        engagementHistory: { totalEndorsements: 0, totalCitations: 0 },
      },
      reachCeiling: 'private',
      recentContributions: { last30Days: 0, averageTrustScore: 0, topContentTypes: [] },
      computedAt: new Date().toISOString(),
    };
  }

  private buildDefaultGraphNeighborhood(): GraphNeighborhoodLayer {
    return {
      relatedNodes: [],
      neighborhoodAttestationSummary: {
        totalAttestations: 0,
        averageTrustScore: 0,
        dominantReach: 'private',
      },
    };
  }

  private buildDefaultNegotiationResult(createContext: CreateContext): ReachNegotiationResult {
    return {
      type: CAPABILITY_REACH_NEGOTIATION,
      recommendedReach: createContext.payload.requestedReach,
      safetyCleared: true,
      conditions: [],
      transparencyReport: {
        layersConsidered: createContext.layersPopulated,
        decisionFactors: [],
        authorStanding: 'Default — negotiation unavailable',
        communityHealth: 'Unknown',
        neighborhoodInfluence: 'Unknown',
      },
    };
  }

  private buildFallbackResult(
    payload: CreatePayload,
    contextId: string,
    assembledAt: string,
    startTime: number,
    timedOut: boolean
  ): ContextAssemblyResult {
    const authorId = this.agentService.getCurrentAgentId();
    const reason = timedOut ? REASON_TIMED_OUT : REASON_FAILED;
    const createContext: CreateContext = {
      contextId,
      assembledAt,
      assemblyDurationMs: Date.now() - startTime,
      constitutional: this.buildDefaultConstitutional(),
      communityNorms: this.buildDefaultCommunityNorms(),
      authorProfile: this.buildDefaultAuthorProfile(authorId),
      graphNeighborhood: this.buildDefaultGraphNeighborhood(),
      threadContext: { sessionActions: [] },
      payload,
      layersPopulated: ['payload'],
      layersSkipped: [
        { layer: 'constitutional', reason },
        { layer: LAYER_COMMUNITY_NORMS, reason },
        { layer: LAYER_AUTHOR_PROFILE, reason },
        { layer: LAYER_GRAPH_NEIGHBORHOOD, reason },
        { layer: 'thread-context', reason },
      ],
      assemblyCost: {
        layerTimings: {},
        dhtLookups: 0,
        cacheHits: 0,
        cacheMisses: 0,
        totalSizeBytes: 0,
      },
    };

    const negotiationResult = this.buildDefaultNegotiationResult(createContext);
    const birthContext = deriveBirthContext(createContext, negotiationResult, 'fallback');

    return { createContext, negotiationResult, birthContext, timedOut };
  }

  // ==========================================================================
  // Utilities
  // ==========================================================================

  private generateId(): string {
    const randomBytes = crypto.getRandomValues(new Uint8Array(8));
    return Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 12);
  }

  /**
   * Prototype context hash. Uses btoa fingerprint for now.
   * Upgrade path: crypto.subtle.digest('SHA-256', ...) for production.
   */
  private hashContext(context: CreateContext): string {
    const fingerprint = `${context.contextId}:${context.assembledAt}:${context.authorProfile.authorId}:${context.constitutional.stackHash}`;
    return btoa(fingerprint).substring(0, 32);
  }
}
