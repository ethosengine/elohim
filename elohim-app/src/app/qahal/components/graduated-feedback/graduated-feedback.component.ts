import { CommonModule } from '@angular/common';
import { Component, Input, Output, EventEmitter, OnInit, OnDestroy } from '@angular/core';
import { FormsModule } from '@angular/forms';

// @coverage: 100.0% (2026-02-04)

import { Subject, takeUntil } from 'rxjs';

import {
  GovernanceSignalService,
  GraduatedFeedbackInput,
  FeedbackStats,
} from '@app/elohim/services/governance-signal.service';

/**
 * GraduatedFeedbackComponent - Context-Aware Scaled Responses
 *
 * Inspired by Loomio's Gradients of Agreement and Forby's ARCH voting:
 * - Positions are context-specific (accuracy, usefulness, proposals)
 * - Intensity captures how strongly respondent feels (ARCH pattern)
 * - Optional reasoning for deliberative engagement
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
  templateUrl: './graduated-feedback.component.html',
  styleUrls: ['./graduated-feedback.component.css'],
})
export class GraduatedFeedbackComponent implements OnInit, OnDestroy {
  @Input() contentId!: string;
  @Input() context: FeedbackContext = 'usefulness';
  @Input() requiresReasoning = false;
  @Input() showAggregates = true;
  @Input() compact = false;

  @Output() feedbackSubmitted = new EventEmitter<GraduatedFeedbackInput>();

  // Current selection state
  selectedPosition: number | null = null;
  intensity = 5; // 1-10 scale (ARCH pattern)
  reasoning = '';

  // UI state
  isSubmitting = false;
  hasSubmitted = false;
  showReasoningField = false;

  // Aggregate stats
  stats: FeedbackStats | null = null;

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

  constructor(private readonly signalService: GovernanceSignalService) {}

  ngOnInit(): void {
    if (this.showAggregates) {
      this.loadStats();
    }
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
    return this.scales[this.context];
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
   */
  get reasoningRequired(): boolean {
    if (this.requiresReasoning) return true;
    return this.selectedPositionData?.requiresReasoning ?? false;
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

    // Show reasoning field if required or if it's a critical position
    if (position.requiresReasoning || this.requiresReasoning) {
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
   * Submit the feedback.
   */
  submit(): void {
    if (!this.canSubmit || this.isSubmitting) return;

    this.isSubmitting = true;
    const positionData = this.selectedPositionData!;

    const feedback: GraduatedFeedbackInput = {
      context: this.context,
      position: positionData.label,
      positionIndex: positionData.index,
      intensity: this.intensity,
      reasoning: this.reasoning.trim() || undefined,
    };

    this.signalService
      .recordGraduatedFeedback(this.contentId, feedback)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: success => {
          this.isSubmitting = false;
          if (success) {
            this.hasSubmitted = true;
            this.feedbackSubmitted.emit(feedback);
            // Refresh stats
            this.loadStats();
          }
        },
        error: () => {
          this.isSubmitting = false;
        },
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
      .getFeedbackStats(this.contentId)
      .pipe(takeUntil(this.destroy$))
      .subscribe(stats => {
        this.stats = stats;
      });
  }

  private subscribeToChanges(): void {
    this.signalService.signalChanges$.pipe(takeUntil(this.destroy$)).subscribe(change => {
      if (change?.type === 'graduated-feedback' && change.contentId === this.contentId) {
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
