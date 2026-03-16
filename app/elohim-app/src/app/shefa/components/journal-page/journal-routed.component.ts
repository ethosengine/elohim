import { Component, ChangeDetectionStrategy, input, computed, output } from '@angular/core';

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
  selector: 'app-journal-routed',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="routed-container">
      <p class="routed-message" data-testid="routed-message">Your words are where they belong.</p>

      <ul class="routed-list">
        @for (card of postedCards(); track card.id) {
          <li class="routed-card">
            <span class="routed-check" aria-hidden="true">&#10003;</span>
            <div class="routed-card-body">
              <span class="routed-title">{{ card.title }}</span>
              <span class="routed-summary">{{ card.summary }}</span>
            </div>
            <span class="routed-reach" data-testid="routed-reach">
              {{ reachIcon(card.reach) }} {{ reachLabel(card.reach) }}
            </span>
          </li>
        }
      </ul>

      <button
        class="write-another-btn"
        data-testid="write-another-btn"
        (click)="writeAnother.emit()"
      >
        Write another
      </button>
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .routed-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 2rem 1rem;
        gap: 1.5rem;
      }

      .routed-message {
        font-size: 1.25rem;
        font-weight: 500;
        color: var(--text-primary, #202124);
        text-align: center;
        margin: 0;
      }

      .routed-list {
        list-style: none;
        margin: 0;
        padding: 0;
        width: 100%;
        max-width: 480px;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
      }

      .routed-card {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.75rem 1rem;
        border: 1px solid var(--border-color, #e9ecef);
        border-radius: 8px;
        background: var(--surface-elevated, #fff);
      }

      .routed-check {
        color: #34a853;
        font-size: 1.25rem;
        flex-shrink: 0;
      }

      .routed-card-body {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-width: 0;
      }

      .routed-title {
        font-weight: 500;
        font-size: 0.9375rem;
        color: var(--text-primary, #202124);
      }

      .routed-summary {
        font-size: 0.8125rem;
        color: var(--text-secondary, #5f6368);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .routed-reach {
        flex-shrink: 0;
        font-size: 0.75rem;
        color: var(--text-secondary, #5f6368);
        white-space: nowrap;
      }

      .write-another-btn {
        padding: 0.625rem 1.5rem;
        border: none;
        border-radius: 6px;
        background: var(--primary, #4285f4);
        color: #fff;
        font-size: 0.9375rem;
        font-weight: 500;
        cursor: pointer;
      }

      .write-another-btn:hover {
        background: var(--primary-dark, #3367d6);
      }
    `,
  ],
})
export class JournalRoutedComponent {
  readonly suggestions = input.required<RoutingSuggestion[]>();
  readonly writeAnother = output<void>();

  readonly postedCards = computed(() => this.suggestions().filter(s => s.status === 'posted'));

  reachIcon(reach: string): string {
    return REACH_ICONS[reach] ?? '';
  }

  reachLabel(reach: string): string {
    return REACH_LABELS[reach] ?? reach;
  }
}
