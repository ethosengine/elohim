/**
 * StewardshipApiService — Thin HTTP client for graduated capability management.
 *
 * Calls doorway `/api/v1/stewardship/*` endpoints, implementing IStewardshipPolicy.
 * Replaces the fat StewardshipService (Holochain zome calls) when the business
 * logic lives behind the Rust API boundary.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError, map } from 'rxjs/operators';

import { firstValueFrom, of } from 'rxjs';

import type { IStewardshipPolicy } from '../interfaces/stewardship-policy.interface';
import type {
  ComputedPolicy,
  CreateGrantInput,
  DelegateGrantInput,
  FileAppealInput,
  PolicyDecision,
  StewardshipAppeal,
  StewardshipGrant,
} from '../models/stewardship.model';

@Injectable({ providedIn: 'root' })
export class StewardshipApiService implements IStewardshipPolicy {
  private readonly http = inject(HttpClient);

  async checkContentAccess(
    contentHash: string,
    categories: string[],
    ageRating?: string,
    reachLevel?: number
  ): Promise<PolicyDecision> {
    return firstValueFrom(
      this.http
        .post<PolicyDecision>(`/api/v1/stewardship/access/content/${contentHash}`, {
          categories,
          ageRating,
          reachLevel,
        })
        .pipe(catchError(() => of({ type: 'allow' as const })))
    );
  }

  async checkFeatureAccess(feature: string): Promise<boolean> {
    return firstValueFrom(
      this.http.get<{ allowed: boolean }>(`/api/v1/stewardship/access/feature/${feature}`).pipe(
        map(res => res.allowed),
        catchError(() => of(true))
      )
    );
  }

  async checkRouteAccess(route: string): Promise<boolean> {
    return firstValueFrom(
      this.http
        .get<{ allowed: boolean }>(`/api/v1/stewardship/access/route/${encodeURIComponent(route)}`)
        .pipe(
          map(res => res.allowed),
          catchError(() => of(true))
        )
    );
  }

  async getMyPolicy(): Promise<ComputedPolicy | null> {
    return firstValueFrom(
      this.http.get<ComputedPolicy>('/api/v1/stewardship/policy').pipe(catchError(() => of(null)))
    );
  }

  async createGrant(input: CreateGrantInput): Promise<StewardshipGrant | null> {
    return firstValueFrom(
      this.http
        .post<StewardshipGrant>('/api/v1/stewardship/grants', input)
        .pipe(catchError(() => of(null)))
    );
  }

  async delegateGrant(input: DelegateGrantInput): Promise<StewardshipGrant | null> {
    return firstValueFrom(
      this.http
        .post<StewardshipGrant>(`/api/v1/stewardship/grants/${input.parentGrantId}/delegate`, input)
        .pipe(catchError(() => of(null)))
    );
  }

  async revokeGrant(grantId: string): Promise<boolean> {
    return firstValueFrom(
      this.http.delete(`/api/v1/stewardship/grants/${grantId}`).pipe(
        map(() => true),
        catchError(() => of(false))
      )
    );
  }

  async getMySubjects(): Promise<StewardshipGrant[]> {
    return firstValueFrom(
      this.http
        .get<StewardshipGrant[]>('/api/v1/stewardship/subjects')
        .pipe(catchError(() => of([])))
    );
  }

  async getMyStewards(): Promise<StewardshipGrant[]> {
    return firstValueFrom(
      this.http
        .get<StewardshipGrant[]>('/api/v1/stewardship/stewards')
        .pipe(catchError(() => of([])))
    );
  }

  async fileAppeal(input: FileAppealInput): Promise<StewardshipAppeal | null> {
    return firstValueFrom(
      this.http
        .post<StewardshipAppeal>('/api/v1/stewardship/appeals', input)
        .pipe(catchError(() => of(null)))
    );
  }

  async getMyAppeals(): Promise<StewardshipAppeal[]> {
    return firstValueFrom(
      this.http
        .get<StewardshipAppeal[]>('/api/v1/stewardship/appeals')
        .pipe(catchError(() => of([])))
    );
  }
}
