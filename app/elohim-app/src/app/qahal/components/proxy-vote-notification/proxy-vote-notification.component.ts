/**
 * ProxyVoteNotificationComponent — Card notifying a human that their
 * elohim cast a proxy vote on their behalf.
 *
 * Shows the proposal title, the proxy justification (reasoning), and
 * two action buttons: confirm or request override. The parent component
 * handles navigation to the override flow.
 */

import { Component, input, output } from '@angular/core';

import type { RankedVoteView } from '@elohim/storage-client/generated';

@Component({
  selector: 'qahal-proxy-vote-notification',
  standalone: true,
  template: `
    <div class="proxy-notification-card">
      <p class="notification-heading">Your elohim voted on '{{ proposalTitle() }}'</p>

      @if (proxyVote().reasoning) {
        <p class="justification">{{ proxyVote().reasoning }}</p>
      }

      <div class="notification-actions">
        <button class="btn btn-confirm" data-testid="proxy-confirm-btn" (click)="confirmed.emit()">
          Looks right
        </button>
        <button
          class="btn btn-override"
          data-testid="proxy-override-btn"
          (click)="overrideRequested.emit(proxyVote())"
        >
          I disagree
        </button>
      </div>
    </div>
  `,
  styles: `
    .proxy-notification-card {
      padding: 1rem 1.25rem;
      border: 1px solid var(--border-color, #e0e0e0);
      border-radius: 0.5rem;
      background: var(--surface-color, #fff);
    }

    .notification-heading {
      margin: 0 0 0.5rem;
      font-size: 0.9375rem;
      font-weight: 600;
      line-height: 1.4;
    }

    .justification {
      margin: 0 0 0.75rem;
      font-size: 0.8125rem;
      color: var(--text-secondary, #666);
      line-height: 1.5;
      font-style: italic;
    }

    .notification-actions {
      display: flex;
      gap: 0.5rem;
    }

    .btn {
      padding: 0.375rem 0.875rem;
      border: none;
      border-radius: 0.375rem;
      font-size: 0.8125rem;
      font-weight: 500;
      cursor: pointer;
      transition: opacity 0.15s ease;
    }

    .btn:hover {
      opacity: 0.9;
    }

    .btn-confirm {
      background: var(--settled-bg, #d1fae5);
      color: var(--settled-fg, #065f46);
    }

    .btn-override {
      background: var(--surface-variant, #e0e0e0);
      color: var(--text-primary, #333);
    }

    @media (prefers-color-scheme: dark) {
      .proxy-notification-card {
        background: var(--surface-color, #1a1a1a);
        border-color: var(--border-color, #333);
      }

      .btn-confirm {
        background: var(--settled-bg, #14432a);
        color: var(--settled-fg, #6ee7b7);
      }

      .btn-override {
        background: var(--surface-variant, #333);
        color: var(--text-primary, #e0e0e0);
      }
    }
  `,
})
export class ProxyVoteNotificationComponent {
  readonly proxyVote = input.required<RankedVoteView>();
  readonly proposalTitle = input<string>('');

  readonly confirmed = output<void>();
  readonly overrideRequested = output<RankedVoteView>();
}
