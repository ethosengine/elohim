import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';

import { catchError, map, tap } from 'rxjs/operators';

import { Observable } from 'rxjs';

import { extractGateFromResponse } from '../models/gated-response.model';

import { handleGateError } from './gate-error.handler';
import { GateService } from './gate.service';

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
  private readonly gateService = inject(GateService);

  distribute(trigger: RecognitionTrigger): Observable<RecognitionDistributionResult> {
    return this.http.post<unknown>('/api/v1/recognition/distribute', trigger).pipe(
      tap(response => {
        const gate = extractGateFromResponse(response);
        if (gate) this.gateService.handleGateResponse(gate);
      }),
      map(
        response =>
          (response as { data: RecognitionDistributionResult }).data ??
          (response as RecognitionDistributionResult)
      ),
      catchError(error => handleGateError(error, this.gateService))
    );
  }
}
