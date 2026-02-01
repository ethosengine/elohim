/**
 * Event Service Tests
 *
 * Tests the domain service for economic events that wraps StorageApiService.
 * Verifies correct mapping of Lamad event types to hREA actions.
 */

import { TestBed } from '@angular/core/testing';

import { of, throwError } from 'rxjs';

import { EconomicEventView } from '@app/elohim/adapters/storage-types.adapter';
import { StorageApiService } from '@app/elohim/services/storage-api.service';

import { EventService, LamadEventTypes, REAActions } from './event.service';

describe('EventService', () => {
  let service: EventService;
  let storageApiMock: jasmine.SpyObj<StorageApiService>;

  // Use partial mock with type cast to avoid having to specify all required fields
  const mockEventView = {
    id: 'event-1',
    action: 'use',
    provider: 'agent-1',
    receiver: 'content-1',
    createdAt: new Date().toISOString(),
  } as unknown as EconomicEventView;

  beforeEach(() => {
    storageApiMock = jasmine.createSpyObj('StorageApiService', [
      'createEconomicEvent',
      'getEconomicEvents',
    ]);

    // Default: return mock event for create, empty array for queries
    storageApiMock.createEconomicEvent.and.returnValue(of(mockEventView));
    storageApiMock.getEconomicEvents.and.returnValue(of([]));

    TestBed.configureTestingModule({
      providers: [EventService, { provide: StorageApiService, useValue: storageApiMock }],
    });
    service = TestBed.inject(EventService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ==========================================================================
  // Content Interaction Events
  // ==========================================================================

  describe('recordContentView', () => {
    it('should create USE event with CONTENT_VIEW type', done => {
      service.recordContentView('agent-1', 'content-1').subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            action: REAActions.USE,
            provider: 'agent-1',
            receiver: 'content-1',
            lamadEventType: LamadEventTypes.CONTENT_VIEW,
            contentId: 'content-1',
          })
        );
        expect(result).toEqual(mockEventView);
        done();
      });
    });
  });

  describe('recordContentComplete', () => {
    it('should create PRODUCE event with CONTENT_COMPLETE type', done => {
      service.recordContentComplete('agent-1', 'content-1').subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            action: REAActions.PRODUCE,
            provider: 'agent-1',
            receiver: 'agent-1', // Self-receiver for completion
            lamadEventType: LamadEventTypes.CONTENT_COMPLETE,
            contentId: 'content-1',
          })
        );
        done();
      });
    });
  });

  // ==========================================================================
  // Path Progress Events
  // ==========================================================================

  describe('recordStepComplete', () => {
    it('should create PRODUCE event with PATH_STEP_COMPLETE type', done => {
      service.recordStepComplete('agent-1', 'path-1', 'step-1').subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            action: REAActions.PRODUCE,
            provider: 'agent-1',
            receiver: 'agent-1',
            lamadEventType: LamadEventTypes.PATH_STEP_COMPLETE,
            pathId: 'path-1',
            metadata: { stepId: 'step-1' },
          })
        );
        done();
      });
    });
  });

  describe('recordPathComplete', () => {
    it('should create PRODUCE event with PATH_COMPLETE type', done => {
      service.recordPathComplete('agent-1', 'path-1').subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            action: REAActions.PRODUCE,
            provider: 'agent-1',
            receiver: 'agent-1',
            lamadEventType: LamadEventTypes.PATH_COMPLETE,
            pathId: 'path-1',
          })
        );
        done();
      });
    });
  });

  // ==========================================================================
  // Assessment Events
  // ==========================================================================

  describe('recordAssessmentStart', () => {
    it('should create USE event with ASSESSMENT_START type', done => {
      service.recordAssessmentStart('agent-1', 'content-1', 'assessment-1').subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            action: REAActions.USE,
            provider: 'agent-1',
            receiver: 'content-1',
            lamadEventType: LamadEventTypes.ASSESSMENT_START,
            contentId: 'content-1',
            metadata: { assessmentId: 'assessment-1' },
          })
        );
        done();
      });
    });
  });

  describe('recordAssessmentComplete', () => {
    it('should create PRODUCE event with ASSESSMENT_COMPLETE type', done => {
      service
        .recordAssessmentComplete('agent-1', 'content-1', 'assessment-1', 95)
        .subscribe(result => {
          expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
            jasmine.objectContaining({
              action: REAActions.PRODUCE,
              provider: 'agent-1',
              receiver: 'agent-1',
              lamadEventType: LamadEventTypes.ASSESSMENT_COMPLETE,
              contentId: 'content-1',
              metadata: { assessmentId: 'assessment-1', score: 95 },
            })
          );
          done();
        });
    });

    it('should handle missing score', done => {
      service.recordAssessmentComplete('agent-1', 'content-1', 'assessment-1').subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            metadata: { assessmentId: 'assessment-1', score: undefined },
          })
        );
        done();
      });
    });
  });

  describe('recordQuizSubmit', () => {
    it('should create PRODUCE event with QUIZ_SUBMIT type', done => {
      service.recordQuizSubmit('agent-1', 'content-1', 'quiz-1', true, 100).subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            action: REAActions.PRODUCE,
            provider: 'agent-1',
            receiver: 'agent-1',
            lamadEventType: LamadEventTypes.QUIZ_SUBMIT,
            contentId: 'content-1',
            metadata: { quizId: 'quiz-1', correct: true, score: 100 },
          })
        );
        done();
      });
    });

    it('should handle incorrect answer', done => {
      service.recordQuizSubmit('agent-1', 'content-1', 'quiz-1', false, 0).subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            metadata: { quizId: 'quiz-1', correct: false, score: 0 },
          })
        );
        done();
      });
    });
  });

  // ==========================================================================
  // Recognition Events
  // ==========================================================================

  describe('recordRecognitionGiven', () => {
    it('should create APPRECIATE event with RECOGNITION_GIVEN type', done => {
      service.recordRecognitionGiven('agent-1', 'presence-1', 'content-1', 5).subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            action: REAActions.APPRECIATE,
            provider: 'agent-1',
            receiver: 'presence-1',
            lamadEventType: LamadEventTypes.RECOGNITION_GIVEN,
            contentId: 'content-1',
            contributorPresenceId: 'presence-1',
            resourceQuantity: { value: 5, unit: 'recognition' },
          })
        );
        done();
      });
    });

    it('should default amount to 1', done => {
      service.recordRecognitionGiven('agent-1', 'presence-1', 'content-1').subscribe(result => {
        expect(storageApiMock.createEconomicEvent).toHaveBeenCalledWith(
          jasmine.objectContaining({
            resourceQuantity: { value: 1, unit: 'recognition' },
          })
        );
        done();
      });
    });
  });

  // ==========================================================================
  // Query Methods
  // ==========================================================================

  describe('getEventsForAgent', () => {
    it('should query events by agentId', done => {
      const mockEvents: EconomicEventView[] = [mockEventView];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.getEventsForAgent('agent-1').subscribe(result => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({ agentId: 'agent-1' });
        expect(result).toEqual(mockEvents);
        done();
      });
    });
  });

  describe('getEventsForContent', () => {
    it('should query events by contentId', done => {
      const mockEvents: EconomicEventView[] = [mockEventView];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.getEventsForContent('content-1').subscribe(result => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({ contentId: 'content-1' });
        expect(result).toEqual(mockEvents);
        done();
      });
    });
  });

  describe('getEventsForPath', () => {
    it('should query events by pathId', done => {
      const mockEvents: EconomicEventView[] = [mockEventView];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.getEventsForPath('path-1').subscribe(result => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({ pathId: 'path-1' });
        expect(result).toEqual(mockEvents);
        done();
      });
    });
  });

  describe('getEventsByType', () => {
    it('should query events by lamadEventType', done => {
      const mockEvents: EconomicEventView[] = [mockEventView];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.getEventsByType(LamadEventTypes.CONTENT_VIEW).subscribe(result => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({
          eventTypes: [LamadEventTypes.CONTENT_VIEW],
        });
        expect(result).toEqual(mockEvents);
        done();
      });
    });
  });

  describe('getRecentEvents', () => {
    it('should query with default limit of 50', done => {
      const mockEvents: EconomicEventView[] = [mockEventView];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.getRecentEvents('agent-1').subscribe(result => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({
          agentId: 'agent-1',
          limit: 50,
        });
        done();
      });
    });

    it('should accept custom limit', done => {
      const mockEvents: EconomicEventView[] = [mockEventView];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.getRecentEvents('agent-1', 100).subscribe(result => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({
          agentId: 'agent-1',
          limit: 100,
        });
        done();
      });
    });
  });

  // ==========================================================================
  // Analytics Helpers
  // ==========================================================================

  describe('countEventsForContent', () => {
    it('should return count of events', done => {
      const mockEvents: EconomicEventView[] = [
        { ...mockEventView, id: 'event-1' },
        { ...mockEventView, id: 'event-2' },
        { ...mockEventView, id: 'event-3' },
      ];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.countEventsForContent('content-1').subscribe(count => {
        expect(count).toBe(3);
        done();
      });
    });

    it('should filter by event type when provided', done => {
      const mockEvents: EconomicEventView[] = [mockEventView];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.countEventsForContent('content-1', LamadEventTypes.CONTENT_VIEW).subscribe(count => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({
          contentId: 'content-1',
          eventTypes: [LamadEventTypes.CONTENT_VIEW],
        });
        done();
      });
    });
  });

  describe('getViewCount', () => {
    it('should count CONTENT_VIEW events', done => {
      const mockEvents: EconomicEventView[] = [mockEventView, mockEventView];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.getViewCount('content-1').subscribe(count => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({
          contentId: 'content-1',
          eventTypes: [LamadEventTypes.CONTENT_VIEW],
        });
        expect(count).toBe(2);
        done();
      });
    });
  });

  describe('getCompletionCount', () => {
    it('should count CONTENT_COMPLETE events', done => {
      const mockEvents: EconomicEventView[] = [mockEventView];
      storageApiMock.getEconomicEvents.and.returnValue(of(mockEvents));

      service.getCompletionCount('content-1').subscribe(count => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({
          contentId: 'content-1',
          eventTypes: [LamadEventTypes.CONTENT_COMPLETE],
        });
        expect(count).toBe(1);
        done();
      });
    });
  });

  describe('hasViewed', () => {
    it('should return true when view event exists', done => {
      storageApiMock.getEconomicEvents.and.returnValue(of([mockEventView]));

      service.hasViewed('agent-1', 'content-1').subscribe(result => {
        expect(result).toBeTrue();
        done();
      });
    });

    it('should return false when no view event exists', done => {
      storageApiMock.getEconomicEvents.and.returnValue(of([]));

      service.hasViewed('agent-1', 'content-1').subscribe(result => {
        expect(result).toBeFalse();
        done();
      });
    });
  });

  describe('hasCompleted', () => {
    it('should return true when completion event exists', done => {
      storageApiMock.getEconomicEvents.and.returnValue(of([mockEventView]));

      service.hasCompleted('agent-1', 'content-1').subscribe(result => {
        expect(result).toBeTrue();
        done();
      });
    });

    it('should return false when no completion event exists', done => {
      storageApiMock.getEconomicEvents.and.returnValue(of([]));

      service.hasCompleted('agent-1', 'content-1').subscribe(result => {
        expect(result).toBeFalse();
        done();
      });
    });

    it('should query with correct parameters', done => {
      storageApiMock.getEconomicEvents.and.returnValue(of([]));

      service.hasCompleted('agent-1', 'content-1').subscribe(result => {
        expect(storageApiMock.getEconomicEvents).toHaveBeenCalledWith({
          agentId: 'agent-1',
          contentId: 'content-1',
          eventTypes: [LamadEventTypes.CONTENT_COMPLETE],
        });
        done();
      });
    });
  });

  // ==========================================================================
  // Constants Verification
  // ==========================================================================

  describe('LamadEventTypes', () => {
    it('should have all expected event types', () => {
      expect(LamadEventTypes.CONTENT_VIEW).toBe('content-view');
      expect(LamadEventTypes.CONTENT_COMPLETE).toBe('content-complete');
      expect(LamadEventTypes.PATH_STEP_COMPLETE).toBe('path-step-complete');
      expect(LamadEventTypes.PATH_COMPLETE).toBe('path-complete');
      expect(LamadEventTypes.ASSESSMENT_START).toBe('assessment-start');
      expect(LamadEventTypes.ASSESSMENT_COMPLETE).toBe('assessment-complete');
      expect(LamadEventTypes.PRACTICE_ATTEMPT).toBe('practice-attempt');
      expect(LamadEventTypes.QUIZ_SUBMIT).toBe('quiz-submit');
      expect(LamadEventTypes.RECOGNITION_GIVEN).toBe('recognition-given');
      expect(LamadEventTypes.RECOGNITION_RECEIVED).toBe('recognition-received');
    });
  });

  describe('REAActions', () => {
    it('should have all expected hREA action types', () => {
      expect(REAActions.USE).toBe('use');
      expect(REAActions.PRODUCE).toBe('produce');
      expect(REAActions.TRANSFER).toBe('transfer');
      expect(REAActions.CITE).toBe('cite');
      expect(REAActions.APPRECIATE).toBe('appreciate');
    });
  });
});
