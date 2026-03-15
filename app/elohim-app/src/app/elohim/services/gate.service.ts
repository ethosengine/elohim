import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal, computed } from '@angular/core';

import { Observable, tap } from 'rxjs';

import type { GateEvaluationView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class GateService {
  private readonly http = inject(HttpClient);
  private readonly _latestEvaluation = signal<GateEvaluationView | null>(null);

  readonly latestEvaluation = this._latestEvaluation.asReadonly();
  readonly isPaused = computed(() => {
    const prompt = this._latestEvaluation()?.pausePrompt;
    return prompt !== null && prompt !== undefined;
  });
  readonly isSettled = computed(() => {
    const boundary = this._latestEvaluation()?.settlementBoundary;
    return boundary !== null && boundary !== undefined;
  });
  readonly trustContext = computed(() => this._latestEvaluation()?.trustContext ?? null);

  handleGateResponse(gate: GateEvaluationView): void {
    this._latestEvaluation.set(gate);
  }

  confirmPause(confirmToken: string): Observable<unknown> {
    return this.http
      .post('/api/v1/gate/confirm', { confirmToken })
      .pipe(tap(() => this._latestEvaluation.set(null)));
  }

  clearState(): void {
    this._latestEvaluation.set(null);
  }
}
