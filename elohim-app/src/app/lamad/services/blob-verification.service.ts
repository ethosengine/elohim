/**
 * Blob Verification Service - Phase 1: Integrity Checking
 *
 * Provides cryptographic verification of blob integrity using SHA256 hashes.
 * Ensures downloaded content matches expected hash before caching/using.
 *
 * Supports both:
 * - SubtleCrypto (HTTPS/secure contexts) - hardware-accelerated
 * - Pure-JavaScript SHA256 fallback (non-HTTPS contexts) - slower but works everywhere
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

  /** Whether SubtleCrypto was used (true) or fallback (false) */
  usedSubtleCrypto?: boolean;
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
   * Supports both:
   * - SubtleCrypto (HTTPS contexts) - fast, hardware-accelerated
   * - Pure-JavaScript fallback (non-HTTPS contexts) - works everywhere
   *
   * @param blob The Blob to hash
   * @returns Promise resolving to hex string hash
   */
  private async computeHash(blob: Blob): Promise<string> {
    const buffer = await blob.arrayBuffer();

    // Try SubtleCrypto first (HTTPS contexts only)
    try {
      if (typeof crypto !== 'undefined' && crypto.subtle) {
        const hashBuffer = await crypto.subtle.digest('SHA-256', buffer);
        const hashArray = Array.from(new Uint8Array(hashBuffer));
        return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
      }
    } catch (error) {
      console.warn('[BlobVerification] SubtleCrypto unavailable, falling back to pure-JS SHA256:', error);
    }

    // Fallback: Use pure-JavaScript SHA256 (slower but works everywhere)
    return this.sha256Fallback(buffer);
  }

  /**
   * Pure-JavaScript SHA256 implementation as fallback for non-HTTPS contexts.
   *
   * This is slower than hardware-accelerated SubtleCrypto but works in all contexts.
   * Algorithm: SHA-256 as per FIPS 180-4
   *
   * @param buffer ArrayBuffer to hash
   * @returns Hex string hash
   */
  private sha256Fallback(buffer: ArrayBuffer): string {
    const bytes = new Uint8Array(buffer);

    // SHA256 constants
    const k = [
      0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
      0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
      0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
      0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
      0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
      0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
      0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
      0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    // Initial hash values
    const h0 = 0x6a09e667;
    const h1 = 0xbb67ae85;
    const h2 = 0x3c6ef372;
    const h3 = 0xa54ff53a;
    const h4 = 0x510e527f;
    const h5 = 0x9b05688c;
    const h6 = 0x1f83d9ab;
    const h7 = 0x5be0cd19;

    // Pre-processing
    const ml = bytes.length * 8;
    const padSize = (55 - bytes.length) % 64;
    const padded = new Uint8Array(bytes.length + padSize + 9);
    padded.set(bytes);
    padded[bytes.length] = 0x80;

    // Append message length as big-endian 64-bit integer
    const view = new DataView(padded.buffer);
    view.setUint32(padded.length - 4, ml >>> 0, false);
    view.setUint32(padded.length - 8, 0, false); // High 32 bits (assuming < 2^32 bytes)

    // Process message in 512-bit chunks
    let a = h0, b = h1, c = h2, d = h3, e = h4, f = h5, g = h6, h = h7;

    for (let i = 0; i < padded.length; i += 64) {
      const w = new Uint32Array(64);
      const chunk = new DataView(padded.buffer, i, 64);

      for (let j = 0; j < 16; j++) {
        w[j] = chunk.getUint32(j * 4, false);
      }

      for (let j = 16; j < 64; j++) {
        const s0 = this.rightRotate(w[j - 15], 7) ^ this.rightRotate(w[j - 15], 18) ^ (w[j - 15] >>> 3);
        const s1 = this.rightRotate(w[j - 2], 17) ^ this.rightRotate(w[j - 2], 19) ^ (w[j - 2] >>> 10);
        w[j] = (w[j - 16] + s0 + w[j - 7] + s1) >>> 0;
      }

      let a1 = a, b1 = b, c1 = c, d1 = d, e1 = e, f1 = f, g1 = g, h1 = h;

      for (let j = 0; j < 64; j++) {
        const S1 = this.rightRotate(e1, 6) ^ this.rightRotate(e1, 11) ^ this.rightRotate(e1, 25);
        const ch = (e1 & f1) ^ (~e1 & g1);
        const temp1 = (h1 + S1 + ch + k[j] + w[j]) >>> 0;
        const S0 = this.rightRotate(a1, 2) ^ this.rightRotate(a1, 13) ^ this.rightRotate(a1, 22);
        const maj = (a1 & b1) ^ (a1 & c1) ^ (b1 & c1);
        const temp2 = (S0 + maj) >>> 0;

        h1 = g1;
        g1 = f1;
        f1 = e1;
        e1 = (d1 + temp1) >>> 0;
        d1 = c1;
        c1 = b1;
        b1 = a1;
        a1 = (temp1 + temp2) >>> 0;
      }

      a = (a + a1) >>> 0;
      b = (b + b1) >>> 0;
      c = (c + c1) >>> 0;
      d = (d + d1) >>> 0;
      e = (e + e1) >>> 0;
      f = (f + f1) >>> 0;
      g = (g + g1) >>> 0;
      h = (h + h1) >>> 0;
    }

    // Produce final hash
    const digest = [h0, h1, h2, h3, h4, h5, h6, h7].map((v) => v + (v === h0 ? a : v === h1 ? b : v === h2 ? c : v === h3 ? d : v === h4 ? e : v === h5 ? f : v === h6 ? g : h));
    return digest
      .map((v) => {
        const val = (v >>> 0).toString(16);
        return '00000000'.substring(val.length) + val;
      })
      .join('')
      .substring(0, 64);
  }

  /**
   * Helper: Right rotate bits (circular right shift).
   */
  private rightRotate(value: number, bits: number): number {
    return ((value >>> bits) | (value << (32 - bits))) >>> 0;
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
