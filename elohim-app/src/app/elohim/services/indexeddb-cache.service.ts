/**
 * IndexedDB Cache Service
 *
 * Provides persistent caching for content and paths using IndexedDB.
 * This enables data to survive page refreshes, reducing Holochain calls.
 *
 * Features:
 * - TTL-based cache expiration
 * - Version-based cache invalidation
 * - Automatic cleanup of expired entries
 * - Batch operations for efficiency
 *
 * @see DataLoaderService - Uses this for persistent caching
 * @see HolochainContentService - Primary data source
 */

import { Injectable } from '@angular/core';

// @coverage: 3.5% (2026-02-05)

import { ContentNode } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';

// =============================================================================
// Cache Entry Types
// =============================================================================

interface CacheEntry<T> {
  data: T;
  cachedAt: number; // Unix timestamp
  expiresAt: number; // Unix timestamp
  version: number; // Schema version for invalidation
}

interface CacheMetadata {
  version: number;
  lastCleanup: number;
  contentCount: number;
  pathCount: number;
}

// =============================================================================
// Configuration
// =============================================================================

const DB_NAME = 'elohim-cache';
const DB_VERSION = 1;

/** Store names */
const STORES = {
  CONTENT: 'content',
  PATHS: 'paths',
  METADATA: 'metadata',
} as const;

/** Cache TTL in milliseconds */
const CACHE_TTL = {
  CONTENT: 24 * 60 * 60 * 1000, // 24 hours for content
  PATHS: 12 * 60 * 60 * 1000, // 12 hours for paths
} as const;

/** Current schema version - increment to invalidate all cached data */
const SCHEMA_VERSION = 1;

/** Cleanup interval - run cleanup every N cache accesses */
const CLEANUP_INTERVAL = 100;

// =============================================================================
// Service Implementation
// =============================================================================

@Injectable({
  providedIn: 'root',
})
export class IndexedDBCacheService {
  private db: IDBDatabase | null = null;
  private initPromise: Promise<boolean> | null = null;
  private accessCount = 0;

  /**
   * Initialize the IndexedDB database.
   * Call this early in app startup (e.g., APP_INITIALIZER).
   */
  async init(): Promise<boolean> {
    if (this.db) {
      return true;
    }

    if (this.initPromise) {
      return this.initPromise;
    }

    this.initPromise = this.openDatabase();
    return this.initPromise;
  }

  /**
   * Check if IndexedDB is available and initialized.
   */
  isAvailable(): boolean {
    return this.db !== null;
  }

  // ===========================================================================
  // Content Caching
  // ===========================================================================

  /**
   * Get a content node from cache.
   *
   * @param id Content ID
   * @returns ContentNode if found and not expired, null otherwise
   */
  async getContent(id: string): Promise<ContentNode | null> {
    if (!this.db) return null;

    try {
      const entry = await this.get<ContentNode>(STORES.CONTENT, id);
      if (entry && !this.isExpired(entry) && entry.version === SCHEMA_VERSION) {
        return entry.data;
      }
      return null;
    } catch {
      return null;
    }
  }

  /**
   * Store a content node in cache.
   *
   * @param content ContentNode to cache
   */
  async setContent(content: ContentNode): Promise<void> {
    if (!this.db) return;

    try {
      const entry: CacheEntry<ContentNode> = {
        data: content,
        cachedAt: Date.now(),
        expiresAt: Date.now() + CACHE_TTL.CONTENT,
        version: SCHEMA_VERSION,
      };
      await this.put(STORES.CONTENT, content.id, entry);
      this.maybeCleanup();
    } catch {
      // Silently fail - cache is optional
    }
  }

  /**
   * Batch get multiple content nodes.
   *
   * @param ids Array of content IDs
   * @returns Map of id → ContentNode for found items
   */
  async getContentBatch(ids: string[]): Promise<Map<string, ContentNode>> {
    const result = new Map<string, ContentNode>();
    if (!this.db || ids.length === 0) return result;

    try {
      const entries = await this.getBatch<ContentNode>(STORES.CONTENT, ids);
      for (const [id, entry] of entries) {
        if (entry && !this.isExpired(entry) && entry.version === SCHEMA_VERSION) {
          result.set(id, entry.data);
        }
      }
    } catch {
      // Return partial results
    }

    return result;
  }

  /**
   * Batch store multiple content nodes.
   *
   * @param contents Array of ContentNodes to cache
   */
  async setContentBatch(contents: ContentNode[]): Promise<void> {
    if (!this.db || contents.length === 0) return;

    try {
      const entries = new Map<string, CacheEntry<ContentNode>>();
      const now = Date.now();

      for (const content of contents) {
        entries.set(content.id, {
          data: content,
          cachedAt: now,
          expiresAt: now + CACHE_TTL.CONTENT,
          version: SCHEMA_VERSION,
        });
      }

      await this.putBatch(STORES.CONTENT, entries);
      this.maybeCleanup();
    } catch {
      // Silently fail
    }
  }

  // ===========================================================================
  // Path Caching
  // ===========================================================================

  /**
   * Get a learning path from cache.
   *
   * @param id Path ID
   * @returns LearningPath if found and not expired, null otherwise
   */
  async getPath(id: string): Promise<LearningPath | null> {
    if (!this.db) return null;

    try {
      const entry = await this.get<LearningPath>(STORES.PATHS, id);
      if (entry && !this.isExpired(entry) && entry.version === SCHEMA_VERSION) {
        return entry.data;
      }
      return null;
    } catch {
      return null;
    }
  }

  /**
   * Store a learning path in cache.
   *
   * @param path LearningPath to cache
   */
  async setPath(path: LearningPath): Promise<void> {
    if (!this.db) return;

    try {
      const entry: CacheEntry<LearningPath> = {
        data: path,
        cachedAt: Date.now(),
        expiresAt: Date.now() + CACHE_TTL.PATHS,
        version: SCHEMA_VERSION,
      };
      await this.put(STORES.PATHS, path.id, entry);
      this.maybeCleanup();
    } catch {
      // Silently fail
    }
  }

  // ===========================================================================
  // Cache Management
  // ===========================================================================

  /**
   * Clear all cached data.
   */
  async clearAll(): Promise<void> {
    if (!this.db) return;

    try {
      await this.clearStore(STORES.CONTENT);
      await this.clearStore(STORES.PATHS);
    } catch {
      // Silently fail
    }
  }

  /**
   * Clear only content cache.
   */
  async clearContent(): Promise<void> {
    if (!this.db) return;

    try {
      await this.clearStore(STORES.CONTENT);
    } catch {
      // Silently fail
    }
  }

  /**
   * Clear only path cache.
   */
  async clearPaths(): Promise<void> {
    if (!this.db) return;

    try {
      await this.clearStore(STORES.PATHS);
    } catch {
      // Silently fail
    }
  }

  /**
   * Remove a specific content entry from cache.
   *
   * @param id Content ID to remove
   */
  async removeContent(id: string): Promise<void> {
    if (!this.db) return;

    try {
      await this.delete(STORES.CONTENT, id);
    } catch {
      // Silently fail
    }
  }

  /**
   * Remove a specific path entry from cache.
   *
   * @param id Path ID to remove
   */
  async removePath(id: string): Promise<void> {
    if (!this.db) return;

    try {
      await this.delete(STORES.PATHS, id);
    } catch {
      // Silently fail
    }
  }

  /**
   * Get cache statistics.
   */
  async getStats(): Promise<{
    contentCount: number;
    pathCount: number;
    isAvailable: boolean;
  }> {
    if (!this.db) {
      return { contentCount: 0, pathCount: 0, isAvailable: false };
    }

    try {
      const contentCount = await this.count(STORES.CONTENT);
      const pathCount = await this.count(STORES.PATHS);
      return { contentCount, pathCount, isAvailable: true };
    } catch {
      return { contentCount: 0, pathCount: 0, isAvailable: true };
    }
  }

  /**
   * Clean up expired entries from all stores.
   */
  async cleanup(): Promise<{ contentRemoved: number; pathsRemoved: number }> {
    if (!this.db) {
      return { contentRemoved: 0, pathsRemoved: 0 };
    }

    const contentRemoved = await this.cleanupStore(STORES.CONTENT);
    const pathsRemoved = await this.cleanupStore(STORES.PATHS);

    return { contentRemoved, pathsRemoved };
  }

  // ===========================================================================
  // Private Methods - IndexedDB Operations
  // ===========================================================================

  /**
   * Open/create the IndexedDB database.
   */
  private async openDatabase(): Promise<boolean> {
    return new Promise(resolve => {
      if (!('indexedDB' in window)) {
        resolve(false);
        return;
      }

      const request = indexedDB.open(DB_NAME, DB_VERSION);

      request.onerror = () => {
        resolve(false);
      };

      request.onsuccess = () => {
        this.db = request.result;
        resolve(true);
      };

      request.onupgradeneeded = event => {
        const db = (event.target as IDBOpenDBRequest).result;

        // Create content store
        if (!db.objectStoreNames.contains(STORES.CONTENT)) {
          db.createObjectStore(STORES.CONTENT);
        }

        // Create paths store
        if (!db.objectStoreNames.contains(STORES.PATHS)) {
          db.createObjectStore(STORES.PATHS);
        }

        // Create metadata store
        if (!db.objectStoreNames.contains(STORES.METADATA)) {
          db.createObjectStore(STORES.METADATA);
        }
      };
    });
  }

  /**
   * Get a single entry from a store.
   */
  private async get<T>(storeName: string, key: string): Promise<CacheEntry<T> | null> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        resolve(null);
        return;
      }

      const tx = this.db.transaction(storeName, 'readonly');
      const store = tx.objectStore(storeName);
      const request = store.get(key);

      request.onsuccess = () => resolve(request.result ?? null);
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Get multiple entries from a store.
   */
  private async getBatch<T>(
    storeName: string,
    keys: string[]
  ): Promise<Map<string, CacheEntry<T>>> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        resolve(new Map());
        return;
      }

      const result = new Map<string, CacheEntry<T>>();
      const tx = this.db.transaction(storeName, 'readonly');
      const store = tx.objectStore(storeName);
      let completed = 0;

      for (const key of keys) {
        const request = store.get(key);
        request.onsuccess = () => {
          if (request.result) {
            result.set(key, request.result);
          }
          completed++;
          if (completed === keys.length) {
            resolve(result);
          }
        };
        request.onerror = () => {
          completed++;
          if (completed === keys.length) {
            resolve(result);
          }
        };
      }

      // Handle empty keys array
      if (keys.length === 0) {
        resolve(result);
      }

      tx.onerror = () => reject(tx.error);
    });
  }

  /**
   * Put a single entry in a store.
   */
  private async put<T>(storeName: string, key: string, value: CacheEntry<T>): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        resolve();
        return;
      }

      const tx = this.db.transaction(storeName, 'readwrite');
      const store = tx.objectStore(storeName);
      const request = store.put(value, key);

      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Put multiple entries in a store (single transaction).
   */
  private async putBatch<T>(storeName: string, entries: Map<string, CacheEntry<T>>): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.db || entries.size === 0) {
        resolve();
        return;
      }

      const tx = this.db.transaction(storeName, 'readwrite');
      const store = tx.objectStore(storeName);

      for (const [key, value] of entries) {
        store.put(value, key);
      }

      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    });
  }

  /**
   * Delete a single entry from a store.
   */
  private async delete(storeName: string, key: string): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        resolve();
        return;
      }

      const tx = this.db.transaction(storeName, 'readwrite');
      const store = tx.objectStore(storeName);
      const request = store.delete(key);

      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Clear all entries from a store.
   */
  private async clearStore(storeName: string): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        resolve();
        return;
      }

      const tx = this.db.transaction(storeName, 'readwrite');
      const store = tx.objectStore(storeName);
      const request = store.clear();

      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Count entries in a store.
   */
  private async count(storeName: string): Promise<number> {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        resolve(0);
        return;
      }

      const tx = this.db.transaction(storeName, 'readonly');
      const store = tx.objectStore(storeName);
      const request = store.count();

      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Clean up expired entries from a store.
   */
  private async cleanupStore(storeName: string): Promise<number> {
    if (!this.db) return 0;

    return new Promise(resolve => {
      const tx = this.db!.transaction(storeName, 'readwrite');
      const store = tx.objectStore(storeName);
      let removed = 0;

      const request = store.openCursor();

      request.onsuccess = event => {
        const cursor = (event.target as IDBRequest<IDBCursorWithValue>).result;
        if (cursor) {
          const entry = cursor.value as CacheEntry<unknown>;
          if (this.isExpired(entry) || entry.version !== SCHEMA_VERSION) {
            cursor.delete();
            removed++;
          }
          cursor.continue();
        } else {
          resolve(removed);
        }
      };

      request.onerror = () => resolve(removed);
    });
  }

  /**
   * Check if a cache entry is expired.
   */
  private isExpired(entry: CacheEntry<unknown>): boolean {
    return Date.now() > entry.expiresAt;
  }

  /**
   * Maybe run cleanup based on access count.
   */
  private maybeCleanup(): void {
    this.accessCount++;
    if (this.accessCount >= CLEANUP_INTERVAL) {
      this.accessCount = 0;
      // Run cleanup in background
      this.cleanup().catch(() => {
        // Ignore cleanup errors
      });
    }
  }
}
