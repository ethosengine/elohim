/**
 * Network Posture Service
 *
 * Reads from GET /api/v1/network/posture (implemented in F2).
 * If the endpoint is not yet live, returns null gracefully so the
 * UI can render an "unavailable" state rather than blowing up.
 *
 * Type is declared locally because the generated TS type (F1) has not
 * shipped yet.  Once F1+F2 land, replace this interface with the
 * import from @elohim/storage-client/generated.
 */

import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';

// @coverage: 0% (pending F1/F2 landing)

import { catchError } from 'rxjs/operators';

import { Observable, of } from 'rxjs';

import { StorageClientService } from './storage-client.service';

/**
 * P2P network posture snapshot.
 * Mirrors the NetworkPostureView shape from the F1 sprint spec.
 * Replace with generated type once F1+F2 ship:
 *   import type { NetworkPostureView } from '@elohim/storage-client/generated';
 */
export interface NetworkPostureView {
  /** Total peers known to this node */
  totalPeers: number;
  /** Peers seen within the activity window */
  activePeers: number;
  /** Peers beyond the staleness threshold */
  stalePeers: number;
  /** Peers with always-on connectivity commitment */
  alwaysOnPeers: number;
  /** Households actively reciprocating bandwidth/storage */
  householdsReciprocating: number;
  /** Whether compute capacity is available on the mesh */
  computeAvailable: boolean;
  /** Storage pressure as a ratio 0..1 (1 = saturated) */
  storagePressure: number;
  /** ISO-8601 timestamp of when this snapshot was computed */
  computedAt: string;
}

@Injectable({ providedIn: 'root' })
export class NetworkPostureService {
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);

  /**
   * Fetch the current network posture snapshot.
   *
   * Returns null when:
   * - The endpoint has not yet been deployed (F2 pending)
   * - The node is offline or storage is unreachable
   * - Any HTTP error occurs
   *
   * Callers should render an "unavailable" state when null is returned.
   */
  get(): Observable<NetworkPostureView | null> {
    const base = this.storage.getStorageBaseUrl();
    return this.http
      .get<NetworkPostureView>(`${base}/api/v1/network/posture`)
      .pipe(catchError(() => of(null)));
  }
}
