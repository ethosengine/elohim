/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/put-blob-response.schema.json -- DO NOT EDIT */

/**
 * Source of truth: BlobStore filesystem (SHA256-keyed, Category C operational) and IrohBlobStore filesystem (BLAKE3-keyed, Category C operational). Response from PUT /blob/{hash}. Returns the SHA256-keyed shard manifest (legacy wire-compat) and, when the iroh blob store is configured server-side, the BLAKE3 hash the bytes were also written under.
 */
export interface PutBlobResponse {
  /**
   * SHA256-keyed legacy address: 'sha256-{64hex}'
   */
  blobHash: string;
  totalSize: number;
  mimeType: string;
  encoding: 'none' | 'chunked' | 'rs-4-7' | 'rs-8-12' | 'reed-solomon';
  dataShards: number;
  totalShards: number;
  shardSize: number;
  shardHashes: string[];
  reach: string;
  authorId?: string | null;
  createdAt: string;
  verifiedAt?: string | null;
  /**
   * BLAKE3 hash the same bytes were written to in IrohBlobStore. Null when iroh side is not configured or the iroh write failed (legacy SHA256 write still succeeded in that case).
   */
  blake3Hash?: string | null;
}
