/**
 * HeliaFetchService — Verified blob fetch with CID integrity checking.
 *
 * Uses @helia/verified-fetch to retrieve blobs with cryptographic
 * CID verification, falling back to doorway HTTP on timeout.
 *
 * The Helia library is lazy-loaded via dynamic import() to avoid
 * bloating the initial bundle (~150KB gzipped).
 *
 * Only attempts Helia for `bafk*` CIDs — legacy sha256 hashes
 * skip directly to the HTTP fallback.
 */

import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';

import { firstValueFrom } from 'rxjs';

import { StorageClientService } from './storage-client.service';

/** Helia verified-fetch function type (lazy-loaded) */
type VerifiedFetchFn = (input: string) => Promise<Response>;

/** Default timeout for Helia fetch before falling back to HTTP (ms) */
const HELIA_TIMEOUT_MS = 5000;

@Injectable({ providedIn: 'root' })
export class HeliaFetchService {
  private readonly http = inject(HttpClient);
  private readonly storageClient = inject(StorageClientService);

  /** Cached verified-fetch function (lazy-loaded once) */
  private verifiedFetchFn: VerifiedFetchFn | null = null;
  /** Whether lazy-loading has been attempted (avoid re-trying on failure) */
  private loadAttempted = false;

  /**
   * Fetch a blob with CID verification, falling back to doorway HTTP.
   *
   * Resolution order:
   * 1. Helia verified-fetch (CID blobs only, 5s timeout)
   * 2. Doorway HTTP (`/blob/{cid}`)
   *
   * @param cidStr - CID or legacy hash string
   * @param timeoutMs - Helia timeout before fallback (default 5000ms)
   * @returns Raw bytes of the blob
   */
  async fetchVerified(cidStr: string, timeoutMs = HELIA_TIMEOUT_MS): Promise<Uint8Array> {
    // Only attempt Helia for CID strings (bafk...), not legacy hashes
    if (cidStr.startsWith('bafk')) {
      try {
        const result = await this.tryHeliaFetch(cidStr, timeoutMs);
        if (result) return result;
      } catch {
        // Fall through to HTTP fallback
      }
    }

    // HTTP fallback via doorway
    return this.fetchViaHttp(cidStr);
  }

  /**
   * Attempt to fetch via Helia with timeout.
   * Returns null if Helia is unavailable or times out.
   */
  private async tryHeliaFetch(cidStr: string, timeoutMs: number): Promise<Uint8Array | null> {
    const fetchFn = await this.getVerifiedFetch();
    if (!fetchFn) return null;

    // Race between Helia and timeout
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await Promise.race([
        fetchFn(`ipfs://${cidStr}`),
        new Promise<never>((_, reject) => {
          controller.signal.addEventListener('abort', () =>
            reject(new Error('Helia timeout'))
          );
        }),
      ]);

      if (!response.ok) return null;
      const buffer = await response.arrayBuffer();
      return new Uint8Array(buffer);
    } finally {
      clearTimeout(timeout);
    }
  }

  /** Fetch blob via doorway HTTP. */
  private async fetchViaHttp(cidStr: string): Promise<Uint8Array> {
    const blobUrl = this.storageClient.getBlobUrl(cidStr);
    const response = await firstValueFrom(
      this.http.get(blobUrl, { responseType: 'arraybuffer' })
    );
    return new Uint8Array(response);
  }

  /** Lazy-load @helia/verified-fetch. Returns null if unavailable. */
  private async getVerifiedFetch(): Promise<VerifiedFetchFn | null> {
    if (this.verifiedFetchFn) return this.verifiedFetchFn;
    if (this.loadAttempted) return null;

    this.loadAttempted = true;
    try {
      const { verifiedFetch } = await import('@helia/verified-fetch');
      this.verifiedFetchFn = verifiedFetch;
      return verifiedFetch;
    } catch {
      // Helia not available (e.g., build issue, unsupported env)
      return null;
    }
  }
}
