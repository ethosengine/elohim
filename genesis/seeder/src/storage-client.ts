/**
 * Storage Client - HTTP client for elohim-storage operations
 *
 * Provides APIs for:
 * - Pushing shards to elohim-storage nodes
 * - Retrieving shard manifests
 * - Checking shard availability
 *
 * Architecture:
 * - elohim-storage runs alongside Holochain conductor
 * - Stores actual blob bytes with content-addressing
 * - DNA stores ShardManifest (metadata) and ShardLocation (where shards live)
 *
 * Usage:
 *   const client = new StorageClient({ baseUrl: 'http://localhost:8090' });
 *   const manifest = await client.pushBlob(data, 'video/mp4', 'commons');
 *   // Then register manifest in DNA via Holochain
 */

import { byteCid } from './byte-cid.js';

// =============================================================================
// Types
// =============================================================================

export interface StorageClientConfig {
  /** Base URL of the elohim-storage HTTP API (e.g., http://localhost:8090) */
  baseUrl: string;
  /** Request timeout in ms (default: 60000 for large blobs) */
  timeout?: number;
  /** Retry attempts for failed requests (default: 3) */
  retries?: number;
  /** Dry run mode - log but don't actually push */
  dryRun?: boolean;
}

/**
 * The DNA `ShardManifest` (elohim `content_store` integrity), snake_case — the shape
 * elohim-storage serves at `GET /manifest/{cid}` (`sharding.rs` `ShardManifest`) and the
 * seeder registers through `register_shard_manifest`. Every CID is a byte CID.
 */
export interface ShardManifest {
  blob_cid: string;
  byte_len: number;
  mime_type: string;
  chunker: { id: string; min: number; avg: number; max: number; level: number };
  chunks: Array<{ cid: string; len: number }>;
  erasure: { id: string; data: number; parity: number };
  /** Empty iff `erasure.id === 'none'`. */
  stripes: Array<{
    index: number;
    byte_len: number;
    shards: Array<{ cid: string; len: number; index: number }>;
  }>;
  reach: string;
  author_id: string | null;
  created_at: string;
  verified_at: string | null;
}

/** Storage's `PUT /blob/{cid}` answer (`PutBlobResponseView`, camelCase). */
interface PutBlobResponse {
  blobCid: string;
  byteLen: number;
  mimeType: string;
  profile: {
    chunker: ShardManifest['chunker'];
    erasure: ShardManifest['erasure'];
  };
  chunks: ShardManifest['chunks'];
  stripes: Array<{
    index: number;
    byteLen: number;
    shards: Array<{ cid: string; len: number; index: number }>;
  }>;
  authorId: string | null;
  createdAt: string;
  verifiedAt: string | null;
}

/** The DNA manifest a stored blob registers, read off storage's PUT answer. */
export function manifestFromPut(put: PutBlobResponse, reach: string): ShardManifest {
  return {
    blob_cid: put.blobCid,
    byte_len: put.byteLen,
    mime_type: put.mimeType,
    chunker: put.profile.chunker,
    chunks: put.chunks,
    erasure: put.profile.erasure,
    stripes: put.stripes.map(s => ({ index: s.index, byte_len: s.byteLen, shards: s.shards })),
    reach,
    author_id: put.authorId ?? null,
    created_at: put.createdAt,
    verified_at: put.verifiedAt ?? null,
  };
}

/** Shards a manifest declares (0 when the blob is not erasure-coded). */
export function shardCount(manifest: ShardManifest): number {
  return manifest.stripes.reduce((n, s) => n + s.shards.length, 0);
}

export interface StorageHealthStatus {
  healthy: boolean;
  blobs?: number;
  bytes?: number;
  manifests?: number;
  error?: string;
}

export interface PushBlobResult {
  success: boolean;
  manifest?: ShardManifest;
  error?: string;
}

export interface PushShardResult {
  success: boolean;
  cid: string;
  sizeBytes: number;
  alreadyExisted: boolean;
  error?: string;
}

// =============================================================================
// Storage Client
// =============================================================================

export class StorageClient {
  private config: Required<StorageClientConfig>;

  constructor(config: StorageClientConfig) {
    this.config = {
      baseUrl: config.baseUrl.replace(/\/$/, ''), // Remove trailing slash
      timeout: config.timeout || 60000,
      retries: config.retries || 3,
      dryRun: config.dryRun || false,
    };
  }

  /**
   * Compute the byte-profile CID (raw + blake3-256) of data — the address
   * elohim-storage verifies the body against.
   */
  static computeCid(data: Buffer): string {
    return byteCid(data);
  }

  /**
   * Check storage health and statistics.
   */
  async checkHealth(): Promise<StorageHealthStatus> {
    try {
      const response = await this.fetch('/health', {
        method: 'GET',
        timeout: 5000,
      });

      if (!response.ok) {
        return {
          healthy: false,
          error: `HTTP ${response.status}: ${response.statusText}`,
        };
      }

      const data = await response.json();
      return {
        healthy: data.status === 'ok',
        blobs: data.blobs,
        bytes: data.bytes,
        manifests: data.manifests,
      };
    } catch (error) {
      return {
        healthy: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Check if a shard exists in storage.
   */
  async shardExists(cid: string): Promise<boolean> {
    try {
      const response = await this.fetch(`/shard/${cid}`, {
        method: 'HEAD',
      });
      return response.status === 200;
    } catch {
      return false;
    }
  }

  /**
   * Push a shard to storage.
   */
  async pushShard(data: Buffer, expectedCid?: string): Promise<PushShardResult> {
    const cid = expectedCid || StorageClient.computeCid(data);

    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would push shard: ${cid} (${data.length} bytes)`);
      return { success: true, cid, sizeBytes: data.length, alreadyExisted: false };
    }

    try {
      const response = await this.fetch(`/shard/${cid}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/octet-stream',
        },
        body: data,
      });

      if (!response.ok) {
        const errorText = await response.text();
        return {
          success: false,
          cid,
          sizeBytes: data.length,
          alreadyExisted: false,
          error: `HTTP ${response.status}: ${errorText}`,
        };
      }

      const result = await response.json();
      return {
        success: true,
        cid: result.cid,
        sizeBytes: result.size_bytes,
        alreadyExisted: result.already_existed,
      };
    } catch (error) {
      return {
        success: false,
        cid,
        sizeBytes: data.length,
        alreadyExisted: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Push a blob to storage with auto-sharding.
   *
   * Returns the manifest which should then be registered in the DNA.
   */
  async pushBlob(
    data: Buffer,
    mimeType: string,
    reach: string = 'commons'
  ): Promise<PushBlobResult> {
    const cid = StorageClient.computeCid(data);

    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would push blob: ${cid} (${data.length} bytes)`);
      // Return a mock manifest for dry run
      return {
        success: true,
        manifest: {
          blob_cid: cid,
          byte_len: data.length,
          mime_type: mimeType,
          chunker: { id: 'fastcdc-2020', min: 0, avg: 0, max: 0, level: 1 },
          chunks: [{ cid, len: data.length }],
          erasure: { id: 'none', data: 0, parity: 0 },
          stripes: [],
          reach,
          author_id: null,
          created_at: new Date().toISOString(),
          verified_at: new Date().toISOString(),
        },
      };
    }

    try {
      const response = await this.fetch(`/blob/${cid}`, {
        method: 'PUT',
        headers: {
          'Content-Type': mimeType,
        },
        body: data,
      });

      if (!response.ok) {
        const errorText = await response.text();
        return {
          success: false,
          error: `HTTP ${response.status}: ${errorText}`,
        };
      }

      const put = (await response.json()) as PutBlobResponse;
      return {
        success: true,
        // Storage doesn't know about reach; the seeder sets it.
        manifest: manifestFromPut(put, reach),
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Get shard data by CID.
   */
  async getShard(cid: string): Promise<Buffer | null> {
    try {
      const response = await this.fetch(`/shard/${cid}`);
      if (!response.ok) return null;
      return Buffer.from(await response.arrayBuffer());
    } catch {
      return null;
    }
  }

  /**
   * Get manifest for a blob.
   */
  async getManifest(blobCid: string): Promise<ShardManifest | null> {
    try {
      const response = await this.fetch(`/manifest/${blobCid}`);
      if (!response.ok) return null;
      return await response.json();
    } catch {
      return null;
    }
  }

  /**
   * Push multiple blobs in batch, returning manifests.
   */
  async pushBlobs(
    blobs: Array<{ data: Buffer; mimeType: string; reach?: string }>
  ): Promise<{
    success: number;
    failed: number;
    manifests: ShardManifest[];
    errors: Array<{ cid: string; error: string }>;
  }> {
    let success = 0;
    let failed = 0;
    const manifests: ShardManifest[] = [];
    const errors: Array<{ cid: string; error: string }> = [];

    for (const blob of blobs) {
      const cid = StorageClient.computeCid(blob.data);
      const result = await this.pushBlob(blob.data, blob.mimeType, blob.reach || 'commons');

      if (result.success && result.manifest) {
        success++;
        manifests.push(result.manifest);
        console.log(`  ✓ ${cid} (${blob.data.length} bytes, ${shardCount(result.manifest)} shards)`);
      } else {
        failed++;
        errors.push({ cid, error: result.error || 'Unknown error' });
        console.error(`  ✗ ${cid}: ${result.error}`);
      }
    }

    return { success, failed, manifests, errors };
  }

  /**
   * Internal fetch wrapper with timeout and retry logic.
   */
  private async fetch(
    path: string,
    options: {
      method?: string;
      headers?: Record<string, string>;
      body?: Buffer | string;
      timeout?: number;
    } = {}
  ): Promise<Response> {
    const url = `${this.config.baseUrl}${path}`;
    const timeout = options.timeout || this.config.timeout;

    const headers: Record<string, string> = {
      ...options.headers,
    };

    // Convert Buffer to Uint8Array for fetch compatibility
    let body: BodyInit | undefined;
    if (options.body) {
      if (Buffer.isBuffer(options.body)) {
        body = new Uint8Array(options.body);
      } else {
        body = options.body;
      }
    }

    let lastError: Error | null = null;
    for (let attempt = 1; attempt <= this.config.retries; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeout);

        const response = await fetch(url, {
          method: options.method || 'GET',
          headers,
          body,
          signal: controller.signal,
        });

        clearTimeout(timeoutId);
        return response;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        if (attempt < this.config.retries) {
          // Exponential backoff
          const delay = Math.min(1000 * Math.pow(2, attempt - 1), 10000);
          await new Promise(resolve => setTimeout(resolve, delay));
        }
      }
    }

    throw lastError || new Error('Request failed after retries');
  }
}

// =============================================================================
// Validation Helpers
// =============================================================================

/**
 * Pre-flight check for storage node availability.
 */
export async function validateStorageNode(
  storageUrl: string
): Promise<{
  ready: boolean;
  issues: string[];
  stats?: { blobs: number; bytes: number };
}> {
  const issues: string[] = [];
  const client = new StorageClient({ baseUrl: storageUrl });

  const health = await client.checkHealth();
  if (!health.healthy) {
    issues.push(`Storage not reachable: ${health.error}`);
    return { ready: false, issues };
  }

  return {
    ready: true,
    issues: [],
    stats: {
      blobs: health.blobs || 0,
      bytes: health.bytes || 0,
    },
  };
}

export default StorageClient;
