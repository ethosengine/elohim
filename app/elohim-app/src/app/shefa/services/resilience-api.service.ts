/**
 * ResilienceApiService — Thin HTTP client for human resilience profiles.
 *
 * Calls doorway `/api/v1/resilience/{humanId}/profile` endpoint, implementing
 * IResilience. Maintains a local cache of the resilience profile for
 * synchronous accessor methods.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import {
  BehaviorSubject,
  Observable,
  Subscription,
  firstValueFrom,
  switchMap,
  tap,
  timer,
} from 'rxjs';

import type {
  ContentRiskBucket,
  ElohimResilienceAssessment,
  ProtectionStatus,
  ResilienceAction,
  ResilienceProfile,
} from '../models/resilience-profile.model';
import type { IResilience } from '../interfaces/resilience.interface';

@Injectable({ providedIn: 'root' })
export class ResilienceApiService implements IResilience {
  private readonly http = inject(HttpClient);
  private readonly profile$ = new BehaviorSubject<ResilienceProfile | null>(null);
  private pollingSubscription: Subscription | null = null;

  /**
   * Compute a fresh resilience profile for a human and cache it.
   */
  async computeProfile(humanId: string): Promise<ResilienceProfile> {
    const profile = await firstValueFrom(
      this.http.get<ResilienceProfile>(`/api/v1/resilience/${humanId}/profile`)
    );
    this.profile$.next(profile);
    return profile;
  }

  /**
   * Get a resilience profile as a refreshing Observable stream.
   */
  getProfile$(humanId: string, refreshInterval = 60000): Observable<ResilienceProfile> {
    this.pollingSubscription?.unsubscribe();

    const poll$ = timer(0, refreshInterval).pipe(
      switchMap(() =>
        this.http.get<ResilienceProfile>(`/api/v1/resilience/${humanId}/profile`)
      ),
      tap((profile) => this.profile$.next(profile))
    );

    this.pollingSubscription = poll$.subscribe();

    return poll$;
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
