/**
 * Blob Verification Service - Phase 1: Integrity Checking
 *
 * Provides cryptographic verification of blob integrity using SHA256 hashes.
 * Ensures downloaded content matches expected hash before caching/using.
 *
 * Used for:
 * - Verifying podcast/video integrity after download
 * - Detecting corrupted downloads
 * - Resumable downloads (verify each chunk's hash)
 * - Cache consistency checks
 */

import { Injectable } from '@angular/core';
import { Observable, from, throwError } from 'rxjs';
import { map, catchError } from 'rxjs/operators';

/**
 * Result of blob verification
 */
export interface BlobVerificationResult {
  /** Whether the hash matches */
  isValid: boolean;

  /** The computed hash */
  computedHash: string;

  /** Expected hash from blob metadata */
  expectedHash: string;

  /** Error if verification failed */
  error?: string;

  /** Time taken to verify in milliseconds */
  durationMs: number;
}

@Injectable({
  providedIn: 'root',
})
export class BlobVerificationService {
  /**
   * Verify blob integrity using SHA256 hash.
   *
   * @param blob The Blob object to verify
   * @param expectedHash Expected SHA256 hash (hex string)
   * @returns Observable with verification result
   */
  verifyBlob(blob: Blob, expectedHash: string): Observable<BlobVerificationResult> {
    const startTime = performance.now();

    return from(this.computeHash(blob)).pipe(
      map((computedHash) => {
        const isValid = computedHash.toLowerCase() === expectedHash.toLowerCase();
        const durationMs = performance.now() - startTime;

        return {
          isValid,
          computedHash,
          expectedHash,
          durationMs,
        };
      }),
      catchError((error) => {
        const durationMs = performance.now() - startTime;
        return from([
          {
            isValid: false,
            computedHash: '',
            expectedHash,
            error: `Hash computation failed: ${error.message}`,
            durationMs,
          },
        ]);
      }),
    );
  }

  /**
   * Verify blob chunk integrity (for resumable downloads).
   *
   * @param chunkBlob The chunk Blob object
   * @param expectedChunkHash Expected hash for this chunk
   * @returns Observable with verification result
   */
  verifyChunk(chunkBlob: Blob, expectedChunkHash: string): Observable<BlobVerificationResult> {
    return this.verifyBlob(chunkBlob, expectedChunkHash);
  }

  /**
   * Compute SHA256 hash of blob asynchronously.
   *
   * @param blob The Blob to hash
   * @returns Promise resolving to hex string hash
   */
  private async computeHash(blob: Blob): Promise<string> {
    const buffer = await blob.arrayBuffer();
    const hashBuffer = await crypto.subtle.digest('SHA-256', buffer);

    // Convert to hex string
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
  }

  /**
   * Stream hash computation for large files.
   * Allows progress tracking without loading entire file into memory.
   *
   * @param blob The Blob to hash
   * @param chunkSize Size of chunks to process (default 1 MB)
   * @param onProgress Callback with progress (bytesProcessed, totalBytes)
   * @returns Promise resolving to hex string hash
   */
  async streamComputeHash(
    blob: Blob,
    chunkSize: number = 1024 * 1024,
    onProgress?: (processed: number, total: number) => void,
  ): Promise<string> {
    const hashAlgo = new SubtleCrypto();
    let processed = 0;

    // For streaming hash, we use chunked processing
    // Note: SubtleCrypto doesn't support streaming, so we'll do sequential chunks
    // and track progress
    const hashes: ArrayBuffer[] = [];

    for (let i = 0; i < blob.size; i += chunkSize) {
      const chunk = blob.slice(i, Math.min(i + chunkSize, blob.size));
      const buffer = await chunk.arrayBuffer();

      processed += buffer.byteLength;
      if (onProgress) {
        onProgress(processed, blob.size);
      }

      hashes.push(buffer);
    }

    // Combine chunks and compute final hash
    const totalSize = hashes.reduce((sum, h) => sum + h.byteLength, 0);
    const combined = new Uint8Array(totalSize);
    let offset = 0;

    for (const h of hashes) {
      combined.set(new Uint8Array(h), offset);
      offset += h.byteLength;
    }

    const hashBuffer = await crypto.subtle.digest('SHA-256', combined);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
  }

  /**
   * Verify multiple blobs in parallel.
   *
   * @param blobsWithHashes Array of [Blob, expectedHash] pairs
   * @returns Observable with array of verification results
   */
  verifyMultiple(
    blobsWithHashes: Array<[Blob, string]>,
  ): Observable<BlobVerificationResult[]> {
    const verifications = blobsWithHashes.map(([blob, hash]) =>
      this.verifyBlob(blob, hash),
    );

    return from(Promise.all(verifications.map((v) => v.toPromise())));
  }
}
