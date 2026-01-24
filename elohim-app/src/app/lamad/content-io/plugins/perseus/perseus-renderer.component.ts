import {
  Component,
  Input,
  Output,
  EventEmitter,
  ViewChild,
  OnInit,
  OnDestroy,
  OnChanges,
  SimpleChanges,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  inject
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { Subject, takeUntil } from 'rxjs';
import {
  ContentRenderer,
  InteractiveRenderer,
  RendererCompletionEvent
} from '../../interfaces/content-format-plugin.interface';
import { ContentNode } from '../../../models/content-node.model';
import { PerseusWrapperComponent } from './perseus-wrapper.component';
import type { PerseusItem, PerseusScoreResult, RadioWidgetOptions } from './perseus-item.model';

// ─────────────────────────────────────────────────────────────────────────────
// Quiz Mode Configuration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Quiz modes determine how questions are graded and presented.
 * - 'mastery': Traditional graded quiz with correct/incorrect feedback
 * - 'discovery': Self-assessment quiz that tracks subscale preferences
 */
type QuizMode = 'mastery' | 'discovery';

/**
 * Configuration for each quiz mode behavior.
 */
interface QuizModeConfig {
  submitButtonText: string;
  nextButtonText: string;
  showFeedback: boolean;
  showCorrectness: boolean;
  trackSubscales: boolean;
  trackStreak: boolean;
}

/**
 * Preset configurations for each quiz mode.
 */
const QUIZ_MODE_PRESETS: Record<QuizMode, QuizModeConfig> = {
  mastery: {
    submitButtonText: 'Check Answer',
    nextButtonText: 'Next Question',
    showFeedback: true,
    showCorrectness: true,
    trackSubscales: false,
    trackStreak: true
  },
  discovery: {
    submitButtonText: 'Continue',
    nextButtonText: 'Continue',
    showFeedback: false,
    showCorrectness: false,
    trackSubscales: true,
    trackStreak: false
  }
};

/**
 * PerseusRendererComponent - Content renderer for Perseus quiz format.
 *
 * This component implements the InteractiveRenderer interface to integrate
 * with the lamad mastery tracking system. It renders Perseus items and
 * emits completion events when quizzes are answered.
 *
 * Features:
 * - Single question or multi-question quiz rendering
 * - Progress tracking for multi-question quizzes
 * - Streak tracking for inline "practiced" attestation
 * - Score aggregation and completion event emission
 *
 * @example
 * ```html
 * <!-- Used automatically by content-io for perseus format -->
 * <app-perseus-renderer
 *   [node]="contentNode"
 *   (complete)="onQuizComplete($event)">
 * </app-perseus-renderer>
 * ```
 */
@Component({
  selector: 'app-perseus-renderer',
  standalone: true,
  imports: [CommonModule, PerseusWrapperComponent],
  template: `
    <div class="perseus-renderer">
      <!-- Quiz Header -->
      <header class="quiz-header" *ngIf="showHeader">
        <h3 class="quiz-title">{{ title }}</h3>
        <div class="quiz-progress" *ngIf="totalQuestions > 1">
          <span class="progress-text">
            Question {{ currentQuestionIndex + 1 }} of {{ totalQuestions }}
          </span>
          <div class="progress-bar">
            <div
              class="progress-fill"
              [style.width.%]="progressPercentage">
            </div>
          </div>
        </div>
      </header>

      <!-- Question Area -->
      <div class="question-container">
        <app-perseus-question
          #questionComponent
          [item]="currentQuestion"
          [reviewMode]="reviewMode"
          (scored)="handleScore($event)"
          (answerChanged)="handleAnswerChange($event)"
          (ready)="handleReady()">
        </app-perseus-question>
      </div>

      <!-- Answer Feedback (mastery mode only) -->
      <div class="feedback-container" *ngIf="showFeedback && modeConfig.showCorrectness">
        <div
          class="feedback"
          [class.correct]="lastResult?.correct"
          [class.incorrect]="lastResult && !lastResult.correct">
          <span class="feedback-icon">
            {{ lastResult?.correct ? '✓' : '✗' }}
          </span>
          <span class="feedback-text">
            {{ lastResult?.correct ? 'Correct!' : 'Not quite. Try again!' }}
          </span>
          <span class="feedback-message" *ngIf="lastResult?.message">
            {{ lastResult?.message }}
          </span>
        </div>
      </div>

      <!-- Controls -->
      <footer class="quiz-controls" *ngIf="!reviewMode">
        <button
          class="btn btn-secondary"
          *ngIf="showHintButton && !hintShown"
          (click)="showHint()">
          Show Hint
        </button>

        <button
          class="btn btn-primary"
          [disabled]="!hasAnswer || isSubmitting"
          (click)="submitAnswer()">
          {{ submitButtonText }}
        </button>

        <button
          class="btn btn-secondary"
          *ngIf="showNextButton"
          (click)="nextQuestion()">
          {{ isLastQuestion ? 'See Results' : modeConfig.nextButtonText }}
        </button>
      </footer>

      <!-- Streak Indicator (mastery mode only, for inline quizzes) -->
      <div class="streak-indicator" *ngIf="showStreakIndicator && modeConfig.trackStreak">
        <div class="streak-dots">
          <span
            *ngFor="let i of streakDots; let idx = index"
            class="streak-dot"
            [class.filled]="idx < currentStreak"
            [class.correct]="streakHistory[idx] === true"
            [class.incorrect]="streakHistory[idx] === false">
          </span>
        </div>
        <span class="streak-text">
          {{ currentStreak }} / {{ targetStreak }} correct
        </span>
      </div>
    </div>
  `,
  styles: [`
    :host {
      display: block;
    }

    .perseus-renderer {
      padding: 1rem;
    }

    .quiz-header {
      margin-bottom: 1.5rem;
    }

    .quiz-title {
      margin: 0 0 0.5rem;
      font-size: 1.25rem;
      font-weight: 600;
      color: var(--text-primary, #1a1a1a);
    }

    .quiz-progress {
      display: flex;
      align-items: center;
      gap: 1rem;
    }

    .progress-text {
      font-size: 0.875rem;
      color: var(--text-secondary, #666);
    }

    .progress-bar {
      flex: 1;
      height: 6px;
      background: var(--bg-tertiary, #e0e0e0);
      border-radius: 3px;
      overflow: hidden;
    }

    .progress-fill {
      height: 100%;
      background: var(--primary-color, #1976d2);
      border-radius: 3px;
      transition: width 0.3s ease;
    }

    .question-container {
      margin-bottom: 1.5rem;
    }

    .feedback-container {
      margin-bottom: 1rem;
    }

    .feedback {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.75rem 1rem;
      border-radius: 8px;
      font-size: 0.9375rem;
    }

    .feedback.correct {
      background: var(--success-bg, #e8f5e9);
      color: var(--success-color, #2e7d32);
    }

    .feedback.incorrect {
      background: var(--error-bg, #ffebee);
      color: var(--error-color, #c62828);
    }

    .feedback-icon {
      font-size: 1.25rem;
      font-weight: bold;
    }

    .feedback-message {
      margin-left: auto;
      font-size: 0.875rem;
      opacity: 0.8;
    }

    .quiz-controls {
      display: flex;
      gap: 0.75rem;
      padding-top: 1rem;
      border-top: 1px solid var(--border-color, #e0e0e0);
    }

    .btn {
      padding: 0.625rem 1.25rem;
      border: none;
      border-radius: 6px;
      font-size: 0.9375rem;
      font-weight: 500;
      cursor: pointer;
      transition: background-color 0.2s, opacity 0.2s;
    }

    .btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .btn-primary {
      background: var(--primary-color, #1976d2);
      color: white;
    }

    .btn-primary:hover:not(:disabled) {
      background: var(--primary-dark, #1565c0);
    }

    .btn-secondary {
      background: var(--bg-secondary, #f5f5f5);
      color: var(--text-primary, #1a1a1a);
    }

    .btn-secondary:hover:not(:disabled) {
      background: var(--bg-tertiary, #e0e0e0);
    }

    .streak-indicator {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-top: 1rem;
      padding: 0.75rem 1rem;
      background: var(--bg-secondary, #f5f5f5);
      border-radius: 8px;
    }

    .streak-dots {
      display: flex;
      gap: 0.5rem;
    }

    .streak-dot {
      width: 12px;
      height: 12px;
      border-radius: 50%;
      border: 2px solid var(--border-color, #ccc);
      background: transparent;
      transition: all 0.2s ease;
    }

    .streak-dot.filled {
      border-color: var(--primary-color, #1976d2);
    }

    .streak-dot.correct {
      background: var(--success-color, #4caf50);
      border-color: var(--success-color, #4caf50);
    }

    .streak-dot.incorrect {
      background: var(--error-color, #f44336);
      border-color: var(--error-color, #f44336);
    }

    .streak-text {
      font-size: 0.875rem;
      color: var(--text-secondary, #666);
    }
  `],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class PerseusRendererComponent implements ContentRenderer, InteractiveRenderer, OnInit, OnChanges, OnDestroy {
  private readonly cdr = inject(ChangeDetectorRef);
  private readonly destroy$ = new Subject<void>();

  // ─────────────────────────────────────────────────────────────────────────
  // ContentRenderer Interface
  // ─────────────────────────────────────────────────────────────────────────

  @Input() node!: ContentNode;

  // ─────────────────────────────────────────────────────────────────────────
  // InteractiveRenderer Interface
  // ─────────────────────────────────────────────────────────────────────────

  @Output() complete = new EventEmitter<RendererCompletionEvent>();

  // ─────────────────────────────────────────────────────────────────────────
  // Additional Inputs
  // ─────────────────────────────────────────────────────────────────────────

  /** Show quiz header with title and progress */
  @Input() showHeader = true;

  /** Enable review mode (read-only with correct answers) */
  @Input() reviewMode = false;

  /** Target streak for "practiced" attestation (0 to disable) */
  @Input() targetStreak = 3;

  /** Show streak indicator */
  @Input() showStreakIndicator = false;

  // ─────────────────────────────────────────────────────────────────────────
  // View References
  // ─────────────────────────────────────────────────────────────────────────

  @ViewChild('questionComponent') questionComponent!: PerseusWrapperComponent;

  // ─────────────────────────────────────────────────────────────────────────
  // State
  // ─────────────────────────────────────────────────────────────────────────

  questions: PerseusItem[] = [];
  currentQuestionIndex = 0;
  hasAnswer = false;
  isSubmitting = false;
  showFeedback = false;
  showNextButton = false;
  hintShown = false;
  lastResult: PerseusScoreResult | null = null;

  // Scoring
  scores: PerseusScoreResult[] = [];
  correctCount = 0;

  // Streak tracking
  currentStreak = 0;
  streakHistory: (boolean | null)[] = [];
  streakDots: number[] = [];

  // Quiz mode (mastery vs discovery)
  quizMode: QuizMode = 'mastery';
  modeConfig: QuizModeConfig = QUIZ_MODE_PRESETS.mastery;
  subscaleScores: Record<string, number> = {};

  // ─────────────────────────────────────────────────────────────────────────
  // Computed Properties
  // ─────────────────────────────────────────────────────────────────────────

  get title(): string {
    return this.node?.title ?? 'Quiz';
  }

  get currentQuestion(): PerseusItem | null {
    return this.questions[this.currentQuestionIndex] ?? null;
  }

  get totalQuestions(): number {
    return this.questions.length;
  }

  get progressPercentage(): number {
    if (this.totalQuestions === 0) return 0;
    return ((this.currentQuestionIndex + 1) / this.totalQuestions) * 100;
  }

  get isLastQuestion(): boolean {
    return this.currentQuestionIndex >= this.totalQuestions - 1;
  }

  get showHintButton(): boolean {
    return this.currentQuestion?.hints !== undefined &&
           this.currentQuestion.hints.length > 0;
  }

  get submitButtonText(): string {
    if (this.isSubmitting) return this.quizMode === 'discovery' ? 'Recording...' : 'Checking...';
    if (this.showFeedback && this.modeConfig.showFeedback) return 'Submitted';
    return this.modeConfig.submitButtonText;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────────────

  ngOnInit(): void {
    this.initializeStreakTracking();
  }

  ngOnChanges(changes: SimpleChanges): void {
    // Load questions when node is set or changes
    if (changes['node'] && this.node) {
      this.loadQuestions();
      this.cdr.markForCheck();
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Event Handlers
  // ─────────────────────────────────────────────────────────────────────────

  handleReady(): void {
    // Question component is ready
    this.cdr.markForCheck();
  }

  handleAnswerChange(hasAnswer: boolean): void {
    this.hasAnswer = hasAnswer;
    this.cdr.markForCheck();
  }

  handleScore(result: PerseusScoreResult): void {
    this.lastResult = result;
    this.cdr.markForCheck();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Actions
  // ─────────────────────────────────────────────────────────────────────────

  submitAnswer(): void {
    if (!this.questionComponent || this.isSubmitting || this.showFeedback) {
      return;
    }

    this.isSubmitting = true;
    this.cdr.markForCheck();

    // Score the current answer
    const result = this.questionComponent.score();

    if (result) {
      this.lastResult = result;
      this.scores.push(result);

      if (this.quizMode === 'discovery') {
        // Discovery mode: record subscale contributions, no correctness feedback
        this.recordSubscaleContribution();
        this.showNextButton = true;
        // Don't show feedback for discovery mode
        this.showFeedback = false;
      } else {
        // Mastery mode: track correctness and streak
        if (result.correct) {
          this.correctCount++;
          this.updateStreak(true);
        } else {
          this.updateStreak(false);
        }
        this.showFeedback = true;
        this.showNextButton = true;
      }
    }

    this.isSubmitting = false;
    this.cdr.markForCheck();

    // Check if streak target reached (mastery mode only)
    if (this.modeConfig.trackStreak && this.targetStreak > 0 && this.currentStreak >= this.targetStreak) {
      this.emitCompletion(true);
    }
  }

  nextQuestion(): void {
    if (this.isLastQuestion) {
      this.finishQuiz();
    } else {
      this.currentQuestionIndex++;
      this.resetQuestionState();
    }
    this.cdr.markForCheck();
  }

  showHint(): void {
    // TODO: Implement hint display
    this.hintShown = true;
    this.cdr.markForCheck();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────

  private loadQuestions(): void {
    if (!this.node?.content) {
      console.warn('[PerseusRenderer] No content in node');
      return;
    }

    // ContentNode.content should be PerseusItem[] (array of Perseus items)
    // Handle: array, single item, or JSON string
    let content = this.node.content;

    // Parse JSON string if needed
    if (typeof content === 'string') {
      try {
        content = JSON.parse(content);
      } catch {
        console.error('[PerseusRenderer] Failed to parse content as JSON');
        return;
      }
    }

    // Extract questions
    if (Array.isArray(content)) {
      this.questions = content as PerseusItem[];
    } else if (typeof content === 'object' && content !== null) {
      // Single Perseus item (has question.widgets)
      this.questions = [content as PerseusItem];
    } else {
      console.error('[PerseusRenderer] Invalid content format:', typeof content);
      return;
    }

    console.log(`[PerseusRenderer] Loaded ${this.questions.length} question(s)`);
    // Debug: log first question structure
    if (this.questions.length > 0) {
      const q = this.questions[0];
      console.log('[PerseusRenderer] First question structure:', {
        id: q.id,
        hasQuestion: !!q.question,
        hasWidgets: !!q.question?.widgets,
        widgetCount: q.question?.widgets ? Object.keys(q.question.widgets).length : 0,
        contentPreview: q.question?.content?.substring(0, 100) || 'no content',
        discoveryMode: q.discoveryMode
      });
    }

    // Detect quiz mode after loading questions
    this.quizMode = this.detectQuizMode();
    this.modeConfig = QUIZ_MODE_PRESETS[this.quizMode];
    console.log(`[PerseusRenderer] Quiz mode: ${this.quizMode}`);

    // Initialize subscale tracking for discovery mode
    if (this.quizMode === 'discovery') {
      this.initializeSubscaleTracking();
    }
  }

  /**
   * Detect the quiz mode from question data.
   * Discovery mode is indicated by the discoveryMode flag on questions.
   */
  private detectQuizMode(): QuizMode {
    const firstQuestion = this.questions[0];
    if (firstQuestion?.discoveryMode) {
      return 'discovery';
    }
    // Check node metadata as fallback
    const metadata = this.node?.metadata as Record<string, unknown> | undefined;
    if (metadata?.['quizType'] === 'domain-discovery') {
      return 'discovery';
    }
    return 'mastery';
  }

  /**
   * Initialize subscale tracking for discovery assessments.
   * Collects unique subscale names from all question choices.
   */
  private initializeSubscaleTracking(): void {
    const subscales = new Set<string>();

    // Collect all subscale names from choices
    for (const question of this.questions) {
      const widgets = question.question?.widgets ?? {};
      for (const widgetKey of Object.keys(widgets)) {
        const widget = widgets[widgetKey];
        if (widget.type === 'radio') {
          const options = widget.options as RadioWidgetOptions;
          for (const choice of options.choices ?? []) {
            if (choice.subscaleContributions) {
              for (const subscale of Object.keys(choice.subscaleContributions)) {
                subscales.add(subscale);
              }
            }
          }
        }
      }
    }

    // Initialize all subscales to 0
    this.subscaleScores = {};
    Array.from(subscales).forEach(subscale => {
      this.subscaleScores[subscale] = 0;
    });

    console.log('[PerseusRenderer] Initialized subscales:', Object.keys(this.subscaleScores));
  }

  /**
   * Record subscale contributions from the current answer selection.
   * Called in discovery mode instead of tracking correctness.
   */
  private recordSubscaleContribution(): void {
    const question = this.currentQuestion;
    if (!question) return;

    // Get selected choice index from the result
    const result = this.lastResult;
    if (!result || !result.guess) return;

    // Get the radio widget
    const widgets = question.question?.widgets ?? {};
    const radioWidgetKey = Object.keys(widgets).find(key => widgets[key].type === 'radio');
    if (!radioWidgetKey) return;

    const widget = widgets[radioWidgetKey];
    const options = widget.options as RadioWidgetOptions;
    const choices = options?.choices ?? [];

    // Extract selected index from guess
    // Perseus radio guess format: { choicesSelected: [true, false, false, ...] }
    const guess = result.guess as { choicesSelected?: boolean[] } | undefined;
    const choicesSelected = guess?.choicesSelected;
    if (!choicesSelected || !Array.isArray(choicesSelected)) return;

    const selectedIndex = choicesSelected.findIndex(selected => selected);
    if (selectedIndex < 0 || selectedIndex >= choices.length) return;

    const selectedChoice = choices[selectedIndex];
    if (selectedChoice?.subscaleContributions) {
      for (const [subscale, value] of Object.entries(selectedChoice.subscaleContributions)) {
        this.subscaleScores[subscale] = (this.subscaleScores[subscale] ?? 0) + value;
      }
      console.log('[PerseusRenderer] Updated subscale scores:', this.subscaleScores);
    }
  }

  private initializeStreakTracking(): void {
    if (this.targetStreak > 0) {
      this.streakDots = Array(this.targetStreak).fill(0).map((_, i) => i);
      this.streakHistory = Array(this.targetStreak).fill(null);
    }
  }

  private updateStreak(correct: boolean): void {
    if (this.targetStreak <= 0) return;

    if (correct) {
      // Add to streak
      if (this.currentStreak < this.targetStreak) {
        this.streakHistory[this.currentStreak] = true;
        this.currentStreak++;
      }
    } else {
      // Break streak - reset
      this.currentStreak = 0;
      this.streakHistory = Array(this.targetStreak).fill(null);
    }
  }

  private resetQuestionState(): void {
    this.hasAnswer = false;
    this.showFeedback = false;
    this.showNextButton = false;
    this.hintShown = false;
    this.lastResult = null;
  }

  private finishQuiz(): void {
    if (this.quizMode === 'discovery') {
      // Discovery mode: emit subscale scores for path recommendation
      this.emitDiscoveryCompletion();
    } else {
      // Mastery mode: calculate pass/fail based on score
      const totalScore = this.calculateTotalScore();
      const passed = totalScore >= 0.7; // 70% passing threshold
      this.emitCompletion(passed, totalScore);
    }
  }

  private calculateTotalScore(): number {
    if (this.scores.length === 0) return 0;

    const totalScore = this.scores.reduce((sum, s) => sum + s.score, 0);
    return totalScore / this.scores.length;
  }

  private emitCompletion(passed: boolean, score?: number): void {
    const finalScore = score ?? (this.correctCount / Math.max(this.scores.length, 1));

    const event: RendererCompletionEvent = {
      type: 'quiz',
      passed,
      score: Math.round(finalScore * 100),
      details: {
        correct: this.correctCount,
        total: this.scores.length,
        streakAchieved: this.targetStreak > 0 && this.currentStreak >= this.targetStreak,
        questions: this.questions.map((q, i) => ({
          id: q.id,
          correct: this.scores[i]?.correct ?? false,
          score: this.scores[i]?.score ?? 0
        }))
      }
    };

    this.complete.emit(event);
  }

  /**
   * Emit completion event for discovery assessments.
   * Includes subscale scores and primary domain recommendation.
   */
  private emitDiscoveryCompletion(): void {
    // Find the primary domain (highest subscale score)
    let primaryDomain = '';
    let maxScore = 0;
    for (const [subscale, score] of Object.entries(this.subscaleScores)) {
      if (score > maxScore) {
        maxScore = score;
        primaryDomain = subscale;
      }
    }

    console.log('[PerseusRenderer] Discovery complete:', {
      subscaleScores: this.subscaleScores,
      primaryDomain
    });

    const event: RendererCompletionEvent = {
      type: 'quiz',
      passed: true, // Discovery assessments always "pass"
      score: 100, // No score concept for discovery
      details: {
        quizMode: 'discovery',
        subscaleScores: { ...this.subscaleScores },
        primaryDomain,
        total: this.questions.length,
        correct: this.questions.length, // All answers are valid in discovery mode
        questions: this.questions.map((q, i) => ({
          id: q.id,
          correct: true, // All answers are valid
          score: 1
        }))
      }
    };

    this.complete.emit(event);
  }
}
