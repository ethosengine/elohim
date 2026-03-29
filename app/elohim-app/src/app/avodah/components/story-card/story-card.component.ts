import { Component, computed, input, output } from '@angular/core';

import { ContentNode } from '@app/lamad/models/content-node.model';

import { parseWorkStoryMeta, type WorkStoryMeta } from '../../models/work-story.model';

/**
 * StoryCardComponent — compact card for a work-story ContentNode.
 *
 * Displays title, priority, story points, assignee, tags, and protocol badges:
 * - Attestation gate badge (lamad path required to bid/accept)
 * - Exchange badge (story is published to the shefa exchange)
 * - Cadence badge (recurring story with reset schedule)
 *
 * Part of the Avodah (עבודה) pillar — work as service, not commodity.
 */
@Component({
  selector: 'app-story-card',
  standalone: true,
  template: `
    <div
      class="story-card priority-{{ meta().priority }}"
      role="button"
      data-testid="story-card"
      [attr.aria-label]="'Open story: ' + story().title"
      (click)="cardClick.emit(story())"
      (keydown.enter)="cardClick.emit(story())"
      (keydown.space)="cardClick.emit(story())"
      tabindex="0"
    >
      <!-- Header: priority dot + short id -->
      <div class="card-header">
        <span class="priority-dot" [attr.aria-label]="'Priority: ' + meta().priority"></span>
        <span class="short-id">#{{ shortId() }}</span>
        @if (meta().storyPoints !== undefined) {
          <span class="story-points">{{ meta().storyPoints }}pt</span>
        }
      </div>

      <!-- Title -->
      <h3 class="card-title">{{ story().title }}</h3>

      <!-- Protocol badges -->
      <div class="badges">
        @if (hasAttestationGates()) {
          <span class="badge badge--attestation" data-testid="badge-attestation">🎓 attested</span>
        }
        @if (isOnExchange()) {
          <span class="badge badge--exchange" data-testid="badge-exchange">⚖ exchange</span>
        }
        @if (hasCadence()) {
          <span class="badge badge--cadence" data-testid="badge-cadence">
            🔄 {{ meta().cadence?.interval }}
          </span>
        }
      </div>

      <!-- Footer: assignee + tags -->
      <div class="card-footer">
        <span class="assignee">
          {{ meta().assigneeId ? '@' + meta().assigneeId : '@unassigned' }}
        </span>
        @if (story().tags.length > 0) {
          <span class="tags">
            @for (tag of story().tags; track tag) {
              <span class="tag">#{{ tag }}</span>
            }
          </span>
        }
      </div>
    </div>
  `,
  styles: [
    `
      .story-card {
        background: #1a1a2e;
        border: 1px solid #3730a3;
        border-radius: 8px;
        padding: 0.75rem;
        cursor: pointer;
        transition:
          transform 0.15s ease,
          box-shadow 0.15s ease,
          border-color 0.15s ease;
        outline: none;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      .story-card:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 16px rgba(99, 102, 241, 0.25);
        border-color: #6366f1;
      }

      .story-card:focus-visible {
        outline: 2px solid #6366f1;
        outline-offset: 2px;
      }

      /* Priority border accent */
      .story-card.priority-urgent {
        border-left: 3px solid #ef4444;
      }
      .story-card.priority-high {
        border-left: 3px solid #f59e0b;
      }
      .story-card.priority-medium {
        border-left: 3px solid #6366f1;
      }
      .story-card.priority-low {
        border-left: 3px solid #475569;
      }

      .card-header {
        display: flex;
        align-items: center;
        gap: 0.375rem;
      }

      .priority-dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        flex-shrink: 0;
      }

      .priority-urgent .priority-dot {
        background: #ef4444;
      }
      .priority-high .priority-dot {
        background: #f59e0b;
      }
      .priority-medium .priority-dot {
        background: #6366f1;
      }
      .priority-low .priority-dot {
        background: #475569;
      }

      .short-id {
        font-size: 0.6875rem;
        color: #64748b;
        font-family: 'JetBrains Mono', 'Fira Code', monospace;
      }

      .story-points {
        margin-left: auto;
        font-size: 0.6875rem;
        color: #94a3b8;
        background: #1e293b;
        padding: 0.0625rem 0.375rem;
        border-radius: 9999px;
      }

      .card-title {
        margin: 0;
        font-size: 0.875rem;
        font-weight: 500;
        color: #e2e8f0;
        line-height: 1.4;
        overflow: hidden;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
      }

      .badges {
        display: flex;
        flex-wrap: wrap;
        gap: 0.25rem;
      }

      .badge {
        font-size: 0.625rem;
        padding: 0.125rem 0.5rem;
        border-radius: 9999px;
        font-weight: 500;
        letter-spacing: 0.02em;
      }

      .badge--attestation {
        background: rgba(139, 92, 246, 0.15);
        color: #a78bfa;
        border: 1px solid rgba(139, 92, 246, 0.3);
      }

      .badge--exchange {
        background: rgba(245, 158, 11, 0.15);
        color: #fbbf24;
        border: 1px solid rgba(245, 158, 11, 0.3);
      }

      .badge--cadence {
        background: rgba(16, 185, 129, 0.15);
        color: #34d399;
        border: 1px solid rgba(16, 185, 129, 0.3);
      }

      .card-footer {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        flex-wrap: wrap;
      }

      .assignee {
        font-size: 0.6875rem;
        color: #64748b;
      }

      .tags {
        display: flex;
        gap: 0.25rem;
        flex-wrap: wrap;
        margin-left: auto;
      }

      .tag {
        font-size: 0.625rem;
        color: #475569;
      }
    `,
  ],
})
export class StoryCardComponent {
  readonly story = input.required<ContentNode>();
  readonly cardClick = output<ContentNode>();

  readonly meta = computed<WorkStoryMeta>(() =>
    parseWorkStoryMeta(this.story().metadata)
  );

  readonly shortId = computed<string>(() => {
    const id = this.story().id;
    const lastDash = id.lastIndexOf('-');
    return lastDash >= 0 ? id.slice(lastDash + 1) : id;
  });

  readonly hasAttestationGates = computed<boolean>(() => {
    const gates = this.meta().attestationGates;
    return Array.isArray(gates) && gates.length > 0;
  });

  readonly isOnExchange = computed<boolean>(() => this.meta().visibility === 'exchange');

  readonly hasCadence = computed<boolean>(() => this.meta().cadence !== undefined);
}
