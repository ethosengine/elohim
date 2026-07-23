/**
 * ResilienceApiService — Thin HTTP client for human resilience profiles.
 *
 * Implements IResilience. Maintains a local cache of the resilience profile
 * for synchronous accessor methods.
 *
 * MISSING BACKEND SURFACE (code-review finding, not fixed here — do NOT
 * invent a route): neither doorway nor elohim-storage expose a human-scoped
 * `/api/v1/resilience/{humanId}/profile` route. Both backends only offer a
 * CONTENT-scoped resilience surface —
 * `/api/v1/resilience/{content_id}`, `/api/v1/resilience/{content_id}/household`,
 * `/api/v1/resilience/{content_id}/hub`, `/api/v1/resilience/{content_id}/verify`
 * (elohim/elohim-storage/src/api/resilience.rs, resilience_hub.rs) — and this
 * service was previously passing a human id where a content id is expected,
 * a guaranteed 404 on every call. Until a human-scoped aggregate route is
 * designed (through `p2p-design-gate` — see
 * genesis/data/timeline/backlog/resilience-unmeasured-vs-zero-honest-denominators.md
 * for the "unmeasured ≠ zero" convention this follows), this service returns
 * a locally-synthesized, honestly-empty profile instead of issuing that
 * always-404 request.
 */

import { Injectable } from '@angular/core';

import { BehaviorSubject, Observable, Subscription, of, tap } from 'rxjs';

import type { IResilience } from '../interfaces/resilience.interface';
import type {
  ContentRiskBucket,
  ElohimResilienceAssessment,
  ProtectionStatus,
  ResilienceAction,
  ResilienceProfile,
} from '../models/resilience-profile.model';

/**
 * Builds an honestly-empty `ResilienceProfile` for a human, in place of a
 * request to a backend route that does not exist (see the class-level
 * comment). `protectionStatus: 'at-risk'` is the conservative fail-closed
 * floor value (never falsely claim `'protected'`/`'partial'` for unmeasured
 * data); `nextAction` carries the honest reason as its `description` so
 * consumers surfacing `getNextAction()` see WHY, not just a generic ask.
 */
function unavailableProfile(humanId: string): ResilienceProfile {
  return {
    humanId,
    overallScore: 0,
    protectionStatus: 'at-risk',
    shardHealth: {
      totalBlobs: 0,
      totalShards: 0,
      distinctPeers: 0,
      averageShardsPerBlob: 0,
      encodingBreakdown: { single: 0, chunked: 0, reedSolomon: 0 },
      singlePointOfFailureCount: 0,
      lastAccessVerifiedAt: '',
    },
    commitmentHealth: {
      activeCommitments: 0,
      reciprocatedCommitments: 0,
      expiringSoon: 0,
      totalPeersCommitted: 0,
      commitmentCoverage: 0,
    },
    trustCircleDepth: {
      householdPeers: 0,
      friendPeers: 0,
      communityPeers: 0,
      institutionalPeers: 0,
      totalCircles: 0,
    },
    contentRiskBreakdown: [],
    nextAction: {
      type: 'review',
      description:
        'Human-scoped resilience profile is not available: no backend route exists for ' +
        `/api/v1/resilience/${humanId}/profile — only content-scoped resilience routes ` +
        '(/api/v1/resilience/{content_id}[/household|/hub|/verify]) are served.',
      urgency: 'whenever',
    },
    lastComputedAt: new Date(0).toISOString(),
  };
}

@Injectable({ providedIn: 'root' })
export class ResilienceApiService implements IResilience {
  private readonly profile$ = new BehaviorSubject<ResilienceProfile | null>(null);
  private pollingSubscription: Subscription | null = null;

  /**
   * Compute a "fresh" resilience profile for a human and cache it.
   *
   * No backend call is made — see the class-level comment: there is no
   * human-scoped resilience route on either backend to call.
   */
  async computeProfile(humanId: string): Promise<ResilienceProfile> {
    const profile = unavailableProfile(humanId);
    this.profile$.next(profile);
    return await Promise.resolve(profile);
  }

  /**
   * Get a resilience profile as an Observable stream.
   *
   * No backend polling occurs — see the class-level comment: there is no
   * human-scoped resilience route to poll, so this emits the honestly-empty
   * profile once rather than issuing a guaranteed-404 request on an interval.
   */
  getProfile$(humanId: string, _refreshInterval = 60000): Observable<ResilienceProfile> {
    this.pollingSubscription?.unsubscribe();

    const profile$ = of(unavailableProfile(humanId)).pipe(
      tap(profile => this.profile$.next(profile))
    );

    this.pollingSubscription = profile$.subscribe();

    return profile$;
  }

  /**
   * Get current resilience profile snapshot.
   */
  getProfile(): ResilienceProfile | null {
    return this.profile$.getValue();
  }

  /**
   * Get current overall protection status.
   */
  getProtectionStatus(): ProtectionStatus | null {
    const profile = this.profile$.getValue();
    return profile ? profile.protectionStatus : null;
  }

  /**
   * Get the next recommended action to improve resilience.
   */
  getNextAction(): ResilienceAction | null {
    const profile = this.profile$.getValue();
    return profile?.nextAction ?? null;
  }

  /**
   * Get content risk breakdown by reach level.
   */
  getContentRiskBreakdown(): ContentRiskBucket[] {
    const profile = this.profile$.getValue();
    return profile ? profile.contentRiskBreakdown : [];
  }

  /**
   * Get the most recent elohim narrative assessment.
   */
  getElohimAssessment(): ElohimResilienceAssessment | null {
    const profile = this.profile$.getValue();
    return profile?.elohimAssessment ?? null;
  }
}
