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

import * as crypto from 'crypto';

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

export interface ShardManifest {
  blob_hash: string;
  total_size: number;
  mime_type: string;
  encoding: string;
  data_shards: number;
  total_shards: number;
  shard_size: number;
  shard_hashes: string[];
  reach: string;
  author_id?: string;
  created_at: string;
  verified_at?: string;
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
  hash: string;
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
   * Compute SHA256 hash of data (matching elohim-storage format).
   */
  static computeHash(data: Buffer): string {
    const hash = crypto.createHash('sha256').update(data).digest('hex');
    return `sha256-${hash}`;
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
  async shardExists(hash: string): Promise<boolean> {
    try {
      const response = await this.fetch(`/shard/${hash}`, {
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
  async pushShard(data: Buffer, expectedHash?: string): Promise<PushShardResult> {
    const hash = expectedHash || StorageClient.computeHash(data);

    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would push shard: ${hash} (${data.length} bytes)`);
      return { success: true, hash, sizeBytes: data.length, alreadyExisted: false };
    }

    try {
      const response = await this.fetch(`/shard/${hash}`, {
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
          hash,
          sizeBytes: data.length,
          alreadyExisted: false,
          error: `HTTP ${response.status}: ${errorText}`,
        };
      }

      const result = await response.json();
      return {
        success: true,
        hash: result.hash,
        sizeBytes: result.size_bytes,
        alreadyExisted: result.already_existed,
      };
    } catch (error) {
      return {
        success: false,
        hash,
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
    const hash = StorageClient.computeHash(data);

    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would push blob: ${hash} (${data.length} bytes)`);
      // Return a mock manifest for dry run
      return {
        success: true,
        manifest: {
          blob_hash: hash,
          total_size: data.length,
          mime_type: mimeType,
          encoding: 'none',
          data_shards: 1,
          total_shards: 1,
          shard_size: data.length,
          shard_hashes: [hash],
          reach,
          created_at: new Date().toISOString(),
          verified_at: new Date().toISOString(),
        },
      };
    }

    try {
      const response = await this.fetch(`/blob/${hash}`, {
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

      const manifest = await response.json();
      return {
        success: true,
        manifest: {
          ...manifest,
          reach, // Storage doesn't know about reach, we set it
        },
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Get shard data by hash.
   */
  async getShard(hash: string): Promise<Buffer | null> {
    try {
      const response = await this.fetch(`/shard/${hash}`);
      if (!response.ok) return null;
      return Buffer.from(await response.arrayBuffer());
    } catch {
      return null;
    }
  }

  /**
   * Get manifest for a blob.
   */
  async getManifest(blobHash: string): Promise<ShardManifest | null> {
    try {
      const response = await this.fetch(`/manifest/${blobHash}`);
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
    errors: Array<{ hash: string; error: string }>;
  }> {
    let success = 0;
    let failed = 0;
    const manifests: ShardManifest[] = [];
    const errors: Array<{ hash: string; error: string }> = [];

    for (const blob of blobs) {
      const hash = StorageClient.computeHash(blob.data);
      const result = await this.pushBlob(blob.data, blob.mimeType, blob.reach || 'commons');

      if (result.success && result.manifest) {
        success++;
        manifests.push(result.manifest);
        console.log(`  ✓ ${hash} (${blob.data.length} bytes, ${result.manifest.total_shards} shards)`);
      } else {
        failed++;
        errors.push({ hash, error: result.error || 'Unknown error' });
        console.error(`  ✗ ${hash}: ${result.error}`);
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
