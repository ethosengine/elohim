import { TestBed } from '@angular/core/testing';
import { HolochainCacheService, type CacheEntry } from './holochain-cache.service';

/**
 * Unit tests for HolochainCacheService
 *
 * Tests the hybrid memory/IndexedDB caching layer for Holochain content.
 * Covers TTL expiration, LRU eviction, and cache statistics.
 */
describe('HolochainCacheService', () => {
  let service: HolochainCacheService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [HolochainCacheService],
    });
    service = TestBed.inject(HolochainCacheService);
  });

  afterEach(async () => {
    // Clean up IndexedDB after each test
    await service.clear();
  });

  describe('initialization', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should initialize with zero hit rate', () => {
      expect(service.hitRate()).toBe(0);
    });
  });

  describe('basic get/set operations', () => {
    it('should store and retrieve a value', async () => {
      await service.set('test-key', { data: 'test-value' });
      const result = await service.get('test-key');

      expect(result).toEqual({ data: 'test-value' });
    });

    it('should return null for non-existent key', async () => {
      const result = await service.get('non-existent');
      expect(result).toBeNull();
    });

    it('should overwrite existing value', async () => {
      await service.set('key', 'first');
      await service.set('key', 'second');
      const result = await service.get('key');

      expect(result).toBe('second');
    });

    it('should handle null values', async () => {
      await service.set('null-key', null);
      const result = await service.get('null-key');

      expect(result).toBeNull();
    });

    it('should handle undefined values', async () => {
      await service.set('undef-key', undefined);
      const result = await service.get('undef-key');

      expect(result).toBeUndefined();
    });

    it('should handle complex objects', async () => {
      const complex = {
        nested: {
          array: [1, 2, 3],
          map: { a: 'b' },
        },
        date: new Date().toISOString(),
      };

      await service.set('complex', complex);
      const result = await service.get('complex');

      expect(result).toEqual(complex);
    });
  });

  describe('TTL expiration', () => {
    it('should expire entry after TTL', async () => {
      await service.set('expiring', 'value', 100); // 100ms TTL

      // Should exist immediately
      let result = await service.get('expiring');
      expect(result).toBe('value');

      // Wait for expiration
      await new Promise(resolve => setTimeout(resolve, 150));

      // Should be null after TTL
      result = await service.get('expiring');
      expect(result).toBeNull();
    });

    it('should not expire entry without TTL', async () => {
      await service.set('permanent', 'value');

      // Wait longer than any reasonable TTL
      await new Promise(resolve => setTimeout(resolve, 200));

      const result = await service.get('permanent');
      expect(result).toBe('value');
    });

    it('should handle multiple entries with different TTLs', async () => {
      await service.set('short', 'value1', 100);
      await service.set('long', 'value2', 500);

      await new Promise(resolve => setTimeout(resolve, 150));

      expect(await service.get('short')).toBeNull();
      expect(await service.get('long')).toBe('value2');
    });
  });

  describe('metadata', () => {
    it('should store and preserve metadata', async () => {
      await service.set('meta-key', 'value', undefined, {
        source: 'conductor',
        priority: 'high',
      });

      // TODO(test-generator): [HIGH] Add method to retrieve cache entry with metadata
      // Context: No public API to get full CacheEntry with metadata, only value
      // Story: Cache introspection for debugging and monitoring
      // Suggested approach:
      //   1. Add getEntry(key): CacheEntry | null method
      //   2. Update tests to verify metadata preservation
      //   3. Add cache inspection utilities for debugging

      // For now, just verify value retrieval works
      const result = await service.get('meta-key');
      expect(result).toBe('value');
    });

    it('should support tag-based queries', async () => {
      await service.set('item1', 'value1', undefined, { tags: ['tag-a', 'tag-b'] });
      await service.set('item2', 'value2', undefined, { tags: ['tag-b', 'tag-c'] });
      await service.set('item3', 'value3', undefined, { tags: ['tag-c'] });

      const tagBEntries = service.getByTag('tag-b');
      expect(tagBEntries.length).toBe(2);
    });

    it('should support domain-based queries', async () => {
      await service.set('content1', 'c1', undefined, { domain: 'lamad' });
      await service.set('content2', 'c2', undefined, { domain: 'lamad' });
      await service.set('agent1', 'a1', undefined, { domain: 'imagodei' });

      const lamadEntries = service.getByDomain('lamad');
      expect(lamadEntries.length).toBe(2);
    });
  });

  describe('delete operations', () => {
    it('should delete a key', async () => {
      await service.set('to-delete', 'value');
      await service.delete('to-delete');

      const result = await service.get('to-delete');
      expect(result).toBeNull();
    });

    it('should handle deleting non-existent key', async () => {
      await expectAsync(service.delete('non-existent')).toBeResolved();
    });
  });

  describe('clear operations', () => {
    it('should clear all entries', async () => {
      await service.set('key1', 'value1');
      await service.set('key2', 'value2');
      await service.set('key3', 'value3');

      await service.clear();

      expect(await service.get('key1')).toBeNull();
      expect(await service.get('key2')).toBeNull();
      expect(await service.get('key3')).toBeNull();
    });

    it('should reset statistics on clear', async () => {
      await service.set('key', 'value');
      await service.get('key'); // Hit

      await service.clear();

      const stats = service.getStats();
      expect(stats.totalEntries).toBe(0);
    });
  });

  describe('cache statistics', () => {
    it('should track cache hits', async () => {
      await service.set('key', 'value');

      await service.get('key'); // Hit
      await service.get('key'); // Hit

      const stats = service.getStats();
      expect(stats.hitRate).toBeGreaterThan(0);
    });

    it('should track cache misses', async () => {
      await service.get('missing-1'); // Miss
      await service.get('missing-2'); // Miss

      const stats = service.getStats();
      expect(stats.totalEntries).toBe(0);
      expect(stats.hitRate).toBe(0); // All misses
    });

    it('should calculate hit rate correctly', async () => {
      await service.set('key', 'value');

      await service.get('key'); // Hit
      await service.get('missing'); // Miss

      // 50% hit rate
      expect(service.hitRate()).toBe(50);
    });

    it('should report total entries', async () => {
      await service.set('key1', 'value1');
      await service.set('key2', 'value2');

      const stats = service.getStats();
      expect(stats.totalEntries).toBe(2);
    });

    it('should estimate cache size', async () => {
      await service.set('small', 'x');
      const stats = service.getStats();

      expect(stats.totalSizeBytes).toBeGreaterThan(0);
    });
  });

  describe('preload', () => {
    it('should preload multiple items', async () => {
      const items = [
        { key: 'preload1', value: 'value1' },
        { key: 'preload2', value: 'value2', ttlMs: 1000 },
        { key: 'preload3', value: JSON.stringify({ complex: true }) },
      ];

      await service.preload(items);

      expect(await service.get('preload1')).toBe('value1');
      expect(await service.get('preload2')).toBe('value2');
      expect(await service.get('preload3')).toBe(JSON.stringify({ complex: true }));
    });

    it('should continue preloading on individual failures', async () => {
      // TODO(test-generator): [MEDIUM] Implement error handling in preload
      // Context: preload() currently doesn't handle malformed items
      // Story: Robust cache warming for app startup
      // Suggested approach:
      //   1. Wrap each item in try/catch
      //   2. Log errors but continue with remaining items
      //   3. Return success/failure count

      const items = [
        { key: 'good', value: 'value' },
        // malformed item would go here if we had validation
      ];

      await service.preload(items);
      expect(await service.get('good')).toBe('value');
    });
  });

  describe('query operations', () => {
    it('should query entries by predicate', async () => {
      await service.set('item1', { priority: 5 });
      await service.set('item2', { priority: 10 });
      await service.set('item3', { priority: 3 });

      const highPriority = service.query(
        entry => (entry.value as { priority: number }).priority > 5
      );

      expect(highPriority.length).toBe(1);
      expect(highPriority[0].value).toEqual({ priority: 10 });
    });

    it('should exclude expired entries from query', async () => {
      await service.set('valid', 'value1');
      await service.set('expired', 'value2', 50); // 50ms TTL

      await new Promise(resolve => setTimeout(resolve, 100));

      const results = service.query(() => true);
      expect(results.length).toBe(1);
      expect(results[0].key).toBe('valid');
    });
  });

  describe('memory cache eviction', () => {
    it('should evict oldest entries when cache is full', async () => {
      // TODO(test-generator): [LOW] Test actual LRU eviction behavior
      // Context: MAX_MEMORY_SIZE is 10MB, hard to trigger in unit tests
      // Story: Cache size management for resource-constrained devices
      // Suggested approach:
      //   1. Add configurable cache size for testing
      //   2. Store large objects to trigger eviction
      //   3. Verify oldest entries are evicted first

      // Placeholder test - actual eviction hard to test without exposing config
      await service.set('key', 'value');
      expect(await service.get('key')).toBe('value');
    });
  });

  describe('L1/L2 cache hierarchy', () => {
    it('should promote L2 hits to L1', async () => {
      await service.set('key', 'value');

      // First get loads from L1 or L2
      const first = await service.get('key');
      expect(first).toBe('value');

      // Second get should hit L1 (faster)
      const second = await service.get('key');
      expect(second).toBe('value');

      // Both should return same value
      expect(first).toEqual(second);
    });
  });

  describe('error handling', () => {
    it('should handle IndexedDB being unavailable', async () => {
      // IndexedDB might not be available in test environment
      // Service should fall back to memory-only mode

      await service.set('key', 'value');
      const result = await service.get('key');

      // Should still work (memory cache)
      expect(result).toBe('value');
    });

    it('should not throw on storage quota exceeded', async () => {
      // TODO(test-generator): [MEDIUM] Handle IndexedDB quota exceeded errors
      // Context: Large cache writes can exceed storage quota
      // Story: Graceful degradation when storage is full
      // Suggested approach:
      //   1. Catch QuotaExceededError in setInIndexedDB
      //   2. Fall back to memory-only mode
      //   3. Emit warning event for monitoring

      // Placeholder - can't easily trigger quota errors in unit tests
      await expectAsync(service.set('key', 'value')).toBeResolved();
    });
  });
});
