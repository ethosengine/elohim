/**
 * LearnerBackendApiService -- Thin HTTP client for learner mastery operations.
 *
 * Calls doorway `/api/v1/mastery/*` endpoints, implementing
 * ILearnerBackend. Replaces the fat LearnerBackendService
 * when the business logic lives behind the Rust API boundary.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';

import { firstValueFrom, of } from 'rxjs';

import type { ILearnerBackend } from '../interfaces/learner-backend.interface';
import type {
  ContentMasteryOutput,
  RecordEngagementInput,
  RecordAssessmentInput,
  MasterySnapshot,
  PathMasteryOverview,
  MasteryStatsWire,
  CheckPrivilegeInput,
  PrivilegeCheckResult,
} from '@app/lamad/models/content-mastery.model';
import type {
  LearnerPointBalanceOutput,
  LamadPointEventOutput,
  EarnLamadPointsInput,
  EarnLamadPointsResult,
} from '@app/lamad/models/learning-points.model';
import type {
  PracticePoolOutput,
  CreatePoolInput,
  PoolRecommendations,
  CooldownCheckResult,
  MasteryChallengeOutput,
  StartChallengeInput,
  SubmitChallengeInput,
  ChallengeResult,
} from '@app/lamad/models/practice.model';

const BASE = '/api/v1/mastery';

@Injectable({ providedIn: 'root' })
export class LearnerBackendApiService implements ILearnerBackend {
  private readonly http = inject(HttpClient);

  // ==========================================================================
  // Connection Status
  // ==========================================================================

  isAvailable(): boolean {
    return true;
  }

  // ==========================================================================
  // Content Mastery Operations
  // ==========================================================================

  async initializeMastery(contentId: string): Promise<ContentMasteryOutput | null> {
    return firstValueFrom(
      this.http.post<ContentMasteryOutput>(BASE, { contentId }).pipe(catchError(() => of(null)))
    );
  }

  async recordEngagement(input: RecordEngagementInput): Promise<ContentMasteryOutput | null> {
    return firstValueFrom(
      this.http
        .post<ContentMasteryOutput>(`${BASE}/engagement`, input)
        .pipe(catchError(() => of(null)))
    );
  }

  async recordAssessment(input: RecordAssessmentInput): Promise<ContentMasteryOutput | null> {
    return firstValueFrom(
      this.http
        .post<ContentMasteryOutput>(`${BASE}/assessment`, input)
        .pipe(catchError(() => of(null)))
    );
  }

  async getMyMastery(contentId: string): Promise<ContentMasteryOutput | null> {
    return firstValueFrom(
      this.http.get<ContentMasteryOutput>(`${BASE}/${contentId}`).pipe(catchError(() => of(null)))
    );
  }

  async getMyAllMastery(): Promise<ContentMasteryOutput[]> {
    return firstValueFrom(
      this.http.get<ContentMasteryOutput[]>(BASE).pipe(catchError(() => of([])))
    );
  }

  async getMasteryBatch(contentIds: string[]): Promise<MasterySnapshot[]> {
    return firstValueFrom(
      this.http
        .post<MasterySnapshot[]>(`${BASE}/batch`, { contentIds })
        .pipe(catchError(() => of([])))
    );
  }

  async getPathMasteryOverview(pathId: string): Promise<PathMasteryOverview | null> {
    return firstValueFrom(
      this.http.get<PathMasteryOverview>(`${BASE}/path/${pathId}`).pipe(catchError(() => of(null)))
    );
  }

  async getMyMasteryStats(): Promise<MasteryStatsWire | null> {
    return firstValueFrom(
      this.http.get<MasteryStatsWire>(`${BASE}/stats`).pipe(catchError(() => of(null)))
    );
  }

  async checkPrivilege(input: CheckPrivilegeInput): Promise<PrivilegeCheckResult | null> {
    return firstValueFrom(
      this.http
        .post<PrivilegeCheckResult>(`${BASE}/check-privilege`, input)
        .pipe(catchError(() => of(null)))
    );
  }

  // ==========================================================================
  // Practice Pool Operations
  // ==========================================================================

  async getOrCreatePracticePool(input: CreatePoolInput): Promise<PracticePoolOutput | null> {
    return firstValueFrom(
      this.http.post<PracticePoolOutput>(`${BASE}/pool`, input).pipe(catchError(() => of(null)))
    );
  }

  async refreshPracticePool(): Promise<PracticePoolOutput | null> {
    return firstValueFrom(
      this.http
        .post<PracticePoolOutput>(`${BASE}/pool/refresh`, {})
        .pipe(catchError(() => of(null)))
    );
  }

  async addPathToPool(pathId: string): Promise<PracticePoolOutput | null> {
    return firstValueFrom(
      this.http
        .post<PracticePoolOutput>(`${BASE}/pool/add-path`, { pathId })
        .pipe(catchError(() => of(null)))
    );
  }

  async getPoolRecommendations(): Promise<PoolRecommendations | null> {
    return firstValueFrom(
      this.http
        .get<PoolRecommendations>(`${BASE}/pool/recommendations`)
        .pipe(catchError(() => of(null)))
    );
  }

  async checkChallengeCooldown(): Promise<CooldownCheckResult | null> {
    return firstValueFrom(
      this.http
        .get<CooldownCheckResult>(`${BASE}/challenge/cooldown`)
        .pipe(catchError(() => of(null)))
    );
  }

  // ==========================================================================
  // Mastery Challenge Operations
  // ==========================================================================

  async startMasteryChallenge(input: StartChallengeInput): Promise<MasteryChallengeOutput | null> {
    return firstValueFrom(
      this.http
        .post<MasteryChallengeOutput>(`${BASE}/challenge/start`, input)
        .pipe(catchError(() => of(null)))
    );
  }

  async submitMasteryChallenge(input: SubmitChallengeInput): Promise<ChallengeResult | null> {
    return firstValueFrom(
      this.http
        .post<ChallengeResult>(`${BASE}/challenge/submit`, input)
        .pipe(catchError(() => of(null)))
    );
  }

  async getChallengeHistory(): Promise<MasteryChallengeOutput[]> {
    return firstValueFrom(
      this.http
        .get<MasteryChallengeOutput[]>(`${BASE}/challenge/history`)
        .pipe(catchError(() => of([])))
    );
  }

  // ==========================================================================
  // Learning Points Operations (Shefa Integration)
  // ==========================================================================

  async earnLamadPoints(input: EarnLamadPointsInput): Promise<EarnLamadPointsResult | null> {
    return firstValueFrom(
      this.http
        .post<EarnLamadPointsResult>(`${BASE}/points/earn`, input)
        .pipe(catchError(() => of(null)))
    );
  }

  async getMyLamadPointBalance(): Promise<LearnerPointBalanceOutput | null> {
    return firstValueFrom(
      this.http
        .get<LearnerPointBalanceOutput>(`${BASE}/points/balance`)
        .pipe(catchError(() => of(null)))
    );
  }

  async getMyLamadPointHistory(limit?: number): Promise<LamadPointEventOutput[]> {
    const params: Record<string, string> = {};
    if (limit !== undefined) params['limit'] = String(limit);
    return firstValueFrom(
      this.http
        .get<LamadPointEventOutput[]>(`${BASE}/points/history`, { params })
        .pipe(catchError(() => of([])))
    );
  }
}
