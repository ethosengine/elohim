/**
 * Doorway Client - HTTP client for projection cache operations
 *
 * Provides APIs for:
 * - Pushing blobs to projection cache
 * - Checking blob existence
 * - Validating cache availability
 *
 * Architecture:
 * - Doorway exposes /store/{address} for blob serving (CID or hash)
 * - Seeding uses admin API to push blobs
 * - Cache is eventually consistent with DHT
 *
 * Content Addressing:
 * - CID (IPFS-compatible): bafkrei... - preferred, future-proof
 * - SHA256 prefixed: sha256-abc123... - legacy compatibility
 * - Raw SHA256 hex: abc123... - legacy compatibility
 *
 * Usage:
 *   const client = new DoorwayClient({ baseUrl: 'https://doorway.example.com', apiKey: 'xxx' });
 *   await client.checkHealth();
 *   const result = await client.pushBlob(hash, blob, metadata);
 *   console.log(result.cid); // bafkrei...
 */

import { BlobMetadata } from './blob-manager.js';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';
import * as raw from 'multiformats/codecs/raw';

// =============================================================================
// CID Utilities
// =============================================================================

/**
 * Compute a CID (Content Identifier) for raw data.
 *
 * Uses CIDv1 with raw codec (0x55) and SHA256 multihash.
 * This is IPFS-compatible for raw binary content.
 *
 * @param data - Raw bytes to compute CID for
 * @returns CID string (e.g., "bafkreihdwdcefgh...")
 */
export async function computeCid(data: Uint8Array): Promise<string> {
  const hash = await sha256.digest(data);
  const cid = CID.create(1, raw.code, hash);
  return cid.toString();
}

/**
 * Compute both CID and SHA256 hash for backward compatibility.
 *
 * @param data - Raw bytes
 * @returns Object with cid and hash strings
 */
export async function computeContentAddresses(data: Uint8Array): Promise<{
  cid: string;
  hash: string;
}> {
  const hash = await sha256.digest(data);
  const cid = CID.create(1, raw.code, hash);

  // Extract raw SHA256 bytes from multihash and format as sha256-{hex}
  const hashHex = Buffer.from(hash.digest).toString('hex');

  return {
    cid: cid.toString(),
    hash: `sha256-${hashHex}`,
  };
}

// =============================================================================
// Types
// =============================================================================

export interface DoorwayClientConfig {
  /** Base URL of the doorway (e.g., https://doorway.example.com) */
  baseUrl: string;
  /** Direct storage URL for bulk operations (bypasses doorway proxy if set) */
  storageUrl?: string;
  /** API key for admin operations */
  apiKey?: string;
  /** Request timeout in ms (default: 30000) */
  timeout?: number;
  /** Retry attempts for failed requests (default: 3) */
  retries?: number;
  /** Dry run mode - log but don't actually push */
  dryRun?: boolean;
}

export interface HealthStatus {
  healthy: boolean;
  version?: string;
  cacheEnabled: boolean;
  /** Conductor connection status - seeder should check this before seeding! */
  conductor?: {
    connected: boolean;
    connectedWorkers: number;
    totalWorkers: number;
  };
  error?: string;
}

/** Schema capability information from /db/schema endpoint */
export interface SchemaInfo {
  supportedVersions: number[];
  currentVersion: number;
  deprecatedVersions: number[];
}

/** Comprehensive status from /status endpoint */
export interface DoorwayStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  version?: string;
  uptimeSeconds?: number;
  conductor: {
    connected: boolean;
    connectedWorkers: number;
    totalWorkers: number;
  };
  storage: {
    healthy: boolean;
    importEnabled: boolean;
    activeBatches?: number;
    url?: string;
  };
  diagnostics: {
    cellDiscovered: boolean;
    readyForSeeding: boolean;
    recommendations: string[];
  };
  cache?: {
    enabled: boolean;
    entries?: number;
    sizeBytes?: number;
  };
}

export interface PushResult {
  success: boolean;
  /** CID (Content Identifier) - IPFS-compatible, preferred */
  cid: string;
  /** SHA256 hash - legacy format for backward compatibility */
  hash: string;
  cached: boolean;
  error?: string;
}

export interface BatchPushResult {
  success: number;
  failed: number;
  errors: Array<{ hash: string; error: string }>;
}

/** Result from bulk content/path creation */
export interface BulkCreateResult {
  inserted: number;
  skipped: number;
  errors: string[];
}

/** Result from bulk relationship creation */
export interface BulkRelationshipResult {
  created: number;
  errors: string[];
}

/** Result from bulk presence creation */
export interface BulkPresenceResult {
  created: number;
  errors: string[];
}

/** Result from bulk mastery upsert */
export interface BulkMasteryResult {
  created: number;
  updated: number;
  errors: string[];
}

/** Result from bulk event recording */
export interface BulkEventResult {
  recorded: number;
  errors: string[];
}

// =============================================================================
// Import API Types
// =============================================================================

export interface ImportQueueRequest {
  /** Optional batch ID (generated if not provided) */
  batchId?: string;
  /** Hash of the blob in elohim-storage containing the items JSON
   *  REQUIRED - upload to storage first, then queue import */
  blobHash: string;
  /** Total number of items in the blob */
  totalItems: number;
  /** Schema version for the import data */
  schemaVersion?: number;
  /**
   * Items per chunk (optional, uses server default if not provided).
   * Smaller chunks = less conductor pressure but slower throughput.
   * Server default: 50 items per chunk.
   */
  chunkSize?: number;
  /**
   * Delay between chunks in ms (optional, uses server default if not provided).
   * Higher delay = more conductor breathing room but slower throughput.
   * Server default: 300ms.
   */
  chunkDelayMs?: number;
}

export interface ImportQueueResponse {
  batchId: string;
  queuedCount: number;
  processing: boolean;
  message?: string;
}

export interface ImportStatusResponse {
  batchId: string;
  status: 'queued' | 'processing' | 'completed' | 'completed_with_errors' | 'failed';
  totalItems: number;
  processedCount: number;
  errorCount: number;
  skippedCount?: number;  // Items that already existed in DHT
  errors: string[];
  elapsedMs?: number;
  itemsPerSecond?: number;
  completedAt?: string;
}

// =============================================================================
// Doorway Client
// =============================================================================

export class DoorwayClient {
  private config: DoorwayClientConfig & { baseUrl: string; timeout: number; retries: number; dryRun: boolean };

  constructor(config: DoorwayClientConfig) {
    this.config = {
      baseUrl: config.baseUrl.replace(/\/$/, ''), // Remove trailing slash
      storageUrl: config.storageUrl?.replace(/\/$/, ''), // Direct storage URL for /db/* routes
      apiKey: config.apiKey || '',
      timeout: config.timeout || 30000,
      retries: config.retries || 3,
      dryRun: config.dryRun || false,
    };
  }

  /**
   * Check doorway health, conductor connectivity, and cache availability.
   *
   * IMPORTANT: The seeder should verify `conductor.connected === true` before
   * attempting to seed. The doorway's /health endpoint now returns conductor
   * status in the response body.
   */
  async checkHealth(): Promise<HealthStatus> {
    try {
      const response = await this.fetch('/health', {
        method: 'GET',
        timeout: 5000, // Quick timeout for health check
      });

      if (!response.ok) {
        return {
          healthy: false,
          cacheEnabled: false,
          error: `HTTP ${response.status}: ${response.statusText}`,
        };
      }

      const data = await response.json();

      // Check conductor connectivity - this is critical for seeding
      const conductorConnected = data.conductor?.connected ?? false;
      const conductorWorkers = data.conductor?.connectedWorkers ?? 0;
      const totalWorkers = data.conductor?.totalWorkers ?? 0;

      // Include conductor status in health response
      return {
        healthy: conductorConnected, // Only healthy if conductor is connected
        version: data.version,
        cacheEnabled: data.cache?.enabled ?? true,
        conductor: {
          connected: conductorConnected,
          connectedWorkers: conductorWorkers,
          totalWorkers: totalWorkers,
        },
        error: conductorConnected ? undefined : data.error || `Conductor not connected (${conductorWorkers}/${totalWorkers} workers)`,
      };
    } catch (error) {
      return {
        healthy: false,
        cacheEnabled: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Get comprehensive status including conductor, storage, and diagnostics.
   *
   * This is the recommended method for preflight checks before seeding.
   * It provides much more detail than checkHealth().
   */
  async checkStatus(): Promise<DoorwayStatus | null> {
    try {
      const response = await this.fetch('/status', {
        method: 'GET',
        timeout: 10000,
      });

      if (!response.ok) {
        console.warn(`Status check failed: HTTP ${response.status}`);
        return null;
      }

      const data = await response.json();
      return {
        status: data.status || 'unhealthy',
        version: data.version,
        uptimeSeconds: data.uptimeSeconds,
        conductor: {
          connected: data.conductor?.connected ?? false,
          connectedWorkers: data.conductor?.connectedWorkers ?? 0,
          totalWorkers: data.conductor?.totalWorkers ?? 0,
        },
        storage: {
          healthy: data.storage?.healthy ?? false,
          importEnabled: data.storage?.importEnabled ?? false,
          activeBatches: data.storage?.activeBatches,
          url: data.storage?.url,
        },
        diagnostics: {
          cellDiscovered: data.diagnostics?.cellDiscovered ?? false,
          readyForSeeding: data.diagnostics?.readyForSeeding ?? false,
          recommendations: data.diagnostics?.recommendations ?? [],
        },
        cache: data.cache,
      };
    } catch (error) {
      console.warn(`Status check error: ${error instanceof Error ? error.message : error}`);
      return null;
    }
  }

  /**
   * Get schema capability information from storage.
   *
   * Returns supported schema versions, current version, and deprecated versions.
   * Used for pre-flight validation to ensure seeder compatibility.
   */
  async getSchemaInfo(): Promise<SchemaInfo> {
    const response = await this.fetch('/db/schema', {
      method: 'GET',
      timeout: 10000,
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Schema info request failed: HTTP ${response.status}: ${errorText}`);
    }

    return await response.json();
  }

  /**
   * Wait for doorway to be ready for seeding.
   *
   * Polls /status until conductor and storage are healthy.
   * Returns the final status or throws on timeout.
   */
  async waitForReady(options: {
    timeoutMs?: number;
    pollIntervalMs?: number;
    onStatus?: (status: DoorwayStatus) => void;
  } = {}): Promise<DoorwayStatus> {
    const { timeoutMs = 120000, pollIntervalMs = 5000, onStatus } = options;
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      const status = await this.checkStatus();

      if (status) {
        onStatus?.(status);

        // Check if ready for seeding
        if (status.conductor.connected && status.storage.healthy) {
          return status;
        }
      }

      await new Promise(resolve => setTimeout(resolve, pollIntervalMs));
    }

    throw new Error(`Doorway not ready after ${timeoutMs}ms`);
  }

  /**
   * Check if a blob exists in the cache.
   */
  async blobExists(hash: string): Promise<boolean> {
    try {
      const response = await this.fetch(`/store/${hash}`, {
        method: 'HEAD',
      });
      return response.status === 200;
    } catch {
      return false;
    }
  }

  /**
   * Push a blob to the projection cache.
   *
   * Uses the admin seed endpoint which accepts blobs for caching.
   * Computes CID (Content Identifier) for the data and returns it.
   *
   * @param hash - Legacy SHA256 hash (sha256-... or raw hex)
   * @param data - Blob data as Buffer
   * @param metadata - Blob metadata including mime type
   * @returns PushResult with both cid and hash
   */
  async pushBlob(
    hash: string,
    data: Buffer,
    metadata: BlobMetadata
  ): Promise<PushResult> {
    // Compute CID for the data
    const addresses = await computeContentAddresses(new Uint8Array(data));
    const cid = addresses.cid;

    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would push blob: ${cid} (${data.length} bytes)`);
      return { success: true, cid, hash, cached: false };
    }

    // Check if already cached (try CID first, then legacy hash)
    const existsByCid = await this.blobExists(cid);
    if (existsByCid) {
      return { success: true, cid, hash, cached: true };
    }
    const existsByHash = await this.blobExists(hash);
    if (existsByHash) {
      return { success: true, cid, hash, cached: true };
    }

    try {
      const response = await this.fetch('/admin/seed/blob', {
        method: 'PUT',
        headers: {
          'Content-Type': metadata.mimeType,
          'X-Blob-Hash': hash,
          'X-Blob-Cid': cid,
          'X-Blob-Size': String(metadata.sizeBytes),
          ...(metadata.entryPoint && { 'X-Entry-Point': metadata.entryPoint }),
          ...(metadata.fallbackUrl && { 'X-Fallback-Url': metadata.fallbackUrl }),
        },
        body: data,
      });

      if (!response.ok) {
        const errorText = await response.text();
        return {
          success: false,
          cid,
          hash,
          cached: false,
          error: `HTTP ${response.status}: ${errorText}`,
        };
      }

      return { success: true, cid, hash, cached: false };
    } catch (error) {
      return {
        success: false,
        cid,
        hash,
        cached: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Push multiple blobs in batch.
   *
   * Processes blobs sequentially to avoid overwhelming the cache.
   * Returns CIDs for all successfully pushed blobs.
   */
  async pushBlobs(
    blobs: Array<{ hash: string; data: Buffer; metadata: BlobMetadata }>
  ): Promise<BatchPushResult> {
    let success = 0;
    let failed = 0;
    const errors: Array<{ hash: string; error: string }> = [];

    for (const blob of blobs) {
      const result = await this.pushBlob(blob.hash, blob.data, blob.metadata);

      if (result.success) {
        success++;
        if (result.cached) {
          console.log(`  ✓ ${result.cid} (already cached)`);
        } else {
          console.log(`  ✓ ${result.cid} (pushed ${blob.data.length} bytes)`);
        }
      } else {
        failed++;
        errors.push({ hash: blob.hash, error: result.error || 'Unknown error' });
        console.error(`  ✗ ${result.cid || blob.hash}: ${result.error}`);
      }
    }

    return { success, failed, errors };
  }

  /**
   * Register an HTML5 app with the cache.
   *
   * This tells the cache about the app so it can serve files from the zip.
   */
  async registerApp(
    appId: string,
    blobHash: string,
    entryPoint: string = 'index.html',
    fallbackUrl?: string
  ): Promise<{ success: boolean; error?: string }> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would register app: ${appId} -> ${blobHash}`);
      return { success: true };
    }

    try {
      const response = await this.fetch('/admin/apps/register', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          appId: appId,
          blobHash: blobHash,
          entryPoint: entryPoint,
          fallbackUrl: fallbackUrl,
        }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        return {
          success: false,
          error: `HTTP ${response.status}: ${errorText}`,
        };
      }

      return { success: true };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Get cache statistics.
   */
  async getStats(): Promise<{
    entries: number;
    sizeBytes: number;
    hitRate: number;
  } | null> {
    try {
      const response = await this.fetch('/admin/cache/stats');
      if (!response.ok) return null;
      return await response.json();
    } catch {
      return null;
    }
  }

  // ===========================================================================
  // Import API - Queue content for batch import
  // ===========================================================================

  /**
   * Queue content items for import via doorway.
   *
   * @param batchType - Type of content to import (e.g., 'content', 'paths')
   * @param request - Import request with items or blobHash
   * @returns Import queue response with batchId
   */
  async queueImport(
    batchType: string,
    request: ImportQueueRequest
  ): Promise<ImportQueueResponse> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would queue import: ${batchType} (${request.totalItems} items, blob: ${request.blobHash.slice(0, 20)}...)`);
      return {
        batchId: `dry-run-${Date.now()}`,
        queuedCount: request.totalItems,
        processing: false,
        message: 'Dry run - not actually queued',
      };
    }

    // Retry loop for 503 (discovery in progress) errors
    const maxRetries = 12; // ~60 seconds with 5 second delays
    const retryDelay = 5000;

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      const requestBody = JSON.stringify(request);

      // IMPORT_DEBUG: Log request
      if (process.env.IMPORT_DEBUG) {
        console.log(`[IMPORT_DEBUG] seeder -> doorway request:`);
        console.log(`  URL: POST /import/${batchType}`);
        console.log(`  Body: ${requestBody.slice(0, 2000)}${requestBody.length > 2000 ? '...' : ''}`);
      }

      const response = await this.fetch(`/import/${batchType}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: requestBody,
      });

      if (response.ok) {
        const responseBody = await response.json();

        // IMPORT_DEBUG: Log response
        if (process.env.IMPORT_DEBUG) {
          console.log(`[IMPORT_DEBUG] doorway -> seeder response:`);
          console.log(`  Status: ${response.status}`);
          console.log(`  Body: ${JSON.stringify(responseBody).slice(0, 2000)}`);
        }

        return responseBody;
      }

      // Read body once
      const responseText = await response.text();

      // IMPORT_DEBUG: Log error response
      if (process.env.IMPORT_DEBUG) {
        console.log(`[IMPORT_DEBUG] doorway -> seeder ERROR response:`);
        console.log(`  Status: ${response.status}`);
        console.log(`  Body: ${responseText.slice(0, 2000)}`);
      }

      // Handle 503 (discovery in progress) with retry
      if (response.status === 503) {
        const errorData = (() => {
          try { return JSON.parse(responseText); }
          catch { return { error: 'Service unavailable' }; }
        })();
        const retryAfter = response.headers.get('Retry-After');
        const delay = retryAfter ? parseInt(retryAfter, 10) * 1000 : retryDelay;

        if (attempt < maxRetries) {
          console.log(`   ⏳ Doorway not ready: ${errorData.error || 'Discovery in progress'}`);
          console.log(`      Retry ${attempt}/${maxRetries} in ${delay / 1000}s...`);
          await new Promise(resolve => setTimeout(resolve, delay));
          continue;
        }
      }

      throw new Error(`Import queue failed: HTTP ${response.status}: ${responseText}`);
    }

    throw new Error(`Import queue failed: Doorway not ready after ${maxRetries} retries`);
  }

  /**
   * Get import batch status.
   *
   * @param batchType - Type of content being imported
   * @param batchId - Batch ID returned from queueImport
   * @returns Import status with progress and errors
   */
  async getImportStatus(
    batchType: string,
    batchId: string
  ): Promise<ImportStatusResponse | null> {
    try {
      const response = await this.fetch(`/import/${batchType}/${batchId}`);

      if (response.status === 404) {
        return null;
      }

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Get import status failed: HTTP ${response.status}: ${errorText}`);
      }

      return await response.json();
    } catch (error) {
      if (error instanceof Error && error.message.includes('404')) {
        return null;
      }
      throw error;
    }
  }

  /**
   * Queue import and poll until completion.
   *
   * @param batchType - Type of content to import
   * @param request - Import request with items
   * @param options - Polling options
   * @returns Final import status
   */
  async queueImportAndWait(
    batchType: string,
    request: ImportQueueRequest,
    options: {
      pollIntervalMs?: number;
      timeoutMs?: number;
      onProgress?: (status: ImportStatusResponse) => void;
    } = {}
  ): Promise<ImportStatusResponse> {
    const pollInterval = options.pollIntervalMs || 5000;
    const timeout = options.timeoutMs || 300000; // 5 min default

    // Queue the import
    const queueResult = await this.queueImport(batchType, request);
    console.log(`📤 Import queued: ${queueResult.batchId} (${queueResult.queuedCount} items)`);

    // If already completed (synchronous processing)
    if (queueResult.processing === false && queueResult.queuedCount === 0) {
      // Might be an error or already processed
      const status = await this.getImportStatus(batchType, queueResult.batchId);
      if (status) return status;
    }

    // Poll for completion
    const startTime = Date.now();
    let lastProcessed = 0;

    while (Date.now() - startTime < timeout) {
      await new Promise(resolve => setTimeout(resolve, pollInterval));

      const status = await this.getImportStatus(batchType, queueResult.batchId);
      if (!status) {
        throw new Error(`Batch ${queueResult.batchId} not found during polling`);
      }

      // Report progress
      if (status.processedCount !== lastProcessed) {
        if (options.onProgress) {
          options.onProgress(status);
        }
        lastProcessed = status.processedCount;
      }

      // Check for completion
      if (status.status === 'completed' || status.status === 'failed') {
        return status;
      }
    }

    throw new Error(`Import timed out after ${timeout}ms`);
  }

  // ===========================================================================
  // Direct Bulk Operations - Calls elohim-storage via Doorway's /db/* proxy
  // ===========================================================================

  /**
   * Bulk create content items directly via storage service.
   *
   * This bypasses the queue-based import flow and directly calls the
   * elohim-storage bulk endpoint through Doorway's API proxy.
   *
   * @param items - Content items to create (with contentBody field, not content)
   * @returns BulkCreateResult with inserted/skipped counts
   */
  async bulkCreateContent(
    items: Array<{
      schemaVersion?: number;
      id: string;
      title: string;
      contentType?: string;
      contentFormat?: string;
      contentBody?: string;
      description?: string;
      blobHash?: string;
      blobCid?: string;
      metadataJson?: string;
      reach?: string;
      tags?: string[];
    }>
  ): Promise<BulkCreateResult> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would bulk create ${items.length} content items`);
      return { inserted: items.length, skipped: 0, errors: [] };
    }

    const response = await this.fetch('/db/content/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
      body: JSON.stringify(items),
      timeout: 120000, // 2 min for bulk ops
    });

    if (!response.ok) {
      const errorText = await response.text();
      if (response.status === 400 && errorText.includes('Unsupported schema version')) {
        const supported = response.headers.get('X-Supported-Schema-Versions');
        throw new Error(`Schema version mismatch: ${errorText}. Supported: ${supported}`);
      }
      throw new Error(`Bulk create content failed: HTTP ${response.status}: ${errorText}`);
    }

    return await response.json();
  }

  /**
   * Bulk create paths directly via storage service.
   *
   * @param items - Path items to create (with nested chapters/steps)
   * @returns BulkCreateResult with inserted/skipped counts
   */
  async bulkCreatePaths(
    items: Array<{
      schemaVersion?: number;
      id: string;
      title: string;
      description?: string;
      pathType?: string;
      difficulty?: string;
      estimatedDuration?: string;
      visibility?: string;
      metadataJson?: string;
      tags?: string[];
      chapters?: Array<{
        id: string;
        title: string;
        description?: string;
        orderIndex: number;
        steps?: Array<{
          id: string;
          pathId: string;
          chapterId?: string;
          title: string;
          stepType?: string;
          resourceId?: string;
          orderIndex: number;
          metadataJson?: string;
        }>;
      }>;
    }>
  ): Promise<BulkCreateResult> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would bulk create ${items.length} paths`);
      return { inserted: items.length, skipped: 0, errors: [] };
    }

    const response = await this.fetch('/db/paths/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
      body: JSON.stringify(items),
      timeout: 120000,
    });

    if (!response.ok) {
      const errorText = await response.text();
      if (response.status === 400 && errorText.includes('Unsupported schema version')) {
        const supported = response.headers.get('X-Supported-Schema-Versions');
        throw new Error(`Schema version mismatch: ${errorText}. Supported: ${supported}`);
      }
      throw new Error(`Bulk create paths failed: HTTP ${response.status}: ${errorText}`);
    }

    return await response.json();
  }

  /**
   * Bulk create relationships directly via storage service.
   *
   * @param items - Relationship items to create
   * @returns BulkCreateResult with created count
   */
  async bulkCreateRelationships(
    items: Array<{
      schemaVersion?: number;
      sourceId: string;
      targetId: string;
      relationshipType: string;
      confidence?: number;
      inferenceSource?: string;
      metadataJson?: string;
    }>
  ): Promise<BulkRelationshipResult> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would bulk create ${items.length} relationships`);
      return { created: items.length, errors: [] };
    }

    const response = await this.fetch('/db/relationships/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
      body: JSON.stringify(items),
      timeout: 120000,
    });

    if (!response.ok) {
      const errorText = await response.text();
      if (response.status === 400 && errorText.includes('Unsupported schema version')) {
        const supported = response.headers.get('X-Supported-Schema-Versions');
        throw new Error(`Schema version mismatch: ${errorText}. Supported: ${supported}`);
      }
      throw new Error(`Bulk create relationships failed: HTTP ${response.status}: ${errorText}`);
    }

    return await response.json();
  }

  /**
   * Bulk create contributor presences directly via storage service.
   *
   * @param items - Presence items to create
   * @returns BulkPresenceResult with created count
   */
  async bulkCreatePresences(
    items: Array<{
      displayName: string;
      presenceState?: string;
      externalIdentifiersJson?: string;
      establishingContentIdsJson: string;
      affinityTotal?: number;
      uniqueEngagers?: number;
      citationCount?: number;
      recognitionScore?: number;
    }>
  ): Promise<BulkPresenceResult> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would bulk create ${items.length} contributor presences`);
      return { created: items.length, errors: [] };
    }

    const response = await this.fetch('/db/presences/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
      body: JSON.stringify(items),
      timeout: 120000,
    });

    if (!response.ok) {
      const errorText = await response.text();
      if (response.status === 400 && errorText.includes('Unsupported schema version')) {
        const supported = response.headers.get('X-Supported-Schema-Versions');
        throw new Error(`Schema version mismatch: ${errorText}. Supported: ${supported}`);
      }
      throw new Error(`Bulk create presences failed: HTTP ${response.status}: ${errorText}`);
    }

    return await response.json();
  }

  /**
   * Bulk record economic events directly via storage service.
   *
   * @param items - Economic event items to record
   * @returns BulkEventResult with recorded count
   */
  async bulkRecordEvents(
    items: Array<{
      action: string;
      provider: string;
      receiver: string;
      resourceConformsTo?: string;
      resourceQuantityValue?: number;
      resourceQuantityUnit?: string;
      lamadEventType?: string;
      contentId?: string;
      contributorPresenceId?: string;
      pathId?: string;
      metadataJson?: string;
    }>
  ): Promise<BulkEventResult> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would bulk record ${items.length} economic events`);
      return { recorded: items.length, errors: [] };
    }

    const response = await this.fetch('/db/events/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
      body: JSON.stringify(items),
      timeout: 120000,
    });

    if (!response.ok) {
      const errorText = await response.text();
      if (response.status === 400 && errorText.includes('Unsupported schema version')) {
        const supported = response.headers.get('X-Supported-Schema-Versions');
        throw new Error(`Schema version mismatch: ${errorText}. Supported: ${supported}`);
      }
      throw new Error(`Bulk record events failed: HTTP ${response.status}: ${errorText}`);
    }

    return await response.json();
  }

  /**
   * Bulk upsert content mastery records directly via storage service.
   *
   * @param items - Mastery items to create/update
   * @returns BulkMasteryResult with created/updated counts
   */
  async bulkUpsertMastery(
    items: Array<{
      humanId: string;
      contentId: string;
      masteryLevel?: string;
      masteryLevelIndex?: number;
      freshnessScore?: number;
      engagementCount?: number;
    }>
  ): Promise<BulkMasteryResult> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would bulk upsert ${items.length} mastery records`);
      return { created: items.length, updated: 0, errors: [] };
    }

    const response = await this.fetch('/db/mastery/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
      body: JSON.stringify(items),
      timeout: 120000,
    });

    if (!response.ok) {
      const errorText = await response.text();
      if (response.status === 400 && errorText.includes('Unsupported schema version')) {
        const supported = response.headers.get('X-Supported-Schema-Versions');
        throw new Error(`Schema version mismatch: ${errorText}. Supported: ${supported}`);
      }
      throw new Error(`Bulk upsert mastery failed: HTTP ${response.status}: ${errorText}`);
    }

    return await response.json();
  }

  /**
   * Internal fetch wrapper with auth, timeout, and retry logic.
   *
   * For /db/* paths: Uses storageUrl directly if configured (bypasses doorway proxy).
   * This is useful when doorway's proxy is unavailable but storage is accessible.
   */
  protected async fetch(
    path: string,
    options: {
      method?: string;
      headers?: Record<string, string>;
      body?: Buffer | string;
      timeout?: number;
    } = {}
  ): Promise<Response> {
    // Use storage URL directly for /db/* paths if configured
    const baseUrl = (path.startsWith('/db/') && this.config.storageUrl)
      ? this.config.storageUrl
      : this.config.baseUrl;
    const url = `${baseUrl}${path}`;
    const timeout = options.timeout || this.config.timeout;

    const headers: Record<string, string> = {
      ...options.headers,
    };

    if (this.config.apiKey) {
      headers['Authorization'] = `Bearer ${this.config.apiKey}`;
    }

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
// Pre-flight Validation
// =============================================================================

/**
 * Pre-flight check for seeding readiness.
 *
 * Validates:
 * - Doorway is reachable
 * - Cache is enabled
 * - Admin API is accessible
 */
export async function validateSeedingPrerequisites(
  doorwayUrl: string,
  apiKey?: string
): Promise<{
  ready: boolean;
  issues: string[];
}> {
  const issues: string[] = [];
  const client = new DoorwayClient({ baseUrl: doorwayUrl, apiKey });

  // Check doorway health
  const health = await client.checkHealth();
  if (!health.healthy) {
    issues.push(`Doorway not reachable: ${health.error}`);
    return { ready: false, issues };
  }

  if (!health.cacheEnabled) {
    issues.push('Doorway cache is disabled');
  }

  // Try to get cache stats (validates admin access)
  const stats = await client.getStats();
  if (stats === null) {
    issues.push('Cannot access admin API (check API key)');
  }

  return {
    ready: issues.length === 0,
    issues,
  };
}

export default DoorwayClient;
