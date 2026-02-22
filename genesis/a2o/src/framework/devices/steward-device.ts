/**
 * StewardDevice — connects directly to localhost:8090 (elohim-storage sidecar).
 *
 * No doorway proxy — tests the direct storage API path.
 * Implicitly authenticated (local process, no JWT needed).
 */

import { request } from 'undici';

import { Device, type DeviceType } from '../device.js';

export interface StorageStats {
  contentCount: number;
  pathCount: number;
}

export class StewardDevice extends Device {
  readonly type: DeviceType = 'steward';
  readonly label: string;

  constructor(
    label: string,
    private readonly storageUrl = 'http://localhost:8090'
  ) {
    super();
    this.label = label;
  }

  // Steward is implicitly authenticated (local process)
  readonly isAuthenticated = true;

  get url(): string {
    return this.storageUrl;
  }

  /** Check if the local storage service is healthy. */
  async isHealthy(): Promise<boolean> {
    try {
      const { statusCode } = await request(`${this.storageUrl}/db/stats`, {
        method: 'GET',
      });
      return statusCode >= 200 && statusCode < 300;
    } catch {
      return false;
    }
  }

  /** Get storage statistics (content count, path count). */
  async getStats(): Promise<StorageStats> {
    const { statusCode, body } = await request(`${this.storageUrl}/db/stats`, {
      method: 'GET',
    });
    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) {
      throw new Error(`GET /db/stats returned ${statusCode}: ${text}`);
    }
    const data = JSON.parse(text);
    return {
      contentCount: data.content_count ?? data.contentCount ?? 0,
      pathCount: data.path_count ?? data.pathCount ?? 0,
    };
  }

  /** Get content by ID from local storage. */
  async getContent(id: string): Promise<Record<string, unknown>> {
    const { statusCode, body } = await request(`${this.storageUrl}/db/content/${id}`, {
      method: 'GET',
    });
    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) {
      throw new Error(`GET /db/content/${id} returned ${statusCode}: ${text}`);
    }
    return JSON.parse(text);
  }

  /** Create content directly in local storage. */
  async createContent(content: Record<string, unknown>): Promise<Record<string, unknown>> {
    const { statusCode, body } = await request(`${this.storageUrl}/db/content`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(content),
    });
    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) {
      throw new Error(`POST /db/content returned ${statusCode}: ${text}`);
    }
    return JSON.parse(text);
  }

  /** Get all learning paths from local storage. */
  async getPaths(): Promise<Record<string, unknown>[]> {
    const { statusCode, body } = await request(`${this.storageUrl}/db/paths`, {
      method: 'GET',
    });
    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) {
      throw new Error(`GET /db/paths returned ${statusCode}: ${text}`);
    }
    return JSON.parse(text);
  }
}
