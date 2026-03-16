import { CommonModule } from '@angular/common';
import { Component, EventEmitter, OnDestroy, OnInit, Output, inject, input } from '@angular/core';
import { FormsModule } from '@angular/forms';

import { Subject, takeUntil } from 'rxjs';

import {
  GovernanceSignalService,
  GraduatedFeedbackInput,
  FeedbackStats,
} from '@app/elohim/services/governance-signal.service';
import { GovernanceApiService } from '@app/elohim/services/governance-api.service';
import { GovernanceRecognitionService } from '@app/qahal/services/governance-recognition.service';
import type {
  RecordSignalInputView,
  GovernanceSignalView,
} from '@elohim/storage-client/generated';

/**
 * GraduatedFeedbackComponent - Context-Aware Scaled Responses
 *
 * Inspired by Loomio's Gradients of Agreement and Forby's ARCH voting:
 * - Positions are context-specific (accuracy, usefulness, proposals)
 * - Intensity captures how strongly respondent feels (ARCH pattern)
 * - Optional reasoning for deliberative engagement
 * - Records signals to governance API backend (mechanism level 2)
 *
 * Friction Level: Medium
 * - More thoughtful than reactions
 * - Less formal than proposal votes
 * - Aggregates to content quality signals
 */
@Component({
  selector: 'app-graduated-feedback',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: `
    <div class="graduated-feedback" [class.compact]="compact()">
      <h4 class="feedback-label">{{ currentScale.label }}</h4>

      <div class="positions" role="radiogroup" [attr.aria-label]="currentScale.label">
        @for (position of currentScale.positions; track position.index) {
          <button
            class="position-btn"
            [class.selected]="selectedPosition === position.index"
            [style.border-color]="position.color"
            [style.background-color]="
              selectedPosition === position.index ? position.color : 'transparent'
            "
            [style.color]="selectedPosition === position.index ? '#fff' : position.color"
            role="radio"
            [attr.aria-checked]="selectedPosition === position.index"
            [attr.aria-label]="position.label"
            [attr.data-testid]="'position-' + position.label.toLowerCase().replace(' ', '-')"
            (click)="selectPosition(position)"
          >
            {{ position.label }}
          </button>
        }
      </div>

      @if (selectedPosition !== null) {
        <div class="intensity-section">
          <label class="intensity-label" for="intensity-slider">
            Intensity: {{ getIntensityLabel() }} ({{ intensity }}/10)
          </label>
          <input
            id="intensity-slider"
            type="range"
            min="1"
            max="10"
            [(ngModel)]="intensity"
            class="intensity-slider"
            data-testid="intensity-slider"
          />
        </div>
      }

      @if (showReasoningField || reasoningRequired) {
        <div class="reasoning-section">
          <label class="reasoning-label" for="reasoning-text">
            {{ reasoningRequired ? 'Reasoning (required)' : 'Reasoning (optional)' }}
          </label>
          <textarea
            id="reasoning-text"
            [(ngModel)]="reasoning"
            class="reasoning-textarea"
            placeholder="Share your thinking..."
            rows="3"
            data-testid="reasoning-textarea"
          ></textarea>
        </div>
      } @else if (selectedPosition !== null) {
        <button
          class="toggle-reasoning-btn"
          (click)="toggleReasoning()"
          data-testid="toggle-reasoning-btn"
        >
          Add reasoning
        </button>
      }

      @if (selectedPosition !== null) {
        <div class="submit-section">
          <button
            class="submit-btn"
            [disabled]="!canSubmit || isSubmitting"
            (click)="submit()"
            data-testid="submit-feedback-btn"
          >
            {{ isSubmitting ? 'Submitting...' : hasSubmitted ? 'Update Feedback' : 'Submit' }}
          </button>
          @if (hasSubmitted) {
            <button
              class="reset-btn"
              (click)="reset()"
              data-testid="reset-feedback-btn"
            >
              Reset
            </button>
          }
        </div>
      }

      @if (showAggregates() && stats && stats.totalResponses > 0) {
        <div class="aggregate-section">
          <p class="aggregate-header">
            Community response ({{ stats.totalResponses }}
            {{ stats.totalResponses === 1 ? 'response' : 'responses' }})
          </p>
          <div class="distribution-bar">
            @for (position of currentScale.positions; track position.index) {
              @if (getDistributionWidth(position) > 0) {
                <div
                  class="distribution-segment"
                  [style.width.%]="getDistributionWidth(position)"
                  [style.background-color]="position.color"
                  [title]="
                    position.label +
                    ': ' +
                    formatPercentage(getDistributionWidth(position) / 100)
                  "
                ></div>
              }
            }
          </div>
          @if (apiDistribution && hasApiData) {
            <p class="api-aggregate-note">Includes governance network data</p>
          }
        </div>
      }
    </div>
  `,
  styles: [
    `
      .graduated-feedback {
        padding: 1rem;
      }
      .graduated-feedback.compact {
        padding: 0.5rem;
      }
      .feedback-label {
        margin: 0 0 0.75rem;
        font-size: 1rem;
        font-weight: 500;
      }
      .positions {
        display: flex;
        gap: 0.5rem;
        flex-wrap: wrap;
      }
      .position-btn {
        padding: 0.5rem 0.75rem;
        border: 2px solid;
        border-radius: 0.375rem;
        background: transparent;
        cursor: pointer;
        font-size: 0.875rem;
        font-weight: 500;
        transition: all 0.2s ease;
      }
      .position-btn:hover {
        opacity: 0.85;
      }
      .position-btn.selected {
        font-weight: 600;
      }
      .intensity-section {
        margin-top: 1rem;
      }
      .intensity-label {
        display: block;
        font-size: 0.875rem;
        margin-bottom: 0.25rem;
        color: var(--text-secondary, #666);
      }
      .intensity-slider {
        width: 100%;
      }
      .reasoning-section {
        margin-top: 1rem;
      }
      .reasoning-label {
        display: block;
        font-size: 0.875rem;
        margin-bottom: 0.25rem;
        color: var(--text-secondary, #666);
      }
      .reasoning-textarea {
        width: 100%;
        padding: 0.5rem;
        border: 1px solid var(--border-color, #ddd);
        border-radius: 0.25rem;
        font-family: inherit;
        font-size: 0.875rem;
        resize: vertical;
      }
      .toggle-reasoning-btn {
        margin-top: 0.5rem;
        padding: 0.25rem 0.5rem;
        border: none;
        background: transparent;
        color: var(--link-color, #2196f3);
        cursor: pointer;
        font-size: 0.813rem;
      }
      .submit-section {
        margin-top: 1rem;
        display: flex;
        gap: 0.5rem;
      }
      .submit-btn {
        padding: 0.5rem 1rem;
        border: none;
        border-radius: 0.25rem;
        background: var(--primary-color, #2196f3);
        color: #fff;
        cursor: pointer;
        font-size: 0.875rem;
      }
      .submit-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }
      .reset-btn {
        padding: 0.5rem 1rem;
        border: 1px solid var(--border-color, #ddd);
        border-radius: 0.25rem;
        background: transparent;
        cursor: pointer;
        font-size: 0.875rem;
      }
      .aggregate-section {
        margin-top: 1rem;
        padding-top: 0.75rem;
        border-top: 1px solid var(--border-color, #eee);
      }
      .aggregate-header {
        font-size: 0.813rem;
        color: var(--text-secondary, #666);
        margin: 0 0 0.5rem;
      }
      .distribution-bar {
        display: flex;
        height: 0.5rem;
        border-radius: 0.25rem;
        overflow: hidden;
      }
      .distribution-segment {
        transition: width 0.3s ease;
      }
      .api-aggregate-note {
        font-size: 0.75rem;
        color: var(--text-tertiary, #999);
        margin: 0.25rem 0 0;
        font-style: italic;
      }
    `,
  ],
})
export class GraduatedFeedbackComponent implements OnInit, OnDestroy {
  /** Entity type for governance signals (e.g. 'content', 'proposal') */
  readonly entityType = input<string>('content');

  /** Entity ID for governance signals */
  readonly entityId = input<string>('');

  /**
   * Legacy content ID input — used as fallback for entityId when entityId is not provided.
   * Prefer entityId + entityType for new usage.
   */
  readonly contentId = input<string>('');

  /** Feedback context determining the scale used */
  readonly context = input<FeedbackContext>('usefulness');

  /** Whether reasoning is always required */
  readonly requiresReasoningInput = input<boolean>(false, { alias: 'requiresReasoning' });

  /** Resolved entity ID: prefers entityId input, falls back to contentId */
  private get resolvedEntityId(): string {
    return this.entityId() || this.contentId();
  }

  /** Show aggregate distribution */
  readonly showAggregates = input<boolean>(true);

  /** Compact mode */
  readonly compact = input<boolean>(false);

  @Output() feedbackSubmitted = new EventEmitter<GraduatedFeedbackInput>();

  // Current selection state
  selectedPosition: number | null = null;
  intensity = 5; // 1-10 scale (ARCH pattern)
  reasoning = '';

  // UI state
  isSubmitting = false;
  hasSubmitted = false;
  showReasoningField = false;

  // Aggregate stats (local)
  stats: FeedbackStats | null = null;

  // API-sourced distribution data
  apiDistribution: Record<string, number> = {};
  hasApiData = false;

  // Context-specific scales
  readonly scales: Record<FeedbackContext, ScaleDefinition> = {
    accuracy: {
      label: 'How accurate is this content?',
      positions: [
        { index: 0, label: 'False', color: '#e74c3c' },
        { index: 0.25, label: 'Inaccurate', color: '#e67e22' },
        { index: 0.5, label: 'Uncertain', color: '#f1c40f' },
        { index: 0.75, label: 'Mostly Accurate', color: '#2ecc71' },
        { index: 1, label: 'Accurate', color: '#27ae60' },
      ],
    },
    usefulness: {
      label: 'How useful did you find this content?',
      positions: [
        { index: 0, label: 'Not Useful', color: '#95a5a6' },
        { index: 0.25, label: 'Slightly Useful', color: '#bdc3c7' },
        { index: 0.5, label: 'Useful', color: '#3498db' },
        { index: 0.75, label: 'Very Useful', color: '#2980b9' },
        { index: 1, label: 'Transformative', color: '#6c5ce7' },
      ],
    },
    proposal: {
      label: 'What is your position on this proposal?',
      positions: [
        { index: 0, label: 'Block', color: '#e74c3c', requiresReasoning: true },
        { index: 0.25, label: 'Disagree', color: '#e67e22' },
        { index: 0.5, label: 'Abstain', color: '#95a5a6' },
        { index: 0.75, label: 'Agree', color: '#2ecc71' },
        { index: 1, label: 'Strongly Agree', color: '#27ae60' },
      ],
    },
    clarity: {
      label: 'How clear was this content?',
      positions: [
        { index: 0, label: 'Confusing', color: '#e74c3c' },
        { index: 0.25, label: 'Unclear', color: '#e67e22' },
        { index: 0.5, label: 'Adequate', color: '#f1c40f' },
        { index: 0.75, label: 'Clear', color: '#2ecc71' },
        { index: 1, label: 'Crystal Clear', color: '#27ae60' },
      ],
    },
    relevance: {
      label: 'How relevant is this to your learning goals?',
      positions: [
        { index: 0, label: 'Irrelevant', color: '#95a5a6' },
        { index: 0.25, label: 'Tangential', color: '#bdc3c7' },
        { index: 0.5, label: 'Related', color: '#3498db' },
        { index: 0.75, label: 'Highly Relevant', color: '#2980b9' },
        { index: 1, label: 'Essential', color: '#6c5ce7' },
      ],
    },
  };

  private readonly destroy$ = new Subject<void>();

  private readonly signalService = inject(GovernanceSignalService);
  private readonly governanceApi = inject(GovernanceApiService);
  private readonly governanceRecognition = inject(GovernanceRecognitionService);

  private static readonly CURRENT_USER_ID = 'current-user';

  ngOnInit(): void {
    if (this.showAggregates()) {
      this.loadStats();
    }
    this.loadApiSignals();
    this.subscribeToChanges();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Get the current scale definition.
   */
  get currentScale(): ScaleDefinition {
    return this.scales[this.context()];
  }

  /**
   * Get the selected position object.
   */
  get selectedPositionData(): ScalePosition | null {
    if (this.selectedPosition === null) return null;
    return this.currentScale.positions.find(p => p.index === this.selectedPosition) ?? null;
  }

  /**
   * Check if reasoning is required for current selection.
   * Reasoning is required if: explicitly set via input, position demands it,
   * or the selected position is in the bottom 2 options (negative feedback).
   */
  get reasoningRequired(): boolean {
    if (this.requiresReasoningInput()) return true;
    if (this.selectedPositionData?.requiresReasoning) return true;

    // For negative feedback (bottom 2 options on any scale), require reasoning
    if (this.selectedPosition !== null && this.selectedPosition <= 0.25) {
      return true;
    }

    return false;
  }

  /**
   * Check if form is valid for submission.
   */
  get canSubmit(): boolean {
    if (this.selectedPosition === null) return false;
    if (this.reasoningRequired && !this.reasoning.trim()) return false;
    return true;
  }

  /**
   * Handle position selection.
   */
  selectPosition(position: ScalePosition): void {
    this.selectedPosition = position.index;

    // Show reasoning field if required or if it's a critical/negative position
    if (position.requiresReasoning || this.requiresReasoningInput() || position.index <= 0.25) {
      this.showReasoningField = true;
    }
  }

  /**
   * Toggle reasoning field visibility.
   */
  toggleReasoning(): void {
    this.showReasoningField = !this.showReasoningField;
  }

  /**
   * Submit the feedback to both local signal service and governance API.
   */
  submit(): void {
    if (!this.canSubmit || this.isSubmitting) return;

    this.isSubmitting = true;
    const positionData = this.selectedPositionData!;

    const feedback: GraduatedFeedbackInput = {
      context: this.context(),
      position: positionData.label,
      positionIndex: positionData.index,
      intensity: this.intensity,
      reasoning: this.reasoning.trim() || undefined,
    };

    // Record locally via existing signal service
    this.signalService
      .recordGraduatedFeedback(this.resolvedEntityId, feedback)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: success => {
          this.isSubmitting = false;
          if (success) {
            this.hasSubmitted = true;
            this.feedbackSubmitted.emit(feedback);
            this.loadStats();
          }
        },
        error: () => {
          this.isSubmitting = false;
        },
      });

    // Record to governance API backend
    const signalValue = `${this.context()}:${positionData.label.toLowerCase().replace(/\s+/g, '-')}`;
    const signal: RecordSignalInputView = {
      entityType: this.entityType(),
      entityId: this.resolvedEntityId,
      humanId: GraduatedFeedbackComponent.CURRENT_USER_ID,
      signalType: 'graduated',
      signalValue,
      mechanismLevel: 2,
      proxyElohimId: null,
    };

    this.governanceApi.recordSignal(signal).catch(() => {
      // API failure is non-blocking; local signal is already recorded
    });

    // Generate REA economic event for governance participation
    // Block positions (index 0 on proposal scale) use 'block' participation type
    const isBlock = this.context() === 'proposal' && positionData.index === 0;
    this.governanceRecognition
      .recordParticipation({
        entityType: this.entityType(),
        entityId: this.resolvedEntityId,
        humanId: GraduatedFeedbackComponent.CURRENT_USER_ID,
        mechanismLevel: 2,
        participationType: isBlock ? 'block' : 'graduated-feedback',
      })
      .catch(() => {
        // Recognition failure is non-blocking
      });
  }

  /**
   * Reset form to initial state.
   */
  reset(): void {
    this.selectedPosition = null;
    this.intensity = 5;
    this.reasoning = '';
    this.showReasoningField = false;
    this.hasSubmitted = false;
  }

  /**
   * Get position width percentage for distribution bar.
   */
  getDistributionWidth(position: ScalePosition): number {
    if (!this.stats || this.stats.totalResponses === 0) return 0;
    const count = this.stats.distribution[position.label] ?? 0;
    return (count / this.stats.totalResponses) * 100;
  }

  /**
   * Get intensity label.
   */
  getIntensityLabel(): string {
    if (this.intensity <= 2) return 'Slightly';
    if (this.intensity <= 4) return 'Moderately';
    if (this.intensity <= 6) return 'Fairly';
    if (this.intensity <= 8) return 'Strongly';
    return 'Very Strongly';
  }

  /**
   * Format percentage for display.
   */
  formatPercentage(value: number): string {
    return Math.round(value * 100) + '%';
  }

  private loadStats(): void {
    this.signalService
      .getFeedbackStats(this.resolvedEntityId)
      .pipe(takeUntil(this.destroy$))
      .subscribe(stats => {
        this.stats = stats;
      });
  }

  /**
   * Load existing signals from the governance API to show aggregate distribution
   * and determine if the current user has already submitted feedback.
   */
  private loadApiSignals(): void {
    this.governanceApi
      .getSignals(this.entityType(), this.resolvedEntityId)
      .then((signals: GovernanceSignalView[]) => {
        const graduatedSignals = signals.filter(s => s.signalType === 'graduated');

        if (graduatedSignals.length === 0) return;

        // Build distribution from API signals
        const distribution: Record<string, number> = {};
        for (const sig of graduatedSignals) {
          distribution[sig.signalValue] = (distribution[sig.signalValue] ?? 0) + 1;
        }
        this.apiDistribution = distribution;
        this.hasApiData = true;

        // Check if current user already submitted
        const userSignal = graduatedSignals.find(
          s => s.humanId === GraduatedFeedbackComponent.CURRENT_USER_ID
        );
        if (userSignal) {
          this.hasSubmitted = true;
          // Parse signal value to restore selection: "context:position-label"
          const parts = userSignal.signalValue.split(':');
          if (parts.length === 2) {
            const posLabel = parts[1].replace(/-/g, ' ');
            const matchingPosition = this.currentScale.positions.find(
              p => p.label.toLowerCase() === posLabel
            );
            if (matchingPosition) {
              this.selectedPosition = matchingPosition.index;
            }
          }
        }
      })
      .catch(() => {
        // API unavailable — rely on local signals only
      });
  }

  private subscribeToChanges(): void {
    this.signalService.signalChanges$.pipe(takeUntil(this.destroy$)).subscribe(change => {
      if (change?.type === 'graduated-feedback' && change.contentId === this.resolvedEntityId) {
        this.loadStats();
      }
    });
  }
}

// ===========================================================================
// Types
// ===========================================================================

export type FeedbackContext = 'accuracy' | 'usefulness' | 'proposal' | 'clarity' | 'relevance';

interface ScaleDefinition {
  label: string;
  positions: ScalePosition[];
}

interface ScalePosition {
  index: number; // 0-1 normalized
  label: string;
  color: string;
  requiresReasoning?: boolean;
}
