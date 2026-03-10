import {
  StorageConfig,
  StorageError,
  ListDocumentsResponse,
  GetDocumentResponse,
  GetHeadsResponse,
  GetChangesResponse,
  ApplyChangesResponse,
  BlobResult,
  BlobManifest,
  ListOptions,
} from './types';

/**
 * HTTP client for elohim-storage sync API
 *
 * @example
 * ```typescript
 * import { StorageClient } from '@elohim/storage-client';
 *
 * const client = new StorageClient({
 *   baseUrl: 'http://localhost:8080',
 *   appId: 'lamad',
 * });
 *
 * // List documents
 * const { documents, total } = await client.listDocuments({ prefix: 'graph', limit: 10 });
 *
 * // Get document heads
 * const { heads } = await client.getHeads('graph:my-doc');
 *
 * // Get changes since known heads
 * const { changes, new_heads } = await client.getChangesSince('graph:my-doc', knownHeads);
 *
 * // Apply changes
 * await client.applyChanges('graph:my-doc', changesBytes);
 * ```
 */
export class StorageClient {
  private config: Required<Omit<StorageConfig, 'apiKey'>> & Pick<StorageConfig, 'apiKey'>;

  constructor(config: StorageConfig) {
    this.config = {
      timeout: 30000,
      ...config,
    };
  }

  /**
   * Make an HTTP request to the storage API
   */
  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    contentType = 'application/json'
  ): Promise<T> {
    const url = `${this.config.baseUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': contentType,
    };

    if (this.config.apiKey) {
      headers['Authorization'] = `Bearer ${this.config.apiKey}`;
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(url, {
        method,
        headers,
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        body: body ? (contentType === 'application/json' ? JSON.stringify(body) : body as any) : undefined,
        signal: controller.signal,
      });

      if (!response.ok) {
        const responseBody = await response.text();
        throw new StorageError(
          `HTTP ${response.status}: ${response.statusText}`,
          response.status,
          responseBody
        );
      }

      if (contentType === 'application/json' || response.headers.get('content-type')?.includes('application/json')) {
        return await response.json() as T;
      }

      return await response.arrayBuffer() as T;
    } catch (error) {
      if (error instanceof StorageError) {
        throw error;
      }
      if (error instanceof Error && error.name === 'AbortError') {
        throw new StorageError(`Request timeout after ${this.config.timeout}ms`);
      }
      throw new StorageError(`Request failed: ${error}`);
    } finally {
      clearTimeout(timeoutId);
    }
  }

  // ==================== Sync API ====================

  /**
   * List documents for this app
   */
  async listDocuments(options: ListOptions = {}): Promise<ListDocumentsResponse> {
    const params = new URLSearchParams();
    if (options.prefix) params.set('prefix', options.prefix);
    if (options.offset !== undefined) params.set('offset', options.offset.toString());
    if (options.limit !== undefined) params.set('limit', options.limit.toString());

    const query = params.toString();
    const path = `/sync/v1/${this.config.appId}/docs${query ? `?${query}` : ''}`;
    return this.request<ListDocumentsResponse>('GET', path);
  }

  /**
   * Get document info
   */
  async getDocument(docId: string): Promise<GetDocumentResponse> {
    return this.request<GetDocumentResponse>(
      'GET',
      `/sync/v1/${this.config.appId}/docs/${encodeURIComponent(docId)}`
    );
  }

  /**
   * Get current heads for a document
   */
  async getHeads(docId: string): Promise<GetHeadsResponse> {
    return this.request<GetHeadsResponse>(
      'GET',
      `/sync/v1/${this.config.appId}/docs/${encodeURIComponent(docId)}/heads`
    );
  }

  /**
   * Get changes since given heads
   *
   * @param docId Document ID
   * @param haveHeads Heads we already have (empty for full document)
   * @returns Changes as base64-encoded blobs and new heads
   */
  async getChangesSince(docId: string, haveHeads: string[] = []): Promise<GetChangesResponse> {
    const haveParam = haveHeads.length > 0 ? `?have=${haveHeads.join(',')}` : '';
    return this.request<GetChangesResponse>(
      'GET',
      `/sync/v1/${this.config.appId}/docs/${encodeURIComponent(docId)}/changes${haveParam}`
    );
  }

  /**
   * Apply changes to a document
   *
   * @param docId Document ID
   * @param changes Array of Automerge change blobs (Uint8Array)
   * @returns New heads after applying changes
   */
  async applyChanges(docId: string, changes: Uint8Array[]): Promise<ApplyChangesResponse> {
    // Encode changes as base64 for JSON transport
    const changesB64 = changes.map((c) => this.encodeBase64(c));

    return this.request<ApplyChangesResponse>(
      'POST',
      `/sync/v1/${this.config.appId}/docs/${encodeURIComponent(docId)}/changes`,
      { changes: changesB64 }
    );
  }

  /**
   * Get document count for this app
   */
  async countDocuments(): Promise<number> {
    const response = await this.listDocuments({ limit: 0 });
    return response.total;
  }

  // ==================== Blob API ====================

  /**
   * Store a blob
   *
   * @param data Blob data
   * @param mimeType MIME type (default: application/octet-stream)
   * @returns Blob result with CID and hash
   */
  async putBlob(data: Uint8Array, mimeType = 'application/octet-stream'): Promise<BlobManifest> {
    const url = `${this.config.baseUrl}/blob/`;
    const headers: Record<string, string> = {
      'Content-Type': mimeType,
    };

    if (this.config.apiKey) {
      headers['Authorization'] = `Bearer ${this.config.apiKey}`;
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(url, {
        method: 'PUT',
        headers,
        body: data,
        signal: controller.signal,
      });

      if (!response.ok) {
        const responseBody = await response.text();
        throw new StorageError(
          `HTTP ${response.status}: ${response.statusText}`,
          response.status,
          responseBody
        );
      }

      return await response.json() as BlobManifest;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Get a blob by hash or CID
   *
   * @param hashOrCid SHA256 hash or CID
   * @returns Blob data
   */
  async getBlob(hashOrCid: string): Promise<Uint8Array> {
    const url = `${this.config.baseUrl}/blob/${encodeURIComponent(hashOrCid)}`;
    const headers: Record<string, string> = {};

    if (this.config.apiKey) {
      headers['Authorization'] = `Bearer ${this.config.apiKey}`;
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(url, {
        method: 'GET',
        headers,
        signal: controller.signal,
      });

      if (!response.ok) {
        if (response.status === 404) {
          throw new StorageError('Blob not found', 404);
        }
        const responseBody = await response.text();
        throw new StorageError(
          `HTTP ${response.status}: ${response.statusText}`,
          response.status,
          responseBody
        );
      }

      const buffer = await response.arrayBuffer();
      return new Uint8Array(buffer);
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Check if a blob exists
   *
   * @param hashOrCid SHA256 hash or CID
   * @returns true if blob exists
   */
  async blobExists(hashOrCid: string): Promise<boolean> {
    const url = `${this.config.baseUrl}/shard/${encodeURIComponent(hashOrCid)}`;
    const headers: Record<string, string> = {};

    if (this.config.apiKey) {
      headers['Authorization'] = `Bearer ${this.config.apiKey}`;
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(url, {
        method: 'HEAD',
        headers,
        signal: controller.signal,
      });

      return response.ok;
    } catch {
      return false;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Get blob manifest
   *
   * @param hashOrCid SHA256 hash or CID
   * @returns Blob manifest with shard info
   */
  async getManifest(hashOrCid: string): Promise<BlobManifest> {
    return this.request<BlobManifest>(
      'GET',
      `/manifest/${encodeURIComponent(hashOrCid)}`
    );
  }

  // ==================== Utility Methods ====================

  /**
   * Encode bytes as base64
   */
  private encodeBase64(data: Uint8Array): string {
    if (typeof Buffer !== 'undefined') {
      // Node.js
      return Buffer.from(data).toString('base64');
    }
    // Browser
    return btoa(String.fromCharCode(...data));
  }

  /**
   * Decode base64 to bytes
   */
  decodeBase64(str: string): Uint8Array {
    if (typeof Buffer !== 'undefined') {
      // Node.js
      return new Uint8Array(Buffer.from(str, 'base64'));
    }
    // Browser
    const binary = atob(str);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes;
  }
}
