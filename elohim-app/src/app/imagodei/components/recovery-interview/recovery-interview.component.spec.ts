/**
 * RecoveryInterviewComponent Tests
 *
 * Tests for interviewer UI for recovery attestations.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RecoveryInterviewComponent } from './recovery-interview.component';
import { RecoveryCoordinatorService } from '../../services/recovery-coordinator.service';
import { signal } from '@angular/core';

describe('RecoveryInterviewComponent', () => {
  let component: RecoveryInterviewComponent;
  let fixture: ComponentFixture<RecoveryInterviewComponent>;
  let mockRecoveryService: jasmine.SpyObj<RecoveryCoordinatorService>;

  beforeEach(async () => {
    // Create mocks
    mockRecoveryService = jasmine.createSpyObj(
      'RecoveryCoordinatorService',
      [
        'loadPendingRequests',
        'startInterview',
        'generateQuestions',
        'submitAttestation',
        'rejectAttestation',
        'abandonInterview',
        'submitResponse',
        'clearError',
      ],
      {
        pendingRequests: signal([]),
        conductingInterview: signal(null),
        isLoading: signal(false),
        error: signal(null),
      }
    );

    await TestBed.configureTestingModule({
      imports: [RecoveryInterviewComponent],
      providers: [{ provide: RecoveryCoordinatorService, useValue: mockRecoveryService }],
    }).compileComponents();

    fixture = TestBed.createComponent(RecoveryInterviewComponent);
    component = fixture.componentInstance;
  });

  // ==========================================================================
  // Component Creation
  // ==========================================================================

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Delegated Signals
  // ==========================================================================

  it('should delegate pendingRequests from RecoveryCoordinatorService', () => {
    expect(component.pendingRequests).toBeDefined();
  });

  it('should delegate conductingInterview from RecoveryCoordinatorService', () => {
    expect(component.conductingInterview).toBeDefined();
  });

  it('should delegate isLoading from RecoveryCoordinatorService', () => {
    expect(component.isLoading).toBeDefined();
  });

  it('should delegate error from RecoveryCoordinatorService', () => {
    expect(component.error).toBeDefined();
  });

  // ==========================================================================
  // Signals
  // ==========================================================================

  it('should have viewMode signal', () => {
    expect(component.viewMode).toBeDefined();
  });

  it('should have currentQuestionIndex signal', () => {
    expect(component.currentQuestionIndex).toBeDefined();
  });

  it('should have questions signal', () => {
    expect(component.questions).toBeDefined();
  });

  it('should have answers signal', () => {
    expect(component.answers).toBeDefined();
  });

  // ==========================================================================
  // Form Data
  // ==========================================================================

  it('should initialize form data', () => {
    expect(component.decision).toBe('abstain');
    expect(component.confidence).toBe(50);
    expect(component.notes).toBe('');
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have loadPendingRequests method', () => {
    expect(component.loadPendingRequests).toBeDefined();
    expect(typeof component.loadPendingRequests).toBe('function');
  });

  it('should have startInterview method', () => {
    expect(component.startInterview).toBeDefined();
    expect(typeof component.startInterview).toBe('function');
  });

  it('should have submitAnswer method', () => {
    expect(component.submitAnswer).toBeDefined();
    expect(typeof component.submitAnswer).toBe('function');
  });

  it('should have submitAttestation method', () => {
    expect(component.submitAttestation).toBeDefined();
    expect(typeof component.submitAttestation).toBe('function');
  });

  it('should have abandonInterview method', () => {
    expect(component.abandonInterview).toBeDefined();
    expect(typeof component.abandonInterview).toBe('function');
  });

  // ==========================================================================
  // Initial State
  // ==========================================================================

  it('should start with queue view mode', () => {
    expect(component.viewMode()).toBe('queue');
  });

  it('should initialize currentQuestionIndex to 0', () => {
    expect(component.currentQuestionIndex()).toBe(0);
  });

  it('should initialize with empty questions', () => {
    expect(component.questions()).toEqual([]);
  });

  it('should initialize with empty answers', () => {
    expect(component.answers().size).toBe(0);
  });

  // ==========================================================================
  // Abandon Interview
  // ==========================================================================

  it('should return to queue when abandoning interview', () => {
    component.viewMode.set('interview');
    component.abandonInterview();
    expect(component.viewMode()).toBe('queue');
  });

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  it('should implement OnInit', () => {
    expect(component.ngOnInit).toBeDefined();
    expect(typeof component.ngOnInit).toBe('function');
  });

  it('should call loadPendingRequests on init', async () => {
    mockRecoveryService.loadPendingRequests.and.returnValue(Promise.resolve());

    await component.ngOnInit();

    expect(mockRecoveryService.loadPendingRequests).toHaveBeenCalled();
  });

  // ==========================================================================
  // Start Interview
  // ==========================================================================

  describe('startInterview', () => {
    const mockQuestions = [
      {
        id: 'q1',
        type: 'network-history' as const,
        question: 'When did you join?',
        difficulty: 1,
        points: 10,
        verifiable: true,
      },
      {
        id: 'q2',
        type: 'relationship' as const,
        question: 'Who did you meet first?',
        difficulty: 2,
        points: 20,
        verifiable: true,
      },
    ];

    it('should start interview and load questions on success', async () => {
      mockRecoveryService.startInterview.and.returnValue(Promise.resolve(true));
      mockRecoveryService.generateQuestions.and.returnValue(Promise.resolve(mockQuestions));

      await component.startInterview('recovery-123');

      expect(mockRecoveryService.startInterview).toHaveBeenCalledWith('recovery-123');
      expect(mockRecoveryService.generateQuestions).toHaveBeenCalledWith('recovery-123');
      expect(component.questions()).toEqual(mockQuestions);
      expect(component.viewMode()).toBe('interview');
      expect(component.currentQuestionIndex()).toBe(0);
    });

    it('should not change view if interview start fails', async () => {
      mockRecoveryService.startInterview.and.returnValue(Promise.resolve(false));
      component.viewMode.set('queue');

      await component.startInterview('recovery-123');

      expect(component.viewMode()).toBe('queue');
      expect(mockRecoveryService.generateQuestions).not.toHaveBeenCalled();
    });

    it('should reset answers when starting new interview', async () => {
      component.answers.set(new Map([['old-q', 'old answer']]));

      mockRecoveryService.startInterview.and.returnValue(Promise.resolve(true));
      mockRecoveryService.generateQuestions.and.returnValue(Promise.resolve(mockQuestions));

      await component.startInterview('recovery-123');

      expect(component.answers().size).toBe(0);
    });
  });

  // ==========================================================================
  // Submit Answer
  // ==========================================================================

  describe('submitAnswer', () => {
    const mockInterview = {
      id: 'interview-123',
      requestId: 'recovery-123',
      interviewerId: 'interviewer-456',
      interviewerDisplayName: 'Jane',
      status: 'in-progress' as const,
      questions: [],
      responses: [],
      startedAt: new Date(),
    };

    const mockQuestions = [
      {
        id: 'q1',
        type: 'network-history' as const,
        question: 'First question?',
        difficulty: 1,
        points: 10,
        verifiable: true,
      },
      {
        id: 'q2',
        type: 'relationship' as const,
        question: 'Second question?',
        difficulty: 2,
        points: 20,
        verifiable: true,
      },
    ];

    beforeEach(() => {
      // Update the existing signal instead of replacing it
      (mockRecoveryService.conductingInterview as any).set(mockInterview);
      component.questions.set(mockQuestions);
      component.currentQuestionIndex.set(0);
      component.viewMode.set('interview'); // Set to interview mode for these tests
    });

    it('should do nothing if no conducting interview', async () => {
      // Update the existing signal to null instead of replacing it
      (mockRecoveryService.conductingInterview as any).set(null);

      await component.submitAnswer();

      expect(mockRecoveryService.submitResponse).not.toHaveBeenCalled();
    });

    it('should do nothing if current index out of bounds', async () => {
      component.currentQuestionIndex.set(10);

      await component.submitAnswer();

      expect(mockRecoveryService.submitResponse).not.toHaveBeenCalled();
    });

    it('should do nothing if answer is empty', async () => {
      component.answers.set(new Map([['q1', '']]));

      await component.submitAnswer();

      expect(mockRecoveryService.submitResponse).not.toHaveBeenCalled();
    });

    it('should do nothing if answer is only whitespace', async () => {
      component.answers.set(new Map([['q1', '   ']]));

      await component.submitAnswer();

      expect(mockRecoveryService.submitResponse).not.toHaveBeenCalled();
    });

    it('should submit answer and move to next question', async () => {
      component.answers.set(new Map([['q1', 'My answer']]));
      mockRecoveryService.submitResponse.and.returnValue(Promise.resolve(null));

      await component.submitAnswer();

      expect(mockRecoveryService.submitResponse).toHaveBeenCalledWith('q1', 'My answer');
      expect(component.currentQuestionIndex()).toBe(1);
      expect(component.viewMode()).toBe('interview');
    });

    it('should move to attestation view after last question', async () => {
      component.currentQuestionIndex.set(1);
      component.answers.set(new Map([['q2', 'Final answer']]));
      mockRecoveryService.submitResponse.and.returnValue(Promise.resolve(null));

      await component.submitAnswer();

      expect(mockRecoveryService.submitResponse).toHaveBeenCalledWith('q2', 'Final answer');
      expect(component.viewMode()).toBe('attestation');
    });
  });

  // ==========================================================================
  // Answer Management
  // ==========================================================================

  describe('answer management', () => {
    it('should set answer for question', () => {
      component.setAnswer('q1', 'My answer');

      expect(component.answers().get('q1')).toBe('My answer');
    });

    it('should update existing answer', () => {
      component.answers.set(new Map([['q1', 'Old answer']]));

      component.setAnswer('q1', 'New answer');

      expect(component.answers().get('q1')).toBe('New answer');
    });

    it('should handle textarea input event', () => {
      const textarea = document.createElement('textarea');
      textarea.value = 'User typed this';
      const event = { target: textarea } as unknown as Event;

      component.onAnswerInput(event, 'q1');

      expect(component.answers().get('q1')).toBe('User typed this');
    });
  });

  // ==========================================================================
  // Question Navigation
  // ==========================================================================

  describe('previousQuestion', () => {
    it('should go to previous question', () => {
      component.currentQuestionIndex.set(2);

      component.previousQuestion();

      expect(component.currentQuestionIndex()).toBe(1);
    });

    it('should not go below zero', () => {
      component.currentQuestionIndex.set(0);

      component.previousQuestion();

      expect(component.currentQuestionIndex()).toBe(0);
    });
  });

  // ==========================================================================
  // Submit Attestation
  // ==========================================================================

  describe('submitAttestation', () => {
    it('should submit attestation with all form data', async () => {
      component.decision = 'affirm';
      component.confidence = 85;
      component.notes = 'Strong match';

      mockRecoveryService.submitAttestation.and.returnValue(Promise.resolve(true));

      await component.submitAttestation();

      expect(mockRecoveryService.submitAttestation).toHaveBeenCalledWith('affirm', 85, 'Strong match');
    });

    it('should reset state and return to queue on success', async () => {
      component.decision = 'affirm';
      component.confidence = 90;
      component.notes = 'Verified';
      component.questions.set([
        {
          id: 'q1',
          type: 'network-history',
          question: 'Test?',
          difficulty: 1,
          points: 10,
          verifiable: true,
        },
      ]);
      component.answers.set(new Map([['q1', 'answer']]));
      component.currentQuestionIndex.set(1);

      mockRecoveryService.submitAttestation.and.returnValue(Promise.resolve(true));

      await component.submitAttestation();

      expect(component.viewMode()).toBe('queue');
      expect(component.questions()).toEqual([]);
      expect(component.answers().size).toBe(0);
      expect(component.currentQuestionIndex()).toBe(0);
      expect(component.decision).toBe('abstain');
      expect(component.confidence).toBe(50);
      expect(component.notes).toBe('');
    });

    it('should not change state on failure', async () => {
      component.decision = 'affirm';
      component.viewMode.set('attestation');

      mockRecoveryService.submitAttestation.and.returnValue(Promise.resolve(false));

      await component.submitAttestation();

      expect(component.viewMode()).toBe('attestation');
      expect(component.decision).toBe('affirm');
    });

    it('should pass undefined for empty notes', async () => {
      component.notes = '';
      mockRecoveryService.submitAttestation.and.returnValue(Promise.resolve(true));

      await component.submitAttestation();

      expect(mockRecoveryService.submitAttestation).toHaveBeenCalledWith(
        jasmine.any(String),
        jasmine.any(Number),
        undefined
      );
    });

    it('should trim notes before checking if empty', async () => {
      component.notes = '   ';
      mockRecoveryService.submitAttestation.and.returnValue(Promise.resolve(true));

      await component.submitAttestation();

      expect(mockRecoveryService.submitAttestation).toHaveBeenCalledWith(
        jasmine.any(String),
        jasmine.any(Number),
        undefined
      );
    });
  });

  // ==========================================================================
  // Helper Methods
  // ==========================================================================

  describe('getCurrentQuestion', () => {
    it('should return current question', () => {
      const questions = [
        {
          id: 'q1',
          type: 'network-history' as const,
          question: 'First?',
          difficulty: 1,
          points: 10,
          verifiable: true,
        },
        {
          id: 'q2',
          type: 'relationship' as const,
          question: 'Second?',
          difficulty: 2,
          points: 20,
          verifiable: true,
        },
      ];
      component.questions.set(questions);
      component.currentQuestionIndex.set(1);

      const current = component.getCurrentQuestion();

      expect(current).toEqual(questions[1]);
    });

    it('should return null if index out of bounds', () => {
      component.questions.set([
        {
          id: 'q1',
          type: 'network-history',
          question: 'Test?',
          difficulty: 1,
          points: 10,
          verifiable: true,
        },
      ]);
      component.currentQuestionIndex.set(5);

      const current = component.getCurrentQuestion();

      expect(current).toBeNull();
    });

    it('should return null if no questions', () => {
      component.questions.set([]);
      component.currentQuestionIndex.set(0);

      const current = component.getCurrentQuestion();

      expect(current).toBeNull();
    });
  });

  describe('getCurrentAnswer', () => {
    it('should return current answer', () => {
      const question = {
        id: 'q1',
        type: 'network-history' as const,
        question: 'Test?',
        difficulty: 1,
        points: 10,
        verifiable: true,
      };
      component.questions.set([question]);
      component.currentQuestionIndex.set(0);
      component.answers.set(new Map([['q1', 'My answer']]));

      const answer = component.getCurrentAnswer();

      expect(answer).toBe('My answer');
    });

    it('should return empty string if no answer yet', () => {
      const question = {
        id: 'q1',
        type: 'network-history' as const,
        question: 'Test?',
        difficulty: 1,
        points: 10,
        verifiable: true,
      };
      component.questions.set([question]);
      component.currentQuestionIndex.set(0);
      component.answers.set(new Map());

      const answer = component.getCurrentAnswer();

      expect(answer).toBe('');
    });

    it('should return empty string if no current question', () => {
      component.questions.set([]);
      component.currentQuestionIndex.set(0);

      const answer = component.getCurrentAnswer();

      expect(answer).toBe('');
    });
  });

  describe('getProgressPercentage', () => {
    it('should return 0 for no questions', () => {
      component.questions.set([]);

      expect(component.getProgressPercentage()).toBe(0);
    });

    it('should calculate progress percentage', () => {
      component.questions.set([
        {
          id: 'q1',
          type: 'network-history',
          question: 'Test1?',
          difficulty: 1,
          points: 10,
          verifiable: true,
        },
        {
          id: 'q2',
          type: 'relationship',
          question: 'Test2?',
          difficulty: 2,
          points: 20,
          verifiable: true,
        },
        {
          id: 'q3',
          type: 'content',
          question: 'Test3?',
          difficulty: 1,
          points: 10,
          verifiable: true,
        },
      ]);
      component.currentQuestionIndex.set(1);

      expect(component.getProgressPercentage()).toBe(33);
    });

    it('should round progress percentage', () => {
      component.questions.set([
        { id: 'q1', type: 'network-history', question: '1', difficulty: 1, points: 10, verifiable: true },
        { id: 'q2', type: 'relationship', question: '2', difficulty: 1, points: 10, verifiable: true },
        { id: 'q3', type: 'content', question: '3', difficulty: 1, points: 10, verifiable: true },
      ]);
      component.currentQuestionIndex.set(2);

      expect(component.getProgressPercentage()).toBe(67);
    });
  });

  describe('getConfidenceLabel', () => {
    it('should return "Very High" for confidence >= 90', () => {
      component.confidence = 95;
      expect(component.getConfidenceLabel()).toBe('Very High');
    });

    it('should return "High" for confidence >= 70', () => {
      component.confidence = 75;
      expect(component.getConfidenceLabel()).toBe('High');
    });

    it('should return "Moderate" for confidence >= 50', () => {
      component.confidence = 60;
      expect(component.getConfidenceLabel()).toBe('Moderate');
    });

    it('should return "Low" for confidence >= 30', () => {
      component.confidence = 40;
      expect(component.getConfidenceLabel()).toBe('Low');
    });

    it('should return "Very Low" for confidence < 30', () => {
      component.confidence = 20;
      expect(component.getConfidenceLabel()).toBe('Very Low');
    });

    it('should handle boundary values correctly', () => {
      component.confidence = 90;
      expect(component.getConfidenceLabel()).toBe('Very High');

      component.confidence = 70;
      expect(component.getConfidenceLabel()).toBe('High');

      component.confidence = 50;
      expect(component.getConfidenceLabel()).toBe('Moderate');

      component.confidence = 30;
      expect(component.getConfidenceLabel()).toBe('Low');
    });
  });

  describe('getDecisionClass', () => {
    it('should return "decision-affirm" for affirm decision', () => {
      component.decision = 'affirm';
      expect(component.getDecisionClass()).toBe('decision-affirm');
    });

    it('should return "decision-deny" for deny decision', () => {
      component.decision = 'deny';
      expect(component.getDecisionClass()).toBe('decision-deny');
    });

    it('should return "decision-abstain" for abstain decision', () => {
      component.decision = 'abstain';
      expect(component.getDecisionClass()).toBe('decision-abstain');
    });
  });

  describe('trackBy functions', () => {
    it('trackByRequestId should return request ID', () => {
      const request = {
        requestId: 'req-123',
        maskedIdentity: 'joh***',
        doorwayName: 'Main',
        createdAt: new Date(),
        expiresIn: 1000,
        progress: {
          affirmCount: 0,
          denyCount: 0,
          abstainCount: 0,
          requiredCount: 3,
          progressPercent: 0,
          thresholdMet: false,
          isDenied: false,
        },
        alreadyAttested: false,
        priority: 5,
      };

      expect(component.trackByRequestId(0, request)).toBe('req-123');
    });

    it('trackByQuestionId should return question ID', () => {
      const question = {
        id: 'q-456',
        type: 'network-history' as const,
        question: 'Test?',
        difficulty: 1,
        points: 10,
        verifiable: true,
      };

      expect(component.trackByQuestionId(0, question)).toBe('q-456');
    });
  });

  // ==========================================================================
  // Clear Error
  // ==========================================================================

  describe('clearError', () => {
    it('should call service clearError', () => {
      component.clearError();

      expect(mockRecoveryService.clearError).toHaveBeenCalled();
    });
  });
});
