/**
 * EventService - Domain service for economic events via elohim-storage.
 *
 * Migrated to @elohim/rea-runtime from @app/shefa/services/event.service
 * as part of Wave 2 Slice 2.4 of the cross-pillar import cleanup sprint.
 *
 * ## M-REA-1: emitEvent primary API
 *
 * `emitEvent(intent)` is the single canonical entry point. Callers pass a
 * `LamadEventIntent` discriminated union; the substrate (elohim-storage
 * `POST /api/v1/lamad/events`) composes the full EconomicEvent
 * (action, provider, receiver) using PROTOCOL_EVENT_MAPPINGS. No client-side
 * REA composition needed.
 *
 * The `record*` methods below delegate to `emitEvent` and are kept only for
 * backwards compatibility. New call sites MUST use `emitEvent` directly.
 *
 * ValueFlows/hREA Action Types:
 * - 'use': Consuming a resource (e.g., viewing content)
 * - 'produce': Creating value (e.g., completing an assessment)
 * - 'transfer': Moving between agents (e.g., recognition transfer)
 * - 'cite': Attribution/citation
 * - 'raise': Recognition/appreciation (used by recognition-given)
 */

import { Injectable, InjectionToken, inject } from '@angular/core';

import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';

import type { EconomicEventView } from '@elohim/storage-client/generated';
// LamadEventIntent: local re-declaration that stays in sync with the
// canonical generated file in elohim-service/src/generated/lamad-event-intent.ts.
// Source of truth: elohim/sdk/schemas/v1/intents/lamad-event-intent.schema.json
// TODO(post-M-REA-1): add @elohim/service tsconfig path alias + import from there.
import type { LamadEventIntent } from './lamad-event-intent.types';

import { type LamadEventType, LamadEventTypes, REAActions } from './rea-action-types';

export { LamadEventTypes, REAActions };
export type { LamadEventType };

// =============================================================================
// IEconomicEventApi — minimal interface for economic event API calls
// =============================================================================

/**
 * Input shape for creating an economic event via the legacy direct-REA path.
 * Kept for StorageApiService backwards compatibility.
 * New code uses `emitLamadIntent` instead.
 */
export interface CreateEconomicEventParams {
  action: string;
  provider: string;
  receiver: string;
  lamadEventType?: string;
  contentId?: string;
  pathId?: string;
  contributorPresenceId?: string;
  resourceQuantity?: { value: number; unit: string };
  metadata?: Record<string, unknown>;
}

/**
 * Query parameters for fetching economic events.
 */
export interface EconomicEventQuery {
  agentId?: string;
  contentId?: string;
  pathId?: string;
  eventTypes?: string[];
  limit?: number;
}

/**
 * IEconomicEventApi — the minimal HTTP API surface EventService needs.
 *
 * StorageApiService in elohim-app implements this interface. Libraries
 * inject this token; elohim-app provides the concrete implementation.
 *
 * `emitLamadIntent` calls `POST /api/v1/lamad/events` — the M-REA-1
 * conductor-first path where the substrate composes the REA shape.
 *
 * `createEconomicEvent` calls `POST /api/v1/economic-events` — kept for
 * backwards compatibility with existing callers that still build REA params
 * client-side. M-REA-2 will retire this path once all callers migrate to
 * `emitLamadIntent`.
 */
export interface IEconomicEventApi {
  emitLamadIntent(intent: LamadEventIntent): Observable<EconomicEventView>;
  createEconomicEvent(params: CreateEconomicEventParams): Observable<EconomicEventView>;
  getEconomicEvents(query: EconomicEventQuery): Observable<EconomicEventView[]>;
}

/**
 * DI token for the economic event API.
 *
 * elohim-app must provide StorageApiService under this token:
 *   { provide: EVENT_API, useExisting: StorageApiService }
 *
 * Tests provide a mock:
 *   { provide: EVENT_API, useValue: mockEventApi }
 */
export const EVENT_API = new InjectionToken<IEconomicEventApi>('EventApi', {
  factory: () => {
    throw new Error(
      '[rea-runtime] EVENT_API not provided. ' +
        'Add { provide: EVENT_API, useExisting: StorageApiService } to your app providers.'
    );
  },
});

// =============================================================================
// EventService
// =============================================================================

@Injectable({
  providedIn: 'root',
})
export class EventService {
  private readonly eventApi = inject(EVENT_API);

  // ===========================================================================
  // Primary API — M-REA-1
  // ===========================================================================

  /**
   * Emit a lamad event intent to the substrate.
   *
   * The substrate (`POST /api/v1/lamad/events`) composes the full EconomicEvent
   * (action, provider, receiver) from the intent using PROTOCOL_EVENT_MAPPINGS.
   * No client-side REA composition required.
   *
   * This is the single canonical call site for all new code.
   *
   * @example
   * ```ts
   * eventService.emitEvent({
   *   agentId: session.agentId,
   *   lamadEventType: 'content-view',
   *   contentId: content.id,
   * }).subscribe();
   * ```
   */
  emitEvent(intent: LamadEventIntent): Observable<EconomicEventView> {
    return this.eventApi.emitLamadIntent(intent);
  }

  // ===========================================================================
  // Deprecated — kept for backwards compatibility, delegate to emitEvent
  // ===========================================================================

  /**
   * Record a content interaction as a protocol-level REA event.
   *
   * @deprecated Use emitEvent({ agentId, lamadEventType: interactionType, contentId }) instead.
   */
  recordContentInteraction(
    agentId: string,
    contentId: string,
    interactionType: LamadEventType
  ): Observable<EconomicEventView> {
    // LamadEventType is a superset of the schema intent types; the deprecated
    // method accepts the wider type for backwards compat. The substrate validates.
    return this.emitEvent({ agentId, lamadEventType: interactionType as LamadEventIntent['lamadEventType'], contentId });
  }

  /**
   * @deprecated Use emitEvent({ agentId, lamadEventType: 'content-view', contentId }) instead.
   */
  recordContentView(agentId: string, contentId: string): Observable<EconomicEventView> {
    return this.emitEvent({ agentId, lamadEventType: 'content-view', contentId });
  }

  /**
   * @deprecated Use emitEvent({ agentId, lamadEventType: 'content-complete', contentId }) instead.
   */
  recordContentComplete(agentId: string, contentId: string): Observable<EconomicEventView> {
    return this.emitEvent({ agentId, lamadEventType: 'content-complete', contentId });
  }

  /**
   * Record path step completion.
   *
   * @deprecated Use emitEvent({ agentId, lamadEventType: 'path-step-complete', pathId, metadata: { stepId } }) instead.
   */
  recordStepComplete(
    agentId: string,
    pathId: string,
    stepId: string
  ): Observable<EconomicEventView> {
    return this.emitEvent({
      agentId,
      lamadEventType: 'path-step-complete',
      pathId,
      metadata: { stepId },
    });
  }

  /**
   * Record path completion.
   *
   * @deprecated Use emitEvent({ agentId, lamadEventType: 'path-complete', pathId }) instead.
   */
  recordPathComplete(agentId: string, pathId: string): Observable<EconomicEventView> {
    return this.emitEvent({ agentId, lamadEventType: 'path-complete', pathId });
  }

  /**
   * Record assessment start.
   *
   * @deprecated Use emitEvent({ agentId, lamadEventType: 'assessment-start', contentId, metadata: { assessmentId } }) instead.
   */
  recordAssessmentStart(
    agentId: string,
    contentId: string,
    assessmentId: string
  ): Observable<EconomicEventView> {
    return this.emitEvent({
      agentId,
      lamadEventType: 'assessment-start',
      contentId,
      metadata: { assessmentId },
    });
  }

  /**
   * Record assessment completion.
   *
   * @deprecated Use emitEvent({ agentId, lamadEventType: 'assessment-complete', contentId, metadata: { assessmentId, score } }) instead.
   */
  recordAssessmentComplete(
    agentId: string,
    contentId: string,
    assessmentId: string,
    score?: number
  ): Observable<EconomicEventView> {
    return this.emitEvent({
      agentId,
      lamadEventType: 'assessment-complete',
      contentId,
      metadata: { assessmentId, score },
    });
  }

  /**
   * Record quiz submission.
   *
   * @deprecated Use emitEvent({ agentId, lamadEventType: 'quiz-submit', contentId, metadata: { quizId, correct, score } }) instead.
   */
  recordQuizSubmit(
    agentId: string,
    contentId: string,
    quizId: string,
    correct: boolean,
    score?: number
  ): Observable<EconomicEventView> {
    return this.emitEvent({
      agentId,
      lamadEventType: 'quiz-submit',
      contentId,
      metadata: { quizId, correct, score },
    });
  }

  /**
   * Record recognition given to a contributor.
   *
   * @deprecated Use emitEvent({ agentId, lamadEventType: 'recognition-given', contentId, contributorPresenceId, resourceQuantityValue: amount, resourceQuantityUnit: 'recognition' }) instead.
   */
  recordRecognitionGiven(
    fromAgentId: string,
    toPresenceId: string,
    contentId: string,
    amount = 1
  ): Observable<EconomicEventView> {
    return this.emitEvent({
      agentId: fromAgentId,
      lamadEventType: 'recognition-given',
      contentId,
      contributorPresenceId: toPresenceId,
      resourceQuantityValue: amount,
      resourceQuantityUnit: 'recognition',
    });
  }

  // ===========================================================================
  // Query Methods (unchanged)
  // ===========================================================================

  /**
   * Get events for a specific agent.
   */
  getEventsForAgent(agentId: string): Observable<EconomicEventView[]> {
    return this.eventApi.getEconomicEvents({ agentId });
  }

  /**
   * Get events for specific content.
   */
  getEventsForContent(contentId: string): Observable<EconomicEventView[]> {
    return this.eventApi.getEconomicEvents({ contentId });
  }

  /**
   * Get events for a path.
   */
  getEventsForPath(pathId: string): Observable<EconomicEventView[]> {
    return this.eventApi.getEconomicEvents({ pathId });
  }

  /**
   * Get events by Lamad event type.
   */
  getEventsByType(lamadEventType: LamadEventType): Observable<EconomicEventView[]> {
    return this.eventApi.getEconomicEvents({ eventTypes: [lamadEventType] });
  }

  /**
   * Get recent events for an agent.
   */
  getRecentEvents(agentId: string, limit = 50): Observable<EconomicEventView[]> {
    return this.eventApi.getEconomicEvents({ agentId, limit });
  }

  // ===========================================================================
  // Analytics Helpers
  // ===========================================================================
  //
  // M-AGGR-3: countEventsForContent, getViewCount, getCompletionCount retired.
  // These methods queried EconomicEvents and counted client-side (F-AGGR-3 smell).
  // Callers should use ContentApiService.getContentEngagement(contentId) which
  // reads the server-side ContentEngagementStatsView projection at
  // GET /api/v1/lamad/content/{contentId}/engagement.
  // TODO(rust-migration): remove retirement comment once all consumers migrated.

  /**
   * Check if an agent has viewed content.
   */
  hasViewed(agentId: string, contentId: string): Observable<boolean> {
    return this.eventApi
      .getEconomicEvents({
        agentId,
        contentId,
        eventTypes: [LamadEventTypes.CONTENT_VIEW],
      })
      .pipe(map(events => events.length > 0));
  }

  /**
   * Check if an agent has completed content.
   */
  hasCompleted(agentId: string, contentId: string): Observable<boolean> {
    return this.eventApi
      .getEconomicEvents({
        agentId,
        contentId,
        eventTypes: [LamadEventTypes.CONTENT_COMPLETE],
      })
      .pipe(map(events => events.length > 0));
  }
}
