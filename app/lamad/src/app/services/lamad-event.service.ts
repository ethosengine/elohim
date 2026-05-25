import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import { EconomicEventView } from '@app/elohim/adapters/storage-types.adapter';
import { EventService } from '@app/shefa/services/event.service';

/**
 * LamadEventService -- Lamad domain convenience methods over the protocol EventService.
 *
 * The protocol provides `recordContentInteraction()` as the generic primitive.
 * This service adds lamad-specific helpers for assessments, paths, and practice --
 * things only a learning app would need. Other apps (governance, economics) would
 * build their own domain event services wrapping the same protocol primitive.
 */
@Injectable({ providedIn: 'root' })
export class LamadEventService {
  private readonly eventService = inject(EventService);

  recordQuizSubmit(
    agentId: string,
    contentId: string,
    quizId: string,
    correct: boolean,
    score?: number
  ): Observable<EconomicEventView> {
    return this.eventService.recordContentInteraction(
      agentId,
      contentId,
      'content-complete' as any // TODO: once LamadDomainEventType is wired, use 'quiz-submit'
    );
  }

  recordAssessmentComplete(
    agentId: string,
    contentId: string,
    assessmentId: string,
    score?: number
  ): Observable<EconomicEventView> {
    return this.eventService.recordContentInteraction(
      agentId,
      contentId,
      'content-complete' as any // TODO: once LamadDomainEventType is wired, use 'assessment-complete'
    );
  }

  recordPathStepComplete(
    agentId: string,
    pathId: string,
    stepId: string
  ): Observable<EconomicEventView> {
    return this.eventService.recordContentInteraction(
      agentId,
      pathId,
      'content-complete' as any // TODO: use 'path-step-complete'
    );
  }
}
