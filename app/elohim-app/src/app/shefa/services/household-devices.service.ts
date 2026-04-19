/**
 * HouseholdDevicesService — Thin HTTP client for the household devices view.
 *
 * Calls `GET /api/v1/households/{id}/devices`, which returns
 * `HouseholdDevicesView` — the operational join of stewarded_nodes with
 * live peer_statuses rows (Category C projection; no new DHT entry type).
 *
 * Household resolution strategy:
 *   The authoritative path is `GET /api/v1/humans/{id}/household` (Task D3).
 *   Until D3 ships this service derives the householdId by querying the
 *   human's collective participations and picking the first collectiveId
 *   that starts with "household-". This matches the seeding convention
 *   (household-matthew, household-dowell, etc.).
 *
 * Sprint task D3 will add `GET /api/v1/humans/{id}/household`; replace
 * resolveHouseholdId with that direct call once it ships.
 */

import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';

import { map, Observable, of, switchMap } from 'rxjs';

import type { HouseholdDevicesView } from '@app/generated/household-devices-view';
import type { CollectiveParticipationView } from '@elohim/storage-client/generated';

interface ParticipationListResponse {
  items: CollectiveParticipationView[];
  count: number;
}

@Injectable({ providedIn: 'root' })
export class HouseholdDevicesService {
  private readonly http = inject(HttpClient);

  /**
   * Fetch the HouseholdDevicesView for the given household.
   */
  list(householdId: string): Observable<HouseholdDevicesView> {
    return this.http.get<HouseholdDevicesView>(
      `/api/v1/households/${encodeURIComponent(householdId)}/devices`
    );
  }

  /**
   * Resolve the household ID for a human from their collective participations.
   * Returns the first collectiveId prefixed with "household-", or null if none.
   *
   * Sprint task D3: replace with `GET /api/v1/humans/${humanId}/household` once live.
   */
  resolveHouseholdId(humanId: string): Observable<string | null> {
    if (!humanId) return of(null);
    return this.http
      .get<ParticipationListResponse>(`/api/v1/humans/${encodeURIComponent(humanId)}/collectives`)
      .pipe(
        map(res => {
          const items: CollectiveParticipationView[] = res.items ?? [];
          const household = items.find(p => p.collectiveId.startsWith('household-'));
          return household?.collectiveId ?? null;
        })
      );
  }

  /**
   * Convenience: resolve the household for a human, then load devices.
   * Emits null if the human has no household participation yet.
   */
  listForHuman(humanId: string): Observable<HouseholdDevicesView | null> {
    return this.resolveHouseholdId(humanId).pipe(
      switchMap(householdId => {
        if (!householdId) return of(null);
        return this.list(householdId);
      })
    );
  }
}
