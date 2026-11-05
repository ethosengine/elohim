/**
 * Blob Verification Service - integrity checking against a byte CID
 *
 * A blob is named by its byte CID (CIDv1 raw + BLAKE3-256, `bafkr4i…`). Verification is local
 * and deterministic: hash the bytes with BLAKE3-256 (`@noble/hashes/blake3`, pure JavaScript, so
 * it works in every context — HTTP, HTTPS, workers, SSR) and compare the resulting byte CID with
 * the expected one. The expected address must itself be a canonical byte CID (a full decode,
 * never a prefix check); a legacy `sha256-<hex>` marker, bare hex or any other CID form is not
 * an address and fails verification.
 *
 * The former WASM / doorway-server / SubtleCrypto / SHA-256 tiers are gone: SubtleCrypto cannot
 * compute BLAKE3, and a clean break admits no second hash. The pure-JS BLAKE3 is the fallback
 * and the primary at once; a faster native implementation may be added behind this same API.
 *
 * Used for:
 * - Verifying podcast/video integrity after download
 * - Detecting corrupted downloads
 * - Resumable downloads (verify each chunk's address)
 * - Cache consistency checks
 */

import { Injectable } from '@angular/core';

import { blake3 } from '@noble/hashes/blake3';

import { Observable, from, of } from 'rxjs';
import { map, catchError } from 'rxjs/operators';

import { byteCidFromDigest, isByteCid } from '../utils/byte-cid';

/**
 * Result of blob verification
 */
export interface BlobVerificationResult {
  /** Whether the computed byte CID equals the expected one */
  isValid: boolean;

  /** The byte CID the bytes actually hash to */
  computedHash: string;

  /** Expected byte CID from blob metadata */
  expectedHash: string;

  /** Error if verification failed */
  error?: string;

  /** Time taken to verify in milliseconds */
  durationMs: number;

  /** Which verification method was used */
  method?: 'blake3-js';
}

@Injectable({
  providedIn: 'root',
})
export class BlobVerificationService {
  /**
   * Verify blob integrity against its byte CID.
   *
   * @param blob The Blob object to verify
   * @param expectedHash Expected byte CID (`bafkr4i…`)
   * @returns Observable with verification result
   */
  verifyBlob(blob: Blob, expectedHash: string): Observable<BlobVerificationResult> {
    const startTime = performance.now();

    return from(this.verifyLocally(blob, expectedHash)).pipe(
      map(result => ({
        ...result,
        durationMs: performance.now() - startTime,
      })),
      catchError(error => {
        const durationMs = performance.now() - startTime;
        return of({
          isValid: false,
          computedHash: '',
          expectedHash,
          error: `Verification failed: ${error.message}`,
          durationMs,
        });
      })
    );
  }

  /**
   * Verify blob chunk integrity (for resumable downloads).
   *
   * @param chunkBlob The chunk Blob object
   * @param expectedChunkHash Expected byte CID for this chunk
   * @returns Observable with verification result
   */
  verifyChunk(chunkBlob: Blob, expectedChunkHash: string): Observable<BlobVerificationResult> {
    return this.verifyBlob(chunkBlob, expectedChunkHash);
  }

  /**
   * Stream hash computation for large files.
   * Allows progress tracking without a second pass over the data.
   *
   * @param blob The Blob to hash
   * @param chunkSize Size of chunks to process (default 1 MB)
   * @param onProgress Callback with progress (bytesProcessed, totalBytes)
   * @returns Promise resolving to the blob's byte CID
   */
  async streamComputeHash(
    blob: Blob,
    chunkSize: number = 1024 * 1024,
    onProgress?: (processed: number, total: number) => void
  ): Promise<string> {
    const hasher = blake3.create({ dkLen: 32 });
    let processed = 0;

    for (let i = 0; i < blob.size; i += chunkSize) {
      const chunk = blob.slice(i, Math.min(i + chunkSize, blob.size));
      hasher.update(new Uint8Array(await chunk.arrayBuffer()));
      processed += chunk.size;
      if (onProgress) {
        onProgress(processed, blob.size);
      }
    }

    return byteCidFromDigest(hasher.digest());
  }

  /**
   * Verify multiple blobs in parallel.
   *
   * @param blobsWithHashes Array of [Blob, expectedByteCid] pairs
   * @returns Observable with array of verification results
   */
  verifyMultiple(blobsWithHashes: [Blob, string][]): Observable<BlobVerificationResult[]> {
    return from(
      Promise.all(
        blobsWithHashes.map(
          ([blob, hash]) =>
            new Promise<BlobVerificationResult>(resolve => {
              this.verifyBlob(blob, hash).subscribe(resolve);
            })
        )
      )
    );
  }

  /**
   * Check which verification methods are available.
   *
   * @returns Object with availability flags
   */
  checkAvailableMethods(): Promise<{ blake3Js: boolean }> {
    return Promise.resolve({ blake3Js: true });
  }

  private async verifyLocally(
    blob: Blob,
    expectedHash: string
  ): Promise<Omit<BlobVerificationResult, 'durationMs'>> {
    const computedHash = await this.streamComputeHash(blob);
    if (!isByteCid(expectedHash)) {
      return {
        isValid: false,
        computedHash,
        expectedHash,
        error: 'Expected address is not a byte CID (bafkr4i…)',
        method: 'blake3-js',
      };
    }
    return {
      isValid: computedHash === expectedHash,
      computedHash,
      expectedHash,
      method: 'blake3-js',
    };
  }
}
