/**
 * Content Resolver - Unified Tiered Content Source Resolution
 *
 * Provides O(1) content resolution with source learning.
 * Supports both WASM (high-performance) and pure TypeScript (portable) backends.
 *
 * Resolution order:
 * 1. Local (IndexedDB, in-memory) - fastest, offline-capable
 * 2. Projection (Doorway's MongoDB cache) - fast, eventually consistent
 * 3. Authoritative (Conductor → Edgenode → DHT) - slow, source of truth
 * 4. External (fallback URLs) - last resort
 *
 * Usage:
 * ```typescript
 * import { createContentResolver, SourceTier } from '@aspect/elohim-service/cache';
 *
 * const resolver = await createContentResolver();
 *
 * resolver.registerSource('indexeddb', SourceTier.Local, 100, ['path', 'content']);
 * resolver.registerSource('projection', SourceTier.Projection, 80, ['path', 'content'], 'https://doorway.example.com');
 * resolver.registerSource('conductor', SourceTier.Authoritative, 50, ['path', 'content', 'blob']);
 *
 * const result = resolver.resolve('content', 'my-content-id');
 * // { sourceId: 'indexeddb', tier: 0, url: null, cached: false }
 *
 * // After successful fetch, record location
 * resolver.recordContentLocation('my-content-id', 'indexeddb');
 * ```
 */

// ============================================================================
// Delivery Peer Scoring
// ============================================================================

/** Peer information from storage's /api/v1/peers/delivery */
export interface DeliveryPeer {
  peerId: string;
  multiaddrs: string[];
  network: 'lan' | 'wan' | 'relay';
  capabilities: string[];
  lastSeen: number;
  httpPort: number;
}

/** Scored peer ready for ranked delivery attempts */
export interface ScoredPeer {
  peerId: string;
  baseUrl: string;
  score: number;
  network: string;
  servesExtracted: boolean;
  servesCompressed: boolean;
  warm: boolean;
}

/**
 * Extract an IPv4 address from a multiaddr string.
 *
 * @example
 * extractIpFromMultiaddr('/ip4/192.168.1.50/tcp/9876') // '192.168.1.50'
 * extractIpFromMultiaddr('/ip6/::1/tcp/9876') // ''
 */
export function extractIpFromMultiaddr(addr: string): string {
  const match = addr.match(/\/ip4\/([^/]+)/);
  return match ? match[1] : '';
}

/**
 * Score a single delivery peer for a specific content hash.
 *
 * Scoring factors (descending weight):
 * - Network proximity: LAN 1000, WAN 500, relay 100
 * - Warm cache for this content: +300
 * - Serves extracted files: +200
 * - Serves compressed bundles: +50
 * - Recency (lastSeen): <30s +100, <90s +50
 */
export function scorePeer(peer: DeliveryPeer, contentHash: string): ScoredPeer {
  let score = 0;

  // Network proximity (biggest factor)
  if (peer.network === 'lan') score += 1000;
  else if (peer.network === 'wan') score += 500;
  else score += 100; // relay

  const servesExtracted = peer.capabilities.includes('serves_extracted');
  const servesCompressed = peer.capabilities.includes('serves_compressed');
  const warm = peer.capabilities.includes(`warm:${contentHash}`);

  // Delivery capability
  if (servesExtracted) score += 200;
  if (servesCompressed) score += 50;

  // Warm cache for THIS content
  if (warm) score += 300;

  // Recency
  const age = Date.now() - peer.lastSeen;
  if (age < 30000) score += 100;
  else if (age < 90000) score += 50;

  // Construct baseUrl from multiaddr (extract IP) + httpPort
  const ip = extractIpFromMultiaddr(peer.multiaddrs[0] || '');
  const baseUrl = ip ? `http://${ip}:${peer.httpPort}` : '';

  return {
    peerId: peer.peerId,
    baseUrl,
    score,
    network: peer.network,
    servesExtracted,
    servesCompressed,
    warm,
  };
}

/**
 * Score and rank an array of delivery peers for a specific content hash.
 *
 * Returns peers sorted by score descending, filtered to only those
 * with a reachable baseUrl (i.e., have a parseable IPv4 multiaddr).
 */
export function scorePeersForContent(
  peers: DeliveryPeer[],
  contentHash: string
): ScoredPeer[] {
  return peers
    .map(p => scorePeer(p, contentHash))
    .filter(p => p.baseUrl !== '')
    .sort((a, b) => b.score - a.score);
}

// ============================================================================
// Types
// ============================================================================

/** Source tier priority (lower = higher priority) */
export enum SourceTier {
  /** Local storage (IndexedDB, in-memory) - fastest, offline-capable */
  Local = 0,
  /** Projection cache (Doorway's MongoDB) - fast, eventually consistent */
  Projection = 1,
  /** Authoritative source (Conductor → Edgenode → DHT) - slow, source of truth */
  Authoritative = 2,
  /** External fallback (URLs outside the network) - last resort */
  External = 3,
}

/** Resolution result */
export interface ResolutionResult {
  /** Source to try */
  sourceId: string;
  /** Tier of the source */
  tier: SourceTier;
  /** URL if URL-based source */
  url: string | null;
  /** Whether this came from content index (previously found here) */
  cached: boolean;
}

/** Resolution error */
export interface ResolutionError {
  error: string;
  contentType: string;
  contentId: string;
}

/** App resolution result */
export interface AppResolutionResult {
  url: string | null;
  sourceId: string | null;
  blobHash: string | null;
  fallbackUrl: string | null;
}

/** Resolver statistics */
export interface ResolverStats {
  resolutionCount: number;
  cacheHitCount: number;
  cacheHitRate: number;
  sourceCount: number;
  indexedContentCount: number;
  registeredAppCount: number;
}

/** Source info for chain display */
export interface SourceInfo {
  id: string;
  tier: SourceTier;
  priority: number;
  url: string | null;
}

/** Content resolver interface */
export interface IContentResolver {
  /** Register a content source */
  registerSource(
    id: string,
    tier: SourceTier,
    priority: number,
    contentTypes: string[],
    baseUrl?: string
  ): void;

  /** Update source URL */
  setSourceUrl(sourceId: string, baseUrl: string | null): void;

  /** Mark source available/unavailable */
  setSourceAvailable(sourceId: string, available: boolean): void;

  /** Check if source is available */
  isSourceAvailable(sourceId: string): boolean;

  /** Record that content was found at a source */
  recordContentLocation(contentId: string, sourceId: string): void;

  /** Remove content location */
  removeContentLocation(contentId: string, sourceId: string): void;

  /** Clear all locations for a source */
  clearSourceLocations(sourceId: string): void;

  /** Resolve which source to try */
  resolve(contentType: string, contentId: string): ResolutionResult | ResolutionError;

  /** Get ordered sources for content type */
  getResolutionChain(contentType: string): SourceInfo[];

  /** Register an HTML5 app */
  registerApp(appId: string, blobHash: string, entryPoint: string, fallbackUrl?: string): void;

  /** Unregister an app */
  unregisterApp(appId: string): void;

  /** Check if app is registered */
  hasApp(appId: string): boolean;

  /** Get app blob hash */
  getAppBlobHash(appId: string): string | null;

  /** Resolve app URL */
  resolveAppUrl(appId: string, path?: string): string;

  /** Resolve app URL with full metadata */
  resolveAppUrlFull(appId: string, path?: string): AppResolutionResult;

  /** Get statistics */
  getStats(): ResolverStats;

  /** Reset statistics */
  resetStats(): void;

  /** Get counts */
  sourceCount(): number;
  indexedContentCount(): number;
  registeredAppCount(): number;

  /** Cleanup */
  dispose(): void;
}

/** Config for resolver creation */
export interface ResolverConfig {
  preferWasm?: boolean;
}

/** Initialization result */
export interface ResolverInitResult {
  resolver: IContentResolver;
  implementation: 'wasm' | 'typescript';
}

// ============================================================================
// WASM Module Types
// ============================================================================

interface WasmContentResolver {
  register_source(
    id: string,
    tier: number,
    priority: number,
    contentTypesJson: string,
    baseUrl: string | null
  ): void;
  set_source_url(sourceId: string, baseUrl: string | null): void;
  set_source_available(sourceId: string, available: boolean): void;
  is_source_available(sourceId: string): boolean;
  record_content_location(contentId: string, sourceId: string): void;
  remove_content_location(contentId: string, sourceId: string): void;
  clear_source_locations(sourceId: string): void;
  resolve(contentType: string, contentId: string): string;
  get_resolution_chain(contentType: string): string;
  register_app(
    appId: string,
    blobHash: string,
    entryPoint: string,
    fallbackUrl: string | null
  ): void;
  unregister_app(appId: string): void;
  has_app(appId: string): boolean;
  get_app_blob_hash(appId: string): string | null;
  resolve_app_url(appId: string, path: string | null): string;
  resolve_app_url_full(appId: string, path: string | null): string;
  get_stats(): string;
  reset_stats(): void;
  source_count(): number;
  indexed_content_count(): number;
  registered_app_count(): number;
  free(): void;
}

interface WasmModule {
  ContentResolver: new () => WasmContentResolver;
  SourceTier: {
    Local: number;
    Projection: number;
    Authoritative: number;
    External: number;
  };
}

// ============================================================================
// TypeScript Implementation (Fallback)
// ============================================================================

interface ContentSource {
  id: string;
  tier: SourceTier;
  priority: number;
  contentTypes: string[];
  available: boolean;
  baseUrl: string | null;
}

interface AppRegistration {
  blobHash: string;
  entryPoint: string;
  fallbackUrl: string | null;
  registeredAt: number;
}

/**
 * Pure TypeScript content resolver implementation.
 */
export class TsContentResolver implements IContentResolver {
  private sources: ContentSource[] = [];
  private readonly contentIndex = new Map<string, { sourceId: string; lastSeen: number }[]>();
  private readonly appRegistry = new Map<string, AppRegistration>();
  private resolutionCount = 0;
  private cacheHitCount = 0;

  registerSource(
    id: string,
    tier: SourceTier,
    priority: number,
    contentTypes: string[],
    baseUrl?: string
  ): void {
    // Remove existing
    this.sources = this.sources.filter(s => s.id !== id);

    this.sources.push({
      id,
      tier,
      priority: Math.min(priority, 100),
      contentTypes,
      available: true,
      baseUrl: baseUrl ?? null,
    });

    // Sort by tier asc, priority desc
    this.sources.sort((a, b) => {
      if (a.tier !== b.tier) return a.tier - b.tier;
      return b.priority - a.priority;
    });
  }

  setSourceUrl(sourceId: string, baseUrl: string | null): void {
    const source = this.sources.find(s => s.id === sourceId);
    if (source) {
      source.baseUrl = baseUrl;
    }
  }

  setSourceAvailable(sourceId: string, available: boolean): void {
    const source = this.sources.find(s => s.id === sourceId);
    if (source) {
      source.available = available;
    }
  }

  isSourceAvailable(sourceId: string): boolean {
    return this.sources.find(s => s.id === sourceId)?.available ?? false;
  }

  recordContentLocation(contentId: string, sourceId: string): void {
    const now = Date.now();
    let locations = this.contentIndex.get(contentId);
    if (!locations) {
      locations = [];
      this.contentIndex.set(contentId, locations);
    }

    const existing = locations.find(l => l.sourceId === sourceId);
    if (existing) {
      existing.lastSeen = now;
    } else {
      locations.push({ sourceId, lastSeen: now });
    }
  }

  removeContentLocation(contentId: string, sourceId: string): void {
    const locations = this.contentIndex.get(contentId);
    if (locations) {
      const filtered = locations.filter(l => l.sourceId !== sourceId);
      if (filtered.length === 0) {
        this.contentIndex.delete(contentId);
      } else {
        this.contentIndex.set(contentId, filtered);
      }
    }
  }

  clearSourceLocations(sourceId: string): void {
    for (const [contentId, locations] of this.contentIndex.entries()) {
      const filtered = locations.filter(l => l.sourceId !== sourceId);
      if (filtered.length === 0) {
        this.contentIndex.delete(contentId);
      } else {
        this.contentIndex.set(contentId, filtered);
      }
    }
  }

  resolve(contentType: string, contentId: string): ResolutionResult | ResolutionError {
    this.resolutionCount++;

    // 1. Check content index for known locations
    const knownLocations = this.contentIndex.get(contentId);
    if (knownLocations && knownLocations.length > 0) {
      // Sort by recency
      const sorted = [...knownLocations].sort((a, b) => b.lastSeen - a.lastSeen);

      for (const loc of sorted) {
        const source = this.sources.find(s => s.id === loc.sourceId && s.available);
        if (source) {
          this.cacheHitCount++;
          return this.buildResult(source, contentType, contentId, true);
        }
      }
    }

    // 2. Find first available source for content type
    for (const source of this.sources) {
      if (source.available && source.contentTypes.includes(contentType)) {
        return this.buildResult(source, contentType, contentId, false);
      }
    }

    // 3. No source found
    return {
      error: 'no_source_available',
      contentType,
      contentId,
    };
  }

  private buildResult(
    source: ContentSource,
    contentType: string,
    contentId: string,
    cached: boolean
  ): ResolutionResult {
    let url: string | null = null;
    if (source.baseUrl) {
      switch (contentType) {
        case 'app':
          url = `${source.baseUrl}/apps/${contentId}`;
          break;
        case 'blob':
          url = `${source.baseUrl}/store/${contentId}`;
          break;
        case 'stream':
          url = `${source.baseUrl}/stream/${contentId}`;
          break;
        default:
          url = `${source.baseUrl}/api/v1/${contentType}/${contentId}`;
      }
    }

    return {
      sourceId: source.id,
      tier: source.tier,
      url,
      cached,
    };
  }

  getResolutionChain(contentType: string): SourceInfo[] {
    return this.sources
      .filter(s => s.available && s.contentTypes.includes(contentType))
      .map(s => ({
        id: s.id,
        tier: s.tier,
        priority: s.priority,
        url: s.baseUrl,
      }));
  }

  registerApp(appId: string, blobHash: string, entryPoint: string, fallbackUrl?: string): void {
    this.appRegistry.set(appId, {
      blobHash,
      entryPoint,
      fallbackUrl: fallbackUrl ?? null,
      registeredAt: Date.now(),
    });
  }

  unregisterApp(appId: string): void {
    this.appRegistry.delete(appId);
  }

  hasApp(appId: string): boolean {
    return this.appRegistry.has(appId);
  }

  getAppBlobHash(appId: string): string | null {
    return this.appRegistry.get(appId)?.blobHash ?? null;
  }

  resolveAppUrl(appId: string, path?: string): string {
    const reg = this.appRegistry.get(appId);
    const entryPoint = reg?.entryPoint ?? 'index.html';
    const filePath = path ?? entryPoint;

    // Find source that can serve apps
    for (const source of this.sources) {
      if (source.available && source.contentTypes.includes('app') && source.baseUrl) {
        return `${source.baseUrl}/apps/${appId}/${filePath}`;
      }
    }

    // Fall back to fallback URL
    if (reg?.fallbackUrl) {
      return reg.fallbackUrl;
    }

    return '';
  }

  resolveAppUrlFull(appId: string, path?: string): AppResolutionResult {
    const url = this.resolveAppUrl(appId, path);
    const reg = this.appRegistry.get(appId);

    const sourceId =
      this.sources.find(s => s.available && s.contentTypes.includes('app') && s.baseUrl)?.id ??
      null;

    return {
      url: url || null,
      sourceId,
      blobHash: reg?.blobHash ?? null,
      fallbackUrl: reg?.fallbackUrl ?? null,
    };
  }

  getStats(): ResolverStats {
    const hitRate =
      this.resolutionCount > 0 ? (this.cacheHitCount / this.resolutionCount) * 100 : 0;

    return {
      resolutionCount: this.resolutionCount,
      cacheHitCount: this.cacheHitCount,
      cacheHitRate: hitRate,
      sourceCount: this.sources.length,
      indexedContentCount: this.contentIndex.size,
      registeredAppCount: this.appRegistry.size,
    };
  }

  resetStats(): void {
    this.resolutionCount = 0;
    this.cacheHitCount = 0;
  }

  sourceCount(): number {
    return this.sources.length;
  }

  indexedContentCount(): number {
    return this.contentIndex.size;
  }

  registeredAppCount(): number {
    return this.appRegistry.size;
  }

  dispose(): void {
    this.sources = [];
    this.contentIndex.clear();
    this.appRegistry.clear();
  }
}

// ============================================================================
// WASM Wrapper
// ============================================================================

class WasmContentResolverWrapper implements IContentResolver {
  constructor(private readonly wasm: WasmContentResolver) {}

  registerSource(
    id: string,
    tier: SourceTier,
    priority: number,
    contentTypes: string[],
    baseUrl?: string
  ): void {
    this.wasm.register_source(id, tier, priority, JSON.stringify(contentTypes), baseUrl ?? null);
  }

  setSourceUrl(sourceId: string, baseUrl: string | null): void {
    this.wasm.set_source_url(sourceId, baseUrl);
  }

  setSourceAvailable(sourceId: string, available: boolean): void {
    this.wasm.set_source_available(sourceId, available);
  }

  isSourceAvailable(sourceId: string): boolean {
    return this.wasm.is_source_available(sourceId);
  }

  recordContentLocation(contentId: string, sourceId: string): void {
    this.wasm.record_content_location(contentId, sourceId);
  }

  removeContentLocation(contentId: string, sourceId: string): void {
    this.wasm.remove_content_location(contentId, sourceId);
  }

  clearSourceLocations(sourceId: string): void {
    this.wasm.clear_source_locations(sourceId);
  }

  resolve(contentType: string, contentId: string): ResolutionResult | ResolutionError {
    const json = this.wasm.resolve(contentType, contentId);
    const parsed = JSON.parse(json);

    if ('error' in parsed) {
      return parsed as ResolutionError;
    }

    return {
      sourceId: parsed.source_id,
      tier: parsed.tier as SourceTier,
      url: parsed.url ?? null,
      cached: parsed.cached,
    };
  }

  getResolutionChain(contentType: string): SourceInfo[] {
    const json = this.wasm.get_resolution_chain(contentType);
    const parsed = JSON.parse(json);
    return parsed.map((s: { id: string; tier: number; priority: number; url: string | null }) => ({
      id: s.id,
      tier: s.tier as SourceTier,
      priority: s.priority,
      url: s.url,
    }));
  }

  registerApp(appId: string, blobHash: string, entryPoint: string, fallbackUrl?: string): void {
    this.wasm.register_app(appId, blobHash, entryPoint, fallbackUrl ?? null);
  }

  unregisterApp(appId: string): void {
    this.wasm.unregister_app(appId);
  }

  hasApp(appId: string): boolean {
    return this.wasm.has_app(appId);
  }

  getAppBlobHash(appId: string): string | null {
    return this.wasm.get_app_blob_hash(appId);
  }

  resolveAppUrl(appId: string, path?: string): string {
    return this.wasm.resolve_app_url(appId, path ?? null);
  }

  resolveAppUrlFull(appId: string, path?: string): AppResolutionResult {
    const json = this.wasm.resolve_app_url_full(appId, path ?? null);
    const parsed = JSON.parse(json);
    return {
      url: parsed.url ?? null,
      sourceId: parsed.source_id ?? null,
      blobHash: parsed.blob_hash ?? null,
      fallbackUrl: parsed.fallback_url ?? null,
    };
  }

  getStats(): ResolverStats {
    const json = this.wasm.get_stats();
    const parsed = JSON.parse(json);
    return {
      resolutionCount: parsed.resolution_count,
      cacheHitCount: parsed.cache_hit_count,
      cacheHitRate: parsed.cache_hit_rate,
      sourceCount: parsed.source_count,
      indexedContentCount: parsed.indexed_content_count,
      registeredAppCount: parsed.registered_app_count,
    };
  }

  resetStats(): void {
    this.wasm.reset_stats();
  }

  sourceCount(): number {
    return this.wasm.source_count();
  }

  indexedContentCount(): number {
    return this.wasm.indexed_content_count();
  }

  registeredAppCount(): number {
    return this.wasm.registered_app_count();
  }

  dispose(): void {
    this.wasm.free();
  }
}

// ============================================================================
// Factory Function
// ============================================================================

let wasmModule: WasmModule | null = null;
let wasmLoadAttempted = false;

async function loadWasmModule(): Promise<WasmModule | null> {
  if (wasmModule) return wasmModule;
  if (wasmLoadAttempted) return null;

  wasmLoadAttempted = true;

  try {
    // Dynamic import of WASM module from assets path
    // In browser: loads from /wasm/elohim-cache-core/
    // Falls back to TypeScript if WASM not available
    const wasmPath = '/wasm/elohim-cache-core/elohim_cache_core.js';
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const mod: any = await import(/* webpackIgnore: true */ wasmPath);
    await mod.default();
    wasmModule = mod as WasmModule;
    return wasmModule;
  } catch {
    // WASM unavailability is expected in most environments — TypeScript fallback is used
    return null;
  }
}

/**
 * Check if WASM content resolver is available.
 */
export async function isWasmResolverAvailable(): Promise<boolean> {
  const mod = await loadWasmModule();
  return mod !== null;
}

/**
 * Create a content resolver instance.
 *
 * Prefers WASM implementation for performance, falls back to TypeScript.
 */
export async function createContentResolver(config?: ResolverConfig): Promise<ResolverInitResult> {
  const preferWasm = config?.preferWasm ?? true;

  if (preferWasm) {
    const mod = await loadWasmModule();
    if (mod) {
      try {
        const wasmResolver = new mod.ContentResolver();
        return {
          resolver: new WasmContentResolverWrapper(wasmResolver),
          implementation: 'wasm',
        };
      } catch (error) {
        console.warn('[ContentResolver] WASM instantiation failed:', error);
      }
    }
  }

  // Fallback to TypeScript
  return {
    resolver: new TsContentResolver(),
    implementation: 'typescript',
  };
}
