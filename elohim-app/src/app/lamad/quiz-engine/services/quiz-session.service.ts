import { Injectable, inject } from '@angular/core';

// @coverage: 93.5% (2026-02-05)

import { BehaviorSubject, Observable, of, map, switchMap } from 'rxjs';

import {
  QuizSession,
  QuizSessionType,
  QuizSessionState,
  QuizResponse,
  QuizResult,
  PathQuizContext,
  QuizSessionConfig,
  SessionQuestion,
  createQuizSession,
  calculateQuizResult,
  isValidTransition,
  isTerminalState,
} from '../models/quiz-session.model';

import { QuestionPoolService } from './question-pool.service';

import type {
  PerseusItem,
  PerseusScoreResult,
} from '../../content-io/plugins/sophia/sophia-moment.model';

/**
 * QuizSessionService - Manages quiz session lifecycle.
 *
 * Handles the full state machine for practice, mastery, inline, and
 * pre-assessment quiz flows including:
 * - Session creation and configuration
 * - Question navigation
 * - Answer submission and scoring
 * - Streak tracking (for inline quizzes)
 * - Result calculation and attestation
 *
 * @example
 * ```typescript
 * // Start an inline quiz for a content node
 * const session = await sessionService.startInlineQuiz('concept-123', humanId).toPromise();
 *
 * // Submit an answer
 * const response = sessionService.submitAnswer(session.id, questionId, userAnswer, scoreResult);
 *
 * // Complete and get results
 * const result = sessionService.completeSession(session.id);
 * ```
 */
@Injectable({
  providedIn: 'root',
})
export class QuizSessionService {
  private readonly questionPool = inject(QuestionPoolService);

  /** Active sessions by ID */
  private readonly sessions = new Map<string, QuizSession>();

  /** Observable sessions for UI binding */
  private readonly sessionSubjects = new Map<string, BehaviorSubject<QuizSession>>();

  /** Current active session (for convenience) */
  private readonly activeSessionSubject = new BehaviorSubject<QuizSession | null>(null);
  readonly activeSession$ = this.activeSessionSubject.asObservable();

  // ═══════════════════════════════════════════════════════════════════════════
  // Session Creation
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Start a practice quiz from content hierarchy.
   *
   * Practice quizzes are unlimited and draw from all content
   * below the current position in the path.
   */
  startPracticeQuiz(
    pathId: string,
    sectionId: string,
    humanId: string,
    questionCount = 5
  ): Observable<QuizSession> {
    return this.questionPool.getHierarchicalPool(pathId, sectionId).pipe(
      switchMap(source => this.questionPool.loadHierarchicalPools(source)),
      map(source => {
        const selection = this.questionPool.selectPracticeQuestions(source, questionCount);

        const pathContext: PathQuizContext = {
          pathId,
          sectionId,
          sectionContentIds: source.eligibleContentIds,
        };

        const session = createQuizSession('practice', humanId, selection.questions, pathContext);

        this.registerSession(session);
        return session;
      })
    );
  }

  /**
   * Start a mastery quiz for a section.
   *
   * Mastery quizzes are limited to 2 attempts per day,
   * weighted toward practiced content, and gate progression.
   */
  startMasteryQuiz(
    pathId: string,
    sectionId: string,
    humanId: string,
    practicedContentIds: string[] = [],
    questionCount = 5
  ): Observable<QuizSession> {
    return this.questionPool.getHierarchicalPool(pathId, sectionId).pipe(
      switchMap(source => this.questionPool.loadHierarchicalPools(source)),
      map(source => {
        const selection = this.questionPool.selectMasteryQuestions(
          source.combinedPool,
          questionCount,
          practicedContentIds
        );

        const pathContext: PathQuizContext = {
          pathId,
          sectionId,
          sectionContentIds: source.eligibleContentIds,
        };

        const session = createQuizSession('mastery', humanId, selection.questions, pathContext);

        this.registerSession(session);
        return session;
      })
    );
  }

  /**
   * Start an inline quiz for post-content attestation.
   *
   * Inline quizzes appear after content consumption.
   * Get 3 correct in a row to earn "practiced" attestation.
   */
  startInlineQuiz(
    contentId: string,
    humanId: string,
    targetStreak = 3,
    maxQuestions = 10
  ): Observable<QuizSession> {
    return this.questionPool.selectInlineQuestions(contentId, maxQuestions).pipe(
      map(selection => {
        const session = createQuizSession('inline', humanId, selection.questions, undefined, {
          allowRetry: true,
          showImmediateFeedback: true,
        });

        // Configure streak tracking
        if (session.streakInfo) {
          session.streakInfo.targetStreak = targetStreak;
        }

        this.registerSession(session);
        return session;
      })
    );
  }

  /**
   * Start a pre-assessment for skip-ahead.
   *
   * Pre-assessments test knowledge before starting a path,
   * allowing learners to skip content they already know.
   */
  startPreAssessment(pathId: string, humanId: string, questionCount = 10): Observable<QuizSession> {
    // Get all content from the path
    return this.questionPool.getHierarchicalPool(pathId, pathId).pipe(
      switchMap(source => this.questionPool.loadHierarchicalPools(source)),
      map(source => {
        const selection = this.questionPool.selectQuestions(source.combinedPool, {
          count: questionCount,
          randomize: true,
          ensureVariety: true,
        });

        const pathContext: PathQuizContext = {
          pathId,
          sectionId: pathId,
          sectionContentIds: source.eligibleContentIds,
        };

        const session = createQuizSession(
          'pre-assessment',
          humanId,
          selection.questions,
          pathContext
        );

        this.registerSession(session);
        return session;
      })
    );
  }

  /**
   * Create a session with specific questions (for testing/preview).
   */
  createCustomSession(
    type: QuizSessionType,
    humanId: string,
    questions: PerseusItem[],
    config?: Partial<QuizSessionConfig>
  ): QuizSession {
    const session = createQuizSession(type, humanId, questions, undefined, config);
    this.registerSession(session);
    return session;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Session State Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get a session by ID.
   */
  getSession(sessionId: string): QuizSession | null {
    return this.sessions.get(sessionId) ?? null;
  }

  /**
   * Get observable session for reactive updates.
   */
  getSession$(sessionId: string): Observable<QuizSession | null> {
    const subject = this.sessionSubjects.get(sessionId);
    return subject ? subject.asObservable() : of(null);
  }

  /**
   * Start a session (transition from not_started to in_progress).
   */
  startSession(sessionId: string): QuizSession | null {
    const session = this.sessions.get(sessionId);
    if (!session) return null;

    return this.transitionState(sessionId, 'in_progress');
  }

  /**
   * Pause a session.
   */
  pauseSession(sessionId: string): QuizSession | null {
    return this.transitionState(sessionId, 'paused');
  }

  /**
   * Resume a paused session.
   */
  resumeSession(sessionId: string): QuizSession | null {
    return this.transitionState(sessionId, 'in_progress');
  }

  /**
   * Abandon a session.
   */
  abandonSession(sessionId: string): QuizSession | null {
    return this.transitionState(sessionId, 'abandoned');
  }

  /**
   * Transition session to new state.
   */
  private transitionState(sessionId: string, newState: QuizSessionState): QuizSession | null {
    const session = this.sessions.get(sessionId);
    if (!session) return null;

    if (!isValidTransition(session.state, newState)) {
      return null;
    }

    const now = new Date().toISOString();
    const updated: QuizSession = {
      ...session,
      state: newState,
      timing: {
        ...session.timing,
        startedAt:
          newState === 'in_progress' && !session.timing.startedAt ? now : session.timing.startedAt,
        endedAt: isTerminalState(newState) ? now : session.timing.endedAt,
      },
    };

    this.updateSession(updated);
    return updated;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Answer Submission
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Submit an answer for the current question.
   *
   * @param sessionId - Session ID
   * @param questionId - Question being answered
   * @param response - User's answer (Perseus widget format)
   * @param scoreResult - Score result from Perseus
   * @returns Updated response record
   */
  submitAnswer(
    sessionId: string,
    questionId: string,
    response: unknown,
    scoreResult: PerseusScoreResult
  ): QuizResponse | null {
    const session = this.sessions.get(sessionId);
    if (session?.state !== 'in_progress') {
      return null;
    }

    // Find the question
    const questionIndex = session.questions.findIndex(q => q.item.id === questionId);
    if (questionIndex === -1) {
      return null;
    }

    const question = session.questions[questionIndex];
    const now = new Date().toISOString();

    // Calculate time spent on this question
    const startTime =
      session.responses.length > 0
        ? new Date(session.responses.at(-1)!.answeredAt).getTime()
        : new Date(session.timing.startedAt ?? session.timing.createdAt).getTime();
    const timeSpentMs = Date.now() - startTime;

    // Create response record
    // Note: PerseusScoreResult may or may not have a 'score' property depending on the source
    // Fall back to binary scoring (1 for correct, 0 for incorrect) if score is not available
    let scoreValue = 0;
    if ('score' in scoreResult && typeof scoreResult.score === 'number') {
      scoreValue = scoreResult.score;
    } else if (scoreResult.correct) {
      scoreValue = 1;
    }

    const quizResponse: QuizResponse = {
      questionId,
      questionIndex,
      contentId: question.contentId,
      response,
      correct: scoreResult.correct ?? false,
      score: scoreValue,
      answeredAt: now,
      timeSpentMs,
      attemptNumber: question.attempts + 1,
      hintViewed: question.hintUsed,
      perseusResult: scoreResult,
    };

    // Update question state
    const updatedQuestions = [...session.questions];
    updatedQuestions[questionIndex] = {
      ...question,
      answered: true,
      correct: scoreResult.correct ?? false,
      score: scoreValue,
      timeSpentMs: question.timeSpentMs + timeSpentMs,
      attempts: question.attempts + 1,
    };

    // Update streak for inline quizzes
    let updatedStreakInfo = session.streakInfo;
    if (session.type === 'inline' && session.streakInfo) {
      if (scoreResult.correct ?? false) {
        const newStreak = session.streakInfo.currentStreak + 1;
        updatedStreakInfo = {
          ...session.streakInfo,
          currentStreak: newStreak,
          maxStreak: Math.max(session.streakInfo.maxStreak, newStreak),
          recentAnswers: [...session.streakInfo.recentAnswers, true],
          targetAchieved: newStreak >= session.streakInfo.targetStreak,
        };
      } else {
        updatedStreakInfo = {
          ...session.streakInfo,
          currentStreak: 0,
          recentAnswers: [...session.streakInfo.recentAnswers, false],
        };
      }
    }

    // Update session
    const updated: QuizSession = {
      ...session,
      questions: updatedQuestions,
      responses: [...session.responses, quizResponse],
      streakInfo: updatedStreakInfo,
      timing: {
        ...session.timing,
        totalTimeMs: session.timing.totalTimeMs + timeSpentMs,
      },
    };

    this.updateSession(updated);

    // Check for auto-completion conditions
    this.checkAutoComplete(updated);

    return quizResponse;
  }

  /**
   * Navigate to next question.
   */
  nextQuestion(sessionId: string): SessionQuestion | null {
    const session = this.sessions.get(sessionId);
    if (session?.state !== 'in_progress') {
      return null;
    }

    if (session.currentIndex >= session.questions.length - 1) {
      return null; // No more questions
    }

    const updated: QuizSession = {
      ...session,
      currentIndex: session.currentIndex + 1,
    };

    this.updateSession(updated);
    return updated.questions[updated.currentIndex];
  }

  /**
   * Navigate to previous question (if allowed).
   */
  previousQuestion(sessionId: string): SessionQuestion | null {
    const session = this.sessions.get(sessionId);
    if (session?.state !== 'in_progress') {
      return null;
    }

    if (!session.config.allowBackNavigation || session.currentIndex <= 0) {
      return null;
    }

    const updated: QuizSession = {
      ...session,
      currentIndex: session.currentIndex - 1,
    };

    this.updateSession(updated);
    return updated.questions[updated.currentIndex];
  }

  /**
   * Skip current question (if allowed).
   */
  skipQuestion(sessionId: string): SessionQuestion | null {
    const session = this.sessions.get(sessionId);
    if (session?.state !== 'in_progress') {
      return null;
    }

    if (!session.config.allowSkip) {
      return null;
    }

    return this.nextQuestion(sessionId);
  }

  /**
   * Mark hint as used for current question.
   */
  useHint(sessionId: string): boolean {
    const session = this.sessions.get(sessionId);
    if (session?.state !== 'in_progress') {
      return false;
    }

    const currentQuestion = session.questions[session.currentIndex];
    if (currentQuestion.hintUsed) {
      return true; // Already used
    }

    const updatedQuestions = [...session.questions];
    updatedQuestions[session.currentIndex] = {
      ...currentQuestion,
      hintUsed: true,
    };

    this.updateSession({
      ...session,
      questions: updatedQuestions,
    });

    return true;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Session Completion
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Complete session and calculate results.
   */
  completeSession(sessionId: string): QuizResult | null {
    const session = this.sessions.get(sessionId);
    if (!session) return null;

    // Transition to completed state
    const completed = this.transitionState(sessionId, 'completed');
    if (!completed) return null;

    // Calculate result
    const result = calculateQuizResult(completed);

    // Determine pass/fail and update state
    const finalState: QuizSessionState = result.passed ? 'passed' : 'failed';
    this.transitionState(sessionId, finalState);

    return result;
  }

  /**
   * Force complete session (e.g., timeout).
   */
  forceComplete(sessionId: string, reason: string): QuizResult | null {
    const session = this.sessions.get(sessionId);
    if (!session) return null;

    // Mark as timed out if applicable
    if (reason === 'timeout') {
      const updated: QuizSession = {
        ...session,
        timing: {
          ...session.timing,
          timeExceeded: true,
        },
      };
      this.updateSession(updated);
    }

    return this.completeSession(sessionId);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Utility
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get current question for a session.
   */
  getCurrentQuestion(sessionId: string): SessionQuestion | null {
    const session = this.sessions.get(sessionId);
    if (!session) return null;
    return session.questions[session.currentIndex] ?? null;
  }

  /**
   * Get progress information.
   */
  getProgress(sessionId: string): { current: number; total: number; percentage: number } | null {
    const session = this.sessions.get(sessionId);
    if (!session) return null;

    return {
      current: session.currentIndex + 1,
      total: session.questions.length,
      percentage: Math.round(((session.currentIndex + 1) / session.questions.length) * 100),
    };
  }

  /**
   * Check if session is complete.
   */
  isSessionComplete(sessionId: string): boolean {
    const session = this.sessions.get(sessionId);
    return session ? isTerminalState(session.state) : false;
  }

  /**
   * Clean up old sessions.
   */
  cleanupSessions(maxAgeMs: number = 24 * 60 * 60 * 1000): void {
    const now = Date.now();

    for (const [id, session] of this.sessions) {
      const createdAt = new Date(session.timing.createdAt).getTime();
      if (now - createdAt > maxAgeMs && isTerminalState(session.state)) {
        this.sessions.delete(id);
        this.sessionSubjects.get(id)?.complete();
        this.sessionSubjects.delete(id);
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private registerSession(session: QuizSession): void {
    this.sessions.set(session.id, session);
    this.sessionSubjects.set(session.id, new BehaviorSubject(session));
    this.activeSessionSubject.next(session);
  }

  private updateSession(session: QuizSession): void {
    this.sessions.set(session.id, session);
    this.sessionSubjects.get(session.id)?.next(session);

    if (this.activeSessionSubject.value?.id === session.id) {
      this.activeSessionSubject.next(session);
    }
  }

  private checkAutoComplete(session: QuizSession): void {
    // For inline quizzes, auto-complete when streak target reached
    if (session.type === 'inline' && session.streakInfo?.targetAchieved) {
      this.completeSession(session.id);
      return;
    }

    // Auto-complete when all questions answered
    const allAnswered = session.questions.every(q => q.answered);
    if (allAnswered) {
      this.completeSession(session.id);
    }
  }
}
