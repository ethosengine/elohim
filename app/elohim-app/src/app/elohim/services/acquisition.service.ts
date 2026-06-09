/**
 * Acquisition affordances (spec 2026-06-07-epr-acquisition-pull-queue-design §8).
 * Thin service: capability detection + the rung-3 download / rung-4 provide
 * dispositions.
 *  - Peer-capable (direct/Tauri context): POST a DevicePin to the OWN-NODE pins
 *    API — the acquisition stream pulls + verifies (byte-arrival).
 *  - Browser (doorway mode): warm the SW cache lane via the normal content fetch
 *    path (no DevicePin object exists in the browser — spec §8).
 * Rung 4 (pin-as-peer/provide) adds `pinAsPeer()` (peer-only) + a polling
 * `pullStatus$()` stream against the own-node GET /api/v1/pins/{eprId}/pull route.
 */
import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError, map, switchMap } from 'rxjs/operators';

import { type Observable, firstValueFrom, of, timer } from 'rxjs';

import { StorageClientService } from './storage-client.service';

import type { EprPullStatusView } from '../../generated/epr-pull-status-view';

export type AcquisitionCapability = 'peer' | 'browser';

/**
 * The per-EPR pull rollup the pin-progress UI renders. A UI projection of the
 * generated EprPullStatusView wire type (views/epr-pull-status.schema.json) —
 * the `eprId` echo is dropped (the caller already holds it). `total`/`caughtUp`
 * are nullable: null means "state not computable yet" — treat as keep-waiting,
 * NEVER as caught up (the tri-state wait-for-drain contract, spec §4.3).
 */
export interface PullStatusInfo {
  total: number | null;
  fetched: number;
  pending: number;
  failed: number;
  caughtUp: boolean | null;
}

/** The null-guarded waiting state — emitted on any poll error. */
const WAITING_STATE: PullStatusInfo = {
  total: null,
  fetched: 0,
  pending: 0,
  failed: 0,
  caughtUp: null,
};

/** Poll cadence for the pull-status stream (ms). */
const PULL_POLL_MS = 3500;

@Injectable({ providedIn: 'root' })
export class AcquisitionService {
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);

  /**
   * Returns 'peer' when running in the direct/Tauri deployment context
   * (elohim-storage sidecar at :8090 — peer-capable acquisition stream).
   * Returns 'browser' in doorway mode (SW cache lane only).
   */
  capability(): AcquisitionCapability {
    return this.storage.connectionMode === 'direct' ? 'peer' : 'browser';
  }

  /**
   * Initiate download/acquisition for an EPR ref.
   *
   * Peer path: POSTs a DevicePin to /api/v1/pins — the acquisition stream
   * pulls and verifies byte-arrival.
   *
   * Browser path: warms the SW cache lane by fetching the content URL via the
   * doorway proxy; no DevicePin is created.
   *
   * Returns the capability that was used so callers can surface feedback.
   */
  async download(eprRef: string): Promise<AcquisitionCapability> {
    // Canonicalize to the bare content id: the EPR link surface carries an
    // `epr:` prefix, but the reconcile/dispatch/completion loop keys on bare
    // content.id. Strip here so both peer and browser paths are coherent.
    const id = eprRef.replace(/^epr:/, '');
    const base = this.storage.getStorageBaseUrl();

    if (this.capability() === 'peer') {
      await firstValueFrom(this.http.post(`${base}/api/v1/pins`, { headRef: id, kind: 'item' }));
      return 'peer';
    }

    // Browser: warm the SW cache lane via normal content fetch path.
    const resp = await fetch(`${base}/db/content/${encodeURIComponent(id)}`);
    if (!resp.ok) {
      throw new Error(`[AcquisitionService] cache-warm failed for ${eprRef}: ${resp.status}`);
    }
    return 'browser';
  }

  /**
   * Rung 4 (spec §8, provide loop): pin an EPR AND offer to serve it to peers.
   *
   * Peer-only: POSTs an item pin with `provide:true` to the own-node pins API.
   * The provide reconciler (provide_reconcile.rs) sees the caught-up commons pin
   * and authors a replicates-commons commitment so the node advertises + serves
   * the bytes. Browser nodes cannot provide — they have no peer surface — so we
   * reject loudly rather than silently degrade to a plain cache-warm.
   */
  async pinAsPeer(eprRef: string): Promise<void> {
    if (this.capability() !== 'peer') {
      throw new Error(
        `[AcquisitionService] pinAsPeer requires a peer-capable node; got "${this.capability()}"`
      );
    }
    const id = eprRef.replace(/^epr:/, '');
    const base = this.storage.getStorageBaseUrl();
    await firstValueFrom(
      this.http.post(`${base}/api/v1/pins`, { headRef: id, kind: 'item', provide: true })
    );
  }

  /**
   * Poll the per-EPR pull rollup for live progress UI. Fires immediately, then
   * every ~3.5s while subscribed. Null-guarded: a failed poll emits WAITING_STATE
   * (caughtUp:null) rather than completing or claiming caught-up, so the bar keeps
   * polling and the UI never renders "done" on a non-computable state.
   *
   * The own-node GET /api/v1/pins/{eprId}/pull route is NEVER doorway-proxied —
   * pin-as-peer + progress are only reachable when capability() === 'peer'.
   */
  pullStatus$(eprRef: string): Observable<PullStatusInfo> {
    const id = eprRef.replace(/^epr:/, '');
    const base = this.storage.getStorageBaseUrl();
    const url = `${base}/api/v1/pins/${encodeURIComponent(id)}/pull`;
    return timer(0, PULL_POLL_MS).pipe(
      switchMap(() =>
        this.http.get<EprPullStatusView>(url).pipe(
          map(view => ({
            total: view?.total ?? null,
            fetched: view?.fetched ?? 0,
            pending: view?.pending ?? 0,
            failed: view?.failed ?? 0,
            caughtUp: view?.caughtUp ?? null,
          })),
          catchError(() => of(WAITING_STATE))
        )
      )
    );
  }
}
