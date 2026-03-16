import { Component, ChangeDetectionStrategy, EventEmitter, Output, input } from '@angular/core';

import type { RoutingSuggestion } from '../../models/journal-routing.model';

const REACH_ICONS: Record<string, string> = {
  private: '\u{1F512}',
  close: '\u{1F464}',
  community: '\u{1F465}',
  network: '\u{1F310}',
  constitutional: '\u2696\uFE0F',
};

const REACH_LABELS: Record<string, string> = {
  private: 'Private',
  close: 'Close',
  community: 'Community',
  network: 'Network',
  constitutional: 'Constitutional',
};

@Component({
  selector: 'app-journal-routing-cards',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="collapsed-preview" data-testid="collapsed-text">
      <div class="preview-title">{{ previewTitle() }}</div>
      <div class="preview-body">{{ previewBody() }}</div>
      <button
        class="btn-edit"
        data-testid="routing-edit-btn"
        (click)="editRequested.emit()"
      >
        Edit
      </button>
    </div>

    @for (card of suggestions(); track card.id) {
      @if (card.status !== 'dismissed') {
        <div
          class="routing-card"
          [class.filing-card]="card.kind === 'filing'"
          [class.derivative-card]="card.kind === 'derivative'"
          [class.posted-card]="card.status === 'posted'"
          data-testid="routing-card"
        >
          <div class="card-header">
            <span class="card-title" data-testid="card-title">{{ card.title }}</span>
            <span class="reach-badge" data-testid="card-reach">
              {{ reachIcon(card.reach) }} {{ reachLabel(card.reach) }}
            </span>
          </div>
          <p class="card-summary">{{ card.summary }}</p>

          @switch (card.status) {
            @case ('suggested') {
              <div class="card-actions">
                <button
                  class="btn-post"
                  data-testid="card-post-btn"
                  (click)="postCard.emit(card.id)"
                >
                  Post
                </button>
                <button
                  class="btn-dismiss"
                  data-testid="card-dismiss-btn"
                  (click)="dismissCard.emit(card.id)"
                >
                  Dismiss
                </button>
              </div>
            }
            @case ('posting') {
              <div class="card-actions">
                <button class="btn-post" disabled>Posting...</button>
              </div>
            }
            @case ('posted') {
              <span class="posted-badge" data-testid="card-posted-badge">Posted</span>
            }
          }
        </div>
      }
    }
  `,
  styles: [
    `
      :host {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        max-width: 560px;
        margin: 0 auto;
      }

      .collapsed-preview {
        padding: 1rem;
        background: var(--surface-secondary, #f8f9fa);
        border-radius: var(--radius-md, 8px);
        border: 1px solid var(--border-color, #e9ecef);
        position: relative;
      }

      .preview-title {
        font-weight: 600;
        font-size: 0.9375rem;
        color: var(--text-primary, #202124);
        margin-bottom: 0.25rem;
      }

      .preview-body {
        font-size: 0.8125rem;
        color: var(--text-secondary, #5f6368);
        line-height: 1.4;
        white-space: pre-line;
        overflow: hidden;
        display: -webkit-box;
        -webkit-line-clamp: 3;
        -webkit-box-orient: vertical;
      }

      .btn-edit {
        position: absolute;
        top: 0.75rem;
        right: 0.75rem;
        padding: 0.25rem 0.75rem;
        font-size: 0.8125rem;
        font-weight: 500;
        color: var(--primary, #4285f4);
        background: transparent;
        border: 1px solid var(--primary, #4285f4);
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .btn-edit:hover {
        background: var(--primary, #4285f4);
        color: white;
      }

      .routing-card {
        padding: 1rem 1.25rem;
        border-radius: var(--radius-md, 8px);
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      .filing-card {
        background: var(--surface-secondary, #f8f9fa);
        border: 1px solid var(--border-color, #e9ecef);
        font-size: 0.875rem;
      }

      .derivative-card {
        background: var(--surface-elevated, #fff);
        border: 2px solid var(--border-color, #e9ecef);
      }

      .posted-card {
        opacity: 0.7;
      }

      .card-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 0.5rem;
      }

      .card-title {
        font-weight: 600;
        color: var(--text-primary, #202124);
      }

      .reach-badge {
        display: inline-flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.125rem 0.5rem;
        font-size: 0.75rem;
        font-weight: 500;
        color: var(--text-secondary, #5f6368);
        background: var(--surface-secondary, #f8f9fa);
        border-radius: var(--radius-sm, 4px);
        white-space: nowrap;
      }

      .card-summary {
        margin: 0;
        font-size: 0.875rem;
        color: var(--text-secondary, #5f6368);
        line-height: 1.4;
      }

      .card-actions {
        display: flex;
        gap: 0.5rem;
        align-items: center;
        margin-top: 0.25rem;
      }

      .btn-post {
        padding: 0.375rem 1rem;
        font-size: 0.875rem;
        font-weight: 600;
        color: white;
        background: var(--primary, #4285f4);
        border: none;
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .btn-post:hover:not(:disabled) {
        background: var(--primary-dark, #1a73e8);
      }

      .btn-post:disabled {
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-disabled, #9aa0a6);
        cursor: not-allowed;
      }

      .btn-dismiss {
        padding: 0.375rem 1rem;
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-secondary, #5f6368);
        background: transparent;
        border: 1px solid var(--border-color, #e9ecef);
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .btn-dismiss:hover {
        background: var(--surface-secondary, #f8f9fa);
      }

      .posted-badge {
        display: inline-flex;
        align-self: flex-start;
        padding: 0.25rem 0.625rem;
        font-size: 0.8125rem;
        font-weight: 500;
        color: var(--success, #34a853);
        background: var(--success-surface, #e6f4ea);
        border-radius: var(--radius-sm, 4px);
      }
    `,
  ],
})
export class JournalRoutingCardsComponent {
  readonly suggestions = input.required<RoutingSuggestion[]>();
  readonly journalText = input.required<string>();

  @Output() readonly postCard = new EventEmitter<string>();
  @Output() readonly dismissCard = new EventEmitter<string>();
  @Output() readonly editRequested = new EventEmitter<void>();

  previewTitle(): string {
    const text = this.journalText();
    const firstLine = text.split('\n')[0] ?? '';
    return firstLine.trim();
  }

  previewBody(): string {
    const text = this.journalText();
    const lines = text.split('\n');
    return lines.slice(0, 3).join('\n');
  }

  reachIcon(reach: string): string {
    return REACH_ICONS[reach] ?? '';
  }

  reachLabel(reach: string): string {
    return REACH_LABELS[reach] ?? reach;
  }
}
