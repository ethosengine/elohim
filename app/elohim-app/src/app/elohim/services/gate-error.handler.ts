import { HttpErrorResponse } from '@angular/common/http';

import { Observable, throwError } from 'rxjs';

import { isGatedResponse } from '../models/gated-response.model';

import type { GateService } from './gate.service';

/**
 * Handle gate-specific HTTP errors (409 Pause, 403 Settlement).
 * Updates GateService state, then rethrows so callers can react.
 */
export function handleGateError(
  error: HttpErrorResponse,
  gateService: GateService
): Observable<never> {
  if ((error.status === 409 || error.status === 403) && isGatedResponse(error.error)) {
    gateService.handleGateResponse(error.error.gate);
  }
  return throwError(() => error);
}
