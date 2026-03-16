/**
 * ChallengeDetailComponent - Shows a single governance challenge
 *
 * Routable component for /community/governance/challenges/:id.
 * Reads the challenge ID from the route and displays challenge details.
 */

import { Component, inject, signal, type OnInit } from '@angular/core';
import { ActivatedRoute, RouterLink } from '@angular/router';

import type { ChallengeView } from '@elohim/storage-client/generated';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

@Component({
  selector: 'qahal-challenge-detail',
  standalone: true,
  imports: [RouterLink],
  template: `
    <div class="challenge-detail">
      @if (loading()) {
        <p class="detail-status">Loading challenge...</p>
      } @else if (!challenge()) {
        <div class="detail-not-found">
          <h3>Challenge Not Found</h3>
          <a routerLink=".." class="back-link">Back to challenges</a>
        </div>
      } @else {
        <div class="detail-header">
          <a routerLink=".." class="back-link">&larr; All challenges</a>
          <h2 class="detail-title">Challenge: {{ challenge()!.groundsPrimary }}</h2>
          <span class="detail-status-badge">{{ challenge()!.state }}</span>
        </div>

        <dl class="detail-fields">
          <dt>Entity</dt>
          <dd>{{ challenge()!.entityType }} / {{ challenge()!.entityId }}</dd>

          <dt>Grounds</dt>
          <dd>{{ challenge()!.groundsPrimary }}</dd>

          <dt>Evidence</dt>
          <dd class="detail-evidence">{{ challenge()!.evidence }}</dd>

          <dt>Standing Basis</dt>
          <dd>{{ challenge()!.standingBasis }}</dd>

          @if (challenge()!.requestedOutcome) {
            <dt>Requested Outcome</dt>
            <dd>{{ challenge()!.requestedOutcome }}</dd>
          }

          <dt>Filed</dt>
          <dd>{{ challenge()!.createdAt }}</dd>
        </dl>
      }
    </div>
  `,
  styles: `
    .challenge-detail {
      padding: 1.5rem;
      max-width: 720px;
      margin: 0 auto;
    }

    .detail-status {
      color: var(--text-secondary, #999);
      font-size: 0.875rem;
    }

    .detail-not-found {
      text-align: center;
      padding: 2rem 0;
      color: var(--text-secondary, #999);
    }

    .detail-header {
      margin-bottom: 1.5rem;
    }

    .back-link {
      font-size: 0.8125rem;
      color: var(--primary, #6366f1);
      text-decoration: none;
    }

    .back-link:hover {
      text-decoration: underline;
    }

    .detail-title {
      margin: 0.5rem 0 0.25rem;
      font-size: 1.25rem;
      font-weight: 600;
      color: var(--text-primary, #e2e8f0);
      text-transform: capitalize;
    }

    .detail-status-badge {
      display: inline-block;
      padding: 0.125rem 0.5rem;
      border-radius: 4px;
      font-size: 0.75rem;
      font-weight: 500;
      background: var(--surface-elevated, rgba(99, 102, 241, 0.1));
      color: var(--primary, #6366f1);
      text-transform: capitalize;
    }

    .detail-fields {
      display: grid;
      grid-template-columns: 140px 1fr;
      gap: 0.5rem 1rem;
      font-size: 0.875rem;
    }

    .detail-fields dt {
      font-weight: 500;
      color: var(--text-secondary, #64748b);
    }

    .detail-fields dd {
      margin: 0;
      color: var(--text-primary, #e2e8f0);
    }

    .detail-evidence {
      white-space: pre-wrap;
      line-height: 1.5;
    }
  `,
})
export class ChallengeDetailComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly governanceApi = inject(GovernanceApiService);

  readonly challenge = signal<ChallengeView | null>(null);
  readonly loading = signal(true);

  async ngOnInit(): Promise<void> {
    const id = this.route.snapshot.paramMap.get('id');
    if (!id) {
      this.loading.set(false);
      return;
    }

    try {
      const result = await this.governanceApi.getChallenge(id);
      this.challenge.set(result);
    } catch {
      // null challenge triggers not-found UI
    } finally {
      this.loading.set(false);
    }
  }
}
