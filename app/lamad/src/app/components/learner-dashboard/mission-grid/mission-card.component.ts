import { Component, Input } from '@angular/core';
import { RouterModule } from '@angular/router';

import type { DashboardPathEntry } from '../../../models/learner-mastery-profile.model';

/**
 * Individual mission card showing path progress.
 */
@Component({
  selector: 'app-mission-card',
  standalone: true,
  imports: [RouterModule],
  template: `
    <div class="mission-card">
      <div class="mission-header">
        <h3 class="mission-title">{{ path.title }}</h3>
        <div class="mission-meta">{{ path.totalSteps }} steps</div>
      </div>
      <div class="progress-ring-container">
        <svg viewBox="0 0 100 100" class="progress-ring">
          <circle
            class="progress-ring-bg"
            cx="50"
            cy="50"
            r="40"
            fill="none"
            stroke="var(--border-color, #e5e7eb)"
            stroke-width="8"
          />
          <circle
            class="progress-ring-fill"
            cx="50"
            cy="50"
            r="40"
            fill="none"
            stroke="#667eea"
            stroke-width="8"
            [attr.stroke-dasharray]="circumference"
            [attr.stroke-dashoffset]="progressOffset"
            transform="rotate(-90 50 50)"
          />
        </svg>
        <div class="progress-text">{{ path.progressPercent }}%</div>
      </div>
      <a [routerLink]="['/lamad/path', path.pathId]" class="continue-link">Continue Learning</a>
    </div>
  `,
  styles: [
    `
      .mission-card {
        display: flex;
        flex-direction: column;
        padding: 1.5rem;
        background: var(--surface-color, #fff);
        border: 1px solid var(--border-color, #e5e7eb);
        border-radius: 0.5rem;
        transition:
          box-shadow 0.2s ease,
          transform 0.2s ease;
      }

      .mission-card:hover {
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
        transform: translateY(-2px);
      }

      .mission-header {
        margin-bottom: 1rem;
      }

      .mission-title {
        font-size: 1.125rem;
        font-weight: 600;
        margin: 0 0 0.25rem 0;
        color: var(--text-primary, #111827);
        line-height: 1.4;
      }

      .mission-meta {
        font-size: 0.875rem;
        color: var(--text-secondary, #6b7280);
      }

      .progress-ring-container {
        position: relative;
        width: 120px;
        height: 120px;
        margin: 1rem auto;
      }

      .progress-ring {
        width: 100%;
        height: 100%;
      }

      .progress-ring-bg {
        opacity: 0.2;
      }

      .progress-ring-fill {
        transition: stroke-dashoffset 0.5s ease;
      }

      .progress-text {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary, #111827);
      }

      .continue-link {
        display: block;
        padding: 0.75rem 1rem;
        background: #667eea;
        color: white;
        text-align: center;
        text-decoration: none;
        border-radius: 0.375rem;
        font-weight: 500;
        transition: background 0.2s ease;
      }

      .continue-link:hover {
        background: #5568d3;
      }
    `,
  ],
})
export class MissionCardComponent {
  @Input({ required: true }) path!: DashboardPathEntry;

  readonly radius = 40;
  readonly circumference = 2 * Math.PI * this.radius;

  get progressOffset(): number {
    if (!this.path) return this.circumference;
    const progress = this.path.progressPercent / 100;
    return this.circumference * (1 - progress);
  }
}
