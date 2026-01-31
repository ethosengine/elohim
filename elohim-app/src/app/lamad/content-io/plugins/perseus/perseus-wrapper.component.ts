import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  Output,
  EventEmitter,
  AfterViewInit,
  OnDestroy,
  OnChanges,
  SimpleChanges,
  ElementRef,
  ViewChild,
  CUSTOM_ELEMENTS_SCHEMA,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  inject,
} from '@angular/core';

// @coverage: 62.1% (2026-02-04)

import {
  registerPerseusElement,
  type PerseusQuestionElement,
  getPerseusElement,
} from './perseus-element-loader';

import type { PerseusItem, PerseusScoreResult } from './perseus-item.model';

// Use the lazy loader instead of direct import to avoid bundling React at compile time

/**
 * PerseusWrapperComponent - Angular component wrapping Perseus custom element.
 *
 * This component provides an Angular-friendly interface to the Perseus
 * question renderer, handling property binding and event emission.
 *
 * @example
 * ```html
 * <app-perseus-question
 *   [item]="questionItem"
 *   [reviewMode]="false"
 *   (scored)="handleScore($event)"
 *   (answerChanged)="handleAnswerChange($event)">
 * </app-perseus-question>
 * ```
 *
 * @example
 * ```typescript
 * @ViewChild(PerseusWrapperComponent) questionRef!: PerseusWrapperComponent;
 *
 * checkAnswer(): void {
 *   const result = this.questionRef.score();
 *   if (result?.correct) {
 *     console.log('Correct!');
 *   }
 * }
 * ```
 */
@Component({
  selector: 'app-perseus-question',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="perseus-wrapper" #container>
      <perseus-question
        #perseusElement
        [attr.review-mode]="reviewMode ? '' : null"
      ></perseus-question>
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .perseus-wrapper {
        min-height: 100px;
      }

      /* Fallback loading state if element not ready */
      .perseus-wrapper:empty::before {
        content: 'Loading question...';
        display: block;
        padding: 2rem;
        text-align: center;
        color: var(--text-secondary, #666);
      }
    `,
  ],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PerseusWrapperComponent implements AfterViewInit, OnDestroy, OnChanges {
  private readonly cdr = inject(ChangeDetectorRef);

  // ─────────────────────────────────────────────────────────────────────────
  // Inputs
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * The Perseus item to render.
   */
  @Input() item: PerseusItem | null = null;

  /**
   * Whether to show the question in review mode (read-only with correct answers).
   */
  @Input() reviewMode = false;

  /**
   * Whether to auto-focus the question input on load.
   */
  @Input() autoFocus = false;

  // ─────────────────────────────────────────────────────────────────────────
  // Outputs
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Emitted when the question is scored.
   */
  @Output() scored = new EventEmitter<PerseusScoreResult>();

  /**
   * Emitted when the answer state changes.
   */
  @Output() answerChanged = new EventEmitter<boolean>();

  /**
   * Emitted when the component is ready for interaction.
   */
  @Output() ready = new EventEmitter<void>();

  // ─────────────────────────────────────────────────────────────────────────
  // View References
  // ─────────────────────────────────────────────────────────────────────────

  @ViewChild('container') private readonly container!: ElementRef<HTMLDivElement>;

  private perseusElement: PerseusQuestionElement | null = null;
  private initialized = false;
  private loadError: string | null = null;
  // Track pending item changes that arrive before element is ready
  private pendingItem: PerseusItem | null = null;
  private hasPendingItemChange = false;

  // ─────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────────────

  ngAfterViewInit(): void {
    void this.initializeAsync();
  }

  private async initializeAsync(): Promise<void> {
    try {
      // Lazy load React and register the custom element
      await registerPerseusElement();

      // Get reference to the custom element
      this.perseusElement = getPerseusElement(this.container.nativeElement);

      if (this.perseusElement) {
        // Set up event callbacks
        this.perseusElement.onScore = (result: PerseusScoreResult) => {
          this.scored.emit(result);
          this.cdr.markForCheck();
        };

        this.perseusElement.onAnswerChange = (hasAnswer: boolean) => {
          this.answerChanged.emit(hasAnswer);
          this.cdr.markForCheck();
        };

        // Apply pending item that arrived before element was ready
        // Use pendingItem if we have a pending change, otherwise fall back to current input
        const itemToApply = this.hasPendingItemChange ? this.pendingItem : this.item;
        if (itemToApply) {
          this.perseusElement.item = itemToApply;
          this.hasPendingItemChange = false;
        }

        // Apply review mode
        this.perseusElement.reviewMode = this.reviewMode;

        // Auto-focus if requested
        if (this.autoFocus) {
          // Small delay to ensure Perseus is fully initialized
          setTimeout(() => {
            this.perseusElement?.focusInput();
          }, 100);
        }

        this.initialized = true;
        this.ready.emit();
        this.cdr.markForCheck();
      }
    } catch (error) {
      this.loadError = error instanceof Error ? error.message : 'Failed to load Perseus';
      this.cdr.markForCheck();
    }
  }

  ngOnChanges(changes: SimpleChanges): void {
    // Track item changes that arrive before element is ready
    if (changes['item']) {
      this.pendingItem = changes['item'].currentValue as PerseusItem | null;
      this.hasPendingItemChange = true;
    }

    if (!this.perseusElement || !this.initialized) {
      return;
    }

    if (changes['item'] && changes['item'].currentValue !== changes['item'].previousValue) {
      this.perseusElement.item = this.item;
      this.hasPendingItemChange = false;
      // Dark mode refresh disabled for debugging
      // refreshPerseusDarkMode();
    }

    if (changes['reviewMode']) {
      this.perseusElement.reviewMode = this.reviewMode;
    }
  }

  ngOnDestroy(): void {
    // Clean up event callbacks
    if (this.perseusElement) {
      this.perseusElement.onScore = null;
      this.perseusElement.onAnswerChange = null;
    }
    this.perseusElement = null;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Public Methods
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Score the current answer and return the result.
   *
   * @returns The score result, or null if not ready
   */
  score(): PerseusScoreResult | null {
    if (!this.perseusElement) {
      return null;
    }

    return this.perseusElement.score();
  }

  /**
   * Focus the question input.
   */
  focus(): void {
    this.perseusElement?.focusInput();
  }

  /**
   * Check if the component is ready for scoring.
   */
  isReady(): boolean {
    return this.initialized && this.perseusElement !== null;
  }

  /**
   * Get the current state for persistence.
   */
  getState(): unknown {
    return this.perseusElement?.getState() ?? null;
  }

  /**
   * Restore from a saved state.
   */
  restoreState(state: unknown): void {
    this.perseusElement?.restoreState(state);
  }
}
