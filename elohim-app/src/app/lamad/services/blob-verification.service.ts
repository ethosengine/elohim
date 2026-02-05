/**
 * Blob Verification Service - Defense-in-Depth Integrity Checking
 *
 * Provides cryptographic verification of blob integrity using SHA256 hashes
 * with a three-tier fallback chain:
 *
 * 1. **WASM** (Primary) - Client-side, fast, works offline
 * 2. **Doorway Server** - Server-side, authoritative, always available
 * 3. **SubtleCrypto** (Fallback) - Browser native, requires HTTPS
 *
 * This defense-in-depth approach ensures verification is always possible
 * regardless of WASM availability or network conditions.
 *
 * Used for:
 * - Verifying podcast/video integrity after download
 * - Detecting corrupted downloads
 * - Resumable downloads (verify each chunk's hash)
 * - Cache consistency checks
 */

import { Injectable } from '@angular/core';

// @coverage: 40.5% (2026-02-05)

import { map, catchError } from 'rxjs/operators';

import { Observable, from, of, firstValueFrom } from 'rxjs';

import { DoorwayClientService } from '../../elohim/services/doorway-client.service';

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

  /** Which verification method was used */
  method?: 'wasm' | 'server' | 'subtle-crypto' | 'fallback-js';
}

/**
 * WASM module interface (from elohim-wasm crate)
 */
interface ElohimWasmModule {
  verify_blob(
    data: Uint8Array,
    expectedHash: string
  ): {
    is_valid: boolean;
    computed_hash: string;
    expected_hash: string;
    size_bytes: number;
    error?: string;
  };
  compute_hash(data: Uint8Array): string;
  StreamingHasher: new () => {
    update(chunk: Uint8Array): void;
    bytes_processed: number;
    finalize(expectedHash: string): {
      is_valid: boolean;
      computed_hash: string;
      expected_hash: string;
      size_bytes: number;
      error?: string;
    };
    finalize_hash(): string;
  };
}

@Injectable({
  providedIn: 'root',
})
export class BlobVerificationService {
  /** WASM module (loaded lazily) */
  private wasmModule: ElohimWasmModule | null = null;

  /** Whether WASM loading has been attempted */
  private wasmLoadAttempted = false;

  /** Whether WASM is available */
  private wasmAvailable = false;

  constructor(private readonly doorway: DoorwayClientService) {}

  /**
   * Verify blob integrity using SHA256 hash.
   *
   * Uses fallback chain: WASM → Server → SubtleCrypto → JS
   *
   * @param blob The Blob object to verify
   * @param expectedHash Expected SHA256 hash (hex string)
   * @returns Observable with verification result
   */
  verifyBlob(blob: Blob, expectedHash: string): Observable<BlobVerificationResult> {
    const startTime = performance.now();

    return from(this.verifyWithFallbackChain(blob, expectedHash)).pipe(
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
   * @param expectedChunkHash Expected hash for this chunk
   * @returns Observable with verification result
   */
  verifyChunk(chunkBlob: Blob, expectedChunkHash: string): Observable<BlobVerificationResult> {
    return this.verifyBlob(chunkBlob, expectedChunkHash);
  }

  /**
   * Internal: Execute verification with fallback chain.
   *
   * Order: WASM → Server → SubtleCrypto → JS Fallback
   */
  private async verifyWithFallbackChain(
    blob: Blob,
    expectedHash: string
  ): Promise<Omit<BlobVerificationResult, 'durationMs'>> {
    const buffer = await blob.arrayBuffer();
    const data = new Uint8Array(buffer);

    // Try WASM first (fastest, works offline)
    try {
      if (await this.ensureWasmLoaded()) {
        const result = this.wasmModule!.verify_blob(data, expectedHash);
        return {
          isValid: result.is_valid,
          computedHash: result.computed_hash,
          expectedHash: result.expected_hash,
          error: result.error,
          method: 'wasm',
        };
      }
    } catch {
      // WASM verification failed - continue to next method
    }

    // Try server verification (authoritative, always available online)
    try {
      const response = await firstValueFrom(this.doorway.verifyBlobData(data, expectedHash));
      if (response) {
        return {
          isValid: response.isValid,
          computedHash: response.computedHash,
          expectedHash: response.expectedHash,
          error: response.error,
          method: 'server',
        };
      }
    } catch {
      // Server verification failed - continue to next method
    }

    // Try SubtleCrypto (browser native, requires HTTPS)
    try {
      if (typeof crypto !== 'undefined' && crypto.subtle) {
        const hashBuffer = await crypto.subtle.digest('SHA-256', buffer);
        const hashArray = Array.from(new Uint8Array(hashBuffer));
        const computedHash = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
        const isValid = computedHash.toLowerCase() === expectedHash.toLowerCase();
        return {
          isValid,
          computedHash,
          expectedHash,
          method: 'subtle-crypto',
        };
      }
    } catch {
      // SubtleCrypto verification failed - continue to fallback
    }

    // Final fallback: pure JavaScript SHA256
    const computedHash = this.sha256Fallback(buffer);
    const isValid = computedHash.toLowerCase() === expectedHash.toLowerCase();
    return {
      isValid,
      computedHash,
      expectedHash,
      method: 'fallback-js',
    };
  }

  /**
   * Ensure WASM module is loaded.
   *
   * @returns Promise<boolean> True if WASM is available
   */
  private async ensureWasmLoaded(): Promise<boolean> {
    if (this.wasmModule) {
      return true;
    }

    if (this.wasmLoadAttempted) {
      return this.wasmAvailable;
    }

    this.wasmLoadAttempted = true;

    try {
      // Dynamic import of WASM module
      // The elohim-wasm package should be built with wasm-pack and available
      // Use variable to prevent Vite from statically resolving (optional dependency)
      const wasmModulePath = '@elohim/wasm';
      const wasm = await import(/* @vite-ignore */ wasmModulePath);
      await wasm.default(); // Initialize WASM
      this.wasmModule = wasm as unknown as ElohimWasmModule;
      this.wasmAvailable = true;

      return true;
    } catch {
      // WASM module load failed - WASM not available
      this.wasmAvailable = false;
      return false;
    }
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
    onProgress?: (processed: number, total: number) => void
  ): Promise<string> {
    // Try WASM streaming hasher first
    if (await this.ensureWasmLoaded()) {
      try {
        const hasher = new this.wasmModule!.StreamingHasher();
        let processed = 0;

        for (let i = 0; i < blob.size; i += chunkSize) {
          const chunk = blob.slice(i, Math.min(i + chunkSize, blob.size));
          const buffer = await chunk.arrayBuffer();
          hasher.update(new Uint8Array(buffer));

          processed += buffer.byteLength;
          if (onProgress) {
            onProgress(processed, blob.size);
          }
        }

        return hasher.finalize_hash();
      } catch {
        // WASM streaming hash failed - continue to fallback
      }
    }

    // Fallback: accumulate chunks and use SubtleCrypto
    let processed = 0;
    const chunks: ArrayBuffer[] = [];

    for (let i = 0; i < blob.size; i += chunkSize) {
      const chunk = blob.slice(i, Math.min(i + chunkSize, blob.size));
      const buffer = await chunk.arrayBuffer();
      chunks.push(buffer);

      processed += buffer.byteLength;
      if (onProgress) {
        onProgress(processed, blob.size);
      }
    }

    // Combine chunks
    const totalSize = chunks.reduce((sum, c) => sum + c.byteLength, 0);
    const combined = new Uint8Array(totalSize);
    let offset = 0;
    for (const chunk of chunks) {
      combined.set(new Uint8Array(chunk), offset);
      offset += chunk.byteLength;
    }

    // Use SubtleCrypto or fallback
    if (typeof crypto !== 'undefined' && crypto.subtle) {
      const hashBuffer = await crypto.subtle.digest('SHA-256', combined);
      return Array.from(new Uint8Array(hashBuffer))
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');
    }

    return this.sha256Fallback(combined.buffer);
  }

  /**
   * Verify multiple blobs in parallel.
   *
   * @param blobsWithHashes Array of [Blob, expectedHash] pairs
   * @returns Observable with array of verification results
   */
  verifyMultiple(blobsWithHashes: [Blob, string][]): Observable<BlobVerificationResult[]> {
    const verifications = blobsWithHashes.map(([blob, hash]) => this.verifyBlob(blob, hash));

    return from(Promise.all(verifications.map(async v => firstValueFrom(v))));
  }

  /**
   * Check which verification methods are available.
   *
   * @returns Object with availability flags
   */
  async checkAvailableMethods(): Promise<{
    wasm: boolean;
    server: boolean;
    subtleCrypto: boolean;
    fallbackJs: boolean;
  }> {
    const wasmAvailable = await this.ensureWasmLoaded();
    const subtleCryptoAvailable = typeof crypto !== 'undefined' && !!crypto.subtle;

    // Server is always available if we have network
    let serverAvailable = false;
    try {
      await firstValueFrom(this.doorway.checkHealth());
      serverAvailable = true;
    } catch {
      serverAvailable = false;
    }

    return {
      wasm: wasmAvailable,
      server: serverAvailable,
      subtleCrypto: subtleCryptoAvailable,
      fallbackJs: true, // Always available
    };
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
      0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
      0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
      0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
      0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
      0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
      0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
      0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
      0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
      0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
      0xc67178f2,
    ];

    // Initial hash values
    let h0 = 0x6a09e667;
    let h1 = 0xbb67ae85;
    let h2 = 0x3c6ef372;
    let h3 = 0xa54ff53a;
    let h4 = 0x510e527f;
    let h5 = 0x9b05688c;
    let h6 = 0x1f83d9ab;
    let h7 = 0x5be0cd19;

    // Pre-processing
    const ml = bytes.length * 8;
    const padSize = (55 - (bytes.length % 64) + 64) % 64;
    const padded = new Uint8Array(bytes.length + padSize + 9);
    padded.set(bytes);
    padded[bytes.length] = 0x80;

    // Append message length as big-endian 64-bit integer
    const view = new DataView(padded.buffer);
    view.setUint32(padded.length - 4, ml >>> 0, false);
    view.setUint32(padded.length - 8, 0, false);

    // Process message in 512-bit chunks
    for (let i = 0; i < padded.length; i += 64) {
      const w = new Uint32Array(64);
      const chunk = new DataView(padded.buffer, i, 64);

      for (let j = 0; j < 16; j++) {
        w[j] = chunk.getUint32(j * 4, false);
      }

      for (let j = 16; j < 64; j++) {
        const s0 =
          this.rightRotate(w[j - 15], 7) ^ this.rightRotate(w[j - 15], 18) ^ (w[j - 15] >>> 3);
        const s1 =
          this.rightRotate(w[j - 2], 17) ^ this.rightRotate(w[j - 2], 19) ^ (w[j - 2] >>> 10);
        w[j] = (w[j - 16] + s0 + w[j - 7] + s1) >>> 0;
      }

      let a = h0,
        b = h1,
        c = h2,
        d = h3,
        e = h4,
        f = h5,
        g = h6,
        h = h7;

      for (let j = 0; j < 64; j++) {
        const S1 = this.rightRotate(e, 6) ^ this.rightRotate(e, 11) ^ this.rightRotate(e, 25);
        const ch = (e & f) ^ (~e & g);
        const temp1 = (h + S1 + ch + k[j] + w[j]) >>> 0;
        const S0 = this.rightRotate(a, 2) ^ this.rightRotate(a, 13) ^ this.rightRotate(a, 22);
        const maj = (a & b) ^ (a & c) ^ (b & c);
        const temp2 = (S0 + maj) >>> 0;

        h = g;
        g = f;
        f = e;
        e = (d + temp1) >>> 0;
        d = c;
        c = b;
        b = a;
        a = (temp1 + temp2) >>> 0;
      }

      h0 = (h0 + a) >>> 0;
      h1 = (h1 + b) >>> 0;
      h2 = (h2 + c) >>> 0;
      h3 = (h3 + d) >>> 0;
      h4 = (h4 + e) >>> 0;
      h5 = (h5 + f) >>> 0;
      h6 = (h6 + g) >>> 0;
      h7 = (h7 + h) >>> 0;
    }

    // Produce final hash
    return [h0, h1, h2, h3, h4, h5, h6, h7].map(v => v.toString(16).padStart(8, '0')).join('');
  }

  /**
   * Helper: Right rotate bits (circular right shift).
   */
  private rightRotate(value: number, bits: number): number {
    return ((value >>> bits) | (value << (32 - bits))) >>> 0;
  }

  /**
   * Convert ArrayBuffer/Uint8Array to base64 string.
   */
  private arrayBufferToBase64(bytes: Uint8Array): string {
    return btoa(String.fromCodePoint(...Array.from(bytes)));
  }
}
