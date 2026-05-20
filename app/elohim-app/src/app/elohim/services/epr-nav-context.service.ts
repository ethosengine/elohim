import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable, catchError, of } from 'rxjs';

/**
 * Wire shape for the EPR nav-context projection.
 *
 * Source of truth: Holochain DHT (existing EPR Head relationship links,
 * projected via the existing epr_relationships SQLite table). This is a
 * read-only projection — Category C per p2p-design-gate.
 *
 * Types are handwritten here for now. Task 7 will replace them with
 * ts-rs-generated imports from @elohim/storage-client/generated.
 */
export interface EprNavRef {
  cid: string;
  label?: string;
  resilienceTier?: 'high' | 'medium' | 'low' | 'unknown';
}

export interface EprNavContextView {
  cid: string;
  prev?: EprNavRef | null;
  next?: EprNavRef | null;
  partOf: EprNavRef[];
  related: EprNavRef[];
  derivedFrom: string[];
}

/**
 * Thin HTTP client for the EPR nav-context projection endpoint.
 *
 * Returns null gracefully when the endpoint is unreachable (backend not
 * deployed, network error, etc.) so the omni's nav affordances degrade
 * to session-only on cold environments.
 */
@Injectable({ providedIn: 'root' })
export class EprNavContextService {
  private readonly http = inject(HttpClient);

  fetch(cid: string): Observable<EprNavContextView | null> {
    return this.http
      .get<EprNavContextView>(`/api/v1/epr/${encodeURIComponent(cid)}/nav-context`)
      .pipe(catchError(() => of(null)));
  }
}
