/**
 * Acquisition affordances (spec 2026-06-07-epr-acquisition-pull-queue-design §8).
 * Thin service: capability detection + the rung-3 download disposition.
 *  - Peer-capable (direct/Tauri context): POST a DevicePin to the OWN-NODE pins
 *    API — the acquisition stream pulls + verifies (byte-arrival).
 *  - Browser (doorway mode): warm the SW cache lane via the normal content fetch
 *    path (no DevicePin object exists in the browser — spec §8).
 * Rung 4 (pin-as-peer/provide) is Slice 2 — NOT here.
 */
import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { StorageClientService } from './storage-client.service';

export type AcquisitionCapability = 'peer' | 'browser';

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
    const base = this.storage.getStorageBaseUrl();

    if (this.capability() === 'peer') {
      await firstValueFrom(
        this.http.post(`${base}/api/v1/pins`, { headRef: eprRef, kind: 'item' })
      );
      return 'peer';
    }

    // Browser: warm the SW cache lane via normal content fetch path.
    const id = eprRef.replace(/^epr:/, '');
    await fetch(`${base}/db/content/${encodeURIComponent(id)}`);
    return 'browser';
  }
}
