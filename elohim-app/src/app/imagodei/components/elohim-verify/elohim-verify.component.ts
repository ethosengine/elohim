/**
 * ElohimVerifyComponent - AI-assisted identity verification
 *
 * This component provides an alternative to interview-based recovery.
 * Elohim (AI) asks questions derived from the user's actual profile data
 * (paths completed, quiz scores, relationships, preferences).
 *
 * Only the real user should know the answers, making this hard to
 * social-engineer compared to static security questions.
 *
 * Flow:
 * 1. Start verification (fetch questions from doorway)
 * 2. Answer questions within time limit
 * 3. Submit and receive confidence score
 * 4. If passed, contributes to recovery threshold
 */

import { CommonModule } from '@angular/common';
import { Component, OnDestroy, signal, computed, Input } from '@angular/core';
import { FormsModule } from '@angular/forms';

/** Question from the doorway */
interface VerificationQuestion {
  id: string;
  question: string;
  category: string;
  is_multiple_choice: boolean;
  options?: string[];
}

/** Answer to submit */
interface QuestionAnswer {
  questionId: string;
  answer: string;
}

/** Feedback for a question */
interface QuestionFeedback {
  questionId: string;
  correct: boolean;
  message: string;
}

/** Verification result */
interface VerificationResult {
  passed: boolean;
  accuracyPercent: number;
  confidenceScore: number;
  summary: string;
  feedback?: QuestionFeedback[];
}

type VerificationStep = 'intro' | 'questions' | 'submitting' | 'result';

@Component({
  selector: 'app-elohim-verify',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './elohim-verify.component.html',
  styleUrls: ['./elohim-verify.component.css'],
})
export class ElohimVerifyComponent implements OnDestroy {
  /** Recovery request ID to link verification to */
  @Input() requestId = '';

  /** Doorway URL to call */
  @Input() doorwayUrl = '';

  // ===========================================================================
  // State
  // ===========================================================================

  readonly currentStep = signal<VerificationStep>('intro');
  readonly questions = signal<VerificationQuestion[]>([]);
  readonly answers = signal<Map<string, string>>(new Map());
  readonly sessionId = signal<string>('');
  readonly result = signal<VerificationResult | null>(null);
  readonly error = signal<string | null>(null);
  readonly isLoading = signal(false);

  // Timer
  readonly timeRemaining = signal(300); // 5 minutes default
  readonly timeLimitSeconds = signal(300);
  private timerInterval: ReturnType<typeof setInterval> | null = null;

  // ===========================================================================
  // Computed
  // ===========================================================================

  readonly currentQuestionIndex = signal(0);

  readonly currentQuestion = computed(() => {
    const qs = this.questions();
    const idx = this.currentQuestionIndex();
    return qs[idx] ?? null;
  });

  readonly progress = computed(() => {
    const total = this.questions().length;
    const answered = this.answers().size;
    return total > 0 ? Math.round((answered / total) * 100) : 0;
  });

  readonly canSubmit = computed(() => {
    const total = this.questions().length;
    const answered = this.answers().size;
    return answered >= total && !this.isLoading();
  });

  readonly timeDisplay = computed(() => {
    const secs = this.timeRemaining();
    const mins = Math.floor(secs / 60);
    const remaining = secs % 60;
    return `${mins}:${remaining.toString().padStart(2, '0')}`;
  });

  readonly isTimeWarning = computed(() => this.timeRemaining() < 60);

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  ngOnDestroy(): void {
    this.stopTimer();
  }

  // ===========================================================================
  // Actions
  // ===========================================================================

  async startVerification(): Promise<void> {
    this.error.set(null);
    this.isLoading.set(true);

    try {
      const response = await fetch(`${this.doorwayUrl}/auth/elohim-verify/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ requestId: this.requestId }),
      });

      if (!response.ok) {
        const err = await response.json();
        throw new Error(err.error ?? 'Failed to start verification');
      }

      const data = await response.json();
      this.sessionId.set(data.sessionId);
      this.questions.set(data.questions ?? []);
      this.timeLimitSeconds.set(data.timeLimitSeconds ?? 300);
      this.timeRemaining.set(data.timeLimitSeconds ?? 300);
      this.answers.set(new Map());
      this.currentQuestionIndex.set(0);
      this.currentStep.set('questions');
      this.startTimer();
    } catch (e: any) {
      this.error.set(e.message ?? 'Failed to start verification');
    } finally {
      this.isLoading.set(false);
    }
  }

  setAnswer(questionId: string, answer: string): void {
    const newAnswers = new Map(this.answers());
    newAnswers.set(questionId, answer);
    this.answers.set(newAnswers);
  }

  nextQuestion(): void {
    const idx = this.currentQuestionIndex();
    if (idx < this.questions().length - 1) {
      this.currentQuestionIndex.set(idx + 1);
    }
  }

  prevQuestion(): void {
    const idx = this.currentQuestionIndex();
    if (idx > 0) {
      this.currentQuestionIndex.set(idx - 1);
    }
  }

  goToQuestion(index: number): void {
    if (index >= 0 && index < this.questions().length) {
      this.currentQuestionIndex.set(index);
    }
  }

  async submitAnswers(): Promise<void> {
    this.error.set(null);
    this.isLoading.set(true);
    this.currentStep.set('submitting');
    this.stopTimer();

    try {
      // Convert map to array
      const answersArray: QuestionAnswer[] = [];
      this.answers().forEach((answer, questionId) => {
        answersArray.push({ questionId, answer });
      });

      const response = await fetch(`${this.doorwayUrl}/auth/elohim-verify/answer`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sessionId: this.sessionId(),
          answers: answersArray,
        }),
      });

      if (!response.ok) {
        const err = await response.json();
        throw new Error(err.error ?? 'Failed to submit answers');
      }

      const data = await response.json();
      this.result.set(data);
      this.currentStep.set('result');
    } catch (e: any) {
      this.error.set(e.message ?? 'Failed to submit answers');
      this.currentStep.set('questions');
      this.startTimer(); // Resume timer on error
    } finally {
      this.isLoading.set(false);
    }
  }

  retry(): void {
    this.result.set(null);
    this.questions.set([]);
    this.answers.set(new Map());
    this.sessionId.set('');
    this.currentStep.set('intro');
  }

  clearError(): void {
    this.error.set(null);
  }

  // ===========================================================================
  // Timer
  // ===========================================================================

  private startTimer(): void {
    if (this.timerInterval) return;

    this.timerInterval = setInterval(() => {
      const remaining = this.timeRemaining();
      if (remaining <= 0) {
        this.stopTimer();
        void this.submitAnswers(); // Auto-submit when time runs out
      } else {
        this.timeRemaining.set(remaining - 1);
      }
    }, 1000);
  }

  private stopTimer(): void {
    if (this.timerInterval) {
      clearInterval(this.timerInterval);
      this.timerInterval = null;
    }
  }

  // ===========================================================================
  // Template Helpers
  // ===========================================================================

  getCategoryIcon(category: string): string {
    const icons: Record<string, string> = {
      content_mastery: 'school',
      relationships: 'people',
      preferences: 'settings',
      quiz_scores: 'assessment',
      account_history: 'history',
    };
    return icons[category] || 'help';
  }

  getCategoryLabel(category: string): string {
    const labels: Record<string, string> = {
      content_mastery: 'Learning History',
      relationships: 'Connections',
      preferences: 'Preferences',
      quiz_scores: 'Quiz Performance',
      account_history: 'Account',
    };
    return labels[category] || category;
  }

  getAnswerForQuestion(questionId: string): string {
    return this.answers().get(questionId) ?? '';
  }

  isQuestionAnswered(index: number): boolean {
    const question = this.questions()[index];
    return question ? this.answers().has(question.id) : false;
  }

  getFeedbackForQuestion(questionId: string): QuestionFeedback | undefined {
    return this.result()?.feedback?.find(f => f.questionId === questionId);
  }
}
