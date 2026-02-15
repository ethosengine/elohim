import { Component, OnInit, inject } from '@angular/core';
import { RouterModule } from '@angular/router';

import { Observable, map } from 'rxjs';

import { getFreshnessColor, getFreshnessIcon } from '../../../models/mastery-visualization';
import { ContentMasteryService } from '../../../services/content-mastery.service';

import type { ContentMastery } from '../../../models';

/**
 * Queue of content needing refresh, sorted by freshness priority.
 */
@Component({
  selector: 'app-refresh-queue',
  standalone: true,
  imports: [RouterModule],
  template: `
    <div class="refresh-queue">
      <h3 class="queue-title">Practice Queue</h3>
      @if (refreshQueue$ | async; as queue) {
        @if (queue.length > 0) {
          <div class="queue-list">
            @for (item of queue; track item.contentId) {
              <div class="queue-item">
                <div
                  class="freshness-indicator"
                  [style.background-color]="getFreshnessColorValue(item.freshness)"
                >
                  <span class="material-icons">{{ getFreshnessIconValue(item.freshness) }}</span>
                </div>
                <div class="item-content">
                  <div class="item-id">{{ item.contentId }}</div>
                  <div class="item-meta">
                    <span class="freshness-score">
                      {{ formatFreshness(item.freshness) }}% fresh
                    </span>
                    <span class="separator">•</span>
                    <span class="refresh-type">{{ getRefreshTypeLabel(item.refreshType) }}</span>
                  </div>
                </div>
                <a
                  [routerLink]="['/lamad/resource', item.contentId]"
                  class="practice-link"
                  title="Practice this content"
                >
                  <span class="material-icons">play_arrow</span>
                  Practice
                </a>
              </div>
            }
          </div>
        } @else {
          <div class="empty-queue">
            <span class="material-icons empty-icon">check_circle</span>
            <p>All content is fresh! Great work maintaining your mastery.</p>
          </div>
        }
      }
    </div>
  `,
  styles: [
    `
      .refresh-queue {
        padding: 1.5rem;
        background: var(--surface-color, #fff);
        border: 1px solid var(--border-color, #e5e7eb);
        border-radius: 0.5rem;
      }

      .queue-title {
        font-size: 1.125rem;
        font-weight: 600;
        margin: 0 0 1.5rem 0;
        color: var(--text-primary, #111827);
      }

      .queue-list {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
      }

      .queue-item {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1rem;
        background: var(--bg-secondary, #f9fafb);
        border-radius: 0.375rem;
        transition: background 0.2s ease;
      }

      .queue-item:hover {
        background: #f3f4f6;
      }

      .freshness-indicator {
        flex-shrink: 0;
        width: 2.5rem;
        height: 2.5rem;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
      }

      .freshness-indicator .material-icons {
        font-size: 1.25rem;
      }

      .item-content {
        flex: 1;
        min-width: 0;
      }

      .item-id {
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-primary, #111827);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        margin-bottom: 0.25rem;
      }

      .item-meta {
        font-size: 0.75rem;
        color: var(--text-secondary, #6b7280);
      }

      .separator {
        margin: 0 0.375rem;
      }

      .practice-link {
        display: flex;
        align-items: center;
        gap: 0.375rem;
        padding: 0.5rem 1rem;
        background: #667eea;
        color: white;
        text-decoration: none;
        border-radius: 0.375rem;
        font-size: 0.875rem;
        font-weight: 500;
        transition: background 0.2s ease;
        flex-shrink: 0;
      }

      .practice-link:hover {
        background: #5568d3;
      }

      .practice-link .material-icons {
        font-size: 1rem;
      }

      .empty-queue {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 1rem;
        padding: 2rem 1rem;
        text-align: center;
        background: var(--bg-secondary, #f9fafb);
        border-radius: 0.375rem;
      }

      .empty-icon {
        font-size: 3rem;
        color: #4caf50;
      }

      .empty-queue p {
        margin: 0;
        font-size: 0.875rem;
        color: var(--text-secondary, #6b7280);
      }

      @media (max-width: 640px) {
        .queue-item {
          flex-wrap: wrap;
        }

        .practice-link {
          width: 100%;
          justify-content: center;
        }
      }
    `,
  ],
})
export class RefreshQueueComponent implements OnInit {
  private readonly masteryService = inject(ContentMasteryService);

  refreshQueue$!: Observable<ContentMastery[]>;

  ngOnInit(): void {
    this.refreshQueue$ = this.masteryService.getContentNeedingRefresh().pipe(
      map(items => {
        const sorted = [...items].sort((a, b) => a.freshness - b.freshness);
        return sorted.slice(0, 5);
      })
    );
  }

  getFreshnessColorValue(freshness: number): string {
    return getFreshnessColor(freshness);
  }

  getFreshnessIconValue(freshness: number): string {
    return getFreshnessIcon(freshness);
  }

  formatFreshness(freshness: number): number {
    return Math.round(freshness * 100);
  }

  getRefreshTypeLabel(refreshType?: 'review' | 'retest' | 'relearn'): string {
    switch (refreshType) {
      case 'review':
        return 'Review needed';
      case 'retest':
        return 'Retest recommended';
      case 'relearn':
        return 'Relearning required';
      default:
        return 'Practice';
    }
  }
}
