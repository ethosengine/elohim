/**
 * SophiaWrapperComponent - Angular component wrapping Sophia custom element.
 *
 * This component provides an Angular-friendly interface to the Sophia
 * assessment renderer, handling both mastery and discovery/reflection modes.
 *
 * @example
 * ```html
 * <app-sophia-question
 *   [moment]="momentData"
 *   [mode]="'reflection'"
 *   (recognized)="handleRecognition($event)"
 *   (answerChanged)="handleAnswerChange($event)">
 * </app-sophia-question>
 * ```
 */

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

import {
  registerSophiaElement,
  type SophiaQuestionElement,
  type UserInputMap,
  getSophiaElement,
} from './sophia-element-loader';

import type { Moment, Recognition } from './sophia-moment.model';

@Component({
  selector: 'app-sophia-question',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="sophia-wrapper" #container>
      <sophia-question
        #sophiaElement
        [attr.review-mode]="reviewMode ? '' : null"
        [attr.mode]="mode"
      ></sophia-question>
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .sophia-wrapper {
        min-height: 100px;
      }

      .sophia-wrapper:empty::before {
        content: 'Loading assessment...';
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
export class SophiaWrapperComponent implements AfterViewInit, OnDestroy, OnChanges {
  private readonly cdr = inject(ChangeDetectorRef);

  // ─────────────────────────────────────────────────────────────────────────
  // Inputs
  // ─────────────────────────────────────────────────────────────────────────

  /** The Sophia Moment to render */
  @Input() moment: Moment | null = null;

  /** Assessment mode: mastery (graded), discovery, or reflection (psychometric) */
  @Input() mode: 'mastery' | 'discovery' | 'reflection' = 'mastery';

  /** Instrument ID for psyche-core lookup */
  @Input() instrumentId: string | null = null;

  /** Review mode (read-only with results) */
  @Input() reviewMode = false;

  /** Initial user input for answer persistence (set before moment when navigating back) */
  @Input() initialUserInput: UserInputMap | null = null;

  /** Auto-focus on load */
  @Input() autoFocus = false;

  // ─────────────────────────────────────────────────────────────────────────
  // Outputs
  // ─────────────────────────────────────────────────────────────────────────

  /** Emitted when a recognition result is produced */
  @Output() recognized = new EventEmitter<Recognition>();

  /** Emitted when the answer state changes */
  @Output() answerChanged = new EventEmitter<boolean>();

  /** Emitted when the component is ready */
  @Output() ready = new EventEmitter<void>();

  // ─────────────────────────────────────────────────────────────────────────
  // View References
  // ─────────────────────────────────────────────────────────────────────────

  @ViewChild('container') private readonly container!: ElementRef<HTMLDivElement>;

  private sophiaElement: SophiaQuestionElement | null = null;
  private initialized = false;
  private loadError: string | null = null;
  private pendingMoment: Moment | null = null;
  private hasPendingMomentChange = false;

  // ─────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────────────

  async ngAfterViewInit(): Promise<void> {
    console.log('[SophiaWrapper] ngAfterViewInit, moment:', this.moment?.id || 'null');
    try {
      // Lazy load React and register the custom element
      await registerSophiaElement();

      // Get reference to the custom element
      this.sophiaElement = getSophiaElement(this.container.nativeElement);
      console.log('[SophiaWrapper] Element found:', !!this.sophiaElement);

      if (this.sophiaElement) {
        this.initializeElement();
      }
    } catch (error) {
      this.loadError = error instanceof Error ? error.message : 'Failed to load Sophia';
      console.error('[SophiaWrapper] Load error:', error);
      this.cdr.markForCheck();
    }
  }

  private initializeElement(): void {
    if (!this.sophiaElement) return;

    // Set up event callbacks
    this.setupEventCallbacks();

    // Set mode and instrument ID
    this.sophiaElement.mode = this.mode;
    if (this.instrumentId) {
      this.sophiaElement.instrumentId = this.instrumentId;
    }

    // Apply pending moment
    this.applyPendingMoment();

    // Apply review mode
    this.sophiaElement.reviewMode = this.reviewMode;

    // Auto-focus if requested
    if (this.autoFocus) {
      setTimeout(() => this.sophiaElement?.focusInput(), 100);
    }

    this.initialized = true;
    this.ready.emit();
    this.cdr.markForCheck();
  }

  private setupEventCallbacks(): void {
    if (!this.sophiaElement) return;

    this.sophiaElement.onRecognition = (result: Recognition) => {
      this.recognized.emit(result);
      this.cdr.markForCheck();
    };

    this.sophiaElement.onAnswerChange = (hasAnswer: boolean) => {
      this.answerChanged.emit(hasAnswer);
      this.cdr.markForCheck();
    };
  }

  private applyPendingMoment(): void {
    if (!this.sophiaElement) return;

    const momentToApply = this.hasPendingMomentChange ? this.pendingMoment : this.moment;
    console.log('[SophiaWrapper] Applying moment:', {
      hasPendingChange: this.hasPendingMomentChange,
      pendingMomentId: this.pendingMoment?.id || 'null',
      currentMomentId: this.moment?.id || 'null',
      momentToApplyId: momentToApply?.id || 'null',
      hasInitialUserInput: !!this.initialUserInput,
    });

    if (momentToApply) {
      // Set initialUserInput BEFORE moment for answer restoration
      this.sophiaElement.initialUserInput = this.initialUserInput ?? null;
      if (this.initialUserInput) {
        console.log('[SophiaWrapper] Setting initialUserInput before moment');
      }
      console.log('[SophiaWrapper] Setting moment:', momentToApply.id);
      this.sophiaElement.moment = momentToApply;
      this.hasPendingMomentChange = false;
    }
  }

  ngOnChanges(changes: SimpleChanges): void {
    console.log('[SophiaWrapper] ngOnChanges:', {
      hasElement: !!this.sophiaElement,
      initialized: this.initialized,
      momentChanged: !!changes['moment'],
      newMoment: changes['moment']?.currentValue?.id || 'null',
    });

    // Track moment changes that arrive before element is ready
    if (changes['moment']) {
      this.pendingMoment = changes['moment'].currentValue;
      this.hasPendingMomentChange = true;
    }

    if (!this.sophiaElement || !this.initialized) {
      console.log(
        '[SophiaWrapper] Element not ready, storing pending moment:',
        this.pendingMoment?.id || 'null'
      );
      return;
    }

    // Handle initialUserInput changes BEFORE moment changes (important for answer restoration)
    if (changes['initialUserInput']) {
      console.log(
        '[SophiaWrapper] Updating initialUserInput:',
        this.initialUserInput ? 'has value' : 'null'
      );
      this.sophiaElement.initialUserInput = this.initialUserInput;
    }

    if (changes['moment'] && changes['moment'].currentValue !== changes['moment'].previousValue) {
      console.log('[SophiaWrapper] Updating moment to:', this.moment?.id);
      this.sophiaElement.moment = this.moment;
      this.hasPendingMomentChange = false;
    }

    if (changes['mode']) {
      this.sophiaElement.mode = this.mode;
    }

    if (changes['reviewMode']) {
      this.sophiaElement.reviewMode = this.reviewMode;
    }

    if (changes['instrumentId']) {
      this.sophiaElement.instrumentId = this.instrumentId;
    }
  }

  ngOnDestroy(): void {
    if (this.sophiaElement) {
      this.sophiaElement.onRecognition = null;
      this.sophiaElement.onAnswerChange = null;
    }
    this.sophiaElement = null;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Public Methods
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Get the recognition result for the current answer.
   */
  getRecognition(): Recognition | null {
    if (!this.sophiaElement) {
      console.warn('Sophia element not initialized');
      return null;
    }
    return this.sophiaElement.getRecognition();
  }

  /**
   * Focus the input.
   */
  focus(): void {
    this.sophiaElement?.focusInput();
  }

  /**
   * Check if the component is ready.
   */
  isReady(): boolean {
    return this.initialized && this.sophiaElement !== null;
  }

  /**
   * Get the current state for persistence.
   */
  getState(): unknown {
    return this.sophiaElement?.getState() ?? null;
  }

  /**
   * Restore from a saved state.
   */
  restoreState(state: unknown): void {
    this.sophiaElement?.restoreState(state);
  }
}
