import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';

import { Observable } from 'rxjs';

import { StorageClientService } from '@app/elohim/services/storage-client.service';

export interface ResilienceView {
  contentId: string;
  encoding: {
    strategy: string;
    dataShards: number;
    parityShards: number;
    totalSizeBytes: number;
    shardSizeBytes: number;
  };
  distribution: {
    totalShards: number;
    shardsWithLocations: number;
    distinctPeers: number;
    shards: {
      hash: string;
      shardType: string;
      peerIds: string[];
      status: string;
    }[];
  };
  stewardship: {
    stewardCount: number;
    allocations: {
      stewardPresenceId: string;
      allocationRatio: number;
      contributionType: string;
    }[];
  };
  commitments: {
    activePeers: number;
    totalCommittedBytes: number;
    totalUsedBytes: number;
  };
  health: {
    score: number;
    canSurviveFailures: number;
    status: string;
  };
}

export interface VerificationResultView {
  contentId: string;
  verified: boolean;
  encoding: string;
  shardsAvailable: number;
  shardsNeeded: number;
  shardsLocated: number;
  shardsMissing: number;
  reconstructionTimeMs: number;
  originalHash: string;
  reconstructedHash: string;
  hashMatch: boolean;
  error: string | null;
}

@Injectable({ providedIn: 'root' })
export class ResilienceService {
  private readonly http = inject(HttpClient);
  private readonly storageClient = inject(StorageClientService);

  getContentResilience(contentId: string): Observable<ResilienceView> {
    const baseUrl = this.storageClient.getStorageBaseUrl();
    return this.http.get<ResilienceView>(
      `${baseUrl}/api/v1/resilience/${encodeURIComponent(contentId)}`
    );
  }

  verifyResilience(contentId: string): Observable<VerificationResultView> {
    const baseUrl = this.storageClient.getStorageBaseUrl();
    return this.http.post<VerificationResultView>(
      `${baseUrl}/api/v1/resilience/${encodeURIComponent(contentId)}/verify`,
      {}
    );
  }
}
