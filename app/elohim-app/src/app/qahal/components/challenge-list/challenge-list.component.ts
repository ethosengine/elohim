/**
 * ChallengeListComponent - Lists governance challenges
 *
 * Routable component for /community/governance/challenges.
 * Displays all challenges, with future sprint work for filtering and sorting.
 */

import { Component, inject, signal, type OnInit } from '@angular/core';
import { RouterLink } from '@angular/router';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

import type { ChallengeView } from '@elohim/storage-client/generated';

@Component({
  selector: 'qahal-challenge-list',
  standalone: true,
  imports: [RouterLink],
  template: `
    <div class="challenge-list">
      <h2 class="list-title">Governance Challenges</h2>

      @if (loading()) {
        <p class="list-status">Loading challenges...</p>
      } @else if (challenges().length === 0) {
        <p class="list-status">No challenges have been filed yet.</p>
      } @else {
        <ul class="challenges">
          @for (challenge of challenges(); track challenge.id) {
            <li class="challenge-item">
              <a [routerLink]="challenge.id" class="challenge-link">
                <span class="challenge-grounds">{{ challenge.groundsPrimary }}</span>
                <span class="challenge-entity">
                  {{ challenge.entityType }} &middot; {{ challenge.state }}
                </span>
              </a>
            </li>
          }
        </ul>
      }
    </div>
  `,
  styles: `
    .challenge-list {
      padding: 1.5rem;
    }

    .list-title {
      margin: 0 0 1rem;
      font-size: 1.25rem;
      font-weight: 600;
      color: var(--text-primary, #e2e8f0);
    }

    .list-status {
      color: var(--text-secondary, #999);
      font-size: 0.875rem;
    }

    .challenges {
      list-style: none;
      padding: 0;
      margin: 0;
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    .challenge-item {
      border: 1px solid var(--border, rgba(99, 102, 241, 0.15));
      border-radius: 8px;
      overflow: hidden;
    }

    .challenge-link {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
      padding: 0.75rem 1rem;
      text-decoration: none;
      color: inherit;
      transition: background 0.15s;
    }

    .challenge-link:hover {
      background: var(--surface-elevated, rgba(99, 102, 241, 0.05));
    }

    .challenge-grounds {
      font-weight: 500;
      font-size: 0.875rem;
      color: var(--text-primary, #e2e8f0);
      text-transform: capitalize;
    }

    .challenge-entity {
      font-size: 0.75rem;
      color: var(--text-secondary, #64748b);
    }
  `,
})
export class ChallengeListComponent implements OnInit {
  private readonly governanceApi = inject(GovernanceApiService);

  readonly challenges = signal<ChallengeView[]>([]);
  readonly loading = signal(true);

  async ngOnInit(): Promise<void> {
    try {
      // Load all challenges (empty contentId returns all)
      const results = await this.governanceApi.queryChallenges('');
      this.challenges.set(results);
    } catch {
      // Silently handle — empty list shown
    } finally {
      this.loading.set(false);
    }
  }
}
