/**
 * Network Posture Service
 *
 * Reads from GET /api/v1/network/posture (live: handler in
 * elohim/elohim-storage/src/api/network_posture.rs — joins live PeerStatus
 * rows with the stewarded_nodes projection). Returns null gracefully on any
 * HTTP error so the UI can render an honest empty/offline state rather than
 * blowing up — a fresh household with no peers yet is a valid null.
 *
 * The wire shape is the generated NetworkPostureView (camelCase, snake_case
 * never crosses the Rust boundary); we re-export it for consumers that already
 * import it from this service.
 */

import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';

import { catchError } from 'rxjs/operators';

import { Observable, of } from 'rxjs';

import { StorageClientService } from './storage-client.service';

import type { NetworkPostureView } from '@app/generated/network-posture-view';

@Injectable({ providedIn: 'root' })
export class NetworkPostureService {
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);

  /**
   * Fetch the current network posture snapshot.
   *
   * Returns null when:
   * - The node is offline or storage is unreachable
   * - Any HTTP error occurs
   *
   * Callers should render an honest empty/offline state when null is returned.
   */
  get(): Observable<NetworkPostureView | null> {
    const base = this.storage.getStorageBaseUrl();
    return this.http
      .get<NetworkPostureView>(`${base}/api/v1/network/posture`)
      .pipe(catchError(() => of(null)));
  }
}

// Re-export the generated view so existing consumers can keep importing the
// type from this service (doctrine: snake_case never crosses into TS; the
// camelCase generated type is the single source of truth).
export type { NetworkPostureView } from '@app/generated/network-posture-view';
