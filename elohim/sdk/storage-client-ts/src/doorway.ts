import { StorageError } from './types';

/**
 * DoorwayClient configuration. One client targets one doorway's admin/
 * federation-facing HTTP surface — distinct from {@link StorageClient},
 * which talks to a storage pod's dataplane HTTP API.
 */
export interface DoorwayConfig {
  /** Base URL for the doorway HTTP API */
  baseUrl: string;
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;
}

/** Outcome of a `PUT /admin/seed/blob` upload attempt. */
export interface SeedBlobResult {
  /** True for both a fresh 200/201 upload and an idempotent 409 (already present). */
  ok: boolean;
  /** Raw HTTP status code returned by the doorway. */
  status: number;
  /** True only for the 409 branch — distinct from a fresh 200/201 upload. */
  alreadyPresent: boolean;
  /** First ~300 chars of the response body, set only when `ok` is false. */
  errorBody?: string;
}

/**
 * HTTP client for the doorway gateway's admin/federation-facing surface.
 *
 * Accessed directly (no sub-API composition — DoorwayClient is a
 * standalone peer client, not a `StorageClient` extension):
 * ```typescript
 * const doorway = new DoorwayClient({ baseUrl: 'https://doorway.example' });
 * const result = await doorway.seedBlob({ hash, data });
 * ```
 */
export class DoorwayClient {
  private readonly config: Required<DoorwayConfig>;

  constructor(config: DoorwayConfig) {
    this.config = { timeout: 30000, ...config };
  }

  /** `GET /health`. */
  async getHealth(): Promise<unknown> {
    return this.requestJson('/health');
  }

  /**
   * `PUT /admin/seed/blob` — idempotent blob upload.
   *
   * Sends `X-Blob-Hash` and `Content-Type: application/octet-stream`
   * headers. When `opts.token` is provided, sends
   * `Authorization: Bearer <token>`; when omitted, the `Authorization`
   * header is entirely ABSENT — never sent as `Bearer ` with an empty
   * value.
   *
   * 200/201 → `{ ok: true, alreadyPresent: false }` (fresh upload).
   * 409 → `{ ok: true, alreadyPresent: true }` (idempotent already-present,
   * distinct from a fresh 200/201). Anything else →
   * `{ ok: false, errorBody: <first ~300 chars of response body> }`.
   */
  async seedBlob(opts: {
    hash: string;
    data: Uint8Array;
    token?: string;
  }): Promise<SeedBlobResult> {
    const url = `${this.config.baseUrl}/admin/seed/blob`;
    const headers: Record<string, string> = {
      'X-Blob-Hash': opts.hash,
      'Content-Type': 'application/octet-stream',
    };
    if (opts.token) {
      headers['Authorization'] = `Bearer ${opts.token}`;
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(url, {
        method: 'PUT',
        headers,
        body: opts.data,
        signal: controller.signal,
      });

      if (response.status === 200 || response.status === 201) {
        await response.arrayBuffer();
        return { ok: true, status: response.status, alreadyPresent: false };
      }
      if (response.status === 409) {
        await response.arrayBuffer();
        return { ok: true, status: response.status, alreadyPresent: true };
      }
      const bodyText = await response.text();
      return {
        ok: false,
        status: response.status,
        alreadyPresent: false,
        errorBody: bodyText.slice(0, 300),
      };
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        throw new StorageError(`Request timeout after ${this.config.timeout}ms`);
      }
      throw new StorageError(`Request failed: ${error}`);
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /** `GET /api/v1/federation/doorways`. Consumers read `.doorways[].status`. */
  async listFederationDoorways(): Promise<unknown> {
    return this.requestJson('/api/v1/federation/doorways');
  }

  /** `GET /api/v1/federation/p2p-peers`. Consumers read `.total`, `.peers[].capabilities`. */
  async listFederationP2pPeers(): Promise<unknown> {
    return this.requestJson('/api/v1/federation/p2p-peers');
  }

  private async requestJson(path: string): Promise<unknown> {
    const url = `${this.config.baseUrl}${path}`;
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(url, { method: 'GET', signal: controller.signal });
      if (!response.ok) {
        const responseBody = await response.text();
        throw new StorageError(
          `HTTP ${response.status}: ${response.statusText}`,
          response.status,
          responseBody,
        );
      }
      return await response.json();
    } catch (error) {
      if (error instanceof StorageError) throw error;
      if (error instanceof Error && error.name === 'AbortError') {
        throw new StorageError(`Request timeout after ${this.config.timeout}ms`);
      }
      throw new StorageError(`Request failed: ${error}`);
    } finally {
      clearTimeout(timeoutId);
    }
  }
}
