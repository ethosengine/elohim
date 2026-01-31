/**
 * Event Service Tests
 *
 * Tests the event service which provides high-level operations for hREA
 * economic events via elohim-storage SQLite backend.
 */

import { TestBed } from '@angular/core/testing';

import { StorageApiService } from '@app/elohim/services/storage-api.service';

import { EventService, LamadEventTypes, REAActions } from './event.service';

describe('EventService', () => {
  let service: EventService;
  let storageApiMock: jasmine.SpyObj<StorageApiService>;

  beforeEach(() => {
    storageApiMock = jasmine.createSpyObj('StorageApiService', [
      'createEconomicEvent',
      'getEconomicEvents',
    ]);

    TestBed.configureTestingModule({
      providers: [
        EventService,
        { provide: StorageApiService, useValue: storageApiMock },
      ],
    });
    service = TestBed.inject(EventService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('LamadEventTypes constants', () => {
    it('should have CONTENT_VIEW type', () => {
      expect(LamadEventTypes.CONTENT_VIEW).toBe('content-view');
    });

    it('should have CONTENT_COMPLETE type', () => {
      expect(LamadEventTypes.CONTENT_COMPLETE).toBe('content-complete');
    });

    it('should have PATH_STEP_COMPLETE type', () => {
      expect(LamadEventTypes.PATH_STEP_COMPLETE).toBe('path-step-complete');
    });

    it('should have PATH_COMPLETE type', () => {
      expect(LamadEventTypes.PATH_COMPLETE).toBe('path-complete');
    });

    it('should have ASSESSMENT_START type', () => {
      expect(LamadEventTypes.ASSESSMENT_START).toBe('assessment-start');
    });

    it('should have ASSESSMENT_COMPLETE type', () => {
      expect(LamadEventTypes.ASSESSMENT_COMPLETE).toBe('assessment-complete');
    });

    it('should have QUIZ_SUBMIT type', () => {
      expect(LamadEventTypes.QUIZ_SUBMIT).toBe('quiz-submit');
    });

    it('should have RECOGNITION_GIVEN type', () => {
      expect(LamadEventTypes.RECOGNITION_GIVEN).toBe('recognition-given');
    });

    it('should have RECOGNITION_RECEIVED type', () => {
      expect(LamadEventTypes.RECOGNITION_RECEIVED).toBe('recognition-received');
    });
  });

  describe('REAActions constants', () => {
    it('should have USE action', () => {
      expect(REAActions.USE).toBe('use');
    });

    it('should have PRODUCE action', () => {
      expect(REAActions.PRODUCE).toBe('produce');
    });

    it('should have TRANSFER action', () => {
      expect(REAActions.TRANSFER).toBe('transfer');
    });

    it('should have CITE action', () => {
      expect(REAActions.CITE).toBe('cite');
    });

    it('should have APPRECIATE action', () => {
      expect(REAActions.APPRECIATE).toBe('appreciate');
    });
  });

  describe('Content Interaction Events', () => {
    describe('recordContentView', () => {
      it('should have recordContentView method', () => {
        expect(service.recordContentView).toBeDefined();
        expect(typeof service.recordContentView).toBe('function');
      });

      it('should call storageApi.createEconomicEvent', () => {
        const agentId = 'agent-1';
        const contentId = 'content-1';

        service.recordContentView(agentId, contentId);

        expect(storageApiMock.createEconomicEvent).toHaveBeenCalled();
      });
    });

    describe('recordContentComplete', () => {
      it('should have recordContentComplete method', () => {
        expect(service.recordContentComplete).toBeDefined();
        expect(typeof service.recordContentComplete).toBe('function');
      });

      it('should call storageApi.createEconomicEvent', () => {
        const agentId = 'agent-1';
        const contentId = 'content-1';

        service.recordContentComplete(agentId, contentId);

        expect(storageApiMock.createEconomicEvent).toHaveBeenCalled();
      });
    });
  });

  describe('Path Progress Events', () => {
    describe('recordStepComplete', () => {
      it('should have recordStepComplete method', () => {
        expect(service.recordStepComplete).toBeDefined();
        expect(typeof service.recordStepComplete).toBe('function');
      });

      it('should call storageApi.createEconomicEvent', () => {
        const agentId = 'agent-1';
        const pathId = 'path-1';
        const stepId = 'step-1';

        service.recordStepComplete(agentId, pathId, stepId);

        expect(storageApiMock.createEconomicEvent).toHaveBeenCalled();
      });
    });

    describe('recordPathComplete', () => {
      it('should have recordPathComplete method', () => {
        expect(service.recordPathComplete).toBeDefined();
        expect(typeof service.recordPathComplete).toBe('function');
      });

      it('should call storageApi.createEconomicEvent', () => {
        const agentId = 'agent-1';
        const pathId = 'path-1';

        service.recordPathComplete(agentId, pathId);

        expect(storageApiMock.createEconomicEvent).toHaveBeenCalled();
      });
    });
  });

  describe('Assessment Events', () => {
    describe('recordAssessmentStart', () => {
      it('should have recordAssessmentStart method', () => {
        expect(service.recordAssessmentStart).toBeDefined();
        expect(typeof service.recordAssessmentStart).toBe('function');
      });

      it('should call storageApi.createEconomicEvent', () => {
        const agentId = 'agent-1';
        const contentId = 'content-1';
        const assessmentId = 'assessment-1';

        service.recordAssessmentStart(agentId, contentId, assessmentId);

        expect(storageApiMock.createEconomicEvent).toHaveBeenCalled();
      });
    });

    describe('recordAssessmentComplete', () => {
      it('should have recordAssessmentComplete method', () => {
        expect(service.recordAssessmentComplete).toBeDefined();
        expect(typeof service.recordAssessmentComplete).toBe('function');
      });

      it('should call storageApi.createEconomicEvent', () => {
        const agentId = 'agent-1';
        const contentId = 'content-1';
        const assessmentId = 'assessment-1';

        service.recordAssessmentComplete(agentId, contentId, assessmentId);

        expect(storageApiMock.createEconomicEvent).toHaveBeenCalled();
      });
    });

    describe('recordQuizSubmit', () => {
      it('should have recordQuizSubmit method', () => {
        expect(service.recordQuizSubmit).toBeDefined();
        expect(typeof service.recordQuizSubmit).toBe('function');
      });

      it('should call storageApi.createEconomicEvent', () => {
        const agentId = 'agent-1';
        const contentId = 'content-1';
        const quizId = 'quiz-1';

        service.recordQuizSubmit(agentId, contentId, quizId, true);

        expect(storageApiMock.createEconomicEvent).toHaveBeenCalled();
      });
    });
  });

  describe('Recognition Events', () => {
    describe('recordRecognitionGiven', () => {
      it('should have recordRecognitionGiven method', () => {
        expect(service.recordRecognitionGiven).toBeDefined();
        expect(typeof service.recordRecognitionGiven).toBe('function');
      });

      it('should call storageApi.createEconomicEvent', () => {
        const fromAgentId = 'agent-1';
        const toPresenceId = 'presence-1';
        const contentId = 'content-1';

        service.recordRecognitionGiven(fromAgentId, toPresenceId, contentId);

        expect(storageApiMock.createEconomicEvent).toHaveBeenCalled();
      });

      it('should call storageApi.createEconomicEvent with amount parameter', () => {
        const fromAgentId = 'agent-1';
        const toPresenceId = 'presence-1';
        const contentId = 'content-1';
        const amount = 5;

        service.recordRecognitionGiven(fromAgentId, toPresenceId, contentId, amount);

        expect(storageApiMock.createEconomicEvent).toHaveBeenCalled();
      });
    });
  });

  describe('Query Methods', () => {
    describe('getEventsForAgent', () => {
      it('should have getEventsForAgent method', () => {
        expect(service.getEventsForAgent).toBeDefined();
        expect(typeof service.getEventsForAgent).toBe('function');
      });

      it('should call storageApi.getEconomicEvents', () => {
        const agentId = 'agent-1';

        service.getEventsForAgent(agentId);

        expect(storageApiMock.getEconomicEvents).toHaveBeenCalled();
      });
    });

    describe('getEventsForContent', () => {
      it('should have getEventsForContent method', () => {
        expect(service.getEventsForContent).toBeDefined();
        expect(typeof service.getEventsForContent).toBe('function');
      });

      it('should call storageApi.getEconomicEvents', () => {
        const contentId = 'content-1';

        service.getEventsForContent(contentId);

        expect(storageApiMock.getEconomicEvents).toHaveBeenCalled();
      });
    });

    describe('getEventsForPath', () => {
      it('should have getEventsForPath method', () => {
        expect(service.getEventsForPath).toBeDefined();
        expect(typeof service.getEventsForPath).toBe('function');
      });

      it('should call storageApi.getEconomicEvents', () => {
        const pathId = 'path-1';

        service.getEventsForPath(pathId);

        expect(storageApiMock.getEconomicEvents).toHaveBeenCalled();
      });
    });

    describe('getEventsByType', () => {
      it('should have getEventsByType method', () => {
        expect(service.getEventsByType).toBeDefined();
        expect(typeof service.getEventsByType).toBe('function');
      });

      it('should call storageApi.getEconomicEvents', () => {
        service.getEventsByType('content-view');

        expect(storageApiMock.getEconomicEvents).toHaveBeenCalled();
      });
    });

    describe('getRecentEvents', () => {
      it('should have getRecentEvents method', () => {
        expect(service.getRecentEvents).toBeDefined();
        expect(typeof service.getRecentEvents).toBe('function');
      });

      it('should call storageApi.getEconomicEvents', () => {
        const agentId = 'agent-1';

        service.getRecentEvents(agentId);

        expect(storageApiMock.getEconomicEvents).toHaveBeenCalled();
      });
    });
  });

  describe('Analytics Helpers', () => {
    describe('countEventsForContent', () => {
      it('should have countEventsForContent method', () => {
        expect(service.countEventsForContent).toBeDefined();
        expect(typeof service.countEventsForContent).toBe('function');
      });
    });

    describe('getViewCount', () => {
      it('should have getViewCount method', () => {
        expect(service.getViewCount).toBeDefined();
        expect(typeof service.getViewCount).toBe('function');
      });
    });

    describe('getCompletionCount', () => {
      it('should have getCompletionCount method', () => {
        expect(service.getCompletionCount).toBeDefined();
        expect(typeof service.getCompletionCount).toBe('function');
      });
    });

    describe('hasViewed', () => {
      it('should have hasViewed method', () => {
        expect(service.hasViewed).toBeDefined();
        expect(typeof service.hasViewed).toBe('function');
      });
    });

    describe('hasCompleted', () => {
      it('should have hasCompleted method', () => {
        expect(service.hasCompleted).toBeDefined();
        expect(typeof service.hasCompleted).toBe('function');
      });
    });
  });
});
