/**
 * Doorway Client Service
 *
 * HTTP client for the Doorway cache API.
 * elohim-app uses this to read cached content instead of WebSocket zome calls.
 *
 * Routes:
 * - GET /api/v1/cache/{type}/{id} - Get single document
 * - GET /api/v1/cache/{type}?limit=N - Query documents by type
 */

export interface DoorwayClientConfig {
  /** Base URL for the doorway (e.g., "https://doorway-dev.elohim.host") */
  baseUrl: string;
  /** Optional API key for authentication */
  apiKey?: string;
  /** Request timeout in ms (default: 5000) */
  timeout?: number;
}

export interface CacheQueryParams {
  limit?: number;
  skip?: number;
}

export interface DoorwayResponse<T> {
  data: T | null;
  error?: string;
  status: number;
  headers: Record<string, string>;
}

/**
 * Doorway Cache Client
 *
 * Provides methods to query the doorway's generic cache API.
 */
export class DoorwayClient {
  private config: DoorwayClientConfig;

  constructor(config: DoorwayClientConfig) {
    this.config = {
      timeout: 5000,
      ...config,
    };
  }

  /**
   * Get a single document by type and ID
   *
   * @param type - Document type (e.g., "Content", "LearningPath")
   * @param id - Document ID
   * @returns The document data or null if not found
   */
  async get<T>(type: string, id: string): Promise<DoorwayResponse<T>> {
    const url = this.buildUrl(`/api/v1/cache/${encodeURIComponent(type)}/${encodeURIComponent(id)}`);
    return this.fetch<T>(url);
  }

  /**
   * Query documents by type
   *
   * @param type - Document type (e.g., "Content", "LearningPath")
   * @param params - Optional query parameters (limit, skip)
   * @returns Array of documents
   */
  async query<T>(type: string, params?: CacheQueryParams): Promise<DoorwayResponse<T[]>> {
    const url = this.buildUrl(`/api/v1/cache/${encodeURIComponent(type)}`, params);
    return this.fetch<T[]>(url);
  }

  /**
   * Get all content nodes
   */
  async getAllContent<T>(): Promise<DoorwayResponse<T[]>> {
    return this.query<T>('Content', { limit: 1000 });
  }

  /**
   * Get a content node by ID
   */
  async getContent<T>(id: string): Promise<DoorwayResponse<T>> {
    return this.get<T>('Content', id);
  }

  /**
   * Get all learning paths
   */
  async getAllPaths<T>(): Promise<DoorwayResponse<T[]>> {
    return this.query<T>('LearningPath', { limit: 100 });
  }

  /**
   * Get a learning path by ID
   */
  async getPath<T>(id: string): Promise<DoorwayResponse<T>> {
    return this.get<T>('LearningPath', id);
  }

  /**
   * Build URL with query parameters
   */
  private buildUrl(path: string, params?: Record<string, unknown>): string {
    const url = new URL(path, this.config.baseUrl);

    if (params) {
      for (const [key, value] of Object.entries(params)) {
        if (value !== undefined && value !== null) {
          url.searchParams.set(key, String(value));
        }
      }
    }

    // Add API key if configured
    if (this.config.apiKey) {
      url.searchParams.set('apiKey', this.config.apiKey);
    }

    return url.toString();
  }

  /**
   * Make HTTP request with timeout
   */
  private async fetch<T>(url: string): Promise<DoorwayResponse<T>> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(url, {
        method: 'GET',
        headers: {
          'Accept': 'application/json',
        },
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      // Extract headers we care about
      const headers: Record<string, string> = {};
      for (const key of ['x-reach', 'x-cache', 'cache-control', 'etag']) {
        const value = response.headers.get(key);
        if (value) headers[key] = value;
      }

      if (!response.ok) {
        const errorBody = await response.text();
        let errorMessage = `HTTP ${response.status}`;
        try {
          const errorJson = JSON.parse(errorBody);
          errorMessage = errorJson.error || errorMessage;
        } catch {
          errorMessage = errorBody || errorMessage;
        }

        return {
          data: null,
          error: errorMessage,
          status: response.status,
          headers,
        };
      }

      const data = await response.json() as T;
      return {
        data,
        status: response.status,
        headers,
      };
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof Error && error.name === 'AbortError') {
        return {
          data: null,
          error: 'Request timeout',
          status: 0,
          headers: {},
        };
      }

      return {
        data: null,
        error: error instanceof Error ? error.message : 'Unknown error',
        status: 0,
        headers: {},
      };
    }
  }
}

/**
 * Create a doorway client for the dev environment
 */
export function createDevDoorwayClient(): DoorwayClient {
  return new DoorwayClient({
    baseUrl: 'https://doorway-dev.elohim.host',
    apiKey: 'dev-elohim-auth-2024',
  });
}

/**
 * Create a doorway client from environment variables
 */
export function createDoorwayClientFromEnv(): DoorwayClient {
  const baseUrl = process.env['DOORWAY_URL'] || 'https://doorway-dev.elohim.host';
  const apiKey = process.env['DOORWAY_API_KEY'] || 'dev-elohim-auth-2024';
  const timeout = parseInt(process.env['DOORWAY_TIMEOUT'] || '5000', 10);

  return new DoorwayClient({ baseUrl, apiKey, timeout });
}
