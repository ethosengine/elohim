/**
 * Revocation Service — wraps M4 revocation primitives.
 *
 * POST /api/v1/account/self-revocation          → selfRevoke()
 * POST /api/v1/account/recovery/:id/vote        → voteOnRecovery()
 *
 * Both endpoints return 503 PHASE_11_PENDING in M5. The service surfaces
 * a structured error signal rather than throwing, so the UI can still render
 * the revocation history (read side) while mutations are pending.
 *
 * GET /api/v1/account/revocations               → listRevocations()
 * GET /api/v1/account/pending-recovery          → listPendingRecovery()
 */

import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type { KeyRevocationView, RecoveryRequestView } from '../models/account.model';

/** M5 stub notice surfaced to the UI when a write endpoint returns 503. */
const PHASE_11_PENDING_MSG = 'M5 stub — revocation mutations are Phase 11 pending.';

@Injectable({ providedIn: 'root' })
export class RevocationService {
  private readonly http = inject(HttpClient);

  private readonly revocationsSignal = signal<KeyRevocationView[]>([]);
  private readonly pendingRecoverySignal = signal<RecoveryRequestView[]>([]);
  private readonly errorSignal = signal<string | null>(null);

  // ---------------------------------------------------------------------------
  // Public signals
  // ---------------------------------------------------------------------------

  readonly revocations = this.revocationsSignal.asReadonly();
  readonly pendingRecovery = this.pendingRecoverySignal.asReadonly();
  readonly error = this.errorSignal.asReadonly();

  // ---------------------------------------------------------------------------
  // Read
  // ---------------------------------------------------------------------------

  async listRevocations(): Promise<void> {
    this.errorSignal.set(null);
    try {
      const data = await firstValueFrom(
        this.http.get<KeyRevocationView[]>('/api/v1/account/revocations')
      );
      this.revocationsSignal.set(data);
    } catch (e) {
      this.errorSignal.set(toMessage(e));
    }
  }

  async listPendingRecovery(): Promise<void> {
    this.errorSignal.set(null);
    try {
      const data = await firstValueFrom(
        this.http.get<RecoveryRequestView[]>('/api/v1/account/pending-recovery')
      );
      this.pendingRecoverySignal.set(data);
    } catch (e) {
      this.errorSignal.set(toMessage(e));
    }
  }

  // ---------------------------------------------------------------------------
  // Write (Phase 11 pending — surface 503 gracefully)
  // ---------------------------------------------------------------------------

  /**
   * Initiate self-revocation of the given agent public key.
   * Returns true on success, false when the endpoint returns 503 (Phase 11 pending).
   */
  async selfRevoke(revokedPubKey: string): Promise<boolean> {
    this.errorSignal.set(null);
    try {
      await firstValueFrom(this.http.post('/api/v1/account/self-revocation', { revokedPubKey }));
      return true;
    } catch (e) {
      this.errorSignal.set(isPhase11Pending(e) ? PHASE_11_PENDING_MSG : toMessage(e));
      return false;
    }
  }

  /**
   * Cast a vote on a pending recovery request.
   * Returns true on success, false when the endpoint returns 503 (Phase 11 pending).
   */
  async voteOnRecovery(
    recoveryRequestId: string,
    decision: 'approve' | 'reject'
  ): Promise<boolean> {
    this.errorSignal.set(null);
    try {
      await firstValueFrom(
        this.http.post(`/api/v1/account/recovery/${recoveryRequestId}/vote`, { decision })
      );
      return true;
    } catch (e) {
      this.errorSignal.set(isPhase11Pending(e) ? PHASE_11_PENDING_MSG : toMessage(e));
      return false;
    }
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function isPhase11Pending(e: unknown): boolean {
  return e instanceof HttpErrorResponse && e.status === 503;
}

function toMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
