import { inject, Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';

export interface RecognitionTrigger {
  contentId: string;
  eventType: string;
  rawAmount: number;
  triggeredBy?: string;
}

export interface StageTrace {
  stewardPresenceId: string;
  allocationRatio: number;
  storedAffinity: number;
  derivedAffinity: number;
  effectiveRatio: number;
  preLimitShare: number;
  finalShare: number;
  limitReasons: unknown[];
  economicEventId: string;
}

export interface RecognitionDistributionResult {
  contentId: string;
  triggerEventType: string;
  rawAmount: number;
  weightedAmount: number;
  distributions: StageTrace[];
  economicEventIds: string[];
  limitsApplied: unknown[];
}

@Injectable({ providedIn: 'root' })
export class RecognitionApiService {
  private readonly http = inject(HttpClient);

  distribute(trigger: RecognitionTrigger): Observable<RecognitionDistributionResult> {
    return this.http.post<RecognitionDistributionResult>(
      '/api/v1/recognition/distribute',
      trigger,
    );
  }
}
