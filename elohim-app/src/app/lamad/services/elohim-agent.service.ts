import { Injectable } from '@angular/core';
import { Observable, of, timer } from 'rxjs';
import { map, switchMap } from 'rxjs/operators';
import { DataLoaderService } from './data-loader.service';
import {
  ElohimAgent,
  ElohimCapability,
  ElohimLayer,
  ElohimRequest,
  ElohimResponse,
  ElohimIndexEntry,
  ElohimSelectionCriteria,
  ConstitutionalReasoning,
  ContentReviewParams,
  ContentReviewResult,
  AttestationRecommendationParams,
  AttestationRecommendation
} from '../models/elohim-agent.model';

/**
 * ElohimAgentService - Interface to autonomous constitutional guardians.
 *
 * This service provides the protocol for invoking Elohim agents.
 * In production, these calls go to actual AI agents running on edge nodes.
 * For prototype, we simulate the invocation patterns and response structures.
 *
 * Key principles:
 * - Elohim are invoked, not commanded
 * - Every response includes constitutional reasoning
 * - Elohim can decline requests that violate their principles
 * - Layer-appropriate Elohim are selected automatically when requested
 *
 * Usage patterns:
 * 1. Content attestation: Request content review → Get recommendation
 * 2. Knowledge maps: Request map synthesis → Get updated map
 * 3. Spiral detection: Request monitoring → Get intervention suggestions
 * 4. Path analysis: Request analysis → Get improvement suggestions
 */
@Injectable({ providedIn: 'root' })
export class ElohimAgentService {
  private elohimCache: Map<string, ElohimAgent> = new Map();
  private requestLog: ElohimRequest[] = [];

  constructor(private readonly dataLoader: DataLoaderService) {}

  // =========================================================================
  // Elohim Discovery
  // =========================================================================

  /**
   * Get all available Elohim agents.
   */
  getElohimIndex(): Observable<ElohimIndexEntry[]> {
    return this.dataLoader.getAgentIndex().pipe(
      map(response => {
        const elohimAgents = response.agents.filter(a => a.type === 'elohim');
        return elohimAgents.map(e => ({
          id: e.id,
          displayName: e.displayName,
          layer: e.layer as ElohimLayer,
          capabilities: (e.capabilities || []) as ElohimCapability[],
          visibility: e.visibility as 'public' | 'private'
        }));
      })
    );
  }

  /**
   * Get a specific Elohim by ID.
   */
  getElohim(elohimId: string): Observable<ElohimAgent | null> {
    if (this.elohimCache.has(elohimId)) {
      return of(this.elohimCache.get(elohimId)!);
    }

    return this.dataLoader.getAgentIndex().pipe(
      map(response => {
        const agent = response.agents.find(a => a.id === elohimId && a.type === 'elohim');
        if (!agent) return null;

        const elohim: ElohimAgent = {
          id: agent.id,
          displayName: agent.displayName,
          layer: agent.layer as ElohimLayer,
          bio: agent.bio || '',
          attestations: agent.attestations || [],
          capabilities: (agent.capabilities || []) as ElohimCapability[],
          visibility: agent.visibility as 'public' | 'private',
          familyId: agent.familyId,
          createdAt: agent.createdAt,
          updatedAt: agent.updatedAt
        };

        this.elohimCache.set(elohimId, elohim);
        return elohim;
      })
    );
  }

  /**
   * Find the most appropriate Elohim for a given capability and context.
   */
  selectElohim(criteria: ElohimSelectionCriteria): Observable<ElohimAgent | null> {
    return this.getElohimIndex().pipe(
      switchMap(elohimList => {
        // Filter by capability
        const capable = elohimList.filter(e =>
          e.capabilities.includes(criteria.capability)
        );

        if (capable.length === 0) {
          return of(null);
        }

        // Prefer layer-appropriate Elohim
        let selected = capable[0];
        if (criteria.preferredLayer) {
          const layerMatch = capable.find(e => e.layer === criteria.preferredLayer);
          if (layerMatch) selected = layerMatch;
        }

        // For family/individual contexts, prefer more local Elohim
        if (criteria.contextFamilyId) {
          const familyElohim = capable.find(e => e.layer === 'family');
          if (familyElohim) selected = familyElohim;
        }

        return this.getElohim(selected.id);
      })
    );
  }

  // =========================================================================
  // Elohim Invocation
  // =========================================================================

  /**
   * Invoke an Elohim with a request.
   *
   * This is the core method for interacting with Elohim agents.
   * The Elohim will process the request according to its constitutional binding
   * and return a response with full reasoning.
   */
  invoke(request: ElohimRequest): Observable<ElohimResponse> {
    // Log the request
    this.requestLog.push(request);

    // Resolve target Elohim
    const targetElohim$ = request.targetElohimId === 'auto'
      ? this.selectElohim({ capability: request.capability })
      : this.getElohim(request.targetElohimId);

    return targetElohim$.pipe(
      switchMap(elohim => {
        if (!elohim) {
          return of(this.createDeclinedResponse(
            request,
            'unknown',
            'No Elohim available for this capability'
          ));
        }

        // Check capability
        if (!elohim.capabilities.includes(request.capability)) {
          return of(this.createDeclinedResponse(
            request,
            elohim.id,
            `This Elohim does not have the '${request.capability}' capability`
          ));
        }

        // Simulate processing time based on capability
        const processingTime = this.estimateProcessingTime(request.capability);

        return timer(processingTime).pipe(
          map(() => this.processRequest(request, elohim))
        );
      })
    );
  }

  /**
   * Request content review from an Elohim.
   * Convenience method for content-safety-review capability.
   */
  requestContentReview(
    contentId: string,
    reviewType: 'safety' | 'accuracy' | 'constitutional-alignment',
    requesterId: string
  ): Observable<ElohimResponse> {
    const request: ElohimRequest = {
      requestId: this.generateRequestId(),
      targetElohimId: 'auto',
      capability: 'content-safety-review',
      params: {
        type: 'content-review',
        contentId,
        reviewType
      } as ContentReviewParams,
      requesterId,
      priority: 'normal',
      requestedAt: new Date().toISOString()
    };

    return this.invoke(request);
  }

  /**
   * Request attestation recommendation from an Elohim.
   * Convenience method for attestation-recommendation capability.
   */
  requestAttestationRecommendation(
    contentId: string,
    attestationType: string,
    requesterId: string,
    evidence?: string
  ): Observable<ElohimResponse> {
    const request: ElohimRequest = {
      requestId: this.generateRequestId(),
      targetElohimId: 'auto',
      capability: 'attestation-recommendation',
      params: {
        type: 'attestation-recommendation',
        contentId,
        requestedAttestationType: attestationType,
        evidence
      } as AttestationRecommendationParams,
      requesterId,
      priority: 'normal',
      requestedAt: new Date().toISOString()
    };

    return this.invoke(request);
  }

  // =========================================================================
  // Request Processing (Prototype Simulation)
  // =========================================================================

  /**
   * Process a request and generate a response.
   * In production, this calls the actual Elohim AI agent.
   * For prototype, we simulate responses that demonstrate the patterns.
   */
  private processRequest(request: ElohimRequest, elohim: ElohimAgent): ElohimResponse {
    const baseResponse = {
      requestId: request.requestId,
      elohimId: elohim.id,
      respondedAt: new Date().toISOString(),
      cost: {
        tokensProcessed: Math.floor(Math.random() * 1000) + 500,
        timeMs: this.estimateProcessingTime(request.capability),
        constitutionalChecks: Math.floor(Math.random() * 5) + 1,
        precedentLookups: Math.floor(Math.random() * 3)
      }
    };

    // Route to capability-specific handlers
    switch (request.capability) {
      case 'content-safety-review':
        return this.handleContentReview(request, elohim, baseResponse);

      case 'attestation-recommendation':
        return this.handleAttestationRecommendation(request, elohim, baseResponse);

      default:
        return {
          ...baseResponse,
          status: 'fulfilled',
          constitutionalReasoning: this.generateDefaultReasoning(request.capability),
          payload: undefined
        };
    }
  }

  private handleContentReview(
    request: ElohimRequest,
    elohim: ElohimAgent,
    baseResponse: Partial<ElohimResponse>
  ): ElohimResponse {
    const params = request.params as ContentReviewParams;

    // Simulate content review (in production: actual AI analysis)
    const approved = Math.random() > 0.1; // 90% approval for demo
    const issues = approved ? [] : [{
      severity: 'warning' as const,
      category: 'clarity',
      description: 'Content could benefit from additional examples',
      suggestion: 'Consider adding concrete use cases'
    }];

    const payload: ContentReviewResult = {
      type: 'content-review',
      contentId: params.contentId,
      approved,
      issues,
      trustScoreImpact: approved ? 0.1 : -0.05
    };

    return {
      ...baseResponse,
      status: 'fulfilled',
      constitutionalReasoning: {
        primaryPrinciple: 'Love as committed action toward flourishing',
        interpretation: 'Content review ensures learners receive accurate, safe information that supports their growth',
        valuesWeighed: [
          { value: 'Learner safety', weight: 0.4, direction: 'for' },
          { value: 'Knowledge access', weight: 0.3, direction: 'for' },
          { value: 'Author dignity', weight: 0.2, direction: 'for' },
          { value: 'Community trust', weight: 0.1, direction: 'for' }
        ],
        confidence: 0.85,
        precedents: ['content-review-2025-001', 'safety-standard-v1.2']
      },
      payload
    } as ElohimResponse;
  }

  private handleAttestationRecommendation(
    request: ElohimRequest,
    elohim: ElohimAgent,
    baseResponse: Partial<ElohimResponse>
  ): ElohimResponse {
    const params = request.params as AttestationRecommendationParams;

    // Simulate attestation decision (in production: actual AI analysis)
    const recommend = Math.random() > 0.2 ? 'grant' : 'defer';

    const payload: AttestationRecommendation = {
      type: 'attestation-recommendation',
      contentId: params.contentId,
      recommend: recommend as 'grant' | 'deny' | 'defer',
      attestationType: params.requestedAttestationType,
      suggestedReach: recommend === 'grant' ? 'community' : undefined,
      conditions: recommend === 'defer' ? ['Requires peer review', 'Additional evidence needed'] : undefined,
      reasoning: recommend === 'grant'
        ? 'Content meets quality standards and aligns with constitutional principles'
        : 'Additional review recommended before granting attestation'
    };

    return {
      ...baseResponse,
      status: 'fulfilled',
      constitutionalReasoning: {
        primaryPrinciple: 'Boundaries around freedom of reach',
        interpretation: 'Attestations expand content reach; must ensure content serves flourishing before broader distribution',
        valuesWeighed: [
          { value: 'Content quality', weight: 0.35, direction: recommend === 'grant' ? 'for' : 'against' },
          { value: 'Community protection', weight: 0.3, direction: 'for' },
          { value: 'Author recognition', weight: 0.2, direction: 'for' },
          { value: 'Knowledge sharing', weight: 0.15, direction: 'for' }
        ],
        confidence: recommend === 'grant' ? 0.8 : 0.6,
        precedents: ['attestation-standard-v1.0']
      },
      payload
    } as ElohimResponse;
  }

  // =========================================================================
  // Helpers
  // =========================================================================

  private generateRequestId(): string {
    return `req-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
  }

  private estimateProcessingTime(capability: ElohimCapability): number {
    // Simulate realistic processing times (ms)
    const times: Partial<Record<ElohimCapability, number>> = {
      'content-safety-review': 1500,
      'accuracy-verification': 2000,
      'attestation-recommendation': 1200,
      'knowledge-map-synthesis': 3000,
      'spiral-detection': 800,
      'path-analysis': 2500
    };
    return times[capability] || 1000;
  }

  private createDeclinedResponse(
    request: ElohimRequest,
    elohimId: string,
    reason: string
  ): ElohimResponse {
    return {
      requestId: request.requestId,
      elohimId,
      status: 'declined',
      constitutionalReasoning: {
        primaryPrinciple: 'Capability boundaries',
        interpretation: 'Elohim may only exercise capabilities they possess',
        valuesWeighed: [],
        confidence: 1.0
      },
      declineReason: reason,
      respondedAt: new Date().toISOString()
    };
  }

  private generateDefaultReasoning(capability: ElohimCapability): ConstitutionalReasoning {
    return {
      primaryPrinciple: 'Love as committed action toward flourishing',
      interpretation: `Capability '${capability}' exercised in service of human flourishing`,
      valuesWeighed: [
        { value: 'Human dignity', weight: 0.5, direction: 'for' },
        { value: 'Collective wellbeing', weight: 0.5, direction: 'for' }
      ],
      confidence: 0.75
    };
  }

  // =========================================================================
  // Audit and Transparency
  // =========================================================================

  /**
   * Get recent requests for transparency/audit.
   */
  getRecentRequests(limit = 10): ElohimRequest[] {
    return this.requestLog.slice(-limit);
  }

  /**
   * Clear request log (for testing).
   */
  clearRequestLog(): void {
    this.requestLog = [];
  }
}
