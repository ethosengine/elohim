/**
 * elohim-client
 *
 * Mode-aware content client for the Elohim Protocol.
 * Mirrors the Rust elohim-sdk patterns for consistency.
 *
 * # Architecture
 *
 * Content operations (heavy R/W) route to elohim-storage → SQLite:
 * - Browser: Doorway → Projection Store (no offline)
 * - Tauri: Local elohim-storage → SQLite (full offline, syncs with elohim-node)
 *
 * Agent-centric data (attestations, identity, points) uses separate
 * Holochain connection configured via `holochain` config option.
 */

import {
  ClientMode,
  BrowserMode,
  TauriMode,
  HolochainConnection,
  ContentType,
  WritePriority,
  ReachLevel,
  WriteBufferConfig,
  WriteOp,
  ElohimClientConfig,
  ContentQuery,
  ContentReadable,
  ContentWriteable,
  WriteBufferDefaults,
} from './types';

/**
 * Write buffer for backpressure protection
 *
 * Queues write operations and flushes in batches to prevent
 * overwhelming the backend during bulk operations.
 */
export class WriteBuffer {
  private config: WriteBufferConfig;
  private queues: Map<WritePriority, Map<string, WriteOp>> = new Map();
  private flushTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(config: WriteBufferConfig = WriteBufferDefaults.default) {
    this.config = config;
    this.queues.set(WritePriority.High, new Map());
    this.queues.set(WritePriority.Normal, new Map());
    this.queues.set(WritePriority.Bulk, new Map());
  }

  /** Queue a write operation */
  async queue(op: WriteOp): Promise<void> {
    const queue = this.queues.get(op.priority)!;
    const key = `${op.contentType}:${op.id}`;

    // Deduplicate by replacing existing op with same key
    queue.set(key, op);

    // Schedule auto-flush if not already scheduled
    if (!this.flushTimer) {
      this.flushTimer = setTimeout(() => {
        this.flushTimer = null;
      }, this.config.maxAgeMs);
    }
  }

  /** Take all queued operations for flushing */
  async takeBatch(): Promise<WriteOp[]> {
    const batch: WriteOp[] = [];

    // Drain queues in priority order
    for (const priority of [WritePriority.High, WritePriority.Normal, WritePriority.Bulk]) {
      const queue = this.queues.get(priority)!;
      for (const op of queue.values()) {
        batch.push(op);
      }
      queue.clear();
    }

    if (this.flushTimer) {
      clearTimeout(this.flushTimer);
      this.flushTimer = null;
    }

    return batch;
  }

  /** Get current backpressure level (0-100) */
  async backpressure(): Promise<number> {
    let total = 0;
    for (const queue of this.queues.values()) {
      total += queue.size;
    }
    return Math.min(100, Math.floor((total / this.config.maxItems) * 100));
  }

  /** Check if buffer should auto-flush */
  async shouldFlush(): Promise<boolean> {
    const bp = await this.backpressure();
    return bp >= this.config.backpressureThreshold;
  }
}

/**
 * Reach enforcer for access control
 */
export class ReachEnforcer {
  constructor(private agentReach: ReachLevel = ReachLevel.Commons) {}

  /** Create enforcer for anonymous access (commons only) */
  static anonymous(): ReachEnforcer {
    return new ReachEnforcer(ReachLevel.Commons);
  }

  /** Create enforcer for authenticated access (regional by default) */
  static authenticated(): ReachEnforcer {
    return new ReachEnforcer(ReachLevel.Regional);
  }

  /** Check if agent can access content at given reach level */
  canAccess(contentReach: ReachLevel): boolean {
    // Agent can access content if their reach >= content's reach requirement
    return this.agentReach >= contentReach;
  }

  /** Parse reach level from string */
  static parseReach(s: string): ReachLevel {
    switch (s.toLowerCase()) {
      case 'private': return ReachLevel.Private;
      case 'invited': return ReachLevel.Invited;
      case 'local': return ReachLevel.Local;
      case 'neighborhood': return ReachLevel.Neighborhood;
      case 'municipal': return ReachLevel.Municipal;
      case 'bioregional': return ReachLevel.Bioregional;
      case 'regional': return ReachLevel.Regional;
      case 'commons':
      case 'public':
      default:
        return ReachLevel.Commons;
    }
  }
}

/**
 * Unified content client for the Elohim Protocol
 *
 * Provides mode-aware content access that automatically routes to
 * the appropriate backend based on deployment mode.
 *
 * Content operations: browser → doorway, tauri → local elohim-storage
 * Agent operations: separate holochain connection (if configured)
 */
export class ElohimClient {
  private mode: ClientMode;
  private writeBuffer: WriteBuffer;
  private reachEnforcer: ReachEnforcer;
  private holochain?: HolochainConnection;

  constructor(config: ElohimClientConfig) {
    this.mode = config.mode;
    this.holochain = config.holochain;

    // Select buffer config based on mode
    const bufferConfig = config.writeBuffer
      ? { ...WriteBufferDefaults.default, ...config.writeBuffer }
      : this.getDefaultBufferConfig();

    this.writeBuffer = new WriteBuffer(bufferConfig);
    this.reachEnforcer = new ReachEnforcer(config.agentReach ?? ReachLevel.Regional);
  }

  /** Create client for anonymous browser access */
  static anonymousBrowser(doorwayUrl: string): ElohimClient {
    return new ElohimClient({
      mode: { type: 'browser', doorway: { url: doorwayUrl } },
      agentReach: ReachLevel.Commons,
    });
  }

  /** Get the client mode */
  getMode(): ClientMode {
    return this.mode;
  }

  /** Check if this mode supports offline operation */
  supportsOffline(): boolean {
    return this.mode.type === 'tauri';
  }

  /** Check if this mode requires doorway */
  requiresDoorway(): boolean {
    return this.mode.type === 'browser';
  }

  /** Check if Holochain connection is configured */
  hasHolochain(): boolean {
    return this.holochain?.enabled ?? false;
  }

  /**
   * Get the effective Holochain WebSocket URL based on mode
   *
   * - Browser: proxied through doorway (wss://doorway/conductor)
   * - Tauri: direct connection to local conductor
   *
   * Returns null if Holochain is not configured.
   */
  getHolochainUrl(): string | null {
    if (!this.holochain?.enabled) return null;

    switch (this.mode.type) {
      case 'browser':
        // Browser mode: Holochain WebSocket proxied through doorway
        const doorwayUrl = this.mode.doorway.url;
        const wsUrl = doorwayUrl
          .replace('https://', 'wss://')
          .replace('http://', 'ws://');
        return `${wsUrl}/conductor`;

      case 'tauri':
        // Tauri mode: direct connection to local conductor
        return this.holochain.directConductorUrl ?? 'ws://localhost:8888';
    }
  }

  /** Get the Holochain app ID */
  getHolochainAppId(): string | null {
    return this.holochain?.appId ?? null;
  }

  // === Content Operations ===

  /**
   * Get content by ID
   *
   * Routes to appropriate backend based on mode:
   * - Browser: GET {doorway}/api/v1/cache/{type}/{id}
   * - Tauri: IPC call to local elohim-storage
   */
  async get<T extends ContentReadable>(contentType: ContentType, id: string): Promise<T | null> {
    switch (this.mode.type) {
      case 'browser':
        return this.getFromProjection<T>(this.mode, contentType, id);

      case 'tauri':
        return this.getFromTauri<T>(this.mode, contentType, id);
    }
  }

  /**
   * Get multiple content items by ID
   */
  async getBatch<T extends ContentReadable>(
    contentType: ContentType,
    ids: string[]
  ): Promise<Map<string, T>> {
    const results = new Map<string, T>();

    // TODO: Implement batch endpoint for better performance
    for (const id of ids) {
      const content = await this.get<T>(contentType, id);
      if (content) {
        results.set(id, content);
      }
    }

    return results;
  }

  /**
   * Query content with filters
   */
  async query<T extends ContentReadable>(query: ContentQuery): Promise<T[]> {
    switch (this.mode.type) {
      case 'browser':
        return this.queryFromProjection<T>(this.mode, query);

      case 'tauri':
        return this.queryFromTauri<T>(this.mode, query);
    }
  }

  /**
   * Save content (queues for write buffer)
   *
   * Content is queued in the write buffer and will be flushed
   * to the backend when threshold is reached or flush() is called.
   */
  async save<T extends ContentWriteable>(
    contentType: ContentType,
    content: T,
    priority: WritePriority = WritePriority.Normal
  ): Promise<void> {
    // Run validation if defined
    if (content.validate) {
      content.validate();
    }

    const op: WriteOp = {
      contentType,
      id: content.id,
      data: content,
      priority,
      queuedAt: Date.now(),
    };

    await this.writeBuffer.queue(op);

    // Auto-flush if backpressure is high
    if (await this.writeBuffer.shouldFlush()) {
      await this.flush();
    }
  }

  /**
   * Save content with high priority (flushes immediately)
   */
  async saveImmediate<T extends ContentWriteable>(
    contentType: ContentType,
    content: T
  ): Promise<void> {
    await this.save(contentType, content, WritePriority.High);
    await this.flush();
  }

  /**
   * Flush pending writes to backend
   */
  async flush(): Promise<void> {
    const batch = await this.writeBuffer.takeBatch();
    if (batch.length === 0) return;

    switch (this.mode.type) {
      case 'browser':
        await this.flushToProjection(this.mode, batch);
        break;

      case 'tauri':
        await this.flushToTauri(this.mode, batch);
        break;
    }
  }

  /**
   * Get current backpressure level (0-100)
   */
  async backpressure(): Promise<number> {
    return this.writeBuffer.backpressure();
  }

  // === Raw HTTP Operations ===

  /**
   * Make a raw HTTP request to elohim-storage
   *
   * Useful for endpoints not covered by the standard get/query methods.
   * Automatically handles mode detection and authentication.
   *
   * @param path - API path (e.g., '/db/relationships')
   * @param options - Optional fetch options
   * @returns Parsed JSON response or null on 404
   */
  async fetch<T>(path: string, options?: RequestInit): Promise<T | null> {
    switch (this.mode.type) {
      case 'browser':
        return this.fetchFromProjection<T>(this.mode, path, options);

      case 'tauri':
        return this.fetchFromTauri<T>(this.mode, path, options);
    }
  }

  private async fetchFromProjection<T>(
    mode: BrowserMode,
    path: string,
    options?: RequestInit
  ): Promise<T | null> {
    // Use storageUrl directly for /db/* routes if configured (local dev bypass)
    const baseUrl = (path.startsWith('/db/') && mode.storageUrl)
      ? mode.storageUrl
      : mode.doorway.url;
    const url = `${baseUrl}${path}`;

    const headers: Record<string, string> = {
      ...(options?.headers as Record<string, string>),
    };
    // Only include auth header when using doorway (storage doesn't need it in dev)
    const usingStorage = path.startsWith('/db/') && mode.storageUrl;
    if (!usingStorage && mode.doorway.apiKey) {
      headers['Authorization'] = `Bearer ${mode.doorway.apiKey}`;
    }

    const response = await fetch(url, { ...options, headers });

    if (response.status === 404) {
      return null;
    }

    if (!response.ok) {
      const body = await response.text();
      throw new Error(`HTTP ${response.status} - ${body}`);
    }

    return response.json() as Promise<T>;
  }

  private async fetchFromTauri<T>(
    mode: TauriMode,
    path: string,
    options?: RequestInit
  ): Promise<T | null> {
    // For Tauri mode, we call the local storage HTTP server
    const storageUrl = mode.storageUrl ?? 'http://localhost:8090';
    const url = `${storageUrl}${path}`;

    const response = await fetch(url, options);

    if (response.status === 404) {
      return null;
    }

    if (!response.ok) {
      const body = await response.text();
      throw new Error(`HTTP ${response.status} - ${body}`);
    }

    return response.json() as Promise<T>;
  }

  // === Private Implementation ===

  private getDefaultBufferConfig(): WriteBufferConfig {
    switch (this.mode.type) {
      case 'browser':
        return WriteBufferDefaults.interactive;
      case 'tauri':
        return WriteBufferDefaults.default;
    }
  }

  // --- Browser/Projection Mode ---

  private async getFromProjection<T extends ContentReadable>(
    mode: BrowserMode,
    contentType: ContentType,
    id: string
  ): Promise<T | null> {
    // Map content type to elohim-storage route
    // 'content' → /db/content/, 'path' → /db/paths/
    const route = contentType === 'path' ? 'paths' : contentType;
    // Use storageUrl directly for /db/* routes if configured (local dev bypass)
    const baseUrl = mode.storageUrl ?? mode.doorway.url;
    const url = `${baseUrl}/db/${route}/${id}`;

    const headers: Record<string, string> = {};
    // Only include auth header when using doorway (storage doesn't need it in dev)
    if (!mode.storageUrl && mode.doorway.apiKey) {
      headers['Authorization'] = `Bearer ${mode.doorway.apiKey}`;
    }

    const response = await fetch(url, { headers });

    if (response.status === 404) {
      return null;
    }

    if (!response.ok) {
      const body = await response.text();
      throw new Error(`HTTP ${response.status} - ${body}`);
    }

    return response.json() as Promise<T>;
  }

  private async queryFromProjection<T extends ContentReadable>(
    mode: BrowserMode,
    query: ContentQuery
  ): Promise<T[]> {
    // Map content type to elohim-storage route
    // Default to 'content' if not specified
    const contentType = query.contentType ?? 'content';
    const route = contentType === 'path' ? 'paths' : contentType;

    const params = new URLSearchParams();
    if (query.contentType) params.set('content_type', query.contentType);
    if (query.tags?.length) params.set('tags', query.tags.join(','));
    if (query.search) params.set('search', query.search);
    if (query.limit) params.set('limit', String(query.limit));
    if (query.offset) params.set('offset', String(query.offset));

    // Use storageUrl directly for /db/* routes if configured (local dev bypass)
    const baseUrl = mode.storageUrl ?? mode.doorway.url;
    const url = `${baseUrl}/db/${route}?${params}`;

    const headers: Record<string, string> = {};
    // Only include auth header when using doorway (storage doesn't need it in dev)
    if (!mode.storageUrl && mode.doorway.apiKey) {
      headers['Authorization'] = `Bearer ${mode.doorway.apiKey}`;
    }

    const response = await fetch(url, { headers });
    if (!response.ok) {
      const body = await response.text();
      throw new Error(`HTTP ${response.status} - ${body}`);
    }

    // elohim-storage returns { items: [...], count, limit, offset }
    const result = await response.json() as { items: T[], count: number };
    return result.items ?? [];
  }

  private async flushToProjection(mode: BrowserMode, batch: WriteOp[]): Promise<void> {
    // Group by content type
    const byType = new Map<ContentType, WriteOp[]>();
    for (const op of batch) {
      const ops = byType.get(op.contentType) ?? [];
      ops.push(op);
      byType.set(op.contentType, ops);
    }

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    // Only include auth header when using doorway (storage doesn't need it in dev)
    if (!mode.storageUrl && mode.doorway.apiKey) {
      headers['Authorization'] = `Bearer ${mode.doorway.apiKey}`;
    }

    // Use storageUrl directly for /db/* routes if configured (local dev bypass)
    const baseUrl = mode.storageUrl ?? mode.doorway.url;

    for (const [contentType, ops] of byType) {
      // Map content type to elohim-storage route
      // 'content' → /db/content/, 'path' → /db/paths/
      const route = contentType === 'path' ? 'paths' : contentType;
      const url = `${baseUrl}/db/${route}/bulk`;
      const items = ops.map(op => op.data);

      const response = await fetch(url, {
        method: 'POST',
        headers,
        body: JSON.stringify(items),
      });

      if (!response.ok) {
        console.error(`Failed to flush ${ops.length} items: HTTP ${response.status}`);
      }
    }
  }

  // --- Tauri Mode ---

  private async getFromTauri<T>(
    mode: TauriMode,
    contentType: ContentType,
    id: string
  ): Promise<T | null> {
    return mode.invoke<T | null>('get_content', { contentType, id });
  }

  private async queryFromTauri<T>(
    mode: TauriMode,
    query: ContentQuery
  ): Promise<T[]> {
    return mode.invoke<T[]>('query_content', { query });
  }

  private async flushToTauri(mode: TauriMode, batch: WriteOp[]): Promise<void> {
    await mode.invoke('bulk_write', { operations: batch });
  }
}
