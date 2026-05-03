import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type { DistributionDetails } from '@app/generated/distribution-details';

@Injectable({ providedIn: 'root' })
export class DistributionService {
  private readonly http = inject(HttpClient);

  async getDetails(blobHash: string): Promise<DistributionDetails> {
    return firstValueFrom(
      this.http.get<DistributionDetails>(`/api/v1/blob/${blobHash}/distribution/details`)
    );
  }
}
