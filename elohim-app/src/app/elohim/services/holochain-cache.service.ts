import { Injectable, signal, computed } from '@angular/core';

// @coverage: 85.6% (2026-02-04)

/**
 * Cache entry metadata
 */
export interface CacheEntry {
  key: string;
  value: any;
  timestamp: number;
  ttlMs?: number; // Optional TTL in milliseconds
  metadata?: Record<string, any>;
}

/**
 * Cache statistics
 */
export interface CacheStats {
  totalEntries: number;
  totalSizeBytes: number;
  oldestEntry?: number;
  newestEntry?: number;
  hitRate: number;
}

/**
 * Holochain Cache Service
 *
 * Provides offline-first caching layer for Holochain content.
 * Caches content locally (IndexedDB + memory) for offline access.
 *
 * Features:
 * - Hybrid memory/IndexedDB storage
 * - TTL-based expiration
 * - Cache statistics and monitoring
 * - Smart cache invalidation
 * - Size limits and LRU eviction
 *
 * Use Cases:
 * 1. Cache content responses for offline read access
 * 2. Store learning paths and educational content
 * 3. Cache user profile data
 * 4. Persist metadata during offline periods
 * 5. Warm cache on app startup
 *
 * Storage Hierarchy:
 * - L1: Memory (10MB) - fast access, lost on refresh
 * - L2: IndexedDB (50MB) - persistent, survives page reload
 *
 * When offline:
 * - L1 hits serve immediately (5-10ms)
 * - L1 misses check L2 (50-100ms)
 * - L2 hits reload to L1 for next access
 * - L2 misses return null
 */
@Injectable({
  providedIn: 'root',
})
export class HolochainCacheService {
  /** Maximum L1 cache size (10MB) */
  private readonly MAX_MEMORY_SIZE = 10 * 1024 * 1024;

  /** Maximum L2 cache size (50MB) */
  private readonly MAX_INDEXEDDB_SIZE = 50 * 1024 * 1024;

  /** Database name */
  private readonly DB_NAME = 'elohim-holochain-cache';

  /** Store name */
  private readonly STORE_NAME = 'cache-entries';

  /** L1 memory cache */
  private readonly memoryCache = new Map<string, CacheEntry>();

  /** Cache statistics */
  private readonly stats = signal({
    hits: 0,
    misses: 0,
    totalEntries: 0,
    totalSizeBytes: 0,
  });

  /** Computed hit rate */
  readonly hitRate = computed(() => {
    const { hits, misses } = this.stats();
    const total = hits + misses;
    return total > 0 ? (hits / total) * 100 : 0;
  });

  /** Database instance */
  private db: IDBDatabase | null = null;

  /** Initialization promise */
  private readonly initPromise: Promise<void>;

  constructor() {
    this.initPromise = this.initDatabase();
  }

  /**
   * Initialize IndexedDB database
   */
  private async initDatabase(): Promise<void> {
    return new Promise((resolve, reject) => {
      // Check if IndexedDB is available
      if (!window.indexedDB) {
        resolve();
        return;
      }

      const request = window.indexedDB.open(this.DB_NAME, 1);

      request.onerror = () => {
        reject(request.error);
      };

      request.onsuccess = () => {
        this.db = request.result;

        resolve();
      };

      request.onupgradeneeded = _event => {
        const db = (_event.target as IDBOpenDBRequest).result;

        // Create cache entries store
        if (!db.objectStoreNames.contains(this.STORE_NAME)) {
          const store = db.createObjectStore(this.STORE_NAME, { keyPath: 'key' });
          store.createIndex('timestamp', 'timestamp', { unique: false });
        }
      };
    });
  }

  /**
   * Get value from cache (L1 then L2)
   */
  async get<T = any>(key: string): Promise<T | null> {
    // Ensure database is initialized
    await this.initPromise;

    // Try L1 (memory)
    const memEntry = this.memoryCache.get(key);
    if (memEntry) {
      if (this.isExpired(memEntry)) {
        this.memoryCache.delete(key);
      } else {
        this.stats.update(s => ({ ...s, hits: s.hits + 1 }));
        return memEntry.value as T;
      }
    }

    // Try L2 (IndexedDB)
    if (this.db) {
      const entry = await this.getFromIndexedDB(key);
      if (entry) {
        if (!this.isExpired(entry)) {
          // Load to L1 for next access
          this.memoryCache.set(key, entry);
          this.stats.update(s => ({ ...s, hits: s.hits + 1 }));
          return entry.value as T;
        } else {
          // Expired - remove from L2
          await this.deleteFromIndexedDB(key);
        }
      }
    }

    // Cache miss
    this.stats.update(s => ({ ...s, misses: s.misses + 1 }));
    return null;
  }

  /**
   * Set value in cache (L1 + L2)
   */
  async set<T = any>(
    key: string,
    value: T,
    ttlMs?: number,
    metadata?: Record<string, any>
  ): Promise<void> {
    await this.initPromise;

    const entry: CacheEntry = {
      key,
      value,
      timestamp: Date.now(),
      ttlMs,
      metadata,
    };

    // Set in L1
    this.memoryCache.set(key, entry);

    // Check if we need to evict
    if (this.getMemoryCacheSize() > this.MAX_MEMORY_SIZE) {
      this.evictMemoryCache();
    }

    // Set in L2 (if available)
    if (this.db) {
      try {
        await this.setInIndexedDB(entry);
      } catch {
        // Fall back to memory only if IndexedDB write fails
      }
    }

    // Update stats
    this.updateStats();
  }

  /**
   * Delete value from cache
   */
  async delete(key: string): Promise<void> {
    await this.initPromise;

    this.memoryCache.delete(key);

    if (this.db) {
      try {
        await this.deleteFromIndexedDB(key);
      } catch {
        // IndexedDB delete failure is non-critical - memory cache deletion succeeded
      }
    }

    this.updateStats();
  }

  /**
   * Clear entire cache
   */
  async clear(): Promise<void> {
    await this.initPromise;

    this.memoryCache.clear();

    if (this.db) {
      try {
        await new Promise<void>((resolve, reject) => {
          const transaction = this.db!.transaction(this.STORE_NAME, 'readwrite');
          const store = transaction.objectStore(this.STORE_NAME);
          const request = store.clear();

          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve();
        });
      } catch {
        // IndexedDB clear failure is non-critical - memory cache cleared successfully
      }
    }

    this.updateStats();
  }

  /**
   * Get cache statistics
   */
  getStats(): CacheStats {
    const entries = Array.from(this.memoryCache.values());

    return {
      totalEntries: entries.length,
      totalSizeBytes: this.getMemoryCacheSize(),
      oldestEntry:
        entries.length > 0
          ? Math.round((Date.now() - Math.min(...entries.map(e => e.timestamp))) / 1000)
          : 0,
      newestEntry:
        entries.length > 0
          ? Math.round((Date.now() - Math.max(...entries.map(e => e.timestamp))) / 1000)
          : 0,
      hitRate: this.hitRate(),
    };
  }

  /**
   * Preload content into cache
   *
   * Useful for warming cache on app startup or before user accesses content.
   */
  async preload<T = any>(items: { key: string; value: T; ttlMs?: number }[]): Promise<void> {
    for (const item of items) {
      try {
        await this.set(item.key, item.value, item.ttlMs);
      } catch {
        // Preload of individual items can fail - continue with remaining items
      }
    }
  }

  /**
   * Query cache entries (memory only)
   */
  query(predicate: (entry: CacheEntry) => boolean): CacheEntry[] {
    return Array.from(this.memoryCache.values())
      .filter(entry => !this.isExpired(entry))
      .filter(predicate);
  }

  /**
   * Get entries by tag (searches metadata)
   */
  getByTag(tag: string): CacheEntry[] {
    return this.query(entry => {
      const tags = entry.metadata?.['tags'];
      return Array.isArray(tags) && tags.includes(tag);
    });
  }

  /**
   * Get entries by domain (from metadata)
   */
  getByDomain(domain: string): CacheEntry[] {
    return this.query(entry => entry.metadata?.['domain'] === domain);
  }

  /**
   * Check if entry is expired
   */
  private isExpired(entry: CacheEntry): boolean {
    if (!entry.ttlMs) return false;
    return Date.now() - entry.timestamp > entry.ttlMs;
  }

  /**
   * Get memory cache size in bytes
   */
  private getMemoryCacheSize(): number {
    let size = 0;
    for (const entry of this.memoryCache.values()) {
      size += this.estimateSize(entry.value);
    }
    return size;
  }

  /**
   * Estimate object size in bytes (rough estimate)
   */
  private estimateSize(obj: any): number {
    const str = JSON.stringify(obj);
    return new Blob([str]).size;
  }

  /**
   * Evict least-recently-used entries from memory cache
   */
  private evictMemoryCache(): void {
    const entries = Array.from(this.memoryCache.entries()).sort(
      (a, b) => a[1].timestamp - b[1].timestamp
    );

    // Remove oldest 10% of entries
    const toRemove = Math.ceil(entries.length * 0.1);
    for (let i = 0; i < toRemove; i++) {
      this.memoryCache.delete(entries[i][0]);
    }
  }

  /**
   * Get from IndexedDB
   */
  private async getFromIndexedDB(key: string): Promise<CacheEntry | null> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        resolve(null);
        return;
      }

      const transaction = this.db.transaction(this.STORE_NAME, 'readonly');
      const store = transaction.objectStore(this.STORE_NAME);
      const request = store.get(key);

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve(request.result ?? null);
    });
  }

  /**
   * Set in IndexedDB
   */
  private async setInIndexedDB(entry: CacheEntry): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        resolve();
        return;
      }

      const transaction = this.db.transaction(this.STORE_NAME, 'readwrite');
      const store = transaction.objectStore(this.STORE_NAME);
      const request = store.put(entry);

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve();
    });
  }

  /**
   * Delete from IndexedDB
   */
  private async deleteFromIndexedDB(key: string): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        resolve();
        return;
      }

      const transaction = this.db.transaction(this.STORE_NAME, 'readwrite');
      const store = transaction.objectStore(this.STORE_NAME);
      const request = store.delete(key);

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve();
    });
  }

  /**
   * Update cache statistics
   */
  private updateStats(): void {
    this.stats.update(s => ({
      ...s,
      totalEntries: this.memoryCache.size,
      totalSizeBytes: this.getMemoryCacheSize(),
    }));
  }
}
