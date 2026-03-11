/**
 * IBlobFetcher — Abstract interface for CID-verified blob retrieval.
 *
 * Decouples content fetching from the concrete Helia implementation.
 * Consumers inject the BLOB_FETCHER token; the default factory resolves
 * to HeliaFetchService (Helia verified-fetch with HTTP fallback).
 *
 * Tests provide a mock IBlobFetcher via the same token — no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class ContentService {
 *   private readonly blobFetcher = inject(BLOB_FETCHER);
 *
 *   async loadBlob(cid: string): Promise<string> {
 *     const bytes = await this.blobFetcher.fetchVerified(cid);
 *     return new TextDecoder().decode(bytes);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { HeliaFetchService } from '../services/helia-fetch.service';

/**
 * Abstract blob fetcher — retrieves raw bytes by content address.
 *
 * Implementations handle CID verification, transport selection,
 * and fallback strategies. The contract is simple: CID in, bytes out.
 */
export interface IBlobFetcher {
  /**
   * Fetch blob bytes by content address with optional timeout.
   *
   * @param cidStr - CID (bafk...) or legacy hash (sha256-...)
   * @param timeoutMs - Timeout before fallback (default: implementation-specific)
   * @returns Raw bytes of the blob
   */
  fetchVerified(cidStr: string, timeoutMs?: number): Promise<Uint8Array>;
}

/**
 * Injection token for the blob fetcher.
 *
 * Default factory resolves to HeliaFetchService which provides:
 * - Helia verified-fetch for CID blobs (5s timeout)
 * - Doorway HTTP fallback for legacy hashes and timeouts
 *
 * Override in tests:
 * ```typescript
 * { provide: BLOB_FETCHER, useValue: mockBlobFetcher }
 * ```
 */
export const BLOB_FETCHER = new InjectionToken<IBlobFetcher>('BlobFetcher', {
  providedIn: 'root',
  factory: () => inject(HeliaFetchService),
});
