import { Injectable } from '@angular/core';

// @coverage: 96.9% (2026-02-24)

import { map, switchMap } from 'rxjs/operators';

import { Observable, from, of } from 'rxjs';

import {
  ElohimAgent,
  ElohimCapability,
  ElohimLayer,
  ElohimRequest,
  ElohimResponse,
  ElohimIndexEntry,
  ElohimSelectionCriteria,
} from '../models/elohim-agent.model';

import { DataLoaderService } from './data-loader.service';
import { ElohimBackendCatalog, MockBackend } from './elohim-backend';

/**
 * ElohimAgentService - Interface to autonomous constitutional guardians.
 *
 * This service provides the protocol for invoking Elohim agents.
 * Invocations are dispatched to a pluggable backend (mock, Anthropic, native).
 * The ElohimBackendCatalog selects the best available backend with fallback.
 *
 * Key principles:
 * - Elohim are invoked, not commanded
 * - Every response includes constitutional reasoning
 * - Elohim can decline requests that violate their principles
 * - Layer-appropriate Elohim are selected automatically when requested
 */
@Injectable({ providedIn: 'root' })
export class ElohimAgentService {
  private readonly elohimCache = new Map<string, ElohimAgent>();
  private requestLog: ElohimRequest[] = [];

  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly catalog: ElohimBackendCatalog
  ) {
    // Register MockBackend as default fallback.
    // NativeBackend is registered lazily by ElohimConfigComponent
    // when native backend is selected, to avoid network calls at init time.
    this.catalog.register(new MockBackend());
  }

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
          capabilities: (e.capabilities ?? []) as ElohimCapability[],
          visibility: e.visibility as 'public' | 'private',
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
          bio: agent.bio ?? '',
          attestations: agent.attestations ?? [],
          capabilities: (agent.capabilities ?? []) as ElohimCapability[],
          visibility: agent.visibility as 'public' | 'private',
          familyId: agent.familyId,
          createdAt: agent.createdAt,
          updatedAt: agent.updatedAt,
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
        const capable = elohimList.filter(e => e.capabilities.includes(criteria.capability));

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
   * The request is dispatched to the best available backend via the catalog.
   */
  invoke(request: ElohimRequest): Observable<ElohimResponse> {
    // Log the request
    this.requestLog.push(request);

    // Resolve target Elohim
    const targetElohim$ =
      request.targetElohimId === 'auto'
        ? this.selectElohim({ capability: request.capability })
        : this.getElohim(request.targetElohimId);

    return targetElohim$.pipe(
      switchMap(elohim => {
        if (!elohim) {
          return of(
            this.createDeclinedResponse(
              request,
              'unknown',
              'No Elohim available for this capability'
            )
          );
        }

        // Check capability
        if (!elohim.capabilities.includes(request.capability)) {
          return of(
            this.createDeclinedResponse(
              request,
              elohim.id,
              `This Elohim does not have the '${request.capability}' capability`
            )
          );
        }

        // Dispatch to backend
        return from(this.catalog.selectBackend()).pipe(
          switchMap(backend => backend.invoke(request, elohim))
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
        reviewType,
      },
      requesterId,
      priority: 'normal',
      requestedAt: new Date().toISOString(),
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
        evidence,
      },
      requesterId,
      priority: 'normal',
      requestedAt: new Date().toISOString(),
    };

    return this.invoke(request);
  }

  // =========================================================================
  // Helpers
  // =========================================================================

  private generateRequestId(): string {
    const randomBytes = crypto.getRandomValues(new Uint8Array(8));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 9);
    return `req-${Date.now()}-${randomStr}`;
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
        confidence: 1,
      },
      declineReason: reason,
      respondedAt: new Date().toISOString(),
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
