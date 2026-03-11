import { Component, Input } from '@angular/core';

import {
  MASTERY_ACCENT_COLORS,
  LEVEL_CHANGE_ICONS,
  MASTERY_ICONS,
} from '../../../models/mastery-visualization';

import type { LevelUpEvent } from '../../../models/learner-mastery-profile.model';
import type { MasteryLevel } from '@app/elohim/models/agent.model';

/**
 * Timeline of recent mastery level changes.
 */
@Component({
  selector: 'app-level-up-timeline',
  standalone: true,
  imports: [],
  template: `
    <div class="level-up-timeline">
      <h3 class="timeline-title">Recent Progress</h3>
      @if (levelUps.length > 0) {
        <div class="timeline-list">
          @for (levelUp of levelUps; track $index) {
            <div class="timeline-item">
              <div
                class="timeline-marker"
                [style.background-color]="getAccentColor(levelUp.toLevel)"
              >
                <span class="material-icons">{{ getIcon(levelUp.toLevel) }}</span>
              </div>
              <div class="timeline-content">
                <div class="timeline-header">
                  <span class="content-id">{{ levelUp.contentId }}</span>
                  <span class="timeline-date">{{ formatDate(levelUp.timestamp) }}</span>
                </div>
                <div class="level-change">
                  <span class="from-level" [style.color]="getAccentColor(levelUp.fromLevel)">
                    {{ levelUp.fromLevel }}
                  </span>
                  <span class="material-icons change-arrow">{{ getChangeIcon() }}</span>
                  <span class="to-level" [style.color]="getAccentColor(levelUp.toLevel)">
                    {{ levelUp.toLevel }}
                  </span>
                </div>
              </div>
            </div>
          }
        </div>
      } @else {
        <div class="empty-timeline">
          <p>No recent progress. Complete assessments to level up!</p>
        </div>
      }
    </div>
  `,
  styles: [
    `
      .level-up-timeline {
        padding: 1.5rem;
        background: var(--surface-color, #fff);
        border: 1px solid var(--border-color, #e5e7eb);
        border-radius: 0.5rem;
      }

      .timeline-title {
        font-size: 1.125rem;
        font-weight: 600;
        margin: 0 0 1.5rem 0;
        color: var(--text-primary, #111827);
      }

      .timeline-list {
        display: flex;
        flex-direction: column;
        gap: 1rem;
      }

      .timeline-item {
        display: flex;
        gap: 1rem;
        padding-bottom: 1rem;
        border-bottom: 1px solid var(--border-color, #e5e7eb);
      }

      .timeline-item:last-child {
        border-bottom: none;
        padding-bottom: 0;
      }

      .timeline-marker {
        flex-shrink: 0;
        width: 2.5rem;
        height: 2.5rem;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
      }

      .timeline-marker .material-icons {
        font-size: 1.25rem;
      }

      .timeline-content {
        flex: 1;
        min-width: 0;
      }

      .timeline-header {
        display: flex;
        justify-content: space-between;
        align-items: baseline;
        gap: 0.5rem;
        margin-bottom: 0.5rem;
      }

      .content-id {
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-primary, #111827);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .timeline-date {
        font-size: 0.75rem;
        color: var(--text-secondary, #6b7280);
        flex-shrink: 0;
      }

      .level-change {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 0.875rem;
        font-weight: 500;
      }

      .from-level,
      .to-level {
        text-transform: capitalize;
      }

      .change-arrow {
        font-size: 1rem;
        color: var(--text-secondary, #6b7280);
      }

      .empty-timeline {
        padding: 2rem 1rem;
        text-align: center;
        background: var(--bg-secondary, #f9fafb);
        border-radius: 0.375rem;
        color: var(--text-secondary, #6b7280);
      }

      .empty-timeline p {
        margin: 0;
        font-size: 0.875rem;
      }
    `,
  ],
})
export class LevelUpTimelineComponent {
  @Input() levelUps: LevelUpEvent[] = [];

  getAccentColor(level: MasteryLevel): string {
    return MASTERY_ACCENT_COLORS[level];
  }

  getIcon(level: MasteryLevel): string {
    return MASTERY_ICONS[level];
  }

  getChangeIcon(): string {
    return LEVEL_CHANGE_ICONS.up;
  }

  formatDate(timestamp: string): string {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return 'Today';
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return `${diffDays} days ago`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`;
    return `${Math.floor(diffDays / 30)} months ago`;
  }
}
