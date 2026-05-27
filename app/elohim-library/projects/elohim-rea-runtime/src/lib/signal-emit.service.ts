/**
 * SignalEmitService — POST /api/v1/signal/emit (EPR Phase 2B Batch C, Task C.3).
 *
 * Migrated to @elohim/rea-runtime from @app/shefa/services/signal-emit.service
 * as part of Wave 2 Slice 2.4 residual of the cross-pillar import cleanup sprint.
 *
 * Bridges the Angular signal harness to the Rust elohim-storage signal-emit
 * endpoint that composes an EPR Envelope, signs via the conductor, and
 * ingests through the projector. Replaces direct REST writes to legacy
 * routes (e.g. POST /api/v1/economic-events) for pillars whose write-through
 * flag is ON.
 *
 * ## Migration safety
 *
 * The endpoint may return HTTP 503 when the per-pillar write-through flag
 * is OFF (the operator has not opted shefa in yet). Callers MUST treat 503
 * as "fall back to the legacy non-EPR write path" rather than as an error.
 * This is what `tryEmit()` returns — `{ status: 'fallback', reason }` —
 * letting the harness ramp pillars at the operator's pace without code
 * changes.
 */

import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type { SignalIntent, SignalEmitSuccessResponse, SignalEmitResult } from './signal-emit-types';

@Injectable({ providedIn: 'root' })
export class SignalEmitService {
  private readonly http = inject(HttpClient);

  /**
   * Attempt to emit a signal via the EPR write-through endpoint.
   *
   * Returns:
   * - `{ status: 'emitted', response }` on HTTP 201 — the EPR was signed,
   *   ingested, and projected.
   * - `{ status: 'fallback', reason }` on HTTP 503 — write-through is OFF
   *   for this (pillar, kind); caller should use the legacy non-EPR path.
   * - `{ status: 'error', ... }` on other failures (4xx, 5xx, network).
   *
   * Callers MUST handle the `fallback` case explicitly — silently dropping
   * it would silently disable signals when an operator opts a pillar out.
   */
  async tryEmit(intent: SignalIntent): Promise<SignalEmitResult> {
    try {
      const response = await firstValueFrom(
        this.http.post<SignalEmitSuccessResponse>('/api/v1/signal/emit', intent)
      );
      return { status: 'emitted', response };
    } catch (e) {
      return mapEmitError(e);
    }
  }
}

/** Translate any thrown error from `HttpClient.post` into a `SignalEmitResult`. */
function mapEmitError(e: unknown): SignalEmitResult {
  if (!(e instanceof HttpErrorResponse)) {
    return {
      status: 'error',
      status_code: 0,
      message: e instanceof Error ? e.message : String(e),
    };
  }
  if (e.status === 503) {
    return { status: 'fallback', reason: extractErrorReason(e.error) };
  }
  return {
    status: 'error',
    status_code: e.status,
    message: typeof e.error === 'string' ? e.error : (e.message ?? 'signal emit failed'),
  };
}

/** Pull a human-readable reason out of `{ error: "..." }` JSON envelopes. */
function extractErrorReason(body: unknown): string {
  if (typeof body === 'object' && body !== null && 'error' in body) {
    return String((body as { error: unknown }).error);
  }
  return 'write-through OFF for this pillar/kind';
}
