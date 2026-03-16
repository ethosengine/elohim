/**
 * PsephosBallotComponent - Angular component wrapping Psephos custom element.
 *
 * This component provides an Angular-friendly interface to the Psephos
 * governance ballot renderer, handling ballot display and vote recognition.
 *
 * @example
 * ```html
 * <app-psephos-ballot
 *   [ballot]="ballotData"
 *   [reviewMode]="false"
 *   (recognized)="handleRecognition($event)"
 *   (answerChanged)="handleAnswerChange($event)">
 * </app-psephos-ballot>
 * ```
 */

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
  registerPsephosElement,
  getPsephosElement,
  type PsephosBallotElement,
} from './psephos-element-loader';

@Component({
  selector: 'app-psephos-ballot',
  standalone: true,
  template: `
    <div class="psephos-wrapper" #container>
      <psephos-ballot></psephos-ballot>
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .psephos-wrapper {
        width: 100%;
        min-height: 100px;
      }

      .psephos-wrapper:empty::before {
        content: 'Loading ballot...';
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
export class PsephosBallotComponent implements AfterViewInit, OnChanges, OnDestroy {
  private readonly cdr = inject(ChangeDetectorRef);

  // ─────────────────────────────────────────────────────────────────────────
  // Inputs
  // ─────────────────────────────────────────────────────────────────────────

  /** The ballot data to render */
  @Input() ballot: unknown | null = null;

  /** Review mode (read-only with results) */
  @Input() reviewMode = false;

  // ─────────────────────────────────────────────────────────────────────────
  // Outputs
  // ─────────────────────────────────────────────────────────────────────────

  /** Emitted when a recognition result is produced (vote submitted) */
  @Output() recognized = new EventEmitter<unknown>();

  /** Emitted when the answer state changes */
  @Output() answerChanged = new EventEmitter<boolean>();

  /** Emitted when the component is ready */
  @Output() ready = new EventEmitter<void>();

  // ─────────────────────────────────────────────────────────────────────────
  // View References
  // ─────────────────────────────────────────────────────────────────────────

  @ViewChild('container') private readonly container!: ElementRef<HTMLElement>;

  private element: PsephosBallotElement | null = null;
  private initialized = false;
  private loadError: string | null = null;

  // ─────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────────────

  ngAfterViewInit(): void {
    void this.initializeAsync();
  }

  private async initializeAsync(): Promise<void> {
    try {
      await registerPsephosElement();
      this.element = getPsephosElement(this.container.nativeElement);

      if (this.element) {
        this.initializeElement();
      }
    } catch (error) {
      this.loadError = error instanceof Error ? error.message : 'Failed to load Psephos';
      this.cdr.markForCheck();
    }
  }

  private initializeElement(): void {
    if (!this.element) return;

    // Set up event callbacks
    this.element.onRecognition = (rec: unknown) => {
      this.recognized.emit(rec);
      this.cdr.markForCheck();
    };

    this.element.onAnswerChange = (has: boolean) => {
      this.answerChanged.emit(has);
      this.cdr.markForCheck();
    };

    // Apply initial property values
    if (this.ballot) {
      this.element.ballot = this.ballot;
    }
    this.element.reviewMode = this.reviewMode;

    this.initialized = true;
    this.ready.emit();
    this.cdr.markForCheck();
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (!this.element || !this.initialized) return;

    if (changes['ballot']) {
      this.element.ballot = this.ballot;
    }
    if (changes['reviewMode']) {
      this.element.reviewMode = this.reviewMode;
    }
  }

  ngOnDestroy(): void {
    if (this.element) {
      this.element.onRecognition = null;
      this.element.onAnswerChange = null;
    }
    this.element = null;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Public Methods
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Get the recognition result for the current vote.
   */
  getRecognition(): unknown | null {
    return this.element?.getRecognition() ?? null;
  }

  /**
   * Check if the component is ready.
   */
  isReady(): boolean {
    return this.initialized && this.element !== null;
  }
}
