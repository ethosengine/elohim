import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';

import { Observable } from 'rxjs';

import { StorageClientService } from '@app/elohim/services/storage-client.service';

import type { HouseholdResilienceView } from '../../generated/household-resilience-view';

@Injectable({ providedIn: 'root' })
export class HouseholdResilienceService {
  private readonly http = inject(HttpClient);
  private readonly storageClient = inject(StorageClientService);

  get(contentId: string, viewerHouseholdId?: string): Observable<HouseholdResilienceView> {
    const baseUrl = this.storageClient.getStorageBaseUrl();
    const query = viewerHouseholdId
      ? `?viewerHouseholdId=${encodeURIComponent(viewerHouseholdId)}`
      : '';
    return this.http.get<HouseholdResilienceView>(
      `${baseUrl}/api/v1/resilience/${encodeURIComponent(contentId)}/household${query}`
    );
  }
}
