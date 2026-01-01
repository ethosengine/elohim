/**
 * Doorway Client - HTTP client for projection cache operations
 *
 * Provides APIs for:
 * - Pushing blobs to projection cache
 * - Checking blob existence
 * - Validating cache availability
 *
 * Architecture:
 * - Doorway exposes /store/{hash} for blob serving
 * - Seeding uses admin API to push blobs
 * - Cache is eventually consistent with DHT
 *
 * Usage:
 *   const client = new DoorwayClient({ baseUrl: 'https://doorway.example.com', apiKey: 'xxx' });
 *   await client.checkHealth();
 *   await client.pushBlob(hash, blob, mimeType);
 */

import { BlobMetadata } from './blob-manager.js';

// =============================================================================
// Types
// =============================================================================

export interface DoorwayClientConfig {
  /** Base URL of the doorway (e.g., https://doorway.example.com) */
  baseUrl: string;
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
    connected_workers: number;
    total_workers: number;
  };
  error?: string;
}

/** Comprehensive status from /status endpoint */
export interface DoorwayStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  version?: string;
  uptime_seconds?: number;
  conductor: {
    connected: boolean;
    connected_workers: number;
    total_workers: number;
  };
  storage: {
    healthy: boolean;
    import_enabled: boolean;
    active_batches?: number;
    url?: string;
  };
  diagnostics: {
    cell_discovered: boolean;
    ready_for_seeding: boolean;
    recommendations: string[];
  };
  cache?: {
    enabled: boolean;
    entries?: number;
    size_bytes?: number;
  };
}

export interface PushResult {
  success: boolean;
  hash: string;
  cached: boolean;
  error?: string;
}

export interface BatchPushResult {
  success: number;
  failed: number;
  errors: Array<{ hash: string; error: string }>;
}

// =============================================================================
// Import API Types
// =============================================================================

export interface ImportQueueRequest {
  /** Optional batch ID (generated if not provided) */
  batch_id?: string;
  /** Hash of the blob in elohim-storage containing the items JSON
   *  REQUIRED - upload to storage first, then queue import */
  blob_hash: string;
  /** Total number of items in the blob */
  total_items: number;
  /** Schema version for the import data */
  schema_version?: number;
}

export interface ImportQueueResponse {
  batch_id: string;
  queued_count: number;
  processing: boolean;
  message?: string;
}

export interface ImportStatusResponse {
  batch_id: string;
  status: 'queued' | 'processing' | 'completed' | 'failed';
  total_items: number;
  processed_count: number;
  error_count: number;
  errors: string[];
  completed_at?: string;
}

// =============================================================================
// Doorway Client
// =============================================================================

export class DoorwayClient {
  private config: Required<DoorwayClientConfig>;

  constructor(config: DoorwayClientConfig) {
    this.config = {
      baseUrl: config.baseUrl.replace(/\/$/, ''), // Remove trailing slash
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
      const conductorWorkers = data.conductor?.connected_workers ?? 0;
      const totalWorkers = data.conductor?.total_workers ?? 0;

      // Include conductor status in health response
      return {
        healthy: conductorConnected, // Only healthy if conductor is connected
        version: data.version,
        cacheEnabled: data.cache?.enabled ?? true,
        conductor: {
          connected: conductorConnected,
          connected_workers: conductorWorkers,
          total_workers: totalWorkers,
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
        uptime_seconds: data.uptime_seconds,
        conductor: {
          connected: data.conductor?.connected ?? false,
          connected_workers: data.conductor?.connected_workers ?? 0,
          total_workers: data.conductor?.total_workers ?? 0,
        },
        storage: {
          healthy: data.storage?.healthy ?? false,
          import_enabled: data.storage?.import_enabled ?? false,
          active_batches: data.storage?.active_batches,
          url: data.storage?.url,
        },
        diagnostics: {
          cell_discovered: data.diagnostics?.cell_discovered ?? false,
          ready_for_seeding: data.diagnostics?.ready_for_seeding ?? false,
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
   */
  async pushBlob(
    hash: string,
    data: Buffer,
    metadata: BlobMetadata
  ): Promise<PushResult> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would push blob: ${hash} (${data.length} bytes)`);
      return { success: true, hash, cached: false };
    }

    // Check if already cached
    const exists = await this.blobExists(hash);
    if (exists) {
      return { success: true, hash, cached: true };
    }

    try {
      const response = await this.fetch('/admin/seed/blob', {
        method: 'PUT',
        headers: {
          'Content-Type': metadata.mimeType,
          'X-Blob-Hash': hash,
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
          hash,
          cached: false,
          error: `HTTP ${response.status}: ${errorText}`,
        };
      }

      return { success: true, hash, cached: false };
    } catch (error) {
      return {
        success: false,
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
          console.log(`  ✓ ${blob.hash} (already cached)`);
        } else {
          console.log(`  ✓ ${blob.hash} (pushed ${blob.data.length} bytes)`);
        }
      } else {
        failed++;
        errors.push({ hash: blob.hash, error: result.error || 'Unknown error' });
        console.error(`  ✗ ${blob.hash}: ${result.error}`);
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
          app_id: appId,
          blob_hash: blobHash,
          entry_point: entryPoint,
          fallback_url: fallbackUrl,
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
   * @param request - Import request with items or blob_hash
   * @returns Import queue response with batch_id
   */
  async queueImport(
    batchType: string,
    request: ImportQueueRequest
  ): Promise<ImportQueueResponse> {
    if (this.config.dryRun) {
      console.log(`[DRY RUN] Would queue import: ${batchType} (${request.total_items} items, blob: ${request.blob_hash.slice(0, 20)}...)`);
      return {
        batch_id: `dry-run-${Date.now()}`,
        queued_count: request.total_items,
        processing: false,
        message: 'Dry run - not actually queued',
      };
    }

    // Retry loop for 503 (discovery in progress) errors
    const maxRetries = 12; // ~60 seconds with 5 second delays
    const retryDelay = 5000;

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      const response = await this.fetch(`/import/${batchType}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(request),
      });

      if (response.ok) {
        return await response.json();
      }

      // Read body once
      const responseText = await response.text();

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
    console.log(`📤 Import queued: ${queueResult.batch_id} (${queueResult.queued_count} items)`);

    // If already completed (synchronous processing)
    if (queueResult.processing === false && queueResult.queued_count === 0) {
      // Might be an error or already processed
      const status = await this.getImportStatus(batchType, queueResult.batch_id);
      if (status) return status;
    }

    // Poll for completion
    const startTime = Date.now();
    let lastProcessed = 0;

    while (Date.now() - startTime < timeout) {
      await new Promise(resolve => setTimeout(resolve, pollInterval));

      const status = await this.getImportStatus(batchType, queueResult.batch_id);
      if (!status) {
        throw new Error(`Batch ${queueResult.batch_id} not found during polling`);
      }

      // Report progress
      if (status.processed_count !== lastProcessed) {
        if (options.onProgress) {
          options.onProgress(status);
        }
        lastProcessed = status.processed_count;
      }

      // Check for completion
      if (status.status === 'completed' || status.status === 'failed') {
        return status;
      }
    }

    throw new Error(`Import timed out after ${timeout}ms`);
  }

  /**
   * Internal fetch wrapper with auth, timeout, and retry logic.
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
