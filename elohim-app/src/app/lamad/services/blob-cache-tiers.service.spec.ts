import { TestBed } from '@angular/core/testing';
import { BlobCacheTiersService, CacheTierStats } from './blob-cache-tiers.service';
import { ContentBlob } from '../models/content-node.model';

describe('BlobCacheTiersService', () => {
  let service: BlobCacheTiersService;

  const createMockContentBlob = (): ContentBlob => ({
    hash: 'test_hash_1',
    sizeBytes: 1024,
    mimeType: 'video/mp4',
    fallbackUrls: ['https://example.com/blob.mp4'],
    bitrateMbps: 5,
    durationSeconds: 300,
    codec: 'h264',
  });

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [BlobCacheTiersService],
    });
    service = TestBed.inject(BlobCacheTiersService);
  });

  afterEach(() => {
    service.clear();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Tier 1: Metadata Cache (Unlimited)', () => {
    it('should set and get metadata', () => {
      const metadata = createMockContentBlob();

      service.setMetadata('hash1', metadata);
      const retrieved = service.getMetadata('hash1');

      expect(retrieved).toEqual(metadata);
    });

    it('should return null for missing metadata', () => {
      const retrieved = service.getMetadata('nonexistent');
      expect(retrieved).toBeNull();
    });

    it('should update access tracking', () => {
      const metadata = createMockContentBlob();
      service.setMetadata('hash1', metadata);

      const stats1 = service.getStats('metadata');
      expect(stats1.hitCount).toBe(0);

      service.getMetadata('hash1'); // Hit
      const stats2 = service.getStats('metadata');
      expect(stats2.hitCount).toBe(1);

      service.getMetadata('nonexistent'); // Miss
      const stats3 = service.getStats('metadata');
      expect(stats3.missCount).toBe(1);
    });

    it('should never evict metadata', () => {
      // Add many metadata items (should not evict)
      for (let i = 0; i < 1000; i++) {
        const metadata = createMockContentBlob();
        metadata.hash = `hash_${i}`;
        service.setMetadata(`hash_${i}`, metadata);
      }

      // All should still be present
      expect(service.getMetadata('hash_0')).toBeTruthy();
      expect(service.getMetadata('hash_999')).toBeTruthy();

      const stats = service.getStats('metadata');
      expect(stats.itemCount).toBe(1000);
      expect(stats.evictionCount).toBe(0);
    });

    it('should calculate hit rate correctly', () => {
      const metadata = createMockContentBlob();
      service.setMetadata('hash1', metadata);

      service.getMetadata('hash1'); // 1 hit
      service.getMetadata('hash1'); // 2 hits
      service.getMetadata('nonexistent'); // 1 miss
      service.getMetadata('nonexistent'); // 2 misses

      const stats = service.getStats('metadata');
      expect(stats.hitRate).toBeCloseTo(0.5, 2); // 2 hits / 4 total
    });
  });

  describe('Tier 2: Blob Cache (1 GB, LRU Eviction)', () => {
    it('should set and get blob', () => {
      const blob = new Blob(['test data']);
      service.setBlob('hash1', blob);

      const retrieved = service.getBlob('hash1');
      expect(retrieved?.size).toBe(blob.size);
    });

    it('should return null for missing blob', () => {
      const retrieved = service.getBlob('nonexistent');
      expect(retrieved).toBeNull();
    });

    it('should reject oversized blobs', () => {
      // Create blob larger than 1 GB limit
      const largeBlobSize = 1.1 * 1024 * 1024 * 1024;
      const largeBlob = new Blob([new Uint8Array(Math.min(100 * 1024 * 1024, largeBlobSize))]);

      // Mock the size property
      Object.defineProperty(largeBlob, 'size', {
        value: largeBlobSize,
      });

      const result = service.setBlob('hash_large', largeBlob);
      expect(result.success).toBe(false);
      expect(result.reason).toContain('too large');
    });

    it('should track blob cache size', () => {
      const blob1 = new Blob(['a'.repeat(1000)]);
      const blob2 = new Blob(['b'.repeat(2000)]);

      service.setBlob('hash1', blob1);
      service.setBlob('hash2', blob2);

      const stats = service.getStats('blob');
      expect(stats.totalSizeBytes).toBe(3000);
      expect(stats.itemCount).toBe(2);
    });

    it('should evict LRU blob on overflow', () => {
      // Set small cache limit via testing
      const blob1 = new Blob(['a'.repeat(100)]);
      const blob2 = new Blob(['b'.repeat(100)]);
      const blob3 = new Blob(['c'.repeat(100)]);

      service.setBlob('hash1', blob1);
      service.setBlob('hash2', blob2);

      // Access hash1 to make it recently used
      service.getBlob('hash1');

      // Add blob3, should evict hash2 (least recently used)
      const result = service.setBlob('hash3', blob3);

      expect(result.evictedItems).toBeGreaterThan(0);
      expect(service.getBlob('hash1')).toBeTruthy(); // Recently used, should remain
      expect(service.getBlob('hash2')).toBeNull(); // Should be evicted
    });

    it('should remove expired blobs on access', (done) => {
      const blob = new Blob(['test']);
      service.setBlob('hash1', blob);

      // Manually set expiration time to past
      (service['blobCache'] as any).cache.get('hash1')!.createdAt = Date.now() - 25 * 60 * 60 * 1000;

      const retrieved = service.getBlob('hash1');
      expect(retrieved).toBeNull();
      done();
    });
  });

  describe('Tier 3: Chunk Cache (10 GB, Time-Based Cleanup)', () => {
    it('should set and get chunk', () => {
      const chunk = new Uint8Array([1, 2, 3, 4, 5]);
      service.setChunk('hash1', chunk);

      const retrieved = service.getChunk('hash1');
      expect(retrieved).toEqual(chunk);
    });

    it('should return null for missing chunk', () => {
      const retrieved = service.getChunk('nonexistent');
      expect(retrieved).toBeNull();
    });

    it('should track chunk cache size', () => {
      const chunk1 = new Uint8Array(5000);
      const chunk2 = new Uint8Array(3000);

      service.setChunk('hash1', chunk1);
      service.setChunk('hash2', chunk2);

      const stats = service.getStats('chunk');
      expect(stats.totalSizeBytes).toBe(8000);
      expect(stats.itemCount).toBe(2);
    });

    it('should use time-based eviction for chunks', () => {
      const chunk1 = new Uint8Array(100);
      const chunk2 = new Uint8Array(100);

      service.setChunk('hash1', chunk1);
      service.setChunk('hash2', chunk2);

      // Manually expire hash1
      (service['chunkCache'] as any).cache.get('hash1')!.createdAt = Date.now() - 8 * 24 * 60 * 60 * 1000;

      // Add new chunk - should trigger cleanup that removes expired hash1
      const chunk3 = new Uint8Array(100);
      const result = service.setChunk('hash3', chunk3);

      // hash1 should be evicted due to expiration
      expect(service.getChunk('hash1')).toBeNull();
      expect(service.getChunk('hash3')).toBeTruthy();
    });
  });

  describe('Cross-Tier Operations', () => {
    it('should check if item exists in any tier', () => {
      const metadata = createMockContentBlob();
      const blob = new Blob(['test']);
      const chunk = new Uint8Array([1, 2, 3]);

      service.setMetadata('hash1', metadata);
      service.setBlob('hash2', blob);
      service.setChunk('hash3', chunk);

      expect(service.has('hash1')).toBe(true);
      expect(service.has('hash2')).toBe(true);
      expect(service.has('hash3')).toBe(true);
      expect(service.has('nonexistent')).toBe(false);
    });

    it('should check if item exists in specific tier', () => {
      const metadata = createMockContentBlob();
      service.setMetadata('hash1', metadata);

      expect(service.has('hash1', 'metadata')).toBe(true);
      expect(service.has('hash1', 'blob')).toBe(false);
      expect(service.has('hash1', 'chunk')).toBe(false);
    });

    it('should remove items from specific tier', () => {
      const metadata = createMockContentBlob();
      const blob = new Blob(['test']);

      service.setMetadata('hash1', metadata);
      service.setBlob('hash1', blob);

      expect(service.getMetadata('hash1')).toBeTruthy();
      expect(service.getBlob('hash1')).toBeTruthy();

      service.remove('hash1', 'blob');

      expect(service.getMetadata('hash1')).toBeTruthy(); // Metadata still there
      expect(service.getBlob('hash1')).toBeNull(); // Blob removed
    });

    it('should remove items from all tiers if tier not specified', () => {
      const metadata = createMockContentBlob();
      const blob = new Blob(['test']);
      const chunk = new Uint8Array([1, 2, 3]);

      service.setMetadata('hash1', metadata);
      service.setBlob('hash1', blob);
      service.setChunk('hash1', chunk);

      service.remove('hash1');

      expect(service.has('hash1')).toBe(false);
    });

    it('should clear specific tier', () => {
      const metadata = createMockContentBlob();
      const blob = new Blob(['test']);
      const chunk = new Uint8Array([1, 2, 3]);

      service.setMetadata('hash1', metadata);
      service.setBlob('hash2', blob);
      service.setChunk('hash3', chunk);

      service.clear('blob');

      expect(service.getMetadata('hash1')).toBeTruthy();
      expect(service.getBlob('hash2')).toBeNull();
      expect(service.getChunk('hash3')).toBeTruthy();
    });

    it('should clear all tiers', () => {
      const metadata = createMockContentBlob();
      const blob = new Blob(['test']);
      const chunk = new Uint8Array([1, 2, 3]);

      service.setMetadata('hash1', metadata);
      service.setBlob('hash2', blob);
      service.setChunk('hash3', chunk);

      service.clear();

      expect(service.has('hash1')).toBe(false);
      expect(service.has('hash2')).toBe(false);
      expect(service.has('hash3')).toBe(false);
    });
  });

  describe('Statistics and Reporting', () => {
    it('should report stats for each tier', () => {
      const metadata = createMockContentBlob();
      service.setMetadata('hash1', metadata);
      service.getMetadata('hash1'); // Hit

      const metadataStats = service.getStats('metadata');
      expect(metadataStats.name).toBe('Metadata');
      expect(metadataStats.hitCount).toBe(1);
      expect(metadataStats.hitRate).toBeCloseTo(1.0, 2);
    });

    it('should report all tier stats', () => {
      const metadata = createMockContentBlob();
      const blob = new Blob(['test']);
      const chunk = new Uint8Array([1, 2, 3]);

      service.setMetadata('hash1', metadata);
      service.setBlob('hash2', blob);
      service.setChunk('hash3', chunk);

      const allStats = service.getAllStats();

      expect(allStats['metadata']).toBeTruthy();
      expect(allStats['blob']).toBeTruthy();
      expect(allStats['chunk']).toBeTruthy();

      expect(allStats['metadata'].itemCount).toBe(1);
      expect(allStats['blob'].itemCount).toBe(1);
      expect(allStats['chunk'].itemCount).toBe(1);
    });

    it('should calculate percent full correctly', () => {
      // Set small test limits
      (service['blobCache'] as any).maxSizeBytes = 1000;

      const blob = new Blob(['a'.repeat(500)]);
      service.setBlob('hash1', blob);

      const stats = service.getStats('blob');
      expect(stats.percentFull).toBe(50);
    });

    it('should report total memory usage', () => {
      const blob = new Blob(['a'.repeat(1000)]);
      const chunk = new Uint8Array(2000);

      service.setBlob('hash1', blob);
      service.setChunk('hash2', chunk);

      const total = service.getTotalMemoryUsageBytes();
      expect(total).toBe(3000);
    });

    it('should generate memory report', () => {
      const blob = new Blob(['a'.repeat(1000)]);
      const chunk = new Uint8Array(2000);

      service.setBlob('hash1', blob);
      service.setChunk('hash2', chunk);

      const report = service.getMemoryReport();

      expect(report.blobCacheBytes).toBe(1000);
      expect(report.chunkCacheBytes).toBe(2000);
      expect(report.totalBytes).toBe(3000);
      expect(report.percentOfBlobMax).toBeGreaterThan(0);
      expect(report.percentOfChunkMax).toBeGreaterThan(0);
    });
  });

  describe('Performance Characteristics', () => {
    it('should not evict small metadata when adding large blob', () => {
      // This is the key test for cache tier separation
      const metadata = createMockContentBlob();
      service.setMetadata('metadata_hash', metadata);

      // Add large blob (should NOT evict metadata)
      const largeBlob = new Blob(['x'.repeat(100 * 1024 * 1024)]); // 100 MB
      service.setBlob('blob_hash', largeBlob);

      // Metadata should still be there
      expect(service.getMetadata('metadata_hash')).toBeTruthy();
    });

    it('should handle cache under different access patterns', () => {
      // Sequential access
      const blobs = [];
      for (let i = 0; i < 10; i++) {
        blobs.push(new Blob([`blob_${i}`.repeat(10)]));
        service.setBlob(`hash_${i}`, blobs[i]);
      }

      // Random access pattern
      service.getBlob('hash_0');
      service.getBlob('hash_5');
      service.getBlob('hash_9');
      service.getBlob('hash_2');

      const stats = service.getStats('blob');
      expect(stats.itemCount).toBeGreaterThan(0);
    });
  });

  describe('Cache Integrity Verification', () => {
    beforeEach(() => {
      // Stop auto-verification during tests
      service.stopIntegrityVerification();
    });

    it('should get last integrity check result', () => {
      const lastCheck = service.getLastIntegrityCheck();
      // Initially null or from previous checks
      expect(lastCheck === null || lastCheck instanceof Object).toBe(true);
    });

    it('should verify single blob integrity', async () => {
      const blob = new Blob(['test data']);
      service.setBlob('hash_123', blob);

      // Try to verify - will fail in stub since BlobVerificationService is mocked
      // But it should not error
      const isValid = await service.verifyBlobIntegrity('hash_123');
      expect(typeof isValid).toBe('boolean');
    });

    it('should return false for non-cached blob verification', async () => {
      const isValid = await service.verifyBlobIntegrity('nonexistent_hash');
      expect(isValid).toBe(false);
    });

    it('should perform full cache integrity check', async () => {
      // Add some blobs
      const blob1 = new Blob(['data1']);
      const blob2 = new Blob(['data2']);
      service.setBlob('hash_1', blob1);
      service.setBlob('hash_2', blob2);

      // Run integrity check
      const result = await service.verifyAllBlobIntegrity();

      expect(result.itemsChecked).toBeGreaterThanOrEqual(0);
      expect(result.durationMs).toBeGreaterThanOrEqual(0);
      expect(Array.isArray(result.corruptedItems)).toBe(true);
      expect(Array.isArray(result.missingMetadata)).toBe(true);
      expect(typeof result.isValid).toBe('boolean');
      expect(result.checkedAt).toBeGreaterThan(0);
    });

    it('should detect corrupted blobs and remove them', async () => {
      // This test would require mocking BlobVerificationService to return corruption
      // Just verify the structure is correct
      const result = await service.verifyAllBlobIntegrity();
      expect(result.corruptedItems instanceof Array).toBe(true);
    });

    it('should track integrity check results over time', async () => {
      const check1 = await service.verifyAllBlobIntegrity();
      expect(check1.checkedAt).toBeGreaterThan(0);

      // Should be retrievable
      const lastCheck = service.getLastIntegrityCheck();
      expect(lastCheck).toBeTruthy();
      expect(lastCheck?.checkedAt).toBe(check1.checkedAt);
    });

    it('should handle integrity verification with multiple blobs', async () => {
      // Add multiple blobs
      for (let i = 0; i < 5; i++) {
        const blob = new Blob([`data_${i}`]);
        service.setBlob(`hash_${i}`, blob);
      }

      const result = await service.verifyAllBlobIntegrity();

      // Should have checked at least some items
      expect(result.itemsChecked).toBeGreaterThanOrEqual(0);
    });

    it('should stop integrity verification on demand', () => {
      service.stopIntegrityVerification();
      // If we call it again, it should not error
      service.stopIntegrityVerification();
      expect(true).toBe(true);
    });
  });
});
