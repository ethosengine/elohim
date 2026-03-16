/**
 * ChallengeRouteComponent - Route wrapper for FileChallengeComponent
 *
 * Reads entityType and entityId from query params and passes them to
 * the FileChallengeComponent. Used for standalone route access at
 * /community/governance/challenges/new?entityType=content&entityId=abc
 *
 * On successful filing, navigates to the challenge detail page.
 */

import { Component, computed, inject } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { toSignal } from '@angular/core/rxjs-interop';

import type { ChallengeView } from '@elohim/storage-client/generated';

import { FileChallengeComponent } from '../file-challenge/file-challenge.component';

@Component({
  selector: 'qahal-challenge-route',
  standalone: true,
  imports: [FileChallengeComponent],
  template: `
    @if (entityType() && entityId()) {
      <div class="challenge-route-container">
        <qahal-file-challenge
          [entityType]="entityType()!"
          [entityId]="entityId()!"
          (challengeFiled)="onChallengeFiled($event)" />
      </div>
    } @else {
      <div class="challenge-route-error">
        <h3>Missing Parameters</h3>
        <p>
          A challenge requires an entity to challenge.
          Please use the context menu on content to file a challenge.
        </p>
      </div>
    }
  `,
  styles: `
    .challenge-route-container {
      max-width: 640px;
      margin: 2rem auto;
      background: var(--surface, #fff);
      border-radius: 12px;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    }

    .challenge-route-error {
      max-width: 640px;
      margin: 2rem auto;
      padding: 2rem;
      text-align: center;
      color: var(--text-secondary, #666);
    }

    .challenge-route-error h3 {
      margin: 0 0 0.5rem;
      color: var(--text-primary, #333);
    }

    @media (prefers-color-scheme: dark) {
      .challenge-route-container {
        background: var(--surface, #1a1a1a);
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
      }
    }
  `,
})
export class ChallengeRouteComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);

  private readonly queryParams = toSignal(this.route.queryParamMap);

  readonly entityType = computed(() => this.queryParams()?.get('entityType') ?? '');
  readonly entityId = computed(() => this.queryParams()?.get('entityId') ?? '');

  onChallengeFiled(challenge: ChallengeView): void {
    this.router.navigate(['..', challenge.id], {
      relativeTo: this.route,
    });
  }
}
