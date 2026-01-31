import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  Output,
  EventEmitter,
  OnInit,
  OnDestroy,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  signal,
  computed,
} from '@angular/core';

// @coverage: 48.8% (2026-02-04)

import { Subject, takeUntil } from 'rxjs';

import { GovernanceSignalService } from '@app/elohim/services/governance-signal.service';

import { Moment, Recognition } from '../../../content-io/plugins/sophia/sophia-moment.model';
import { SophiaWrapperComponent } from '../../../content-io/plugins/sophia/sophia-wrapper.component';
import { StreakState, getStreakProgress } from '../../models/streak-state.model';
import { QuestionPoolService } from '../../services/question-pool.service';
import { QuizSoundService } from '../../services/quiz-sound.service';
import { StreakTrackerService } from '../../services/streak-tracker.service';

/**
 * Completion event for inline quiz.
 */
export interface InlineQuizCompletionEvent {
  /** Content ID that was quizzed */
  contentId: string;

  /** Human ID who completed */
  humanId: string;

  /** Whether streak target was achieved */
  achieved: boolean;

  /** Final streak count */
  finalStreak: number;

  /** Target streak */
  targetStreak: number;

  /** Total questions answered */
  totalAnswered: number;

  /** Number answered correctly */
  correctCount: number;
}

/**
 * InlineQuizComponent - Post-content knowledge check with streak tracking.
 *
 * Displays a "Test Your Understanding" section after content consumption.
 * Learner must get 3 questions correct in a row to earn the "practiced"
 * attestation, following Khan Academy's inline quiz model.
 *
 * Features:
 * - Streak indicator showing progress toward goal
 * - One question at a time display
 * - Immediate feedback with explanation
 * - Success celebration with jingle
 * - Collapse/expand for returning learners
 *
 * @example
 * ```html
 * <app-inline-quiz
 *   [contentId]="currentContent.id"
 *   [humanId]="currentUser.id"
 *   [targetStreak]="3"
 *   (completed)="onQuizCompleted($event)"
 *   (attestationEarned)="onPracticedAttestation($event)">
 * </app-inline-quiz>
 * ```
 */
@Component({
  selector: 'app-inline-quiz',
  standalone: true,
  imports: [CommonModule, SophiaWrapperComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <section
      class="inline-quiz"
      [class.achieved]="streakAchieved()"
      [class.collapsed]="collapsed()"
      [class.celebrating]="celebrating()"
    >
      <!-- Header with toggle -->
      <header class="quiz-header" (click)="toggleCollapsed()">
        <div class="header-content">
          <span class="quiz-icon">{{ getHeaderIcon() }}</span>
          <h3 class="quiz-title">{{ getHeaderTitle() }}</h3>
          @if (streakAchieved()) {
            <span class="practiced-badge">Practiced!</span>
          }
        </div>
        <button
          class="collapse-btn"
          [attr.aria-expanded]="!collapsed()"
          aria-controls="quiz-content"
        >
          {{ collapsed() ? '▼' : '▲' }}
        </button>
      </header>

      <!-- Quiz content -->
      @if (!collapsed()) {
        <div id="quiz-content" class="quiz-content">
          <!-- Streak indicator -->
          <div class="streak-indicator">
            <div class="streak-dots">
              @for (dot of streakDots(); track $index) {
                <span
                  class="streak-dot"
                  [class.filled]="dot.filled"
                  [class.current]="dot.current"
                  [class.incorrect]="dot.incorrect"
                ></span>
              }
            </div>
            <span class="streak-label">
              {{ getStreakLabel() }}
            </span>
          </div>

          <!-- Question display -->
          @if (currentQuestion()) {
            <div class="question-container">
              <app-sophia-question
                [moment]="currentQuestion()!"
                [reviewMode]="showFeedback()"
                [autoFocus]="true"
                (recognition)="onQuestionScored($event)"
                (answerChanged)="onAnswerChanged($event)"
              ></app-sophia-question>

              <!-- Answer controls -->
              <div class="answer-controls">
                @if (!showFeedback()) {
                  <button class="btn-check" [disabled]="!hasAnswer()" (click)="checkAnswer()">
                    Check Answer
                  </button>
                } @else {
                  <div class="feedback-section" [class.correct]="lastWasCorrect()">
                    <div class="feedback-message">
                      @if (lastWasCorrect()) {
                        <span class="feedback-icon correct">✓</span>
                        <span>Correct!</span>
                      } @else {
                        <span class="feedback-icon incorrect">✗</span>
                        <span>Not quite. {{ streakBroken() ? 'Streak reset.' : '' }}</span>
                      }
                    </div>
                    @if (!streakAchieved()) {
                      <button class="btn-next" (click)="nextQuestion()">
                        {{ getNextButtonLabel() }}
                      </button>
                    }
                  </div>
                }
              </div>
            </div>
          } @else if (loading()) {
            <div class="loading-state">
              <span class="loading-spinner"></span>
              Loading questions...
            </div>
          } @else if (noQuestions()) {
            <div class="empty-state">
              <p>No practice questions available for this content yet.</p>
            </div>
          }

          <!-- Achievement celebration -->
          @if (celebrating()) {
            <div class="celebration-overlay">
              <div class="celebration-content">
                <span class="celebration-icon">🎉</span>
                <h4>Great job!</h4>
                <p>You've mastered this concept.</p>
              </div>
            </div>
          }
        </div>
      }
    </section>
  `,
  styles: [
    `
      .inline-quiz {
        margin-top: 2rem;
        border: 1px solid var(--border-color, #e9ecef);
        border-radius: var(--radius-lg, 12px);
        background: var(--surface-secondary, #f8f9fa);
        overflow: hidden;
        transition: all 0.3s ease;
      }

      .inline-quiz.achieved {
        border-color: var(--success, #34a853);
        background: linear-gradient(
          to bottom,
          var(--success-surface, #e6f4ea) 0%,
          var(--surface-secondary, #f8f9fa) 100%
        );
      }

      .inline-quiz.celebrating {
        transform: scale(1.02);
        box-shadow: 0 8px 32px rgba(52, 168, 83, 0.25);
      }

      /* Header */
      .quiz-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 1rem 1.25rem;
        cursor: pointer;
        user-select: none;
        transition: background 0.15s ease;
      }

      .quiz-header:hover {
        background: rgba(0, 0, 0, 0.03);
      }

      .header-content {
        display: flex;
        align-items: center;
        gap: 0.75rem;
      }

      .quiz-icon {
        font-size: 1.25rem;
      }

      .quiz-title {
        margin: 0;
        font-size: 1rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
      }

      .practiced-badge {
        padding: 0.25rem 0.625rem;
        font-size: 0.6875rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.03em;
        background: var(--success, #34a853);
        color: white;
        border-radius: var(--radius-full, 999px);
      }

      .collapse-btn {
        width: 2rem;
        height: 2rem;
        display: flex;
        align-items: center;
        justify-content: center;
        background: none;
        border: none;
        color: var(--text-tertiary, #80868b);
        cursor: pointer;
        border-radius: var(--radius-sm, 4px);
        transition: all 0.15s ease;
      }

      .collapse-btn:hover {
        background: rgba(0, 0, 0, 0.06);
        color: var(--text-primary, #202124);
      }

      /* Quiz content */
      .quiz-content {
        padding: 1.25rem;
        padding-top: 0;
      }

      /* Streak indicator */
      .streak-indicator {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 0.75rem 1rem;
        margin-bottom: 1rem;
        background: var(--surface-elevated, #fff);
        border-radius: var(--radius-md, 8px);
        border: 1px solid var(--border-color, #e9ecef);
      }

      .streak-dots {
        display: flex;
        gap: 0.5rem;
      }

      .streak-dot {
        width: 1rem;
        height: 1rem;
        border-radius: 50%;
        background: var(--surface-tertiary, #e8eaed);
        border: 2px solid var(--border-color, #dadce0);
        transition: all 0.3s ease;
      }

      .streak-dot.filled {
        background: var(--success, #34a853);
        border-color: var(--success-dark, #1e8e3e);
        transform: scale(1.1);
      }

      .streak-dot.current {
        border-color: var(--primary, #4285f4);
        box-shadow: 0 0 0 3px rgba(66, 133, 244, 0.2);
      }

      .streak-dot.incorrect {
        background: var(--error, #ea4335);
        border-color: var(--error-dark, #c5221f);
        animation: shake 0.4s ease;
      }

      @keyframes shake {
        0%,
        100% {
          transform: translateX(0);
        }
        25% {
          transform: translateX(-4px);
        }
        75% {
          transform: translateX(4px);
        }
      }

      .streak-label {
        font-size: 0.875rem;
        color: var(--text-secondary, #5f6368);
      }

      /* Question container */
      .question-container {
        background: var(--surface-elevated, #fff);
        border-radius: var(--radius-md, 8px);
        border: 1px solid var(--border-color, #e9ecef);
        padding: 1.25rem;
      }

      /* Answer controls */
      .answer-controls {
        margin-top: 1.25rem;
        padding-top: 1.25rem;
        border-top: 1px solid var(--border-color, #e9ecef);
      }

      .btn-check,
      .btn-next {
        padding: 0.75rem 1.5rem;
        font-size: 1rem;
        font-weight: 500;
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .btn-check {
        background: var(--primary, #4285f4);
        color: white;
        border: none;
      }

      .btn-check:hover:not(:disabled) {
        background: var(--primary-dark, #1a73e8);
      }

      .btn-check:disabled {
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-disabled, #9aa0a6);
        cursor: not-allowed;
      }

      .btn-next {
        background: var(--surface-secondary, #f8f9fa);
        color: var(--text-primary, #202124);
        border: 1px solid var(--border-color, #e9ecef);
      }

      .btn-next:hover {
        background: var(--surface-tertiary, #e8eaed);
      }

      /* Feedback */
      .feedback-section {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 1rem;
      }

      .feedback-message {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 1rem;
        font-weight: 500;
      }

      .feedback-icon {
        width: 1.5rem;
        height: 1.5rem;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 50%;
        font-size: 0.875rem;
        font-weight: 600;
      }

      .feedback-icon.correct {
        background: var(--success, #34a853);
        color: white;
      }

      .feedback-icon.incorrect {
        background: var(--error, #ea4335);
        color: white;
      }

      .feedback-section.correct .feedback-message {
        color: var(--success-text, #137333);
      }

      /* Loading & empty states */
      .loading-state,
      .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 2rem;
        text-align: center;
        color: var(--text-secondary, #5f6368);
      }

      .loading-spinner {
        width: 1.5rem;
        height: 1.5rem;
        border: 2px solid var(--border-color, #e9ecef);
        border-top-color: var(--primary, #4285f4);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
        margin-bottom: 0.75rem;
      }

      @keyframes spin {
        to {
          transform: rotate(360deg);
        }
      }

      /* Celebration overlay */
      .celebration-overlay {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(255, 255, 255, 0.95);
        animation: fadeIn 0.3s ease;
      }

      @keyframes fadeIn {
        from {
          opacity: 0;
        }
        to {
          opacity: 1;
        }
      }

      .celebration-content {
        text-align: center;
        animation: scaleIn 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
      }

      @keyframes scaleIn {
        from {
          opacity: 0;
          transform: scale(0.5);
        }
        to {
          opacity: 1;
          transform: scale(1);
        }
      }

      .celebration-icon {
        font-size: 3rem;
        display: block;
        margin-bottom: 0.5rem;
      }

      .celebration-content h4 {
        margin: 0 0 0.25rem;
        font-size: 1.25rem;
        font-weight: 600;
        color: var(--success-text, #137333);
      }

      .celebration-content p {
        margin: 0;
        color: var(--text-secondary, #5f6368);
      }

      /* Collapsed state */
      .inline-quiz.collapsed .quiz-content {
        display: none;
      }
    `,
  ],
})
export class InlineQuizComponent implements OnInit, OnDestroy {
  /** Content node ID to quiz on */
  @Input({ required: true }) contentId!: string;

  /** Human ID taking the quiz */
  @Input({ required: true }) humanId!: string;

  /** Target streak to achieve (default: 3) */
  @Input() targetStreak = 3;

  /** Maximum questions before forced completion */
  @Input() maxQuestions = 10;

  /** Start collapsed if already achieved */
  @Input() collapseIfAchieved = true;

  /** Emitted when quiz completes (achieved or max reached) */
  @Output() completed = new EventEmitter<InlineQuizCompletionEvent>();

  /** Emitted when streak is achieved (practiced attestation earned) */
  @Output() attestationEarned = new EventEmitter<void>();

  // State signals
  protected loading = signal(true);
  protected noQuestions = signal(false);
  protected collapsed = signal(false);
  protected showFeedback = signal(false);
  protected lastWasCorrect = signal(false);
  protected streakBroken = signal(false);
  protected celebrating = signal(false);
  protected hasAnswer = signal(false);

  protected questions = signal<Moment[]>([]);
  protected currentQuestionIndex = signal(0);
  protected streakState = signal<StreakState | null>(null);

  // Computed values
  protected currentQuestion = computed(() => {
    const qs = this.questions();
    const idx = this.currentQuestionIndex();
    return qs[idx] ?? null;
  });

  protected streakAchieved = computed(() => {
    return this.streakState()?.achieved ?? false;
  });

  protected streakDots = computed(() => {
    const state = this.streakState();
    if (!state) {
      return Array(this.targetStreak).fill({ filled: false, current: false, incorrect: false });
    }

    const progress = getStreakProgress(state);
    return progress.indicators.map((ind, i) => ({
      filled: ind.state === 'correct',
      current: i === state.currentStreak && !this.showFeedback(),
      incorrect: ind.state === 'incorrect' || (ind.state === 'current' && this.streakBroken()),
    }));
  });

  private readonly destroy$ = new Subject<void>();
  private readonly sophiaWrapper: SophiaWrapperComponent | null = null;
  private pendingRecognition: Recognition | null = null;

  constructor(
    private readonly streakTracker: StreakTrackerService,
    private readonly soundService: QuizSoundService,
    private readonly poolService: QuestionPoolService,
    private readonly governanceSignalService: GovernanceSignalService,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.loadQuestions();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.streakTracker.offAchieved(this.contentId);
  }

  /**
   * Toggle collapsed state.
   */
  toggleCollapsed(): void {
    this.collapsed.update(v => !v);
  }

  /**
   * Handle answer change from Sophia.
   * @param hasValidAnswer - Whether the user has provided a valid/complete answer
   */
  onAnswerChanged(hasValidAnswer: boolean): void {
    this.hasAnswer.set(hasValidAnswer);
  }

  /**
   * Handle recognition result from Sophia.
   */
  onQuestionScored(result: Recognition): void {
    this.pendingRecognition = result;
  }

  /**
   * Check the current answer.
   */
  checkAnswer(): void {
    if (!this.pendingRecognition || !this.currentQuestion()) return;

    // For mastery quizzes, check if the answer is correct
    const correct = this.pendingRecognition.mastery?.demonstrated ?? false;
    this.lastWasCorrect.set(correct);
    this.showFeedback.set(true);

    // Play sound
    if (correct) {
      this.soundService.playCorrectAnswerFeedback();
    } else {
      this.soundService.playIncorrectAnswerFeedback();
    }

    // Update streak
    const questionId = this.currentQuestion()!.id;
    const newState = this.streakTracker.recordAnswer(this.contentId, questionId, correct);

    if (newState) {
      // Check if streak was broken
      if (!correct && (this.streakState()?.currentStreak ?? 0) > 0) {
        this.streakBroken.set(true);
      } else {
        this.streakBroken.set(false);
      }

      this.streakState.set(newState);

      // Emit quiz attempt learning signal
      this.governanceSignalService
        .recordLearningSignal({
          contentId: this.contentId,
          signalType: 'quiz_attempt',
          payload: {
            questionId,
            correct,
            currentStreak: newState.currentStreak,
            totalAttempted: newState.totalAttempted,
            totalCorrect: newState.totalCorrect,
            achieved: newState.achieved,
          },
        })
        .pipe(takeUntil(this.destroy$))
        .subscribe();

      // Check for achievement
      if (newState.achieved && !this.celebrating()) {
        this.onStreakAchieved();
      }
    }

    this.cdr.markForCheck();
  }

  /**
   * Move to next question.
   */
  nextQuestion(): void {
    // Reset state
    this.showFeedback.set(false);
    this.hasAnswer.set(false);
    this.streakBroken.set(false);
    this.pendingRecognition = null;

    // Move to next question (wrap around if needed)
    const nextIdx = this.currentQuestionIndex() + 1;
    if (nextIdx < this.questions().length) {
      this.currentQuestionIndex.set(nextIdx);
    } else {
      // Shuffle and restart
      this.shuffleQuestions();
      this.currentQuestionIndex.set(0);
    }

    this.cdr.markForCheck();
  }

  /**
   * Get header icon based on state.
   */
  getHeaderIcon(): string {
    if (this.streakAchieved()) return '✓';
    if (this.loading()) return '⏳';
    return '📝';
  }

  /**
   * Get header title based on state.
   */
  getHeaderTitle(): string {
    if (this.streakAchieved()) {
      return 'Knowledge Check Complete';
    }
    return 'Test Your Understanding';
  }

  /**
   * Get streak label for indicator.
   */
  getStreakLabel(): string {
    const state = this.streakState();
    if (!state) return `Get ${this.targetStreak} in a row`;

    if (state.achieved) {
      return 'Streak achieved!';
    }

    const remaining = state.targetStreak - state.currentStreak;
    if (state.currentStreak === 0) {
      return `Get ${state.targetStreak} in a row`;
    }
    return `${remaining} more to go!`;
  }

  /**
   * Get next button label.
   */
  getNextButtonLabel(): string {
    if (this.lastWasCorrect()) {
      return 'Next Question';
    }
    return 'Try Again';
  }

  /**
   * Load questions for this content.
   */
  private loadQuestions(): void {
    this.loading.set(true);

    // Start streak tracking
    const state = this.streakTracker.startTracking(this.contentId, this.humanId, {
      targetStreak: this.targetStreak,
      maxQuestions: this.maxQuestions,
    });
    this.streakState.set(state);

    // Check if already achieved
    if (state.achieved && this.collapseIfAchieved) {
      this.collapsed.set(true);
    }

    // Register achievement callback
    this.streakTracker.onAchieved(this.contentId, () => {
      this.onStreakAchieved();
    });

    // Load question pool
    this.poolService
      .getPoolForContent(this.contentId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: pool => {
          if (pool && pool.questions.length > 0) {
            // Shuffle questions
            const shuffled = this.shuffleArray([...pool.questions]);
            this.questions.set(shuffled);
            this.noQuestions.set(false);
          } else {
            this.noQuestions.set(true);
          }
          this.loading.set(false);
          this.cdr.markForCheck();
        },
        error: _err => {
          this.noQuestions.set(true);
          this.loading.set(false);
          this.cdr.markForCheck();
        },
      });
  }

  /**
   * Shuffle questions for retry.
   */
  private shuffleQuestions(): void {
    const qs = this.questions();
    this.questions.set(this.shuffleArray([...qs]));
  }

  /**
   * Shuffle array using Fisher-Yates with crypto-secure random.
   */
  private shuffleArray<T>(array: T[]): T[] {
    const shuffled = [...array];
    for (let i = shuffled.length - 1; i > 0; i--) {
      const j = Math.floor((crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32) * (i + 1));
      [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
    }
    return shuffled;
  }

  /**
   * Handle streak achievement.
   */
  private onStreakAchieved(): void {
    // Play success jingle
    this.soundService.playStreakAchieved();

    // Show celebration
    this.celebrating.set(true);

    // Emit mastery achieved learning signal
    const state = this.streakState();
    this.governanceSignalService
      .recordLearningSignal({
        contentId: this.contentId,
        signalType: 'mastery_achieved',
        payload: {
          humanId: this.humanId,
          streakAchieved: state?.currentStreak ?? this.targetStreak,
          targetStreak: this.targetStreak,
          totalAttempts: state?.totalAttempted ?? 0,
          accuracy: state ? (state.totalCorrect / state.totalAttempted) * 100 : 0,
        },
      })
      .pipe(takeUntil(this.destroy$))
      .subscribe();

    // Emit events
    this.attestationEarned.emit();
    this.emitCompletion(true);

    // Hide celebration after delay
    setTimeout(() => {
      this.celebrating.set(false);
      this.cdr.markForCheck();
    }, 3000);
  }

  /**
   * Emit completion event.
   */
  private emitCompletion(achieved: boolean): void {
    const state = this.streakState();
    this.completed.emit({
      contentId: this.contentId,
      humanId: this.humanId,
      achieved,
      finalStreak: state?.currentStreak ?? 0,
      targetStreak: this.targetStreak,
      totalAnswered: state?.totalAttempted ?? 0,
      correctCount: state?.totalCorrect ?? 0,
    });
  }
}
