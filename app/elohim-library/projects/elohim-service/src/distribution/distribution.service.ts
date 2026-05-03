import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { firstValueFrom } from 'rxjs';

import type { DistributionDetails } from '../generated/distribution-details';

/**
 * Fetches the lazy "deep tier" of distribution data for a single blob.
 *
 * The cheap inline tier (`DistributionSummary`) is hydrated on every
 * EPR head response and read directly from `node.distribution`. This
 * service is only invoked when the user asks for more — typically on
 * tooltip expand of `<elohim-distribution-badge>`.
 */
@Injectable({ providedIn: 'root' })
export class DistributionService {
  private readonly http = inject(HttpClient);

  async getDetails(blobHash: string): Promise<DistributionDetails> {
    return firstValueFrom(
      this.http.get<DistributionDetails>(`/api/v1/blob/${blobHash}/distribution/details`)
    );
  }
}
