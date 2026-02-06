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

// @coverage: 88.6% (2026-02-05)

import { Subject, takeUntil } from 'rxjs';

import { CooldownStatus } from '../../models/attempt-record.model';
import { AttemptCooldownService } from '../../services/attempt-cooldown.service';
import { PathAdaptationService, GateStatus } from '../../services/path-adaptation.service';

/**
 * Event emitted when mastery quiz is requested.
 */
export interface MasteryQuizRequestEvent {
  /** Section ID the quiz unlocks */
  sectionId: string;

  /** Section title */
  sectionTitle: string;

  /** Content IDs in the section */
  contentIds: string[];
}

/**
 * MasteryGateComponent - Section boundary gate UI.
 *
 * Displays when a learner tries to proceed to a locked section.
 * Shows:
 * - Gate status (locked, cooldown, attempts remaining)
 * - Best score achieved
 * - Option to start mastery quiz
 * - Cooldown timer when applicable
 *
 * Gates are invisible until triggered - they don't appear in the path
 * as steps, only when the learner tries to proceed.
 *
 * @example
 * ```html
 * <app-mastery-gate
 *   [pathId]="currentPath.id"
 *   [sectionId]="nextSection.id"
 *   [sectionTitle]="nextSection.title"
 *   [humanId]="currentUser.id"
 *   [contentIds]="nextSection.conceptIds"
 *   (startQuiz)="onStartMasteryQuiz($event)"
 *   (bypassed)="onGateBypassed()">
 * </app-mastery-gate>
 * ```
 */
@Component({
  selector: 'app-mastery-gate',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div
      class="mastery-gate"
      [class.unlocked]="!isLocked()"
      [class.in-cooldown]="inCooldown()"
      role="dialog"
      aria-labelledby="gate-title"
    >
      <!-- Gate header -->
      <header class="gate-header">
        <div class="gate-icon" [class.unlocked]="!isLocked()">
          @if (isLocked()) {
            <span class="icon-locked">🔒</span>
          } @else {
            <span class="icon-unlocked">🔓</span>
          }
        </div>
        <div class="gate-title-area">
          <h2 id="gate-title" class="gate-title">
            @if (isLocked()) {
              Section Checkpoint
            } @else {
              Section Unlocked!
            }
          </h2>
          <p class="gate-subtitle">{{ sectionTitle }}</p>
        </div>
      </header>

      <!-- Gate body -->
      <div class="gate-body">
        @if (isLocked()) {
          <!-- Locked state -->
          <p class="gate-message">Complete a mastery check to continue to the next section.</p>

          <!-- Progress/attempts info -->
          <div class="gate-stats">
            @if (bestScore() > 0) {
              <div class="stat">
                <span class="stat-label">Best Score</span>
                <span class="stat-value">{{ formatPercent(bestScore()) }}</span>
              </div>
            }
            <div class="stat">
              <span class="stat-label">Attempts Today</span>
              <span class="stat-value">{{ attemptsUsed() }} / {{ maxAttempts() }}</span>
            </div>
          </div>

          <!-- Cooldown timer -->
          @if (inCooldown()) {
            <div class="cooldown-notice">
              <span class="cooldown-icon">⏳</span>
              <div class="cooldown-text">
                <span class="cooldown-label">Next attempt available in</span>
                <span class="cooldown-time">{{ cooldownRemaining() }}</span>
              </div>
            </div>
          }

          <!-- Quiz button -->
          <div class="gate-actions">
            <button class="btn-start-quiz" [disabled]="!canAttempt()" (click)="onStartQuiz()">
              @if (inCooldown()) {
                Cooldown Active
              } @else if (attemptsRemaining() === 0) {
                No Attempts Left
              } @else if (bestScore() > 0) {
                Try Again
              } @else {
                Start Mastery Check
              }
            </button>

            @if (attemptsRemaining() > 0 && !inCooldown()) {
              <p class="attempts-hint">
                {{ attemptsRemaining() }} attempt{{ attemptsRemaining() === 1 ? '' : 's' }}
                remaining today
              </p>
            }
          </div>
        } @else {
          <!-- Unlocked state -->
          <div class="unlocked-message">
            <span class="success-icon">✓</span>
            <p>You've mastered this section!</p>
            @if (bestScore() > 0) {
              <p class="final-score">Final Score: {{ formatPercent(bestScore()) }}</p>
            }
          </div>

          <div class="gate-actions">
            <button class="btn-continue" (click)="onContinue()">Continue to Next Section</button>
          </div>
        }
      </div>

      <!-- Tip footer -->
      @if (isLocked() && !inCooldown()) {
        <footer class="gate-footer">
          <p class="gate-tip">
            💡 Review the content in this section before attempting the mastery check.
          </p>
        </footer>
      }
    </div>
  `,
  styles: [
    `
      .mastery-gate {
        max-width: 480px;
        margin: 2rem auto;
        background: var(--surface-elevated, #fff);
        border-radius: var(--radius-lg, 12px);
        border: 2px solid var(--warning, #fbbc04);
        overflow: hidden;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.1);
      }

      .mastery-gate.unlocked {
        border-color: var(--success, #34a853);
      }

      .mastery-gate.in-cooldown {
        border-color: var(--border-color, #e9ecef);
      }

      /* Header */
      .gate-header {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1.25rem 1.5rem;
        background: linear-gradient(
          to bottom,
          var(--warning-surface, #fef7e0) 0%,
          var(--surface-elevated, #fff) 100%
        );
      }

      .mastery-gate.unlocked .gate-header {
        background: linear-gradient(
          to bottom,
          var(--success-surface, #e6f4ea) 0%,
          var(--surface-elevated, #fff) 100%
        );
      }

      .gate-icon {
        width: 3rem;
        height: 3rem;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 1.5rem;
        background: var(--warning, #fbbc04);
        border-radius: 50%;
      }

      .gate-icon.unlocked {
        background: var(--success, #34a853);
      }

      .gate-title-area {
        flex: 1;
      }

      .gate-title {
        margin: 0;
        font-size: 1.125rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
      }

      .gate-subtitle {
        margin: 0.25rem 0 0;
        font-size: 0.875rem;
        color: var(--text-secondary, #5f6368);
      }

      /* Body */
      .gate-body {
        padding: 1.5rem;
      }

      .gate-message {
        margin: 0 0 1.25rem;
        font-size: 1rem;
        color: var(--text-primary, #202124);
        text-align: center;
      }

      /* Stats */
      .gate-stats {
        display: flex;
        justify-content: center;
        gap: 2rem;
        margin-bottom: 1.5rem;
      }

      .stat {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.25rem;
      }

      .stat-label {
        font-size: 0.75rem;
        font-weight: 500;
        text-transform: uppercase;
        letter-spacing: 0.03em;
        color: var(--text-tertiary, #80868b);
      }

      .stat-value {
        font-size: 1.25rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
      }

      /* Cooldown */
      .cooldown-notice {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.75rem;
        padding: 1rem;
        margin-bottom: 1.5rem;
        background: var(--surface-secondary, #f8f9fa);
        border-radius: var(--radius-md, 8px);
      }

      .cooldown-icon {
        font-size: 1.5rem;
      }

      .cooldown-text {
        display: flex;
        flex-direction: column;
        gap: 0.125rem;
      }

      .cooldown-label {
        font-size: 0.75rem;
        color: var(--text-secondary, #5f6368);
      }

      .cooldown-time {
        font-size: 1.25rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
        font-variant-numeric: tabular-nums;
      }

      /* Actions */
      .gate-actions {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.75rem;
      }

      .btn-start-quiz,
      .btn-continue {
        width: 100%;
        padding: 0.875rem 1.5rem;
        font-size: 1rem;
        font-weight: 600;
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .btn-start-quiz {
        background: var(--primary, #4285f4);
        color: white;
        border: none;
      }

      .btn-start-quiz:hover:not(:disabled) {
        background: var(--primary-dark, #1a73e8);
        transform: translateY(-1px);
      }

      .btn-start-quiz:disabled {
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-disabled, #9aa0a6);
        cursor: not-allowed;
      }

      .btn-continue {
        background: var(--success, #34a853);
        color: white;
        border: none;
      }

      .btn-continue:hover {
        background: var(--success-dark, #1e8e3e);
        transform: translateY(-1px);
      }

      .attempts-hint {
        margin: 0;
        font-size: 0.8125rem;
        color: var(--text-tertiary, #80868b);
      }

      /* Unlocked state */
      .unlocked-message {
        text-align: center;
        margin-bottom: 1.5rem;
      }

      .success-icon {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 3rem;
        height: 3rem;
        margin-bottom: 0.75rem;
        font-size: 1.5rem;
        background: var(--success, #34a853);
        color: white;
        border-radius: 50%;
      }

      .unlocked-message p {
        margin: 0;
        font-size: 1rem;
        color: var(--text-primary, #202124);
      }

      .final-score {
        margin-top: 0.5rem !important;
        font-weight: 600;
        color: var(--success-text, #137333) !important;
      }

      /* Footer */
      .gate-footer {
        padding: 1rem 1.5rem;
        background: var(--surface-secondary, #f8f9fa);
        border-top: 1px solid var(--border-color, #e9ecef);
      }

      .gate-tip {
        margin: 0;
        font-size: 0.8125rem;
        color: var(--text-secondary, #5f6368);
        text-align: center;
      }
    `,
  ],
})
export class MasteryGateComponent implements OnInit, OnDestroy {
  /** Path ID */
  @Input({ required: true }) pathId!: string;

  /** Section ID this gate protects */
  @Input({ required: true }) sectionId!: string;

  /** Section display title */
  @Input({ required: true }) sectionTitle!: string;

  /** Human ID */
  @Input({ required: true }) humanId!: string;

  /** Content IDs in the section (for quiz) */
  @Input() contentIds: string[] = [];

  /** Emitted when user wants to start mastery quiz */
  @Output() startQuiz = new EventEmitter<MasteryQuizRequestEvent>();

  /** Emitted when gate is passed and user continues */
  @Output() bypassed = new EventEmitter<void>();

  // Signals for reactive state
  protected gateStatus = signal<GateStatus | null>(null);
  protected cooldownStatus = signal<CooldownStatus | null>(null);

  // Computed values
  protected isLocked = computed(() => this.gateStatus()?.locked ?? true);
  protected bestScore = computed(() => this.gateStatus()?.bestScore ?? 0);
  protected attemptsRemaining = computed(() => this.gateStatus()?.remainingAttempts ?? 0);
  protected attemptsUsed = computed(() => {
    const remaining = this.attemptsRemaining();
    return this.maxAttempts() - remaining;
  });
  protected maxAttempts = computed(() => 2); // From config
  protected inCooldown = computed(() => this.cooldownStatus()?.inCooldown ?? false);
  protected cooldownRemaining = computed(() => this.cooldownStatus()?.remainingFormatted ?? '');
  protected canAttempt = computed(() => {
    const gate = this.gateStatus();
    return gate?.quizAvailable ?? false;
  });

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly adaptationService: PathAdaptationService,
    private readonly cooldownService: AttemptCooldownService,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.loadGateStatus();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Format score as percentage.
   */
  formatPercent(score: number): string {
    return `${Math.round(score * 100)}%`;
  }

  /**
   * Start mastery quiz.
   */
  onStartQuiz(): void {
    if (!this.canAttempt()) return;

    this.startQuiz.emit({
      sectionId: this.sectionId,
      sectionTitle: this.sectionTitle,
      contentIds: this.contentIds,
    });
  }

  /**
   * Continue past unlocked gate.
   */
  onContinue(): void {
    this.bypassed.emit();
  }

  /**
   * Load initial gate status.
   */
  private loadGateStatus(): void {
    // Subscribe to gate status
    this.adaptationService
      .getGateStatus$(this.pathId, this.sectionId, this.humanId)
      .pipe(takeUntil(this.destroy$))
      .subscribe(status => {
        this.gateStatus.set(status);
        this.cdr.markForCheck();
      });

    // Subscribe to cooldown status for timer
    this.cooldownService
      .getCooldownStatus$(this.sectionId, this.humanId)
      .pipe(takeUntil(this.destroy$))
      .subscribe(status => {
        this.cooldownStatus.set(status);
        this.cdr.markForCheck();
      });
  }
}
