import { CommonModule } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  EventEmitter,
  Input,
  Output,
  computed,
  signal,
} from '@angular/core';

/**
 * PinProgressComponent — stateless per-EPR pull-progress bar (rung-4 provide UI).
 *
 * Mirrors commitment-bar.component.ts: signal-backed @Input setters, OnPush, a
 * filled bar plus a fraction/percent readout. The data comes from
 * AcquisitionService.pullStatus$() (PullStatusInfo). This element owns NO state
 * and does NO fetching — the Angular host (EprLinkComponent) owns the
 * pullStatus$ subscription lifecycle and feeds these inputs (blank-slate
 * element discipline). cancel/retry are surfaced upward as @Output events; the
 * host disposes them.
 *
 * Null-guards (spec §4.3, the null≠done contract): a null `total` means "state
 * not computable yet" — the bar shows a waiting affordance and NEVER the
 * caught-up badge, regardless of the `caughtUp` input.
 *
 * Internal signals are suffixed `*Sig` so the public input setters (`total`,
 * `fetched`, …) don't shadow the template's signal getters (commitment-bar
 * sidesteps this by naming inputs distinctly; we keep the wire-aligned input
 * names and rename the signals instead).
 */
@Component({
  selector: 'app-pin-progress',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="pin-progress" data-testid="pin-progress">
      @if (totalSig() === null) {
        <span data-testid="pin-progress-waiting" class="waiting">Resolving…</span>
      } @else {
        <div class="bar">
          <div class="fill" [style.width.%]="percent()"></div>
        </div>
        <span data-testid="pin-progress-fraction" class="frac">
          {{ fetchedSig() }} / {{ totalSig() }}
        </span>
        <span data-testid="pin-progress-percent">{{ percent() | number: '1.0-0' }}%</span>
        @if (failedSig() > 0) {
          <span data-testid="pin-progress-failed" class="failed">{{ failedSig() }} failed</span>
          <button type="button" data-testid="pin-progress-retry" class="ctl" (click)="retry.emit()">
            Retry
          </button>
        }
        @if (caughtUpSig() === true) {
          <span data-testid="pin-progress-caught-up" class="done">✓ serving</span>
        }
      }
      @if (!isCaughtUp()) {
        <button type="button" data-testid="pin-progress-cancel" class="ctl" (click)="cancel.emit()">
          Cancel
        </button>
      }
    </div>
  `,
  styles: [
    `
      .pin-progress {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 0.85rem;
      }
      .bar {
        width: 100px;
        height: 6px;
        background: #eee;
        border-radius: 3px;
        overflow: hidden;
      }
      .fill {
        height: 100%;
        background: var(--pin-progress-fill, #2a7);
      }
      .failed {
        color: var(--pin-progress-failed, #c33);
      }
      .done {
        color: var(--pin-progress-fill, #2a7);
      }
      .waiting {
        color: #888;
      }
      .ctl {
        background: none;
        border: none;
        padding: 0;
        font-size: inherit;
        color: var(--pin-progress-ctl, #555);
        cursor: pointer;
        text-decoration: underline;
      }
    `,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PinProgressComponent {
  protected readonly totalSig = signal<number | null>(null);
  protected readonly fetchedSig = signal(0);
  protected readonly pendingSig = signal(0);
  protected readonly failedSig = signal(0);
  protected readonly caughtUpSig = signal<boolean | null>(null);

  @Input() set total(v: number | null) {
    this.totalSig.set(v);
  }
  @Input() set fetched(v: number) {
    this.fetchedSig.set(v);
  }
  @Input() set pending(v: number) {
    this.pendingSig.set(v);
  }
  @Input() set failed(v: number) {
    this.failedSig.set(v);
  }
  @Input() set caughtUp(v: boolean | null) {
    this.caughtUpSig.set(v);
  }

  /** Cancel the pull (un-pin). The host owns the disposition. */
  @Output() cancel = new EventEmitter<void>();

  /** Retry failed items. The host owns the disposition. */
  @Output() retry = new EventEmitter<void>();

  /** Percent complete. Null/zero total = 0% (never NaN/Infinity). */
  protected readonly percent = computed(() => {
    const t = this.totalSig();
    if (t === null || t === 0) return 0;
    return Math.min(100, (this.fetchedSig() / t) * 100);
  });

  /**
   * The null≠done guard: caught-up is true ONLY when total is computable AND the
   * backend asserts caughtUp. A null total can never read as serving/done.
   */
  protected readonly isCaughtUp = computed(
    () => this.totalSig() !== null && this.caughtUpSig() === true
  );
}
