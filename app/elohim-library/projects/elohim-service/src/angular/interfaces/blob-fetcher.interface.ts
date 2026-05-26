/**
 * IBlobFetcher — Abstract interface for CID-verified blob retrieval.
 *
 * Decouples content fetching from the concrete Helia implementation.
 * Consumers inject the BLOB_FETCHER token. The default factory
 * lives in elohim-app (HeliaFetchService), registered in app.config.ts.
 *
 * Slice 2.1c: Exported from @elohim/service so lamad services can inject
 * via token without a cross-pillar import of the concrete class.
 */

import { InjectionToken } from '@angular/core';

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
 * No factory defined here — registered in elohim-app's app.config.ts:
 *   { provide: BLOB_FETCHER, useClass: HeliaFetchService }
 *
 * Override in tests:
 * ```typescript
 * { provide: BLOB_FETCHER, useValue: mockBlobFetcher }
 * ```
 */
export const BLOB_FETCHER = new InjectionToken<IBlobFetcher>('BlobFetcher');
