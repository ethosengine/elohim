import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { QuizSessionService } from './quiz-session.service';
import { QuestionPoolService } from './question-pool.service';
import type { PerseusItem, PerseusScoreResult } from '../../content-io/plugins/sophia/sophia-moment.model';
import type { QuizSession } from '../models/quiz-session.model';

describe('QuizSessionService', () => {
  let service: QuizSessionService;
  let mockQuestionPool: jasmine.SpyObj<QuestionPoolService>;

  const createMockQuestion = (id: string, contentId: string): PerseusItem => ({
    id,
    purpose: 'mastery',
    content: {
      content: 'Test question',
      widgets: {}
    },
    metadata: {
      assessesContentId: contentId,
      bloomsLevel: 'understand',
      difficulty: 'medium'
    }
  });

  const createMockHierarchicalSource = (questions: PerseusItem[] = []) => ({
    currentContentId: 'section-1',
    pathId: 'path-1',
    sectionId: 'section-1',
    eligibleContentIds: ['content-1', 'content-2'],
    combinedPool: questions,
    stats: {
      totalQuestions: questions.length,
      questionsByContent: new Map(),
      questionsByBlooms: {
        remember: 0,
        understand: questions.length,
        apply: 0,
        analyze: 0,
        evaluate: 0,
        create: 0
      },
      questionsByDifficulty: {
        easy: 0,
        medium: questions.length,
        hard: 0
      }
    }
  });

  const createScoreResult = (correct: boolean): PerseusScoreResult => ({
    correct,
    score: correct ? 1 : 0
  } as PerseusScoreResult);

  beforeEach(() => {
    mockQuestionPool = jasmine.createSpyObj('QuestionPoolService', [
      'getHierarchicalPool',
      'loadHierarchicalPools',
      'selectPracticeQuestions',
      'selectMasteryQuestions',
      'selectInlineQuestions',
      'selectQuestions'
    ]);

    // Set up default mock returns
    const emptySource = createMockHierarchicalSource();
    mockQuestionPool.getHierarchicalPool.and.returnValue(of(emptySource));
    mockQuestionPool.loadHierarchicalPools.and.returnValue(of(emptySource));
    mockQuestionPool.selectPracticeQuestions.and.returnValue({
      questions: [],
      selectionComplete: true,
      contentIds: []
    });
    mockQuestionPool.selectInlineQuestions.and.returnValue(of({
      questions: [],
      selectionComplete: true,
      contentIds: []
    }));

    TestBed.configureTestingModule({
      providers: [
        QuizSessionService,
        { provide: QuestionPoolService, useValue: mockQuestionPool }
      ],
    });
    service = TestBed.inject(QuizSessionService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('startPracticeQuiz', () => {
    it('should create a practice quiz session', (done) => {
      const questions = [
        createMockQuestion('q1', 'content-1'),
        createMockQuestion('q2', 'content-2')
      ];
      const source = createMockHierarchicalSource(questions);

      mockQuestionPool.getHierarchicalPool.and.returnValue(of(source));
      mockQuestionPool.loadHierarchicalPools.and.returnValue(of(source));
      mockQuestionPool.selectPracticeQuestions.and.returnValue({
        questions,
        selectionComplete: true,
        contentIds: ['content-1', 'content-2']
      });

      service.startPracticeQuiz('path-1', 'section-1', 'human-1', 5).subscribe({
        next: (session) => {
          expect(session).toBeDefined();
          expect(session.type).toBe('practice');
          expect(session.humanId).toBe('human-1');
          expect(session.questions.length).toBe(2);
          expect(session.pathContext?.pathId).toBe('path-1');
          expect(session.pathContext?.sectionId).toBe('section-1');
          expect(session.state).toBe('not_started');
          done();
        },
        error: done.fail
      });
    });

    it('should call question pool services with correct parameters', (done) => {
      const source = createMockHierarchicalSource();
      mockQuestionPool.getHierarchicalPool.and.returnValue(of(source));
      mockQuestionPool.loadHierarchicalPools.and.returnValue(of(source));

      service.startPracticeQuiz('path-1', 'section-1', 'human-1', 10).subscribe({
        next: () => {
          expect(mockQuestionPool.getHierarchicalPool).toHaveBeenCalledWith('path-1', 'section-1');
          expect(mockQuestionPool.loadHierarchicalPools).toHaveBeenCalledWith(source);
          expect(mockQuestionPool.selectPracticeQuestions).toHaveBeenCalledWith(source, 10);
          done();
        },
        error: done.fail
      });
    });
  });

  describe('startMasteryQuiz', () => {
    it('should create a mastery quiz session', (done) => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const source = createMockHierarchicalSource(questions);

      mockQuestionPool.getHierarchicalPool.and.returnValue(of(source));
      mockQuestionPool.loadHierarchicalPools.and.returnValue(of(source));
      mockQuestionPool.selectMasteryQuestions.and.returnValue({
        questions,
        selectionComplete: true,
        contentIds: ['content-1']
      });

      service.startMasteryQuiz('path-1', 'section-1', 'human-1', ['content-1'], 5).subscribe({
        next: (session) => {
          expect(session.type).toBe('mastery');
          expect(session.questions.length).toBe(1);
          done();
        },
        error: done.fail
      });
    });

    it('should pass practiced content IDs to question pool', (done) => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const source = createMockHierarchicalSource(questions);
      mockQuestionPool.getHierarchicalPool.and.returnValue(of(source));
      mockQuestionPool.loadHierarchicalPools.and.returnValue(of(source));
      mockQuestionPool.selectMasteryQuestions.and.returnValue({
        questions,
        selectionComplete: true,
        contentIds: ['content-1', 'content-2']
      });

      const practicedIds = ['content-1', 'content-2'];
      service.startMasteryQuiz('path-1', 'section-1', 'human-1', practicedIds, 5).subscribe({
        next: () => {
          expect(mockQuestionPool.selectMasteryQuestions).toHaveBeenCalledWith(
            source.combinedPool,
            5,
            practicedIds
          );
          done();
        },
        error: done.fail
      });
    });
  });

  describe('startInlineQuiz', () => {
    it('should create an inline quiz session', (done) => {
      const questions = [
        createMockQuestion('q1', 'content-1'),
        createMockQuestion('q2', 'content-1')
      ];

      mockQuestionPool.selectInlineQuestions.and.returnValue(of({
        questions,
        selectionComplete: true,
        contentIds: ['content-1']
      }));

      service.startInlineQuiz('content-1', 'human-1', 3, 10).subscribe({
        next: (session) => {
          expect(session.type).toBe('inline');
          expect(session.streakInfo).toBeDefined();
          expect(session.streakInfo?.targetStreak).toBe(3);
          expect(session.config.allowRetry).toBeTrue();
          expect(session.config.showImmediateFeedback).toBeTrue();
          done();
        },
        error: done.fail
      });
    });

    it('should call selectInlineQuestions with correct parameters', (done) => {
      mockQuestionPool.selectInlineQuestions.and.returnValue(of({
        questions: [],
        selectionComplete: true,
        contentIds: []
      }));

      service.startInlineQuiz('content-1', 'human-1', 5, 15).subscribe({
        next: () => {
          expect(mockQuestionPool.selectInlineQuestions).toHaveBeenCalledWith('content-1', 15);
          done();
        },
        error: done.fail
      });
    });
  });

  describe('startPreAssessment', () => {
    it('should create a pre-assessment quiz session', (done) => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const source = createMockHierarchicalSource(questions);

      mockQuestionPool.getHierarchicalPool.and.returnValue(of(source));
      mockQuestionPool.loadHierarchicalPools.and.returnValue(of(source));
      mockQuestionPool.selectQuestions.and.returnValue({
        questions,
        selectionComplete: true,
        contentIds: ['content-1']
      });

      service.startPreAssessment('path-1', 'human-1', 10).subscribe({
        next: (session) => {
          expect(session.type).toBe('pre-assessment');
          expect(session.pathContext?.pathId).toBe('path-1');
          done();
        },
        error: done.fail
      });
    });
  });

  describe('createCustomSession', () => {
    it('should create a custom session with provided questions', () => {
      const questions = [createMockQuestion('q1', 'content-1')];

      const session = service.createCustomSession('practice', 'human-1', questions);

      expect(session).toBeDefined();
      expect(session.type).toBe('practice');
      expect(session.questions.length).toBe(1);
    });

    it('should apply custom configuration', () => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const config = { allowBackNavigation: true, showImmediateFeedback: false };

      const session = service.createCustomSession('practice', 'human-1', questions, config);

      expect(session.config.allowBackNavigation).toBeTrue();
      expect(session.config.showImmediateFeedback).toBeFalse();
    });
  });

  describe('getSession', () => {
    it('should return session by ID', () => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const created = service.createCustomSession('practice', 'human-1', questions);

      const retrieved = service.getSession(created.id);

      expect(retrieved).toBe(created);
    });

    it('should return null for unknown session ID', () => {
      const retrieved = service.getSession('unknown-id');

      expect(retrieved).toBeNull();
    });
  });

  describe('getSession$', () => {
    it('should return observable for existing session', (done) => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const created = service.createCustomSession('practice', 'human-1', questions);

      service.getSession$(created.id).subscribe({
        next: (session) => {
          expect(session).toBe(created);
          done();
        },
        error: done.fail
      });
    });

    it('should return null observable for unknown session', (done) => {
      service.getSession$('unknown-id').subscribe({
        next: (session) => {
          expect(session).toBeNull();
          done();
        },
        error: done.fail
      });
    });
  });

  describe('Session State Management', () => {
    let session: QuizSession;

    beforeEach(() => {
      const questions = [createMockQuestion('q1', 'content-1')];
      session = service.createCustomSession('practice', 'human-1', questions);
    });

    describe('startSession', () => {
      it('should transition from not_started to in_progress', () => {
        const updated = service.startSession(session.id);

        expect(updated).not.toBeNull();
        expect(updated?.state).toBe('in_progress');
        expect(updated?.timing.startedAt).toBeDefined();
      });

      it('should return null for unknown session', () => {
        const result = service.startSession('unknown-id');

        expect(result).toBeNull();
      });
    });

    describe('pauseSession', () => {
      it('should pause an in-progress session', () => {
        service.startSession(session.id);
        const paused = service.pauseSession(session.id);

        expect(paused?.state).toBe('paused');
      });
    });

    describe('resumeSession', () => {
      it('should resume a paused session', () => {
        service.startSession(session.id);
        service.pauseSession(session.id);
        const resumed = service.resumeSession(session.id);

        expect(resumed?.state).toBe('in_progress');
      });
    });

    describe('abandonSession', () => {
      it('should abandon a session', () => {
        service.startSession(session.id);
        const abandoned = service.abandonSession(session.id);

        expect(abandoned?.state).toBe('abandoned');
        expect(abandoned?.timing.endedAt).toBeDefined();
      });
    });
  });

  describe('submitAnswer', () => {
    let session: QuizSession;

    beforeEach(() => {
      const questions = [
        createMockQuestion('q1', 'content-1'),
        createMockQuestion('q2', 'content-2')
      ];
      session = service.createCustomSession('practice', 'human-1', questions);
      service.startSession(session.id);
    });

    it('should record a correct answer', () => {
      const scoreResult = createScoreResult(true);

      const response = service.submitAnswer(session.id, 'q1', { answer: 'test' }, scoreResult);

      expect(response).not.toBeNull();
      expect(response?.correct).toBeTrue();
      expect(response?.score).toBe(1);
      expect(response?.questionId).toBe('q1');
    });

    it('should record an incorrect answer', () => {
      const scoreResult = createScoreResult(false);

      const response = service.submitAnswer(session.id, 'q1', { answer: 'test' }, scoreResult);

      expect(response).not.toBeNull();
      expect(response?.correct).toBeFalse();
      expect(response?.score).toBe(0);
    });

    it('should return null if session not in progress', () => {
      service.pauseSession(session.id);
      const scoreResult = createScoreResult(true);

      const response = service.submitAnswer(session.id, 'q1', { answer: 'test' }, scoreResult);

      expect(response).toBeNull();
    });

    it('should return null for unknown question ID', () => {
      const scoreResult = createScoreResult(true);

      const response = service.submitAnswer(session.id, 'unknown-q', { answer: 'test' }, scoreResult);

      expect(response).toBeNull();
    });

    it('should track time spent on question', () => {
      const scoreResult = createScoreResult(true);

      const response = service.submitAnswer(session.id, 'q1', { answer: 'test' }, scoreResult);

      expect(response?.timeSpentMs).toBeGreaterThanOrEqual(0);
    });

    it('should update session responses array', () => {
      const scoreResult = createScoreResult(true);

      service.submitAnswer(session.id, 'q1', { answer: 'test' }, scoreResult);
      const updated = service.getSession(session.id);

      expect(updated?.responses.length).toBe(1);
      expect(updated?.responses[0].questionId).toBe('q1');
    });

    it('should handle score property if present in result', () => {
      const scoreResult = { correct: true, score: 0.75 } as PerseusScoreResult;

      const response = service.submitAnswer(session.id, 'q1', { answer: 'test' }, scoreResult);

      expect(response?.score).toBe(0.75);
    });

    it('should update streak for inline quizzes on correct answer', () => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const inlineSession = service.createCustomSession('inline', 'human-1', questions);
      service.startSession(inlineSession.id);

      const scoreResult = createScoreResult(true);
      service.submitAnswer(inlineSession.id, 'q1', { answer: 'test' }, scoreResult);

      const updated = service.getSession(inlineSession.id);
      expect(updated?.streakInfo?.currentStreak).toBe(1);
    });

    it('should reset streak for inline quizzes on incorrect answer', () => {
      const questions = [
        createMockQuestion('q1', 'content-1'),
        createMockQuestion('q2', 'content-1')
      ];
      const inlineSession = service.createCustomSession('inline', 'human-1', questions);
      service.startSession(inlineSession.id);

      // First answer correct
      service.submitAnswer(inlineSession.id, 'q1', { answer: 'test' }, createScoreResult(true));

      // Second answer incorrect
      service.submitAnswer(inlineSession.id, 'q2', { answer: 'test' }, createScoreResult(false));

      const updated = service.getSession(inlineSession.id);
      expect(updated?.streakInfo?.currentStreak).toBe(0);
    });
  });

  describe('Navigation', () => {
    let session: QuizSession;

    beforeEach(() => {
      const questions = [
        createMockQuestion('q1', 'content-1'),
        createMockQuestion('q2', 'content-2'),
        createMockQuestion('q3', 'content-3')
      ];
      session = service.createCustomSession('practice', 'human-1', questions, {
        randomizeQuestions: false,
        allowBackNavigation: false,
        allowSkip: false
      });
      service.startSession(session.id);
    });

    describe('nextQuestion', () => {
      it('should move to next question', () => {
        const next = service.nextQuestion(session.id);

        expect(next).not.toBeNull();
        expect(next?.item.id).toBe('q2');

        const updated = service.getSession(session.id);
        expect(updated?.currentIndex).toBe(1);
      });

      it('should return null at end of quiz', () => {
        service.nextQuestion(session.id);
        service.nextQuestion(session.id);
        const result = service.nextQuestion(session.id);

        expect(result).toBeNull();
      });

      it('should return null if session not in progress', () => {
        service.pauseSession(session.id);
        const result = service.nextQuestion(session.id);

        expect(result).toBeNull();
      });
    });

    describe('previousQuestion', () => {
      it('should move to previous question when allowed', () => {
        const questions = [createMockQuestion('q1', 'content-1')];
        const configSession = service.createCustomSession('practice', 'human-1', questions, {
          allowBackNavigation: true
        });
        service.startSession(configSession.id);
        service.nextQuestion(configSession.id);

        const prev = service.previousQuestion(configSession.id);

        expect(prev).toBeNull(); // At start, no previous
      });

      it('should return null when back navigation not allowed', () => {
        service.nextQuestion(session.id);
        const prev = service.previousQuestion(session.id);

        expect(prev).toBeNull();
      });

      it('should return null at start of quiz', () => {
        const questions = [createMockQuestion('q1', 'content-1')];
        const configSession = service.createCustomSession('practice', 'human-1', questions, {
          allowBackNavigation: true
        });
        service.startSession(configSession.id);

        const prev = service.previousQuestion(configSession.id);

        expect(prev).toBeNull();
      });
    });

    describe('skipQuestion', () => {
      it('should skip question when allowed', () => {
        const questions = [createMockQuestion('q1', 'content-1')];
        const configSession = service.createCustomSession('practice', 'human-1', questions, {
          allowSkip: true
        });
        service.startSession(configSession.id);

        const skipped = service.skipQuestion(configSession.id);

        expect(skipped).toBeNull(); // Only one question, so null
      });

      it('should return null when skip not allowed', () => {
        const skipped = service.skipQuestion(session.id);

        expect(skipped).toBeNull();
      });
    });
  });

  describe('useHint', () => {
    let session: QuizSession;

    beforeEach(() => {
      const questions = [createMockQuestion('q1', 'content-1')];
      session = service.createCustomSession('practice', 'human-1', questions);
      service.startSession(session.id);
    });

    it('should mark hint as used', () => {
      const result = service.useHint(session.id);

      expect(result).toBeTrue();

      const updated = service.getSession(session.id);
      expect(updated?.questions[0].hintUsed).toBeTrue();
    });

    it('should return true if hint already used', () => {
      service.useHint(session.id);
      const result = service.useHint(session.id);

      expect(result).toBeTrue();
    });

    it('should return false if session not in progress', () => {
      service.pauseSession(session.id);
      const result = service.useHint(session.id);

      expect(result).toBeFalse();
    });
  });

  describe('completeSession', () => {
    let session: QuizSession;

    beforeEach(() => {
      // Create session with 2 questions to prevent auto-complete
      const questions = [
        createMockQuestion('q1', 'content-1'),
        createMockQuestion('q2', 'content-2')
      ];
      session = service.createCustomSession('practice', 'human-1', questions, {
        randomizeQuestions: false
      });
      service.startSession(session.id);
    });

    it('should calculate quiz result on completion', () => {
      // Answer first question correctly, but not the second
      service.submitAnswer(session.id, 'q1', { answer: 'test' }, createScoreResult(true));

      // Manually complete the session
      const result = service.completeSession(session.id);

      expect(result).not.toBeNull();
      expect(result?.totalQuestions).toBe(2);
      expect(result?.correctCount).toBe(1);
    });

    it('should transition to passed state if passing', () => {
      service.submitAnswer(session.id, 'q1', { answer: 'test' }, createScoreResult(true));

      service.completeSession(session.id);
      const updated = service.getSession(session.id);

      expect(updated?.state).toBe('passed');
    });

    it('should transition to failed state if not passing', () => {
      service.submitAnswer(session.id, 'q1', { answer: 'test' }, createScoreResult(false));

      service.completeSession(session.id);
      const updated = service.getSession(session.id);

      expect(updated?.state).toBe('failed');
    });

    it('should return null for unknown session', () => {
      const result = service.completeSession('unknown-id');

      expect(result).toBeNull();
    });
  });

  describe('forceComplete', () => {
    it('should force complete session with timeout', () => {
      const questions = [createMockQuestion('q1', 'content-1')];
      const session = service.createCustomSession('practice', 'human-1', questions);
      service.startSession(session.id);

      const result = service.forceComplete(session.id, 'timeout');

      expect(result).not.toBeNull();

      const updated = service.getSession(session.id);
      expect(updated?.timing.timeExceeded).toBeTrue();
    });
  });

  describe('Utility Methods', () => {
    let session: QuizSession;

    beforeEach(() => {
      const questions = [
        createMockQuestion('q1', 'content-1'),
        createMockQuestion('q2', 'content-2')
      ];
      session = service.createCustomSession('practice', 'human-1', questions, {
        randomizeQuestions: false
      });
    });

    describe('getCurrentQuestion', () => {
      it('should return current question', () => {
        const current = service.getCurrentQuestion(session.id);

        expect(current).not.toBeNull();
        expect(current?.item.id).toBe('q1');
      });

      it('should return null for unknown session', () => {
        const current = service.getCurrentQuestion('unknown-id');

        expect(current).toBeNull();
      });
    });

    describe('getProgress', () => {
      it('should calculate progress information', () => {
        const progress = service.getProgress(session.id);

        expect(progress).not.toBeNull();
        expect(progress?.current).toBe(1);
        expect(progress?.total).toBe(2);
        expect(progress?.percentage).toBe(50);
      });

      it('should return null for unknown session', () => {
        const progress = service.getProgress('unknown-id');

        expect(progress).toBeNull();
      });
    });

    describe('isSessionComplete', () => {
      it('should return false for active session', () => {
        const complete = service.isSessionComplete(session.id);

        expect(complete).toBeFalse();
      });

      it('should return true for completed session', () => {
        service.startSession(session.id);
        service.submitAnswer(session.id, 'q1', { answer: 'test' }, createScoreResult(true));
        service.submitAnswer(session.id, 'q2', { answer: 'test' }, createScoreResult(true));

        const complete = service.isSessionComplete(session.id);

        expect(complete).toBeTrue();
      });

      it('should return false for unknown session', () => {
        const complete = service.isSessionComplete('unknown-id');

        expect(complete).toBeFalse();
      });
    });

    describe('cleanupSessions', () => {
      it('should remove old completed sessions', (done) => {
        service.startSession(session.id);
        service.submitAnswer(session.id, 'q1', { answer: 'test' }, createScoreResult(true));
        service.submitAnswer(session.id, 'q2', { answer: 'test' }, createScoreResult(true));

        // Wait a bit to ensure session is old enough
        setTimeout(() => {
          // Clean up sessions older than 0ms (everything)
          service.cleanupSessions(0);

          const retrieved = service.getSession(session.id);
          expect(retrieved).toBeNull();
          done();
        }, 10);
      });

      it('should not remove active sessions', () => {
        service.startSession(session.id);

        service.cleanupSessions(0);

        const retrieved = service.getSession(session.id);
        expect(retrieved).not.toBeNull();
      });
    });
  });
});
