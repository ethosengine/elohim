/**
 * EventService - Domain service for economic events via elohim-storage.
 *
 * This service provides high-level operations for hREA economic events:
 * - Recording content interactions (views, completions, assessments)
 * - Recording path progress events
 * - Querying events for analytics
 *
 * NOTE: This service uses the elohim-storage SQLite backend via StorageApiService.
 * For Holochain-backed events, see EconomicService.
 *
 * ValueFlows/hREA Action Types:
 * - 'use': Consuming a resource (e.g., viewing content)
 * - 'produce': Creating value (e.g., completing an assessment)
 * - 'transfer': Moving between agents (e.g., recognition transfer)
 * - 'cite': Attribution/citation
 * - 'appreciate': Recognition/appreciation
 *
 * Uses StorageApiService for HTTP communication with elohim-storage.
 */

import { Injectable } from '@angular/core';
import { Observable, forkJoin, of } from 'rxjs';
import { map } from 'rxjs/operators';

import {
  StorageApiService,
  CreateEventInput,
} from '@app/elohim/services/storage-api.service';
import { EconomicEventView } from '@app/elohim/adapters/storage-types.adapter';
import { EventQuery } from '@app/elohim/models/economic-event.model';

/**
 * Lamad-specific event types (extends hREA actions with domain semantics)
 */
export const LamadEventTypes = {
  CONTENT_VIEW: 'content-view',
  CONTENT_COMPLETE: 'content-complete',
  PATH_STEP_COMPLETE: 'path-step-complete',
  PATH_COMPLETE: 'path-complete',
  ASSESSMENT_START: 'assessment-start',
  ASSESSMENT_COMPLETE: 'assessment-complete',
  PRACTICE_ATTEMPT: 'practice-attempt',
  QUIZ_SUBMIT: 'quiz-submit',
  RECOGNITION_GIVEN: 'recognition-given',
  RECOGNITION_RECEIVED: 'recognition-received',
} as const;

export type LamadEventType = typeof LamadEventTypes[keyof typeof LamadEventTypes];

/**
 * hREA action types
 */
export const REAActions = {
  USE: 'use',
  PRODUCE: 'produce',
  TRANSFER: 'transfer',
  CITE: 'cite',
  APPRECIATE: 'appreciate',
} as const;

export type REAAction = typeof REAActions[keyof typeof REAActions];

@Injectable({
  providedIn: 'root',
})
export class EventService {
  constructor(private storageApi: StorageApiService) {}

  // ===========================================================================
  // Content Interaction Events
  // ===========================================================================

  /**
   * Record a content view event.
   */
  recordContentView(agentId: string, contentId: string): Observable<EconomicEventView> {
    return this.storageApi.createEconomicEvent({
      action: REAActions.USE,
      provider: agentId,
      receiver: contentId,
      lamadEventType: LamadEventTypes.CONTENT_VIEW,
      contentId,
    });
  }

  /**
   * Record content completion.
   */
  recordContentComplete(agentId: string, contentId: string): Observable<EconomicEventView> {
    return this.storageApi.createEconomicEvent({
      action: REAActions.PRODUCE,
      provider: agentId,
      receiver: agentId,
      lamadEventType: LamadEventTypes.CONTENT_COMPLETE,
      contentId,
    });
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
    return this.storageApi.createEconomicEvent({
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
    return this.storageApi.createEconomicEvent({
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
    return this.storageApi.createEconomicEvent({
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
    return this.storageApi.createEconomicEvent({
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
    return this.storageApi.createEconomicEvent({
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
    amount: number = 1
  ): Observable<EconomicEventView> {
    return this.storageApi.createEconomicEvent({
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
    return this.storageApi.getEconomicEvents({ agentId });
  }

  /**
   * Get events for specific content.
   */
  getEventsForContent(contentId: string): Observable<EconomicEventView[]> {
    return this.storageApi.getEconomicEvents({ contentId });
  }

  /**
   * Get events for a path.
   */
  getEventsForPath(pathId: string): Observable<EconomicEventView[]> {
    return this.storageApi.getEconomicEvents({ pathId });
  }

  /**
   * Get events by Lamad event type.
   */
  getEventsByType(lamadEventType: LamadEventType): Observable<EconomicEventView[]> {
    return this.storageApi.getEconomicEvents({ lamadEventType });
  }

  /**
   * Get recent events for an agent.
   */
  getRecentEvents(agentId: string, limit: number = 50): Observable<EconomicEventView[]> {
    return this.storageApi.getEconomicEvents({ agentId, limit });
  }

  // ===========================================================================
  // Analytics Helpers
  // ===========================================================================

  /**
   * Count events of a specific type for content.
   */
  countEventsForContent(
    contentId: string,
    lamadEventType?: LamadEventType
  ): Observable<number> {
    return this.storageApi.getEconomicEvents({
      contentId,
      lamadEventType,
    }).pipe(
      map(events => events.length)
    );
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
    return this.storageApi.getEconomicEvents({
      agentId,
      contentId,
      lamadEventType: LamadEventTypes.CONTENT_VIEW,
    }).pipe(
      map(events => events.length > 0)
    );
  }

  /**
   * Check if an agent has completed content.
   */
  hasCompleted(agentId: string, contentId: string): Observable<boolean> {
    return this.storageApi.getEconomicEvents({
      agentId,
      contentId,
      lamadEventType: LamadEventTypes.CONTENT_COMPLETE,
    }).pipe(
      map(events => events.length > 0)
    );
  }
}
