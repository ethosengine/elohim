/**
 * Elohim Backend — Pluggable inference abstraction.
 *
 * Mirrors the Rust `LlmBackend` trait from elohim-agent crate.
 * Provides interface, backend catalog with fallback chain,
 * and MockBackend (extracted from the original ElohimAgentService simulation).
 */

import { Injectable } from '@angular/core';

import { map } from 'rxjs/operators';

import { Observable, timer } from 'rxjs';

import {
  ElohimAgent,
  ElohimCapability,
  ElohimRequest,
  ElohimResponse,
  ConstitutionalReasoning,
  ContentReviewParams,
  ContentReviewResult,
  AttestationRecommendationParams,
  AttestationRecommendation,
  PathAnalysisParams,
  PathAnalysisResult,
} from '../models/elohim-agent.model';

// ============================================================================
// Backend Interface
// ============================================================================

/**
 * Backend type identifiers.
 */
export type ElohimBackendType = 'mock' | 'native';

/**
 * Pluggable backend interface — one implementation per inference source.
 *
 * Mirrors the Rust `LlmBackend` trait: each backend knows how to check
 * availability and invoke an ElohimRequest against a resolved ElohimAgent.
 */
export interface ElohimBackend {
  /** Unique backend identifier */
  readonly id: string;

  /** Display name */
  readonly name: string;

  /** Backend type */
  readonly type: ElohimBackendType;

  /** Check if this backend is currently available */
  isAvailable(): Promise<boolean>;

  /** Invoke a request against a resolved elohim agent */
  invoke(request: ElohimRequest, elohim: ElohimAgent): Observable<ElohimResponse>;
}

// ============================================================================
// Backend Catalog
// ============================================================================

/**
 * Catalog of registered backends with selection + fallback.
 *
 * Selection order: preferred → first available non-mock → mock fallback.
 */
@Injectable({ providedIn: 'root' })
export class ElohimBackendCatalog {
  private readonly backends = new Map<string, ElohimBackend>();
  private preferredId: string | null = null;

  /**
   * Register a backend. Replaces any existing backend with the same ID.
   */
  register(backend: ElohimBackend): void {
    this.backends.set(backend.id, backend);
  }

  /**
   * Unregister a backend by ID.
   */
  unregister(id: string): void {
    this.backends.delete(id);
    if (this.preferredId === id) {
      this.preferredId = null;
    }
  }

  /**
   * Set the preferred backend ID.
   */
  setPreferred(id: string): void {
    this.preferredId = id;
  }

  /**
   * Get the current preferred backend ID.
   */
  getPreferredId(): string | null {
    return this.preferredId;
  }

  /**
   * Select the best available backend.
   * Fallback chain: preferred → first available non-mock → mock.
   */
  async selectBackend(): Promise<ElohimBackend> {
    // Try preferred first
    if (this.preferredId) {
      const preferred = this.backends.get(this.preferredId);
      if (preferred && (await preferred.isAvailable())) {
        return preferred;
      }
    }

    // Try first available non-mock
    for (const backend of this.backends.values()) {
      if (backend.type !== 'mock' && (await backend.isAvailable())) {
        return backend;
      }
    }

    // Fall back to mock
    const mock = this.backends.get('mock');
    if (mock) return mock;

    // Last resort: create inline mock
    return new MockBackend();
  }

  /**
   * Get all registered backends.
   */
  getAll(): ElohimBackend[] {
    return Array.from(this.backends.values());
  }

  /**
   * Get a specific backend by ID.
   */
  get(id: string): ElohimBackend | undefined {
    return this.backends.get(id);
  }
}

// ============================================================================
// Mock Backend (extracted from original ElohimAgentService simulation)
// ============================================================================

const CAPABILITY_ATTESTATION_RECOMMENDATION = 'attestation-recommendation';
const CAPABILITY_PATH_RECOMMENDATION = 'path-recommendation';
const CAPABILITY_REACH_NEGOTIATION = 'reach-negotiation';

/**
 * Mock backend — preserves the original simulation behavior.
 *
 * Uses timer delays and crypto.getRandomValues for demo data,
 * exactly as the original ElohimAgentService.processRequest() did.
 * This is the default fallback when no real backend is configured.
 */
export class MockBackend implements ElohimBackend {
  readonly id = 'mock';
  readonly name = 'Simulation (Mock)';
  readonly type: ElohimBackendType = 'mock';

  // eslint-disable-next-line @typescript-eslint/require-await -- interface contract requires Promise
  async isAvailable(): Promise<boolean> {
    return true;
  }

  invoke(request: ElohimRequest, elohim: ElohimAgent): Observable<ElohimResponse> {
    const processingTime = this.estimateProcessingTime(request.capability);
    return timer(processingTime).pipe(map(() => this.processRequest(request, elohim)));
  }

  private processRequest(request: ElohimRequest, elohim: ElohimAgent): ElohimResponse {
    const baseResponse = {
      requestId: request.requestId,
      elohimId: elohim.id,
      respondedAt: new Date().toISOString(),
      cost: {
        tokensProcessed: this.randomInt(500, 1500),
        timeMs: this.estimateProcessingTime(request.capability),
        constitutionalChecks: this.randomInt(1, 6),
        precedentLookups: this.randomInt(0, 3),
      },
    };

    switch (request.capability) {
      case 'content-safety-review':
        return this.handleContentReview(request, baseResponse);

      case CAPABILITY_ATTESTATION_RECOMMENDATION:
        return this.handleAttestationRecommendation(request, baseResponse);

      case CAPABILITY_PATH_RECOMMENDATION:
        return this.handlePathRecommendation(request, baseResponse);

      case CAPABILITY_REACH_NEGOTIATION:
        return this.handleReachNegotiation(request, baseResponse);

      default:
        return {
          ...baseResponse,
          status: 'fulfilled',
          constitutionalReasoning: this.generateDefaultReasoning(request.capability),
          payload: undefined,
        };
    }
  }

  private handleContentReview(
    request: ElohimRequest,
    baseResponse: Partial<ElohimResponse>
  ): ElohimResponse {
    const params = request.params as ContentReviewParams;
    const approved = this.randomFloat() > 0.1; // 90% approval for demo
    const issues = approved
      ? []
      : [
          {
            severity: 'warning' as const,
            category: 'clarity',
            description: 'Content could benefit from additional examples',
            suggestion: 'Consider adding concrete use cases',
          },
        ];

    const payload: ContentReviewResult = {
      type: 'content-review',
      contentId: params.contentId,
      approved,
      issues,
      trustScoreImpact: approved ? 0.1 : -0.05,
    };

    return {
      ...baseResponse,
      status: 'fulfilled',
      constitutionalReasoning: {
        primaryPrinciple: 'Love as committed action toward flourishing',
        interpretation:
          'Content review ensures learners receive accurate, safe information that supports their growth',
        valuesWeighed: [
          { value: 'Learner safety', weight: 0.4, direction: 'for' },
          { value: 'Knowledge access', weight: 0.3, direction: 'for' },
          { value: 'Author dignity', weight: 0.2, direction: 'for' },
          { value: 'Community trust', weight: 0.1, direction: 'for' },
        ],
        confidence: 0.85,
        precedents: ['content-review-2025-001', 'safety-standard-v1.2'],
      },
      payload,
    } as ElohimResponse;
  }

  private handleAttestationRecommendation(
    request: ElohimRequest,
    baseResponse: Partial<ElohimResponse>
  ): ElohimResponse {
    const params = request.params as AttestationRecommendationParams;
    const recommend = this.randomFloat() > 0.2 ? 'grant' : 'defer';

    const payload: AttestationRecommendation = {
      type: CAPABILITY_ATTESTATION_RECOMMENDATION,
      contentId: params.contentId,
      recommend: recommend as 'grant' | 'deny' | 'defer',
      attestationType: params.requestedAttestationType,
      suggestedReach: recommend === 'grant' ? 'community' : undefined,
      conditions:
        recommend === 'defer' ? ['Requires peer review', 'Additional evidence needed'] : undefined,
      reasoning:
        recommend === 'grant'
          ? 'Content meets quality standards and aligns with constitutional principles'
          : 'Additional review recommended before granting attestation',
    };

    return {
      ...baseResponse,
      status: 'fulfilled',
      constitutionalReasoning: {
        primaryPrinciple: 'Boundaries around freedom of reach',
        interpretation:
          'Attestations expand content reach; must ensure content serves flourishing before broader distribution',
        valuesWeighed: [
          {
            value: 'Content quality',
            weight: 0.35,
            direction: recommend === 'grant' ? 'for' : 'against',
          },
          { value: 'Community protection', weight: 0.3, direction: 'for' },
          { value: 'Author recognition', weight: 0.2, direction: 'for' },
          { value: 'Knowledge sharing', weight: 0.15, direction: 'for' },
        ],
        confidence: recommend === 'grant' ? 0.8 : 0.6,
        precedents: ['attestation-standard-v1.0'],
      },
      payload,
    } as ElohimResponse;
  }

  private handlePathRecommendation(
    request: ElohimRequest,
    baseResponse: Partial<ElohimResponse>
  ): ElohimResponse {
    const params = request.params as PathAnalysisParams;

    const payload: PathAnalysisResult = {
      type: 'path-analysis',
      pathId: params.pathId || 'know-thyself',
      analysisType: 'learning-objectives',
      findings: [
        {
          aspect: 'Discovery Profile Match',
          status: 'good',
          details: 'Your discovery results suggest this path aligns with your learning style.',
          suggestions: ['Start with the self-discovery chapter', 'Focus on reflective exercises'],
        },
      ],
      overallAssessment: 'This path aligns well with your emerging profile.',
    };

    return {
      ...baseResponse,
      status: 'fulfilled',
      constitutionalReasoning: {
        primaryPrinciple: 'Paths serve discovery, not conformity',
        interpretation:
          "Path recommendations should align with the learner's unique profile " +
          'and respect their sovereignty in choosing their learning journey.',
        valuesWeighed: [
          { value: 'Learner sovereignty', weight: 0.4, direction: 'for' },
          { value: 'Profile alignment', weight: 0.3, direction: 'for' },
          { value: 'Growth trajectory', weight: 0.3, direction: 'for' },
        ],
        confidence: 0.72,
      },
      payload,
    } as ElohimResponse;
  }

  private generateDefaultReasoning(capability: ElohimCapability): ConstitutionalReasoning {
    return {
      primaryPrinciple: 'Love as committed action toward flourishing',
      interpretation: `Capability '${capability}' exercised in service of human flourishing`,
      valuesWeighed: [
        { value: 'Human dignity', weight: 0.5, direction: 'for' },
        { value: 'Collective wellbeing', weight: 0.5, direction: 'for' },
      ],
      confidence: 0.75,
    };
  }

  private handleReachNegotiation(
    request: ElohimRequest,
    baseResponse: Partial<ElohimResponse>
  ): ElohimResponse {
    const trustScore = this.randomFloat() * 0.5 + 0.5; // 0.5-1.0 for demo
    const requestedReach = 'community'; // Default assumption for mock
    const approved = trustScore > 0.6;

    const payload = {
      type: CAPABILITY_REACH_NEGOTIATION as 'reach-negotiation',
      recommendedReach: approved ? requestedReach : 'local',
      reachDelta: approved
        ? undefined
        : {
            requested: requestedReach,
            recommended: 'local',
            reason: 'Author trust score below threshold for requested reach',
          },
      safetyCleared: true,
      conditions: approved ? [] : ['Earn community endorsements to expand reach'],
      transparencyReport: {
        layersConsidered: ['constitutional', 'author-profile', 'graph-neighborhood', 'payload'],
        decisionFactors: [
          {
            factor: 'Author trust score',
            influence: approved ? 'positive' : 'negative',
            weight: 0.4,
            evidence: `Trust score: ${trustScore.toFixed(2)}`,
          },
          {
            factor: 'Content safety',
            influence: 'positive',
            weight: 0.3,
            evidence: 'No safety issues detected',
          },
          {
            factor: 'Community health',
            influence: 'neutral',
            weight: 0.2,
            evidence: 'Community activity level: normal',
          },
          {
            factor: 'Graph neighborhood',
            influence: 'positive',
            weight: 0.1,
            evidence: 'Content aligns with neighborhood patterns',
          },
        ],
        authorStanding: `Trust score ${trustScore.toFixed(2)} — ${approved ? 'good standing' : 'building trust'}`,
        communityHealth: 'Normal activity, low flag rate',
        neighborhoodInfluence: 'Content graph neighborhood is healthy',
      },
    };

    return {
      ...baseResponse,
      status: 'fulfilled',
      constitutionalReasoning: {
        primaryPrinciple: 'Free speech does not mean free reach',
        interpretation:
          "Reach is negotiated before distribution. The author's trust profile " +
          'and content context determine the appropriate reach level.',
        valuesWeighed: [
          { value: 'Author sovereignty', weight: 0.3, direction: 'for' },
          { value: 'Community protection', weight: 0.3, direction: approved ? 'for' : 'against' },
          { value: 'Knowledge access', weight: 0.2, direction: 'for' },
          { value: 'Constitutional alignment', weight: 0.2, direction: 'for' },
        ],
        confidence: approved ? 0.88 : 0.72,
        precedents: ['content-reach-v1.0', 'trust-adaptive-pipeline-v1.0'],
      },
      payload,
    } as ElohimResponse;
  }

  private estimateProcessingTime(capability: ElohimCapability): number {
    const times: Partial<Record<ElohimCapability, number>> = {
      'content-safety-review': 1500,
      'accuracy-verification': 2000,
      [CAPABILITY_ATTESTATION_RECOMMENDATION]: 1200,
      'knowledge-map-synthesis': 3000,
      'spiral-detection': 800,
      'path-analysis': 2500,
      'path-recommendation': 1800,
      [CAPABILITY_REACH_NEGOTIATION]: 2000,
    };
    return times[capability] ?? 1000;
  }

  private randomInt(min: number, max: number): number {
    return (
      Math.floor((crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32) * (max - min)) + min
    );
  }

  private randomFloat(): number {
    return crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32;
  }
}
