/**
 * Discovery Quiz Component - Self-discovery assessments for path personalization.
 *
 * Unlike graded quizzes, discovery quizzes have no right/wrong answers.
 * Each answer contributes to subscale scores that determine which
 * epic domain resonates most with the user.
 *
 * Features:
 * - Progress indicator showing question position
 * - One question at a time display
 * - Subscale tracking across all answers
 * - Final result showing recommended domain
 * - Integration with DiscoveryAttestationService
 *
 * @example
 * ```html
 * <app-discovery-quiz
 *   [quizId]="'quiz-who-are-you'"
 *   [humanId]="currentUser.id"
 *   (completed)="onDiscoveryComplete($event)">
 * </app-discovery-quiz>
 * ```
 */

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
  computed
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { Subject, takeUntil } from 'rxjs';

import { PerseusWrapperComponent } from '../../../content-io/plugins/perseus/perseus-wrapper.component';
import {
  PerseusItem,
  PerseusScoreResult,
  RadioWidgetOptions,
  RadioChoice
} from '../../../content-io/plugins/perseus/perseus-item.model';
import { QuestionPoolService } from '../../services/question-pool.service';
import { DiscoveryAttestationService } from '../../services/discovery-attestation.service';
import { type DiscoveryResultSummary } from '../../models/discovery-assessment.model';

// =============================================================================
// Types
// =============================================================================

/**
 * Subscale mappings for epic domains.
 */
export const EPIC_SUBSCALES: Record<string, { name: string; color: string; icon: string }> = {
  governance: {
    name: 'AI Constitutional',
    color: '#8B5CF6', // Purple
    icon: '🏛️'
  },
  care: {
    name: 'Value Scanner',
    color: '#EC4899', // Pink
    icon: '💝'
  },
  economic: {
    name: 'Economic Coordination',
    color: '#10B981', // Green
    icon: '📊'
  },
  public: {
    name: 'Public Observer',
    color: '#3B82F6', // Blue
    icon: '🔍'
  },
  social: {
    name: 'Social Medium',
    color: '#F59E0B', // Amber
    icon: '💬'
  }
};

/**
 * Completion event for discovery quiz.
 */
export interface DiscoveryQuizCompletionEvent {
  /** Quiz ID */
  quizId: string;

  /** Human ID who completed */
  humanId: string;

  /** All subscale scores */
  subscaleScores: Record<string, number>;

  /** Primary (highest scoring) subscale */
  primarySubscale: string;

  /** Recommended epic domain */
  recommendedEpic: string;

  /** Total questions answered */
  totalAnswered: number;
}

// =============================================================================
// Component
// =============================================================================

@Component({
  selector: 'app-discovery-quiz',
  standalone: true,
  imports: [CommonModule, PerseusWrapperComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <section
      class="discovery-quiz"
      [class.completed]="showResults()"
      [class.loading]="loading()">

      <!-- Header -->
      <header class="quiz-header">
        <div class="header-content">
          <span class="quiz-icon">{{ showResults() ? '✨' : '🎯' }}</span>
          <h3 class="quiz-title">{{ getHeaderTitle() }}</h3>
        </div>
      </header>

      <!-- Progress indicator -->
      @if (!showResults() && totalQuestions() > 0) {
        <div class="progress-bar">
          <div class="progress-fill" [style.width.%]="progressPercent()"></div>
        </div>
        <div class="progress-label">
          Question {{ currentQuestionIndex() + 1 }} of {{ totalQuestions() }}
        </div>
      }

      <!-- Quiz content -->
      <div class="quiz-content">
        @if (loading()) {
          <div class="loading-state">
            <span class="loading-spinner"></span>
            Loading discovery assessment...
          </div>
        } @else if (noQuestions()) {
          <div class="empty-state">
            <p>No questions available for this assessment.</p>
          </div>
        } @else if (showResults()) {
          <!-- Results display -->
          <div class="results-section">
            <div class="result-card">
              <div class="result-icon">{{ getResultIcon() }}</div>
              <h4 class="result-title">Your Path Resonates With</h4>
              <div class="result-name">{{ getResultName() }}</div>
              <p class="result-description">{{ getResultDescription() }}</p>
            </div>

            <!-- Subscale breakdown -->
            <div class="subscale-breakdown">
              <h5>Your Interest Profile</h5>
              @for (subscale of sortedSubscales(); track subscale.key) {
                <div class="subscale-row">
                  <span class="subscale-label">
                    <span class="subscale-icon">{{ subscale.icon }}</span>
                    {{ subscale.name }}
                  </span>
                  <div class="subscale-bar">
                    <div
                      class="subscale-fill"
                      [style.width.%]="subscale.percent"
                      [style.backgroundColor]="subscale.color">
                    </div>
                  </div>
                  <span class="subscale-percent">{{ subscale.percent | number:'1.0-0' }}%</span>
                </div>
              }
            </div>

            <button class="btn-explore" (click)="onExploreEpic()">
              Explore {{ getResultName() }} →
            </button>
          </div>
        } @else {
          <!-- Question display -->
          @if (currentQuestion()) {
            <div class="question-container">
              <app-perseus-question
                [item]="currentQuestion()!"
                [reviewMode]="false"
                [autoFocus]="true"
                (scored)="onQuestionScored($event)"
                (answerChanged)="onAnswerChanged()">
              </app-perseus-question>

              <div class="answer-controls">
                <button
                  class="btn-continue"
                  [disabled]="!hasAnswer()"
                  (click)="continueToNext()">
                  {{ isLastQuestion() ? 'See Results' : 'Continue' }}
                </button>
              </div>
            </div>
          }
        }
      </div>
    </section>
  `,
  styles: [`
    .discovery-quiz {
      margin-top: 2rem;
      border: 1px solid var(--border-color, #e9ecef);
      border-radius: var(--radius-lg, 12px);
      background: var(--surface-secondary, #f8f9fa);
      overflow: hidden;
      transition: all 0.3s ease;
    }

    .discovery-quiz.completed {
      border-color: var(--primary, #4285f4);
      background: linear-gradient(
        to bottom,
        var(--primary-surface, #e8f0fe) 0%,
        var(--surface-secondary, #f8f9fa) 100%
      );
    }

    /* Header */
    .quiz-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 1rem 1.25rem;
      background: var(--surface-elevated, #fff);
      border-bottom: 1px solid var(--border-color, #e9ecef);
    }

    .header-content {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .quiz-icon {
      font-size: 1.5rem;
    }

    .quiz-title {
      margin: 0;
      font-size: 1.125rem;
      font-weight: 600;
      color: var(--text-primary, #202124);
    }

    /* Progress */
    .progress-bar {
      height: 4px;
      background: var(--surface-tertiary, #e8eaed);
      overflow: hidden;
    }

    .progress-fill {
      height: 100%;
      background: var(--primary, #4285f4);
      transition: width 0.3s ease;
    }

    .progress-label {
      padding: 0.5rem 1.25rem;
      font-size: 0.875rem;
      color: var(--text-secondary, #5f6368);
      text-align: center;
    }

    /* Content */
    .quiz-content {
      padding: 1.25rem;
    }

    /* Question container */
    .question-container {
      background: var(--surface-elevated, #fff);
      border-radius: var(--radius-md, 8px);
      border: 1px solid var(--border-color, #e9ecef);
      padding: 1.25rem;
    }

    /* Controls */
    .answer-controls {
      margin-top: 1.25rem;
      padding-top: 1.25rem;
      border-top: 1px solid var(--border-color, #e9ecef);
      display: flex;
      justify-content: flex-end;
    }

    .btn-continue,
    .btn-explore {
      padding: 0.75rem 1.5rem;
      font-size: 1rem;
      font-weight: 500;
      border-radius: var(--radius-md, 8px);
      cursor: pointer;
      transition: all 0.15s ease;
    }

    .btn-continue {
      background: var(--primary, #4285f4);
      color: white;
      border: none;
    }

    .btn-continue:hover:not(:disabled) {
      background: var(--primary-dark, #1a73e8);
    }

    .btn-continue:disabled {
      background: var(--surface-tertiary, #e8eaed);
      color: var(--text-disabled, #9aa0a6);
      cursor: not-allowed;
    }

    /* Results */
    .results-section {
      text-align: center;
    }

    .result-card {
      padding: 2rem;
      margin-bottom: 1.5rem;
      background: var(--surface-elevated, #fff);
      border-radius: var(--radius-md, 8px);
      border: 1px solid var(--border-color, #e9ecef);
    }

    .result-icon {
      font-size: 3rem;
      margin-bottom: 0.75rem;
    }

    .result-title {
      margin: 0 0 0.5rem;
      font-size: 0.875rem;
      font-weight: 500;
      color: var(--text-secondary, #5f6368);
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .result-name {
      font-size: 1.5rem;
      font-weight: 700;
      color: var(--primary, #4285f4);
      margin-bottom: 0.5rem;
    }

    .result-description {
      margin: 0;
      color: var(--text-secondary, #5f6368);
      font-size: 0.9375rem;
      line-height: 1.5;
      max-width: 400px;
      margin-inline: auto;
    }

    /* Subscale breakdown */
    .subscale-breakdown {
      text-align: left;
      padding: 1.25rem;
      margin-bottom: 1.5rem;
      background: var(--surface-elevated, #fff);
      border-radius: var(--radius-md, 8px);
      border: 1px solid var(--border-color, #e9ecef);
    }

    .subscale-breakdown h5 {
      margin: 0 0 1rem;
      font-size: 0.9375rem;
      font-weight: 600;
      color: var(--text-primary, #202124);
    }

    .subscale-row {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      margin-bottom: 0.75rem;
    }

    .subscale-row:last-child {
      margin-bottom: 0;
    }

    .subscale-label {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      width: 160px;
      font-size: 0.875rem;
      color: var(--text-primary, #202124);
    }

    .subscale-icon {
      font-size: 1rem;
    }

    .subscale-bar {
      flex: 1;
      height: 8px;
      background: var(--surface-tertiary, #e8eaed);
      border-radius: 4px;
      overflow: hidden;
    }

    .subscale-fill {
      height: 100%;
      border-radius: 4px;
      transition: width 0.5s ease;
    }

    .subscale-percent {
      width: 3rem;
      text-align: right;
      font-size: 0.875rem;
      font-weight: 500;
      color: var(--text-secondary, #5f6368);
    }

    .btn-explore {
      background: var(--primary, #4285f4);
      color: white;
      border: none;
      width: 100%;
      max-width: 300px;
    }

    .btn-explore:hover {
      background: var(--primary-dark, #1a73e8);
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
      to { transform: rotate(360deg); }
    }
  `]
})
export class DiscoveryQuizComponent implements OnInit, OnDestroy {
  /** Quiz ID to load */
  @Input({ required: true }) quizId!: string;

  /** Human ID taking the quiz */
  @Input({ required: true }) humanId!: string;

  /** Emitted when quiz completes */
  @Output() completed = new EventEmitter<DiscoveryQuizCompletionEvent>();

  /** Emitted when user wants to explore the recommended epic */
  @Output() exploreEpic = new EventEmitter<string>();

  // State signals
  protected loading = signal(true);
  protected noQuestions = signal(false);
  protected showResults = signal(false);
  protected hasAnswer = signal(false);

  protected questions = signal<PerseusItem[]>([]);
  protected currentQuestionIndex = signal(0);
  protected subscaleScores = signal<Record<string, number>>({
    governance: 0,
    care: 0,
    economic: 0,
    public: 0,
    social: 0
  });
  protected selectedChoiceIndex = signal<number | null>(null);

  // Computed values
  protected totalQuestions = computed(() => this.questions().length);

  protected currentQuestion = computed(() => {
    const qs = this.questions();
    const idx = this.currentQuestionIndex();
    return qs[idx] ?? null;
  });

  protected progressPercent = computed(() => {
    const total = this.totalQuestions();
    if (total === 0) return 0;
    return ((this.currentQuestionIndex() + 1) / total) * 100;
  });

  protected isLastQuestion = computed(() => {
    return this.currentQuestionIndex() === this.totalQuestions() - 1;
  });

  protected sortedSubscales = computed(() => {
    const scores = this.subscaleScores();
    const total = Object.values(scores).reduce((sum, v) => sum + v, 0) || 1;

    return Object.entries(scores)
      .map(([key, value]) => ({
        key,
        name: EPIC_SUBSCALES[key]?.name ?? key,
        icon: EPIC_SUBSCALES[key]?.icon ?? '📌',
        color: EPIC_SUBSCALES[key]?.color ?? '#888',
        score: value,
        percent: (value / total) * 100
      }))
      .sort((a, b) => b.score - a.score);
  });

  protected primarySubscale = computed(() => {
    const sorted = this.sortedSubscales();
    return sorted[0]?.key ?? 'governance';
  });

  private destroy$ = new Subject<void>();
  private lastScoreResult: PerseusScoreResult | null = null;

  constructor(
    private readonly poolService: QuestionPoolService,
    private readonly discoveryService: DiscoveryAttestationService,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.loadQuestions();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Handle answer change from Perseus.
   */
  onAnswerChanged(): void {
    this.hasAnswer.set(true);
  }

  /**
   * Handle score result from Perseus (used to get selected choice).
   */
  onQuestionScored(result: PerseusScoreResult): void {
    this.lastScoreResult = result;
  }

  /**
   * Continue to next question or show results.
   */
  continueToNext(): void {
    // Record subscale contributions from current answer
    this.recordSubscaleContribution();

    if (this.isLastQuestion()) {
      this.completeQuiz();
    } else {
      // Move to next question
      this.currentQuestionIndex.update(idx => idx + 1);
      this.hasAnswer.set(false);
      this.selectedChoiceIndex.set(null);
      this.lastScoreResult = null;
      this.cdr.markForCheck();
    }
  }

  /**
   * Get header title based on state.
   */
  getHeaderTitle(): string {
    if (this.showResults()) {
      return 'Your Results';
    }
    return 'Find Your Path';
  }

  /**
   * Get result icon for the primary subscale.
   */
  getResultIcon(): string {
    const key = this.primarySubscale();
    return EPIC_SUBSCALES[key]?.icon ?? '🎯';
  }

  /**
   * Get result name for the primary subscale.
   */
  getResultName(): string {
    const key = this.primarySubscale();
    return EPIC_SUBSCALES[key]?.name ?? key;
  }

  /**
   * Get result description for the primary subscale.
   */
  getResultDescription(): string {
    const descriptions: Record<string, string> = {
      governance: 'You\'re drawn to shaping how AI systems are governed and ensuring they serve humanity through proper constitutional frameworks.',
      care: 'You\'re passionate about recognizing and valuing care work, supporting caregivers, and making invisible contributions visible.',
      economic: 'You\'re interested in transforming workplace dynamics, promoting worker ownership, and creating more equitable economic systems.',
      public: 'You\'re committed to strengthening democratic participation, increasing transparency, and empowering civic engagement.',
      social: 'You\'re focused on building healthier digital spaces, fostering genuine connection, and improving online communication.'
    };

    const key = this.primarySubscale();
    return descriptions[key] ?? 'Your interests align with this domain.';
  }

  /**
   * Handle explore epic button click.
   */
  onExploreEpic(): void {
    this.exploreEpic.emit(this.primarySubscale());
  }

  // ===========================================================================
  // Private Methods
  // ===========================================================================

  private loadQuestions(): void {
    this.loading.set(true);

    this.poolService.getPoolForContent(this.quizId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: pool => {
          if (pool && pool.questions.length > 0) {
            // Mark questions as discovery mode
            const discoveryQuestions = pool.questions.map(q => ({
              ...q,
              discoveryMode: true
            }));
            this.questions.set(discoveryQuestions);
            this.noQuestions.set(false);
          } else {
            this.noQuestions.set(true);
          }
          this.loading.set(false);
          this.cdr.markForCheck();
        },
        error: err => {
          console.error('Failed to load discovery questions:', err);
          this.noQuestions.set(true);
          this.loading.set(false);
          this.cdr.markForCheck();
        }
      });
  }

  private recordSubscaleContribution(): void {
    const question = this.currentQuestion();
    if (!question || !this.lastScoreResult) return;

    // Get the selected choice index from the score result
    const guess = this.lastScoreResult.guess as { choicesSelected?: number[] };
    const selectedIndex = guess?.choicesSelected?.[0];

    if (selectedIndex === undefined || selectedIndex === null) return;

    // Get the choice from the question
    const radioWidget = question.question.widgets['radio 1'] as { options: RadioWidgetOptions } | undefined;
    if (!radioWidget) return;

    const choices = radioWidget.options.choices as RadioChoice[];
    const selectedChoice = choices[selectedIndex];

    if (!selectedChoice) return;

    // Check for subscale contributions
    if (selectedChoice.subscaleContributions) {
      // Use explicit subscale contributions
      this.subscaleScores.update(scores => {
        const newScores = { ...scores };
        for (const [subscale, contribution] of Object.entries(selectedChoice.subscaleContributions!)) {
          newScores[subscale] = (newScores[subscale] || 0) + contribution;
        }
        return newScores;
      });
    } else {
      // Fall back to index-based mapping (A=governance, B=care, etc.)
      const subscaleOrder = ['governance', 'care', 'economic', 'public', 'social'];
      const subscale = subscaleOrder[selectedIndex];
      if (subscale) {
        this.subscaleScores.update(scores => ({
          ...scores,
          [subscale]: (scores[subscale] || 0) + 1
        }));
      }
    }
  }

  private completeQuiz(): void {
    this.showResults.set(true);

    const scores = this.subscaleScores();
    const primary = this.primarySubscale();

    // Create discovery attestation
    const primaryResult: DiscoveryResultSummary = {
      typeId: primary,
      name: EPIC_SUBSCALES[primary]?.name ?? primary,
      shortCode: primary.substring(0, 3).toUpperCase(),
      score: scores[primary] / this.totalQuestions(),
      icon: EPIC_SUBSCALES[primary]?.icon,
      color: EPIC_SUBSCALES[primary]?.color
    };

    // Emit completion event
    const event: DiscoveryQuizCompletionEvent = {
      quizId: this.quizId,
      humanId: this.humanId,
      subscaleScores: scores,
      primarySubscale: primary,
      recommendedEpic: primary,
      totalAnswered: this.totalQuestions()
    };

    this.completed.emit(event);
    this.cdr.markForCheck();
  }
}
