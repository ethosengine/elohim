import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  Output,
  EventEmitter,
  OnInit,
  ChangeDetectionStrategy,
  signal,
  computed,
} from '@angular/core';

import { SkipAheadResult } from '../../services/path-adaptation.service';

/**
 * Pre-assessment decision event.
 */
export interface PreAssessmentDecision {
  /** Whether user chose to take pre-assessment */
  takeAssessment: boolean;

  /** If skipping, start from beginning */
  startFromBeginning?: boolean;
}

/**
 * Skip selection event after pre-assessment.
 */
export interface SkipSelectionEvent {
  /** Section IDs to skip */
  sectionsToSkip: string[];

  /** Section ID to start from */
  startSection: string;
}

/**
 * PreAssessmentComponent - Skip-ahead offer at path start.
 *
 * Shown when a learner starts a new path, offering the option to:
 * 1. Start from the beginning (default path)
 * 2. Take a pre-assessment to potentially skip ahead
 *
 * After pre-assessment, shows which sections can be skipped
 * based on demonstrated knowledge.
 *
 * @example
 * ```html
 * <!-- Offer phase -->
 * <app-pre-assessment
 *   [pathId]="path.id"
 *   [pathTitle]="path.title"
 *   [estimatedTime]="path.estimatedMinutes"
 *   (decision)="onPreAssessmentDecision($event)">
 * </app-pre-assessment>
 *
 * <!-- Results phase -->
 * <app-pre-assessment
 *   [pathId]="path.id"
 *   [pathTitle]="path.title"
 *   [skipAheadResult]="skipResult"
 *   (skipSelection)="onSkipSelection($event)">
 * </app-pre-assessment>
 * ```
 */
@Component({
  selector: 'app-pre-assessment',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="pre-assessment" [class.results-phase]="showResults()">
      @if (!showResults()) {
        <!-- Offer Phase -->
        <div class="offer-phase">
          <header class="offer-header">
            <div class="offer-icon">🎯</div>
            <h2 class="offer-title">Ready to Begin?</h2>
            <p class="offer-subtitle">{{ pathTitle }}</p>
          </header>

          <div class="offer-body">
            <p class="offer-message">
              Already familiar with some of this material? Take a quick pre-assessment to skip
              ahead.
            </p>

            <div class="time-estimate">
              <span class="time-icon">⏱️</span>
              <div class="time-text">
                <span class="time-label">Pre-assessment takes</span>
                <span class="time-value">~{{ assessmentMinutes }} minutes</span>
              </div>
            </div>
          </div>

          <div class="offer-actions">
            <button class="btn-assess" (click)="onTakeAssessment()">Take Pre-Assessment</button>
            <button class="btn-skip" (click)="onStartFromBeginning()">Start from Beginning</button>
          </div>

          <p class="offer-hint">
            The pre-assessment is optional. You can always review skipped sections later.
          </p>
        </div>
      } @else {
        <!-- Results Phase -->
        <div class="results-phase-content">
          <header class="results-header">
            <div class="results-icon">✨</div>
            <h2 class="results-title">Pre-Assessment Complete</h2>
            <p class="results-score">
              You demonstrated {{ formatPercent(skipAheadResult!.preAssessmentScore) }} mastery
            </p>
          </header>

          <div class="results-body">
            @if (skippableSections().length > 0) {
              <p class="results-message">Based on your results, you can skip these sections:</p>

              <div class="section-list">
                @for (section of skipAheadResult!.skippableSections; track section.sectionId) {
                  <label
                    class="section-item"
                    [class.skippable]="section.recommendSkip"
                    [class.selected]="isSectionSelected(section.sectionId)"
                  >
                    <input
                      type="checkbox"
                      [checked]="isSectionSelected(section.sectionId)"
                      [disabled]="!section.recommendSkip"
                      (change)="toggleSection(section.sectionId)"
                    />
                    <div class="section-info">
                      <span class="section-title">{{ section.title }}</span>
                      <span class="section-score">
                        {{ formatPercent(section.masteryScore) }} mastery
                      </span>
                    </div>
                    @if (section.recommendSkip) {
                      <span class="skip-badge">Can Skip</span>
                    }
                  </label>
                }
              </div>
            } @else {
              <p class="results-message no-skip">
                The pre-assessment shows you'd benefit from the full learning path. Let's start from
                the beginning!
              </p>
            }
          </div>

          <div class="results-actions">
            @if (skippableSections().length > 0) {
              <button
                class="btn-apply"
                [disabled]="selectedSections().size === 0"
                (click)="onApplySkips()"
              >
                Skip Selected ({{ selectedSections().size }})
              </button>
              <button class="btn-no-skip" (click)="onNoSkip()">Don't Skip Any</button>
            } @else {
              <button class="btn-start" (click)="onNoSkip()">Start Learning</button>
            }
          </div>

          @if (skipAheadResult!.recommendedStartSection) {
            <p class="recommendation-hint">
              💡 Recommended: Start from "{{ getRecommendedSectionTitle() }}"
            </p>
          }
        </div>
      }
    </div>
  `,
  styles: [
    `
      .pre-assessment {
        max-width: 520px;
        margin: 2rem auto;
        background: var(--surface-elevated, #fff);
        border-radius: var(--radius-lg, 12px);
        border: 1px solid var(--border-color, #e9ecef);
        overflow: hidden;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
      }

      /* Offer Phase */
      .offer-phase {
        padding: 2rem;
        text-align: center;
      }

      .offer-header {
        margin-bottom: 1.5rem;
      }

      .offer-icon {
        font-size: 3rem;
        margin-bottom: 0.75rem;
      }

      .offer-title {
        margin: 0;
        font-size: 1.5rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
      }

      .offer-subtitle {
        margin: 0.5rem 0 0;
        font-size: 1rem;
        color: var(--text-secondary, #5f6368);
      }

      .offer-body {
        margin-bottom: 1.5rem;
      }

      .offer-message {
        margin: 0 0 1.25rem;
        font-size: 1rem;
        color: var(--text-primary, #202124);
        line-height: 1.5;
      }

      .time-estimate {
        display: inline-flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.75rem 1.25rem;
        background: var(--surface-secondary, #f8f9fa);
        border-radius: var(--radius-md, 8px);
      }

      .time-icon {
        font-size: 1.25rem;
      }

      .time-text {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 0.125rem;
      }

      .time-label {
        font-size: 0.75rem;
        color: var(--text-tertiary, #80868b);
      }

      .time-value {
        font-size: 1rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
      }

      .offer-actions {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        margin-bottom: 1rem;
      }

      .btn-assess,
      .btn-skip,
      .btn-apply,
      .btn-no-skip,
      .btn-start {
        width: 100%;
        padding: 0.875rem 1.5rem;
        font-size: 1rem;
        font-weight: 600;
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .btn-assess {
        background: var(--primary, #4285f4);
        color: white;
        border: none;
      }

      .btn-assess:hover {
        background: var(--primary-dark, #1a73e8);
      }

      .btn-skip {
        background: var(--surface-secondary, #f8f9fa);
        color: var(--text-primary, #202124);
        border: 1px solid var(--border-color, #e9ecef);
      }

      .btn-skip:hover {
        background: var(--surface-tertiary, #e8eaed);
      }

      .offer-hint {
        margin: 0;
        font-size: 0.8125rem;
        color: var(--text-tertiary, #80868b);
      }

      /* Results Phase */
      .results-phase-content {
        padding: 2rem;
      }

      .results-header {
        text-align: center;
        margin-bottom: 1.5rem;
      }

      .results-icon {
        font-size: 2.5rem;
        margin-bottom: 0.5rem;
      }

      .results-title {
        margin: 0;
        font-size: 1.25rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
      }

      .results-score {
        margin: 0.5rem 0 0;
        font-size: 1rem;
        color: var(--success-text, #137333);
        font-weight: 500;
      }

      .results-body {
        margin-bottom: 1.5rem;
      }

      .results-message {
        margin: 0 0 1rem;
        font-size: 1rem;
        color: var(--text-primary, #202124);
        text-align: center;
      }

      .results-message.no-skip {
        color: var(--text-secondary, #5f6368);
      }

      .section-list {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      .section-item {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.75rem 1rem;
        background: var(--surface-secondary, #f8f9fa);
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .section-item:not(.skippable) {
        opacity: 0.6;
        cursor: not-allowed;
      }

      .section-item.skippable:hover {
        background: var(--primary-surface, #e8f0fe);
      }

      .section-item.selected {
        background: var(--primary-surface, #e8f0fe);
        border: 1px solid var(--primary, #4285f4);
      }

      .section-item input[type='checkbox'] {
        width: 1.125rem;
        height: 1.125rem;
        accent-color: var(--primary, #4285f4);
      }

      .section-info {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 0.125rem;
      }

      .section-title {
        font-size: 0.9375rem;
        font-weight: 500;
        color: var(--text-primary, #202124);
      }

      .section-score {
        font-size: 0.75rem;
        color: var(--text-secondary, #5f6368);
      }

      .skip-badge {
        padding: 0.25rem 0.5rem;
        font-size: 0.6875rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.03em;
        background: var(--success, #34a853);
        color: white;
        border-radius: var(--radius-sm, 4px);
      }

      .results-actions {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
      }

      .btn-apply {
        background: var(--success, #34a853);
        color: white;
        border: none;
      }

      .btn-apply:hover:not(:disabled) {
        background: var(--success-dark, #1e8e3e);
      }

      .btn-apply:disabled {
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-disabled, #9aa0a6);
        cursor: not-allowed;
      }

      .btn-no-skip {
        background: var(--surface-secondary, #f8f9fa);
        color: var(--text-primary, #202124);
        border: 1px solid var(--border-color, #e9ecef);
      }

      .btn-no-skip:hover {
        background: var(--surface-tertiary, #e8eaed);
      }

      .btn-start {
        background: var(--primary, #4285f4);
        color: white;
        border: none;
      }

      .btn-start:hover {
        background: var(--primary-dark, #1a73e8);
      }

      .recommendation-hint {
        margin: 1rem 0 0;
        font-size: 0.8125rem;
        color: var(--text-secondary, #5f6368);
        text-align: center;
      }
    `,
  ],
})
export class PreAssessmentComponent implements OnInit {
  /** Path ID */
  @Input({ required: true }) pathId!: string;

  /** Path title for display */
  @Input({ required: true }) pathTitle!: string;

  /** Estimated assessment time in minutes */
  @Input() assessmentMinutes = 10;

  /** Skip-ahead result (shows results phase if provided) */
  @Input() skipAheadResult?: SkipAheadResult;

  /** Emitted when user makes initial decision */
  @Output() decision = new EventEmitter<PreAssessmentDecision>();

  /** Emitted when user selects sections to skip */
  @Output() skipSelection = new EventEmitter<SkipSelectionEvent>();

  // Signals
  protected selectedSections = signal<Set<string>>(new Set());

  // Computed
  protected showResults = computed(() => !!this.skipAheadResult);
  protected skippableSections = computed(
    () => this.skipAheadResult?.skippableSections.filter(s => s.recommendSkip) ?? []
  );

  ngOnInit(): void {
    // Pre-select recommended skips
    if (this.skipAheadResult) {
      const preSelected = new Set(
        this.skipAheadResult.skippableSections.filter(s => s.recommendSkip).map(s => s.sectionId)
      );
      this.selectedSections.set(preSelected);
    }
  }

  /**
   * Format score as percentage.
   */
  formatPercent(score: number): string {
    return `${Math.round(score * 100)}%`;
  }

  /**
   * Check if section is selected for skipping.
   */
  isSectionSelected(sectionId: string): boolean {
    return this.selectedSections().has(sectionId);
  }

  /**
   * Toggle section selection.
   */
  toggleSection(sectionId: string): void {
    const current = this.selectedSections();
    const updated = new Set(current);

    if (updated.has(sectionId)) {
      updated.delete(sectionId);
    } else {
      updated.add(sectionId);
    }

    this.selectedSections.set(updated);
  }

  /**
   * Get recommended section title.
   */
  getRecommendedSectionTitle(): string {
    if (!this.skipAheadResult?.recommendedStartSection) return '';

    const section = this.skipAheadResult.skippableSections.find(
      s => s.sectionId === this.skipAheadResult!.recommendedStartSection
    );

    return section?.title ?? '';
  }

  /**
   * User chose to take pre-assessment.
   */
  onTakeAssessment(): void {
    this.decision.emit({ takeAssessment: true });
  }

  /**
   * User chose to start from beginning.
   */
  onStartFromBeginning(): void {
    this.decision.emit({
      takeAssessment: false,
      startFromBeginning: true,
    });
  }

  /**
   * Apply selected skips.
   */
  onApplySkips(): void {
    const sectionsToSkip = Array.from(this.selectedSections());

    // Find first non-skipped section
    let startSection = '';
    if (this.skipAheadResult) {
      for (const section of this.skipAheadResult.skippableSections) {
        if (!sectionsToSkip.includes(section.sectionId)) {
          startSection = section.sectionId;
          break;
        }
      }
    }

    this.skipSelection.emit({
      sectionsToSkip,
      startSection,
    });
  }

  /**
   * Don't skip any sections.
   */
  onNoSkip(): void {
    const firstSection = this.skipAheadResult?.skippableSections[0]?.sectionId ?? '';

    this.skipSelection.emit({
      sectionsToSkip: [],
      startSection: firstSection,
    });
  }
}
