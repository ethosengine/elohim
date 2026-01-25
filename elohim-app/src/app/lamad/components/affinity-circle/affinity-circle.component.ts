import { CommonModule } from '@angular/common';
import { Component, Input } from '@angular/core';

@Component({
  selector: 'app-affinity-circle',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div
      class="affinity-circle"
      [class]="'affinity-' + getAffinityLevel()"
      [style.width.px]="size"
      [style.height.px]="size"
    >
      <svg [attr.width]="size" [attr.height]="size" [attr.viewBox]="'0 0 ' + size + ' ' + size">
        <!-- Background circle -->
        <circle
          [attr.cx]="size / 2"
          [attr.cy]="size / 2"
          [attr.r]="size / 2 - 4"
          fill="none"
          stroke="#e0e0e0"
          stroke-width="4"
        />
        <!-- Progress circle -->
        <circle
          [attr.cx]="size / 2"
          [attr.cy]="size / 2"
          [attr.r]="size / 2 - 4"
          fill="none"
          [attr.stroke]="getStrokeColor()"
          stroke-width="4"
          stroke-linecap="round"
          [attr.stroke-dasharray]="circumference"
          [attr.stroke-dashoffset]="strokeDashoffset"
          transform="rotate(-90)"
          [attr.transform-origin]="size / 2 + ' ' + size / 2"
        />
      </svg>
      <div class="percentage-text">{{ getPercentage() }}%</div>
    </div>
  `,
  styles: [
    `
      .affinity-circle {
        position: relative;
        display: inline-flex;
        align-items: center;
        justify-content: center;
      }

      svg {
        position: absolute;
        top: 0;
        left: 0;
      }

      .percentage-text {
        font-size: 0.875rem;
        font-weight: 600;
        color: var(--text-primary, #1a1a1a);
        z-index: 1;
      }

      .affinity-circle.affinity-none .percentage-text {
        color: var(--text-secondary, #999);
      }

      .affinity-circle.affinity-low .percentage-text {
        color: var(--warning-color, #f57c00);
      }

      .affinity-circle.affinity-medium .percentage-text {
        color: var(--primary-color, #1976d2);
      }

      .affinity-circle.affinity-high .percentage-text {
        color: var(--success-color, #2e7d32);
      }
    `,
  ],
})
export class AffinityCircleComponent {
  @Input() affinity = 0;
  @Input() size = 80;

  get circumference(): number {
    const radius = this.size / 2 - 4;
    return 2 * Math.PI * radius;
  }

  get strokeDashoffset(): number {
    return this.circumference - this.affinity * this.circumference;
  }

  getPercentage(): number {
    return Math.round(this.affinity * 100);
  }

  getAffinityLevel(): string {
    if (this.affinity >= 0.8) return 'high';
    if (this.affinity >= 0.5) return 'medium';
    if (this.affinity >= 0.2) return 'low';
    return 'none';
  }

  getStrokeColor(): string {
    const level = this.getAffinityLevel();
    const colors: Record<string, string> = {
      none: '#e0e0e0',
      low: '#f57c00',
      medium: '#1976d2',
      high: '#2e7d32',
    };
    return colors[level] || '#e0e0e0';
  }
}
