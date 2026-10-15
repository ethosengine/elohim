import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import { StorageClientService } from './storage-client.service';

import type { VerificationResultView } from '@elohim/storage-client/generated';
import type { ResilienceView } from '@elohim/storage-client/generated/ResilienceView';

/**
 * ResilienceDetailService — thin read of the network-detail resilience view
 * (encoding, pieces, peers, health) and the "is it still whole" check.
 *
 * Shell-owned so the atom home (/epr/{id}) reaches it without a lamad edge.
 * Both answers are computed by elohim-storage; this service only binds them.
 * Lamad's `services/resilience.service.ts` is the duplicate to shrink onto this
 * one (spec 2026-09-02 §11.5).
 *
 * `ResilienceView` is imported from its own generated module: the generated
 * barrel (`@elohim/storage-client/generated`) does not re-export it today.
 */
@Injectable({ providedIn: 'root' })
export class ResilienceDetailService {
  private readonly http = inject(HttpClient);
  private readonly storageClient = inject(StorageClientService);

  getResilience(id: string): Observable<ResilienceView> {
    return this.http.get<ResilienceView>(this.url(id));
  }

  verifyResilience(id: string): Observable<VerificationResultView> {
    return this.http.post<VerificationResultView>(`${this.url(id)}/verify`, {});
  }

  private url(id: string): string {
    return `${this.storageClient.getStorageBaseUrl()}/api/v1/resilience/${encodeURIComponent(id)}`;
  }
}
