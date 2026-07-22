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
  private readonly config: WriteBufferConfig;
  private readonly queues = new Map<WritePriority, Map<string, WriteOp>>();
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
  constructor(private readonly agentReach: ReachLevel = ReachLevel.Commons) {}

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
      case 'private':
        return ReachLevel.Private;
      case 'invited':
        return ReachLevel.Invited;
      case 'local':
        return ReachLevel.Local;
      case 'neighborhood':
        return ReachLevel.Neighborhood;
      case 'municipal':
        return ReachLevel.Municipal;
      case 'bioregional':
        return ReachLevel.Bioregional;
      case 'regional':
        return ReachLevel.Regional;
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
  private readonly mode: ClientMode;
  private readonly writeBuffer: WriteBuffer;
  private readonly reachEnforcer: ReachEnforcer;
  private readonly holochain?: HolochainConnection;
  /**
   * Sticky-preferred doorway host for this client instance (multi-host
   * failover, spec: dual-wan-utility-plane-failover-design §3a). Set when a
   * fallback host proves reachable; reset to null when the primary doorway
   * URL proves reachable again. Never mutates `mode.doorway`.
   */
  private activeDoorwayUrl: string | null = null;

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
      case 'browser': {
        // Browser mode: Holochain WebSocket proxied through doorway
        const doorwayUrl = this.mode.doorway.url;
        const wsUrl = doorwayUrl.replace('https://', 'wss://').replace('http://', 'ws://');
        return `${wsUrl}/conductor`;
      }

      case 'tauri':
        // Tauri mode: direct connection to local conductor
        return this.holochain.directConductorUrl ?? 'ws://localhost:8888';
    }
  }

  /** Get the Holochain app ID */
  getHolochainAppId(): string | null {
    return this.holochain?.hAppId ?? null;
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
    const fetched = await Promise.all(
      ids.map(async id => [id, await this.get<T>(contentType, id)] as const)
    );

    const results = new Map<string, T>();
    for (const [id, content] of fetched) {
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
    const headers: Record<string, string> = {
      ...(options?.headers as Record<string, string>),
    };

    // Use storageUrl directly for /db/* routes if configured (local dev bypass).
    // Direct connection to local storage, not the public doorway plane — no
    // multi-host failover applies here.
    const usingStorage = path.startsWith('/db/') && !!mode.storageUrl;
    let response: Response;
    if (usingStorage) {
      response = await fetch(`${mode.storageUrl}${path}`, { ...options, headers });
    } else {
      // Only include auth header when using doorway (storage doesn't need it in dev)
      if (mode.doorway.apiKey) {
        headers['Authorization'] = `Bearer ${mode.doorway.apiKey}`;
      }
      // Write-shaped methods (POST/PUT/etc.) must not auto-retry on network
      // failure — duplicate-write risk. GET/HEAD fail over across hosts.
      const method = (options?.method ?? 'GET').toUpperCase();
      const allowFailover = method === 'GET' || method === 'HEAD';
      response = await this.fetchWithFailover(
        mode,
        baseUrl => `${baseUrl}${path}`,
        { ...options, headers },
        { allowFailover }
      );
    }

    if (response.status === 404) {
      return null;
    }

    if (!response.ok) {
      const body = await response.text();
      throw new Error(`HTTP ${response.status} - ${body}`);
    }

    return response.json() as Promise<T>;
  }

  /**
   * Failover-aware fetch across the sticky-preferred host, the configured
   * doorway, and any configured fallbacks (spec:
   * dual-wan-utility-plane-failover-design §3a).
   *
   * Only a network-level failure (the `fetch` promise rejecting — DNS,
   * connection-refused, CORS-network class — or a per-attempt timeout)
   * advances to the next candidate host. Any HTTP response, including
   * 4xx/5xx, means the host is reachable and is returned as-is.
   *
   * Read-shaped calls (`allowFailover: true`) try every candidate in order.
   * Write-shaped calls (`allowFailover: false`) try only the first candidate
   * — retrying a write on a different host risks a duplicate — but still
   * advance the sticky preference on failure so the NEXT call skips the
   * unreachable host.
   */
  private async fetchWithFailover(
    mode: BrowserMode,
    buildUrl: (baseUrl: string) => string,
    init: RequestInit | undefined,
    opts: { allowFailover: boolean }
  ): Promise<Response> {
    const hosts = this.candidateDoorwayHosts(mode);

    for (let i = 0; i < hosts.length; i++) {
      const host = hosts[i];
      const attemptInit = this.withPerAttemptTimeout(init);

      // .then/.catch attached synchronously on the fetch() call itself, not
      // via await + try/catch — zone.js checks for unhandled rejections at
      // drain-end, before a native `await`'s thenable job attaches its
      // handler, and can false-flag an otherwise-handled rejection.
      const outcome = await fetch(buildUrl(host), attemptInit).then(
        (response): { kind: 'response'; response: Response } => ({ kind: 'response', response }),
        (error: unknown): { kind: 'error'; error: unknown } => ({ kind: 'error', error })
      );

      if (outcome.kind === 'response') {
        // Reachable — sticky-prefer this host (or reset to primary) for the next call.
        this.activeDoorwayUrl = host === this.normalizeHost(mode.doorway.url) ? null : host;
        return outcome.response;
      }

      // Caller-initiated cancellation must never trigger failover or mutate
      // stickiness — rethrow immediately. The caller signal's `.aborted` flag
      // is the primary discriminator (robust across environments); the
      // error's DOMException 'AbortError' name is a secondary signal, since
      // some environments (Node/undici) may surface the abort as a plain
      // rejection without a caller-visible `.aborted` flip in every path.
      // `AbortSignal.timeout()`-produced 'TimeoutError' rejections are NOT
      // caller cancellation and remain failover triggers, as do TypeError
      // network failures.
      const callerAborted = init?.signal?.aborted === true;
      const isAbortError =
        outcome.error instanceof DOMException && outcome.error.name === 'AbortError';
      if (callerAborted || isAbortError) {
        throw outcome.error;
      }

      const hasMoreCandidates = i < hosts.length - 1;
      if (opts.allowFailover && hasMoreCandidates) {
        continue;
      }

      // Out of candidates, or a write-shaped call that must not auto-retry:
      // advance the sticky preference so the NEXT call skips this unreachable host.
      this.activeDoorwayUrl = hasMoreCandidates ? hosts[i + 1] : null;
      throw outcome.error;
    }

    // Unreachable in practice — candidateDoorwayHosts() always includes mode.doorway.url.
    throw new Error('fetchWithFailover: no candidate hosts configured');
  }

  /** Ordered, deduped candidate hosts: sticky-preferred, then primary, then configured fallbacks. */
  private candidateDoorwayHosts(mode: BrowserMode): string[] {
    const ordered = [
      ...(this.activeDoorwayUrl ? [this.activeDoorwayUrl] : []),
      mode.doorway.url,
      ...(mode.doorway.fallbacks ?? []),
    ].map(host => this.normalizeHost(host));
    return Array.from(new Set(ordered));
  }

  /**
   * Strip trailing slashes so a fallback configured as 'https://host/' joins
   * cleanly with a leading-slash path (no 'host//db/...' double slash) and
   * compares/dedupes correctly against other candidate hosts.
   */
  private normalizeHost(url: string): string {
    return url.replace(/\/+$/, '');
  }

  /** Respect a caller-supplied AbortSignal; otherwise cap each attempt so a black-holing host can't stall failover. */
  private withPerAttemptTimeout(init?: RequestInit): RequestInit {
    if (init?.signal) {
      return init;
    }
    return { ...init, signal: AbortSignal.timeout(8000) };
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
    // All content (including paths) lives in /db/content
    const headers: Record<string, string> = {};

    // Use storageUrl directly for /db/* routes if configured (local dev bypass).
    // Direct connection to local storage, not the public doorway plane — no
    // multi-host failover applies here.
    let response: Response;
    if (mode.storageUrl) {
      response = await fetch(`${mode.storageUrl}/db/content/${id}`, { headers });
    } else {
      // Only include auth header when using doorway (storage doesn't need it in dev)
      if (mode.doorway.apiKey) {
        headers['Authorization'] = `Bearer ${mode.doorway.apiKey}`;
      }
      response = await this.fetchWithFailover(
        mode,
        baseUrl => `${baseUrl}/db/content/${id}`,
        { headers },
        { allowFailover: true }
      );
    }

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
    // All content (including paths) lives in /db/content.
    // contentType is sent as a query parameter for server-side filtering.
    const params = new URLSearchParams();
    if (query.contentType) params.set('contentType', query.contentType);
    if (query.tags?.length) params.set('tags', query.tags.join(','));
    if (query.search) params.set('search', query.search);
    if (query.limit) params.set('limit', String(query.limit));
    if (query.offset) params.set('offset', String(query.offset));

    const headers: Record<string, string> = {};

    // Use storageUrl directly for /db/* routes if configured (local dev bypass).
    // Direct connection to local storage, not the public doorway plane — no
    // multi-host failover applies here.
    let response: Response;
    if (mode.storageUrl) {
      response = await fetch(`${mode.storageUrl}/db/content?${params}`, { headers });
    } else {
      // Only include auth header when using doorway (storage doesn't need it in dev)
      if (mode.doorway.apiKey) {
        headers['Authorization'] = `Bearer ${mode.doorway.apiKey}`;
      }
      response = await this.fetchWithFailover(
        mode,
        baseUrl => `${baseUrl}/db/content?${params}`,
        { headers },
        { allowFailover: true }
      );
    }
    if (!response.ok) {
      const body = await response.text();
      throw new Error(`HTTP ${response.status} - ${body}`);
    }

    // elohim-storage returns { items: [...], count, limit, offset }
    const result = (await response.json()) as { items: T[]; count: number };
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

    for (const [_contentType, ops] of byType) {
      // All content (including paths) goes to /db/content/bulk
      const items = ops.map(op => op.data);

      // Use storageUrl directly for /db/* routes if configured (local dev bypass).
      // Direct connection to local storage, not the public doorway plane — no
      // multi-host failover applies here.
      let response: Response;
      if (mode.storageUrl) {
        response = await fetch(`${mode.storageUrl}/db/content/bulk`, {
          method: 'POST',
          headers,
          body: JSON.stringify(items),
        });
      } else {
        // Write-shaped: no auto-retry on network failure (duplicate-write
        // risk) — the error propagates, but the sticky preference still
        // advances so the NEXT flush skips the unreachable host.
        response = await this.fetchWithFailover(
          mode,
          baseUrl => `${baseUrl}/db/content/bulk`,
          { method: 'POST', headers, body: JSON.stringify(items) },
          { allowFailover: false }
        );
      }

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

  private async queryFromTauri<T>(mode: TauriMode, query: ContentQuery): Promise<T[]> {
    return mode.invoke<T[]>('query_content', { query });
  }

  private async flushToTauri(mode: TauriMode, batch: WriteOp[]): Promise<void> {
    await mode.invoke('bulk_write', { operations: batch });
  }
}
