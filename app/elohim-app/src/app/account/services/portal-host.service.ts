/**
 * Portal Host Service — wraps portal-host CRUD endpoints.
 *
 * GET  /api/v1/account/portal-hosts  → list()
 * POST /api/v1/account/portal-hosts  → add()    (503 PHASE_11_PENDING — handled gracefully)
 * DELETE /api/v1/account/portal-hosts/:url_b64 → remove() (503 PHASE_11_PENDING)
 *
 * On 503 responses the service surfaces an error signal but does not throw,
 * so the UI can still render the current (read-only) host list.
 */

import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject, signal, computed } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type { PortalHostView, AddPortalHostInputView } from '../models/portal-host.model';

/** M5 stub notice surfaced to the UI when a write endpoint returns 503. */
const PHASE_11_PENDING_MSG = 'M5 stub — portal host mutations are Phase 11 pending.';

@Injectable({ providedIn: 'root' })
export class PortalHostService {
  private readonly http = inject(HttpClient);

  private readonly hostsSignal = signal<PortalHostView[]>([]);
  private readonly errorSignal = signal<string | null>(null);
  private readonly loadingSignal = signal(false);

  // ---------------------------------------------------------------------------
  // Public signals
  // ---------------------------------------------------------------------------

  readonly hosts = this.hostsSignal.asReadonly();
  readonly error = this.errorSignal.asReadonly();
  readonly loading = this.loadingSignal.asReadonly();
  readonly count = computed(() => this.hostsSignal().length);

  // ---------------------------------------------------------------------------
  // Read
  // ---------------------------------------------------------------------------

  async list(): Promise<void> {
    this.loadingSignal.set(true);
    this.errorSignal.set(null);
    try {
      const hosts = await firstValueFrom(
        this.http.get<PortalHostView[]>('/api/v1/account/portal-hosts')
      );
      this.hostsSignal.set(hosts);
    } catch (e) {
      this.errorSignal.set(e instanceof Error ? e.message : String(e));
    } finally {
      this.loadingSignal.set(false);
    }
  }

  // ---------------------------------------------------------------------------
  // Write (Phase 11 pending — surface 503 gracefully)
  // ---------------------------------------------------------------------------

  async add(input: AddPortalHostInputView): Promise<PortalHostView | null> {
    this.errorSignal.set(null);
    try {
      const created = await firstValueFrom(
        this.http.post<PortalHostView>('/api/v1/account/portal-hosts', input)
      );
      this.hostsSignal.update(hs => [created, ...hs]);
      return created;
    } catch (e) {
      this.errorSignal.set(isPhase11Pending(e) ? PHASE_11_PENDING_MSG : toMessage(e));
      return null;
    }
  }

  async remove(hostUrl: string): Promise<void> {
    this.errorSignal.set(null);
    try {
      const b64 = btoa(hostUrl);
      await firstValueFrom(this.http.delete(`/api/v1/account/portal-hosts/${b64}`));
      this.hostsSignal.update(hs => hs.filter(h => h.hostUrl !== hostUrl));
    } catch (e) {
      this.errorSignal.set(isPhase11Pending(e) ? PHASE_11_PENDING_MSG : toMessage(e));
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
