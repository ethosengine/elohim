/**
 * Type declarations for @elohim/wasm module.
 *
 * This module is built from holochain/elohim-wasm using wasm-pack.
 * If the WASM module is not available, the blob-verification.service
 * will gracefully fall back to server-side or SubtleCrypto verification.
 */

declare module '@elohim/wasm' {
  /**
   * Initialize the WASM module.
   * Must be called before using other functions.
   */
  export default function init(): Promise<void>;

  /**
   * Verification result from WASM functions.
   */
  export interface VerificationResult {
    /** Whether verification succeeded */
    is_valid(): boolean;
    /** Get computed hash (hex string) */
    computed_hash(): string;
    /** Get expected hash (echoed back) */
    expected_hash(): string;
    /** Get error message if verification failed */
    error(): string | undefined;
  }

  /**
   * Verify blob data against expected SHA256 hash.
   *
   * @param data - Blob data as Uint8Array
   * @param expectedHash - Expected SHA256 hash (hex string, 64 chars)
   * @returns VerificationResult with is_valid, computed_hash, etc.
   */
  export function verify_blob(data: Uint8Array, expectedHash: string): VerificationResult;

  /**
   * Compute SHA256 hash of data.
   *
   * @param data - Data to hash as Uint8Array
   * @returns SHA256 hash as hex string (64 chars)
   */
  export function compute_hash(data: Uint8Array): string;

  /**
   * Streaming hasher for incremental hash computation.
   * Useful for large files to avoid loading entire file into memory.
   */
  export class StreamingHasher {
    /** Create a new streaming hasher */
    constructor();

    /**
     * Update hasher with more data.
     * @param data - Chunk of data to add
     */
    update(data: Uint8Array): void;

    /**
     * Finalize and return the computed hash.
     * Consumes the hasher - cannot be used after calling finalize.
     * @returns SHA256 hash as hex string
     */
    finalize(): string;

    /**
     * Verify final hash against expected value.
     * Consumes the hasher - cannot be used after calling verify.
     * @param expectedHash - Expected SHA256 hash
     * @returns VerificationResult
     */
    verify(expectedHash: string): VerificationResult;
  }
}
