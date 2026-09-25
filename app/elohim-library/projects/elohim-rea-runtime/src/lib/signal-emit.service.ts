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
 * The node declares, per (pillar, kind), whether write-through is on
 * (`GET /api/v1/status/write-through`). The service reads that once per
 * session (memoised; a failed read means nothing is on) and posts only for an
 * effective-on pair or an integrity kind. Any other signal returns
 * `{ status: 'fallback', reason }` without a request — a routine 503 per page
 * view is a browser console error, and the node already gave the answer
 * (ruling R-A16). A 503 on an actual post (the status went stale) still maps
 * to `fallback`. Callers MUST treat `fallback` as "use the legacy non-EPR
 * write path", so pillars ramp at the operator's pace without code changes.
 */

import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type {
  SignalIntent,
  SignalEmitSuccessResponse,
  SignalEmitResult,
  WriteThroughStatusView,
} from './signal-emit-types';

/** The node's effective write-through flags, read once per session. */
export const WRITE_THROUGH_STATUS_URL = '/api/v1/status/write-through';

@Injectable({ providedIn: 'root' })
export class SignalEmitService {
  private readonly http = inject(HttpClient);
  /** Memoised status read; `null` when it could not be read (nothing is on). */
  private statusRead: Promise<WriteThroughStatusView | null> | null = null;

  /**
   * Attempt to emit a signal via the EPR write-through endpoint.
   *
   * Returns:
   * - `{ status: 'emitted', response }` on HTTP 201 — the EPR was signed,
   *   ingested, and projected.
   * - `{ status: 'fallback', reason }` when the node's status does not show
   *   this (pillar, kind) on — no request is made — or on HTTP 503 from an
   *   actual post; caller should use the legacy non-EPR path.
   * - `{ status: 'error', ... }` on other failures (4xx, 5xx, network).
   *
   * Callers MUST handle the `fallback` case explicitly — silently dropping
   * it would silently disable signals when an operator opts a pillar out.
   */
  async tryEmit(intent: SignalIntent): Promise<SignalEmitResult> {
    const status = await this.writeThroughStatus();
    if (!isWriteThroughOn(status, intent.pillar, intent.signalType)) {
      return {
        status: 'fallback',
        reason: status
          ? `write-through is not on for pillar=${intent.pillar} kind=${intent.signalType}`
          : 'write-through status unreadable; treated as off',
      };
    }
    try {
      const response = await firstValueFrom(
        this.http.post<SignalEmitSuccessResponse>('/api/v1/signal/emit', intent)
      );
      return { status: 'emitted', response };
    } catch (e) {
      return mapEmitError(e);
    }
  }

  /** The session's one status read; a failed read resolves to `null`. */
  private writeThroughStatus(): Promise<WriteThroughStatusView | null> {
    this.statusRead ??= firstValueFrom(
      this.http.get<WriteThroughStatusView>(WRITE_THROUGH_STATUS_URL)
    ).catch(() => null);
    return this.statusRead;
  }
}

/** Effective-on for (pillar, kind): an integrity kind, or a row saying `on`. */
function isWriteThroughOn(
  status: WriteThroughStatusView | null,
  pillar: string,
  kind: string
): boolean {
  if (!status) return false;
  if (status.integrityKinds?.includes(kind)) return true;
  return (status.effective ?? []).some(row => row.pillar === pillar && row.kind === kind && row.on);
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
