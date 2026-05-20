import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable, catchError, of } from 'rxjs';

// Wire types are generated from Rust via ts-rs. Do not redeclare inline.
// Regenerate with: cd elohim/elohim-views && cargo test export_bindings
import type { EprNavContextView, EprNavRef } from '@elohim/storage-client/generated';
export type { EprNavContextView, EprNavRef };

/**
 * Thin HTTP client for the EPR nav-context projection endpoint.
 *
 * Source of truth: Holochain DHT (existing EPR Head relationship links,
 * projected via the existing epr_coupling, epr_supersedence, and epr_claims
 * SQLite tables). This is a read-only projection — Category C per
 * p2p-design-gate.
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
