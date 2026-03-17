/**
 * ProxyVoteListComponent — Lists pending proxy vote notifications.
 *
 * Loads proxy votes cast by elohim agents on behalf of the current user
 * and renders a ProxyVoteNotificationComponent card for each. Confirmed
 * votes are dismissed; override requests navigate to the proposal voting
 * UI so the human can cast their own vote.
 */

import { Component, inject, OnInit, signal } from '@angular/core';
import { Router } from '@angular/router';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';
import type { RankedVoteView } from '@elohim/storage-client/generated';

import { ProxyVoteNotificationComponent } from '../proxy-vote-notification/proxy-vote-notification.component';

/** A proxy vote paired with its proposal title for display. */
interface ProxyVoteEntry {
  vote: RankedVoteView;
  proposalTitle: string;
}

@Component({
  selector: 'qahal-proxy-vote-list',
  standalone: true,
  imports: [ProxyVoteNotificationComponent],
  template: `
    <div class="proxy-vote-list">
      <h2>Proxy Vote Notifications</h2>

      @if (loading()) {
        <p class="status-text">Loading proxy votes...</p>
      } @else if (entries().length === 0) {
        <p class="status-text">No pending proxy votes.</p>
      } @else {
        <div class="vote-cards">
          @for (entry of entries(); track entry.vote.id) {
            <qahal-proxy-vote-notification
              [proxyVote]="entry.vote"
              [proposalTitle]="entry.proposalTitle"
              (confirmed)="dismiss(entry.vote.id)"
              (overrideRequested)="navigateToProposal($event)" />
          }
        </div>
      }
    </div>
  `,
  styles: `
    .proxy-vote-list {
      max-width: 600px;
      margin: 0 auto;
      padding: 1.5rem 1rem;
    }

    h2 {
      margin: 0 0 1.25rem;
      font-size: 1.375rem;
      font-weight: 600;
    }

    .status-text {
      color: var(--text-secondary, #666);
      font-size: 0.875rem;
    }

    .vote-cards {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
    }
  `,
})
export class ProxyVoteListComponent implements OnInit {
  private readonly governanceApi = inject(GovernanceApiService);
  private readonly router = inject(Router);

  readonly loading = signal(true);
  readonly entries = signal<ProxyVoteEntry[]>([]);

  async ngOnInit(): Promise<void> {
    this.loading.set(true);
    try {
      // Load proxy votes for the current user.
      // In production this would query by the authenticated humanId;
      // for now we use a placeholder that the backend can resolve.
      const votes = await this.governanceApi.getProxyVotes('current-user');
      const proxyEntries: ProxyVoteEntry[] = votes
        .filter(v => v.proxyElohimId != null)
        .map(v => ({
          vote: v,
          proposalTitle: v.proposalId,
        }));
      this.entries.set(proxyEntries);
    } finally {
      this.loading.set(false);
    }
  }

  /** Remove a confirmed proxy vote from the visible list. */
  dismiss(voteId: string): void {
    this.entries.update(list => list.filter(e => e.vote.id !== voteId));
  }

  /** Navigate to the proposal so the human can override the proxy vote. */
  navigateToProposal(vote: RankedVoteView): void {
    void this.router.navigate(['/community', 'governance', 'challenges', vote.proposalId]);
  }
}
