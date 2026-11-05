/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/put-blob-response.schema.json -- DO NOT EDIT */

/**
 * raw (0x55) + blake3-256 (0x1e); the pattern is a syntactic floor — readers fully decode (elohim-address)
 */
export type ByteCid = string;

/**
 * Source of truth: the BlobStore filesystem (byte-CID-keyed, Category C operational) and the DNA-notarized ShardManifest. Response from PUT /blob/{cid}. Declares the byte CID the bytes were stored under and the full byte profile that produced the representation (chunker and erasure parameters) so a reader never consults current thresholds. `stripes` is empty when `profile.erasure.id` is "none"; with `rs-gf8-0x11d-v1` each stripe covers at most 16 MiB of original bytes as data+parity shards. Address strings are CIDv1 base32 (byte CID: raw 0x55 + blake3-256 0x1e, prefix `bafkr4i`); readers fully decode (elohim-address).
 */
export interface PutBlobResponse {
  /**
   * Byte CID of the stored bytes.
   */
  blobCid: string;
  /**
   * Exact byte length the blobCid commits to.
   */
  byteLen: number;
  mimeType: string;
  /**
   * The byte profile this representation was produced under. Readers use these recorded parameters, never the server's current thresholds.
   */
  profile: {
    /**
     * Byte-profile epoch. An unknown epoch is HELD by the reader.
     */
    epoch: number;
    chunker: {
      /**
       * Chunker id, e.g. "fastcdc-2020".
       */
      id: string;
      min: number;
      avg: number;
      max: number;
      level: number;
    };
    erasure: {
      /**
       * Erasure code id: "none" or e.g. "rs-gf8-0x11d-v1".
       */
      id: string;
      data: number;
      parity: number;
    };
  };
  /**
   * Content-defined chunks in order; each chunk is itself a byte CID. Empty for an empty blob.
   */
  chunks: ChunkRef[];
  /**
   * Erasure stripes in order (empty when erasure id is "none"). Each stripe lists its data then parity shards by index.
   */
  stripes: {
    index: number;
    /**
     * Original (unpadded) bytes this stripe covers.
     */
    byteLen: number;
    shards: {
      cid: ByteCid;
      len: number;
      index: number;
    }[];
  }[];
  reach: string;
  authorId?: string | null;
  createdAt: string;
  verifiedAt?: string | null;
}
export interface ChunkRef {
  cid: ByteCid;
  len: number;
}
