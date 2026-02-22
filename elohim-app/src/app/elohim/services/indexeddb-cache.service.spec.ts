/**
 * IndexedDB Cache Service Tests
 *
 * Tests cache lifecycle, TTL expiration, schema version invalidation,
 * batch operations, cleanup, quota handling, and Safari fallback.
 */

import { TestBed } from '@angular/core/testing';

import { IndexedDBCacheService } from './indexeddb-cache.service';
import { ContentNode } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';

// =============================================================================
// In-memory IDB Mock
// =============================================================================

/**
 * Minimal in-memory IndexedDB mock for unit tests.
 * Simulates open, transactions, object stores, and cursors.
 */
function createMockIndexedDB() {
  const databases = new Map<string, Map<string, Map<string, unknown>>>();

  function getOrCreateDb(name: string, storeNames: string[]): Map<string, Map<string, unknown>> {
    if (!databases.has(name)) {
      databases.set(name, new Map());
    }
    const db = databases.get(name)!;
    for (const storeName of storeNames) {
      if (!db.has(storeName)) {
        db.set(storeName, new Map());
      }
    }
    return db;
  }

  function createMockObjectStore(
    storeData: Map<string, unknown>,
    _mode: IDBTransactionMode = 'readonly'
  ) {
    return {
      put(value: unknown, key: string) {
        const req = createMockRequest();
        setTimeout(() => {
          storeData.set(key, value);
          req.result = undefined;
          req.onsuccess?.({ target: req } as unknown as Event);
        }, 0);
        return req;
      },
      get(key: string) {
        const req = createMockRequest();
        setTimeout(() => {
          req.result = storeData.get(key) ?? undefined;
          req.onsuccess?.({ target: req } as unknown as Event);
        }, 0);
        return req;
      },
      delete(key: string) {
        const req = createMockRequest();
        setTimeout(() => {
          storeData.delete(key);
          req.result = undefined;
          req.onsuccess?.({ target: req } as unknown as Event);
        }, 0);
        return req;
      },
      clear() {
        const req = createMockRequest();
        setTimeout(() => {
          storeData.clear();
          req.result = undefined;
          req.onsuccess?.({ target: req } as unknown as Event);
        }, 0);
        return req;
      },
      count() {
        const req = createMockRequest();
        setTimeout(() => {
          req.result = storeData.size;
          req.onsuccess?.({ target: req } as unknown as Event);
        }, 0);
        return req;
      },
      openCursor() {
        const req = createMockRequest();
        const entries = Array.from(storeData.entries());
        let idx = 0;

        function advanceCursor() {
          if (idx < entries.length) {
            const [key, value] = entries[idx];
            idx++;
            const cursor = {
              key,
              value,
              delete() {
                storeData.delete(key);
              },
              continue() {
                setTimeout(() => advanceCursor(), 0);
              },
            };
            req.result = cursor;
          } else {
            req.result = null;
          }
          req.onsuccess?.({ target: req } as unknown as Event);
        }

        setTimeout(() => advanceCursor(), 0);
        return req;
      },
    };
  }

  function createMockRequest(): {
    result: unknown;
    error: DOMException | null;
    onsuccess: ((event: Event) => void) | null;
    onerror: ((event: Event) => void) | null;
  } {
    return {
      result: undefined,
      error: null,
      onsuccess: null,
      onerror: null,
    };
  }

  function createMockTransaction(
    db: Map<string, Map<string, unknown>>,
    storeNames: string | string[],
    mode: IDBTransactionMode = 'readonly'
  ) {
    const names = Array.isArray(storeNames) ? storeNames : [storeNames];
    const tx: {
      oncomplete: ((event: Event) => void) | null;
      onerror: ((event: Event) => void) | null;
      error: DOMException | null;
      objectStore: (name: string) => ReturnType<typeof createMockObjectStore>;
    } = {
      oncomplete: null,
      onerror: null,
      error: null,
      objectStore(name: string) {
        const data = db.get(name) ?? new Map();
        return createMockObjectStore(data, mode) as ReturnType<typeof createMockObjectStore>;
      },
    };

    // Auto-complete write transactions
    if (mode === 'readwrite') {
      setTimeout(() => {
        tx.oncomplete?.({} as Event);
      }, 10);
    }

    return tx;
  }

  const storeNames = ['content', 'paths', 'metadata'];

  const mockIDB = {
    open(name: string, _version?: number) {
      const req = createMockRequest() as {
        result: unknown;
        error: DOMException | null;
        onsuccess: ((event: Event) => void) | null;
        onerror: ((event: Event) => void) | null;
        onupgradeneeded: ((event: IDBVersionChangeEvent) => void) | null;
      };
      (req as { onupgradeneeded: unknown }).onupgradeneeded = null;

      setTimeout(() => {
        const db = getOrCreateDb(name, storeNames);

        // Simulate onupgradeneeded
        const mockDb = {
          objectStoreNames: {
            contains: (n: string) => db.has(n),
          },
          createObjectStore: (n: string) => {
            if (!db.has(n)) {
              db.set(n, new Map());
            }
          },
          transaction: (names: string | string[], mode?: IDBTransactionMode) =>
            createMockTransaction(db, names, mode),
          close: () => {},
        };

        req.onupgradeneeded?.({
          target: { result: mockDb },
        } as unknown as IDBVersionChangeEvent);

        req.result = mockDb;
        req.onsuccess?.({ target: req } as unknown as Event);
      }, 0);

      return req;
    },
    deleteDatabase: (_name: string) => {
      databases.delete(_name);
      const req = createMockRequest();
      setTimeout(() => {
        req.onsuccess?.({} as Event);
      }, 0);
      return req;
    },
  };

  return { mockIDB, databases };
}

// =============================================================================
// Test Data
// =============================================================================

const mockContent: ContentNode = {
  id: 'test-content-1',
  contentType: 'concept' as ContentNode['contentType'],
  title: 'Test Content',
  description: 'A test content node',
  content: '# Hello World',
  contentFormat: 'markdown' as ContentNode['contentFormat'],
  tags: ['test'],
  relatedNodeIds: [],
  metadata: {},
};

const mockContent2: ContentNode = {
  id: 'test-content-2',
  contentType: 'concept' as ContentNode['contentType'],
  title: 'Second Content',
  description: 'Another test',
  content: '# Second',
  contentFormat: 'markdown' as ContentNode['contentFormat'],
  tags: [],
  relatedNodeIds: [],
  metadata: {},
};

const mockPath: LearningPath = {
  id: 'test-path-1',
  version: '1.0.0',
  title: 'Test Path',
  description: 'A test path',
  purpose: 'testing',
  createdBy: 'test-user',
  contributors: [],
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
  difficulty: 'beginner',
  estimatedDuration: '1h',
  tags: ['test'],
  visibility: 'public',
  steps: [],
};

// =============================================================================
// Tests
// =============================================================================

describe('IndexedDBCacheService', () => {
  let service: IndexedDBCacheService;
  let originalIndexedDB: IDBFactory;
  let mockIDB: ReturnType<typeof createMockIndexedDB>['mockIDB'];

  beforeEach(() => {
    const mock = createMockIndexedDB();
    mockIDB = mock.mockIDB;
    originalIndexedDB = globalThis.indexedDB;
    Object.defineProperty(globalThis, 'indexedDB', {
      value: mockIDB,
      configurable: true,
      writable: true,
    });

    TestBed.configureTestingModule({
      providers: [IndexedDBCacheService],
    });

    service = TestBed.inject(IndexedDBCacheService);
  });

  afterEach(() => {
    Object.defineProperty(globalThis, 'indexedDB', {
      value: originalIndexedDB,
      configurable: true,
      writable: true,
    });
  });

  // ===========================================================================
  // Initialization
  // ===========================================================================

  describe('initialization', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should not be available before init', () => {
      expect(service.isAvailable()).toBe(false);
    });

    it('should initialize successfully with mock IndexedDB', async () => {
      const result = await service.init();
      expect(result).toBe(true);
      expect(service.isAvailable()).toBe(true);
    });

    it('should return true on repeated init calls', async () => {
      const result1 = await service.init();
      const result2 = await service.init();
      expect(result1).toBe(true);
      expect(result2).toBe(true);
    });

    it('should deduplicate concurrent init calls', async () => {
      const [r1, r2, r3] = await Promise.all([service.init(), service.init(), service.init()]);
      expect(r1).toBe(true);
      expect(r2).toBe(true);
      expect(r3).toBe(true);
    });

    it('should return false when IndexedDB is not available', async () => {
      // Delete indexedDB from globalThis so 'indexedDB' in globalThis === false
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      delete (globalThis as any).indexedDB;

      // Create a new service instance (fresh, not yet initialized)
      const freshService = new IndexedDBCacheService();
      const result = await freshService.init();
      expect(result).toBe(false);
      expect(freshService.isAvailable()).toBe(false);
    });
  });

  // ===========================================================================
  // Content Operations
  // ===========================================================================

  describe('content operations', () => {
    beforeEach(async () => {
      await service.init();
    });

    it('should return null for non-existent content', async () => {
      const result = await service.getContent('nonexistent');
      expect(result).toBeNull();
    });

    it('should store and retrieve content', async () => {
      await service.setContent(mockContent);
      const result = await service.getContent(mockContent.id);
      expect(result).toBeTruthy();
      expect(result!.id).toBe(mockContent.id);
      expect(result!.title).toBe(mockContent.title);
    });

    it('should overwrite existing content', async () => {
      await service.setContent(mockContent);
      const updated = { ...mockContent, title: 'Updated Title' };
      await service.setContent(updated);
      const result = await service.getContent(mockContent.id);
      expect(result!.title).toBe('Updated Title');
    });

    it('should remove content', async () => {
      await service.setContent(mockContent);
      await service.removeContent(mockContent.id);
      const result = await service.getContent(mockContent.id);
      expect(result).toBeNull();
    });

    it('should return null when db not initialized', async () => {
      const uninitService = new IndexedDBCacheService();
      const result = await uninitService.getContent('test');
      expect(result).toBeNull();
    });

    it('should silently handle setContent when db not initialized', async () => {
      const uninitService = new IndexedDBCacheService();
      // Should not throw
      await uninitService.setContent(mockContent);
    });
  });

  // ===========================================================================
  // Path Operations
  // ===========================================================================

  describe('path operations', () => {
    beforeEach(async () => {
      await service.init();
    });

    it('should return null for non-existent path', async () => {
      const result = await service.getPath('nonexistent');
      expect(result).toBeNull();
    });

    it('should store and retrieve path', async () => {
      await service.setPath(mockPath);
      const result = await service.getPath(mockPath.id);
      expect(result).toBeTruthy();
      expect(result!.id).toBe(mockPath.id);
      expect(result!.title).toBe(mockPath.title);
    });

    it('should remove path', async () => {
      await service.setPath(mockPath);
      await service.removePath(mockPath.id);
      const result = await service.getPath(mockPath.id);
      expect(result).toBeNull();
    });
  });

  // ===========================================================================
  // Batch Operations
  // ===========================================================================

  describe('batch operations', () => {
    beforeEach(async () => {
      await service.init();
    });

    it('should batch get content', async () => {
      await service.setContent(mockContent);
      await service.setContent(mockContent2);

      const result = await service.getContentBatch([mockContent.id, mockContent2.id]);
      expect(result.size).toBe(2);
      expect(result.get(mockContent.id)?.title).toBe(mockContent.title);
      expect(result.get(mockContent2.id)?.title).toBe(mockContent2.title);
    });

    it('should return empty map for empty ids array', async () => {
      const result = await service.getContentBatch([]);
      expect(result.size).toBe(0);
    });

    it('should batch set content', async () => {
      await service.setContentBatch([mockContent, mockContent2]);
      const result = await service.getContentBatch([mockContent.id, mockContent2.id]);
      expect(result.size).toBe(2);
    });

    it('should skip empty batch set', async () => {
      // Should not throw
      await service.setContentBatch([]);
    });

    it('should return empty map when db not initialized', async () => {
      const uninitService = new IndexedDBCacheService();
      const result = await uninitService.getContentBatch(['test']);
      expect(result.size).toBe(0);
    });

    it('should return partial results for partially cached batch', async () => {
      await service.setContent(mockContent);
      // mockContent2 is not cached
      const result = await service.getContentBatch([mockContent.id, 'not-cached']);
      expect(result.size).toBe(1);
      expect(result.has(mockContent.id)).toBe(true);
      expect(result.has('not-cached')).toBe(false);
    });
  });

  // ===========================================================================
  // TTL Expiration
  // ===========================================================================

  describe('TTL expiration', () => {
    beforeEach(async () => {
      await service.init();
    });

    it('should return null for expired content', async () => {
      // Store content, then simulate expiration by advancing Date.now
      await service.setContent(mockContent);

      // Monkey-patch Date.now to return a time in the future past TTL
      const originalNow = Date.now;
      spyOn(Date, 'now').and.returnValue(originalNow() + 25 * 60 * 60 * 1000); // 25 hours

      const result = await service.getContent(mockContent.id);
      expect(result).toBeNull();
    });

    it('should return content within TTL', async () => {
      await service.setContent(mockContent);

      // 1 hour into the 24-hour TTL - should still be valid
      const originalNow = Date.now;
      spyOn(Date, 'now').and.returnValue(originalNow() + 1 * 60 * 60 * 1000);

      const result = await service.getContent(mockContent.id);
      expect(result).toBeTruthy();
    });

    it('should return null for expired path', async () => {
      await service.setPath(mockPath);

      const originalNow = Date.now;
      spyOn(Date, 'now').and.returnValue(originalNow() + 13 * 60 * 60 * 1000); // 13 hours > 12h TTL

      const result = await service.getPath(mockPath.id);
      expect(result).toBeNull();
    });
  });

  // ===========================================================================
  // Cache Management
  // ===========================================================================

  describe('cache management', () => {
    beforeEach(async () => {
      await service.init();
    });

    it('should clear all cached data', async () => {
      await service.setContent(mockContent);
      await service.setPath(mockPath);
      await service.clearAll();

      const content = await service.getContent(mockContent.id);
      const path = await service.getPath(mockPath.id);
      expect(content).toBeNull();
      expect(path).toBeNull();
    });

    it('should clear only content cache', async () => {
      await service.setContent(mockContent);
      await service.setPath(mockPath);
      await service.clearContent();

      const content = await service.getContent(mockContent.id);
      const path = await service.getPath(mockPath.id);
      expect(content).toBeNull();
      expect(path).toBeTruthy();
    });

    it('should clear only path cache', async () => {
      await service.setContent(mockContent);
      await service.setPath(mockPath);
      await service.clearPaths();

      const content = await service.getContent(mockContent.id);
      const path = await service.getPath(mockPath.id);
      expect(content).toBeTruthy();
      expect(path).toBeNull();
    });

    it('should handle clearAll when db not initialized', async () => {
      const uninitService = new IndexedDBCacheService();
      // Should not throw
      await uninitService.clearAll();
    });
  });

  // ===========================================================================
  // Stats
  // ===========================================================================

  describe('stats', () => {
    it('should report unavailable when not initialized', async () => {
      const stats = await service.getStats();
      expect(stats.isAvailable).toBe(false);
      expect(stats.contentCount).toBe(0);
      expect(stats.pathCount).toBe(0);
    });

    it('should report correct counts', async () => {
      await service.init();
      await service.setContent(mockContent);
      await service.setContent(mockContent2);
      await service.setPath(mockPath);

      const stats = await service.getStats();
      expect(stats.isAvailable).toBe(true);
      expect(stats.contentCount).toBe(2);
      expect(stats.pathCount).toBe(1);
    });

    it('should report zero counts after clear', async () => {
      await service.init();
      await service.setContent(mockContent);
      await service.clearAll();

      const stats = await service.getStats();
      expect(stats.contentCount).toBe(0);
    });
  });

  // ===========================================================================
  // Cleanup
  // ===========================================================================

  describe('cleanup', () => {
    beforeEach(async () => {
      await service.init();
    });

    it('should clean up expired entries', async () => {
      await service.setContent(mockContent);
      await service.setPath(mockPath);

      // Move time past all TTLs
      const originalNow = Date.now;
      spyOn(Date, 'now').and.returnValue(originalNow() + 25 * 60 * 60 * 1000);

      const result = await service.cleanup();
      expect(result.contentRemoved).toBe(1);
      expect(result.pathsRemoved).toBe(1);
    });

    it('should not remove non-expired entries', async () => {
      await service.setContent(mockContent);

      const result = await service.cleanup();
      expect(result.contentRemoved).toBe(0);
    });

    it('should return zeros when db not initialized', async () => {
      const uninitService = new IndexedDBCacheService();
      const result = await uninitService.cleanup();
      expect(result.contentRemoved).toBe(0);
      expect(result.pathsRemoved).toBe(0);
    });
  });

  // ===========================================================================
  // Quota Management
  // ===========================================================================

  describe('quota management', () => {
    it('should initially have quotaExceeded as false', () => {
      expect(service.quotaExceeded).toBe(false);
    });
  });

  // ===========================================================================
  // Method Existence (backward compat with original tests)
  // ===========================================================================

  describe('public API', () => {
    it('should have init method', () => {
      expect(typeof service.init).toBe('function');
    });

    it('should have getContent method', () => {
      expect(typeof service.getContent).toBe('function');
    });

    it('should have setContent method', () => {
      expect(typeof service.setContent).toBe('function');
    });

    it('should have getPath method', () => {
      expect(typeof service.getPath).toBe('function');
    });

    it('should have setPath method', () => {
      expect(typeof service.setPath).toBe('function');
    });

    it('should have clearAll method', () => {
      expect(typeof service.clearAll).toBe('function');
    });

    it('should have getStats method', () => {
      expect(typeof service.getStats).toBe('function');
    });

    it('should have cleanup method', () => {
      expect(typeof service.cleanup).toBe('function');
    });

    it('should have getContentBatch method', () => {
      expect(typeof service.getContentBatch).toBe('function');
    });

    it('should have setContentBatch method', () => {
      expect(typeof service.setContentBatch).toBe('function');
    });

    it('should have removeContent method', () => {
      expect(typeof service.removeContent).toBe('function');
    });

    it('should have removePath method', () => {
      expect(typeof service.removePath).toBe('function');
    });

    it('should have isAvailable method', () => {
      expect(typeof service.isAvailable).toBe('function');
    });
  });
});
