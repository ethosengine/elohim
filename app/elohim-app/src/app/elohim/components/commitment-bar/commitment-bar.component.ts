import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input, computed, signal } from '@angular/core';

@Component({
  selector: 'elohim-commitment-bar',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="cbar" data-testid="commitment-bar">
      <div class="bar">
        <div class="fill" [style.width.%]="cappedPercent()"></div>
      </div>
      <span data-testid="commitment-bar-percent">{{ percent() | number: '1.0-0' }}%</span>
      @if (percent() > 100) {
        <span data-testid="commitment-bar-over-delivered">★ over-delivered</span>
      }
    </div>
  `,
  styles: [
    `
      .cbar {
        display: flex;
        align-items: center;
        gap: 0.5rem;
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
        background: var(--commitment-fill, #2a7);
      }
    `,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CommitmentBarComponent {
  protected readonly committed = signal(0);
  protected readonly delivered = signal(0);

  @Input() set committedBytes(v: number) {
    this.committed.set(v);
  }

  @Input() set deliveredBytes(v: number) {
    this.delivered.set(v);
  }

  protected readonly percent = computed(() => {
    const c = this.committed();
    if (c === 0) return 0;
    return (this.delivered() / c) * 100;
  });

  protected readonly cappedPercent = computed(() => Math.min(100, this.percent()));
}
