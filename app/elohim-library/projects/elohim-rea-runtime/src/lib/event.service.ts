/**
 * EventService - Domain service for economic events via elohim-storage.
 *
 * Migrated to @elohim/rea-runtime from @app/shefa/services/event.service
 * as part of Wave 2 Slice 2.4 of the cross-pillar import cleanup sprint.
 *
 * This service provides high-level operations for hREA economic events:
 * - Recording content interactions (views, completions, assessments)
 * - Recording path progress events
 * - Querying events for analytics
 *
 * NOTE: This service uses the elohim-storage SQLite backend via the injected
 * IEconomicEventApi token. The concrete StorageApiService implementation is
 * provided by elohim-app at DI root level.
 *
 * ValueFlows/hREA Action Types:
 * - 'use': Consuming a resource (e.g., viewing content)
 * - 'produce': Creating value (e.g., completing an assessment)
 * - 'transfer': Moving between agents (e.g., recognition transfer)
 * - 'cite': Attribution/citation
 * - 'appreciate': Recognition/appreciation
 *
 * TODO(slice-2.1): When StorageApiService migrates to @elohim/service, update
 * the concrete provider registration in elohim-app's DI root to point to the
 * new import path. The EVENT_API token itself needs no change.
 */

import { Injectable, InjectionToken, inject } from '@angular/core';

import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';

import type { EconomicEventView } from '@elohim/storage-client/generated';

import { type LamadEventType, LamadEventTypes, REAActions } from './rea-action-types';
import { type ProtocolEventType, PROTOCOL_EVENT_MAPPINGS } from './protocol-event-types';

export { LamadEventTypes, REAActions };
export type { LamadEventType };

// =============================================================================
// IEconomicEventApi — minimal interface for economic event API calls
// =============================================================================

/**
 * Input shape for creating an economic event.
 * Matches the StorageApiService.createEconomicEvent parameter.
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
 * Matches the StorageApiService.getEconomicEvents parameter.
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
 */
export interface IEconomicEventApi {
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
  // Protocol-Level Content Interaction
  // ===========================================================================

  /**
   * Record a content interaction as a protocol-level REA event.
   *
   * This is the generic protocol primitive. A "view" is an attention resource
   * event. A "complete" is an achievement resource event. The interaction type
   * determines the REA action and resource mapping.
   */
  recordContentInteraction(
    agentId: string,
    contentId: string,
    interactionType: ProtocolEventType
  ): Observable<EconomicEventView> {
    const mapping = PROTOCOL_EVENT_MAPPINGS[interactionType];
    return this.eventApi.createEconomicEvent({
      action: mapping.action,
      provider: agentId,
      receiver: contentId,
      lamadEventType: interactionType,
      contentId,
    });
  }

  // ===========================================================================
  // Content Interaction Events (deprecated — use recordContentInteraction)
  // ===========================================================================

  /**
   * @deprecated Use recordContentInteraction(agentId, contentId, 'content-view') instead.
   */
  recordContentView(agentId: string, contentId: string): Observable<EconomicEventView> {
    return this.recordContentInteraction(agentId, contentId, 'content-view');
  }

  /**
   * @deprecated Use recordContentInteraction(agentId, contentId, 'content-complete') instead.
   */
  recordContentComplete(agentId: string, contentId: string): Observable<EconomicEventView> {
    return this.recordContentInteraction(agentId, contentId, 'content-complete');
  }

  // ===========================================================================
  // Path Progress Events
  // ===========================================================================

  /**
   * Record path step completion.
   */
  recordStepComplete(
    agentId: string,
    pathId: string,
    stepId: string
  ): Observable<EconomicEventView> {
    return this.eventApi.createEconomicEvent({
      action: REAActions.PRODUCE,
      provider: agentId,
      receiver: agentId,
      lamadEventType: LamadEventTypes.PATH_STEP_COMPLETE,
      pathId,
      metadata: { stepId },
    });
  }

  /**
   * Record path completion.
   */
  recordPathComplete(agentId: string, pathId: string): Observable<EconomicEventView> {
    return this.eventApi.createEconomicEvent({
      action: REAActions.PRODUCE,
      provider: agentId,
      receiver: agentId,
      lamadEventType: LamadEventTypes.PATH_COMPLETE,
      pathId,
    });
  }

  // ===========================================================================
  // Assessment Events
  // ===========================================================================

  /**
   * Record assessment start.
   */
  recordAssessmentStart(
    agentId: string,
    contentId: string,
    assessmentId: string
  ): Observable<EconomicEventView> {
    return this.eventApi.createEconomicEvent({
      action: REAActions.USE,
      provider: agentId,
      receiver: contentId,
      lamadEventType: LamadEventTypes.ASSESSMENT_START,
      contentId,
      metadata: { assessmentId },
    });
  }

  /**
   * Record assessment completion.
   */
  recordAssessmentComplete(
    agentId: string,
    contentId: string,
    assessmentId: string,
    score?: number
  ): Observable<EconomicEventView> {
    return this.eventApi.createEconomicEvent({
      action: REAActions.PRODUCE,
      provider: agentId,
      receiver: agentId,
      lamadEventType: LamadEventTypes.ASSESSMENT_COMPLETE,
      contentId,
      metadata: { assessmentId, score },
    });
  }

  /**
   * Record quiz submission.
   */
  recordQuizSubmit(
    agentId: string,
    contentId: string,
    quizId: string,
    correct: boolean,
    score?: number
  ): Observable<EconomicEventView> {
    return this.eventApi.createEconomicEvent({
      action: REAActions.PRODUCE,
      provider: agentId,
      receiver: agentId,
      lamadEventType: LamadEventTypes.QUIZ_SUBMIT,
      contentId,
      metadata: { quizId, correct, score },
    });
  }

  // ===========================================================================
  // Recognition Events
  // ===========================================================================

  /**
   * Record recognition given to a contributor.
   */
  recordRecognitionGiven(
    fromAgentId: string,
    toPresenceId: string,
    contentId: string,
    amount = 1
  ): Observable<EconomicEventView> {
    return this.eventApi.createEconomicEvent({
      action: REAActions.APPRECIATE,
      provider: fromAgentId,
      receiver: toPresenceId,
      lamadEventType: LamadEventTypes.RECOGNITION_GIVEN,
      contentId,
      contributorPresenceId: toPresenceId,
      resourceQuantity: { value: amount, unit: 'recognition' },
    });
  }

  // ===========================================================================
  // Query Methods
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

  /**
   * Count events of a specific type for content.
   */
  countEventsForContent(contentId: string, lamadEventType?: LamadEventType): Observable<number> {
    return this.eventApi
      .getEconomicEvents({
        contentId,
        eventTypes: lamadEventType ? [lamadEventType] : undefined,
      })
      .pipe(map(events => events.length));
  }

  /**
   * Get view count for content.
   */
  getViewCount(contentId: string): Observable<number> {
    return this.countEventsForContent(contentId, LamadEventTypes.CONTENT_VIEW);
  }

  /**
   * Get completion count for content.
   */
  getCompletionCount(contentId: string): Observable<number> {
    return this.countEventsForContent(contentId, LamadEventTypes.CONTENT_COMPLETE);
  }

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
