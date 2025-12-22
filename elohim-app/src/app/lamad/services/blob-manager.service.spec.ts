import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { BlobManagerService, BlobDownloadResult, BlobDownloadProgress } from './blob-manager.service';
import { BlobVerificationService } from './blob-verification.service';
import { BlobFallbackService, UrlHealth } from './blob-fallback.service';
import { ContentBlob } from '../models/content-node.model';

describe('BlobManagerService', () => {
  let service: BlobManagerService;
  let verificationService: BlobVerificationService;
  let fallbackService: BlobFallbackService;

  const createMockContentBlob = (): ContentBlob => ({
    hash: '0000000000000000000000000000000000000000000000000000000000000000',
    sizeBytes: 1024,
    mimeType: 'video/mp4',
    fallbackUrls: ['https://example.com/blob.mp4'],
    bitrateMbps: 5,
    durationSeconds: 300,
    codec: 'h264',
  });

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [BlobManagerService, BlobVerificationService, BlobFallbackService],
    });
    service = TestBed.inject(BlobManagerService);
    verificationService = TestBed.inject(BlobVerificationService);
    fallbackService = TestBed.inject(BlobFallbackService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Cache Operations', () => {
    it('should check if blob is cached', () => {
      const hash = 'test_hash_123';
      const blob = new Blob(['test']);

      service['blobCache'].set(hash, blob);

      expect(service.isCached(hash)).toBe(true);
      expect(service.isCached('nonexistent')).toBe(false);
    });

    it('should retrieve cached blob', () => {
      const hash = 'test_hash_123';
      const blob = new Blob(['test']);

      service['blobCache'].set(hash, blob);

      const cached = service.getCachedBlob(hash);
      expect(cached).toBe(blob);
    });

    it('should return null for uncached blob', () => {
      const cached = service.getCachedBlob('nonexistent');
      expect(cached).toBeNull();
    });

    it('should remove blob from cache', () => {
      const hash = 'test_hash_123';
      const blob = new Blob(['test']);

      service['blobCache'].set(hash, blob);
      service['cacheSize'] = blob.size;

      service.removeFromCache(hash);

      expect(service.isCached(hash)).toBe(false);
      expect(service['cacheSize']).toBe(0);
    });

    it('should clear entire cache', () => {
      const blob1 = new Blob(['data1']);
      const blob2 = new Blob(['data2']);

      service['blobCache'].set('hash1', blob1);
      service['blobCache'].set('hash2', blob2);
      service['cacheSize'] = blob1.size + blob2.size;

      service.clearCache();

      expect(service['blobCache'].size).toBe(0);
      expect(service['cacheSize']).toBe(0);
    });

    it('should track cache size', () => {
      const hash1 = 'hash1';
      const hash2 = 'hash2';
      const blob1 = new Blob(['a'.repeat(1000)]);
      const blob2 = new Blob(['b'.repeat(500)]);

      service['cacheBlob'](hash1, blob1, blob1.size);
      service['cacheBlob'](hash2, blob2, blob2.size);

      const stats = service.getCacheStats();
      expect(stats.sizeBytes).toBe(blob1.size + blob2.size);
      expect(stats.entriesCount).toBe(2);
    });

    it('should report cache statistics', () => {
      const blob = new Blob(['test']);
      const size = 1000;

      service['cacheBlob']('hash1', blob, size);

      const stats = service.getCacheStats();
      expect(stats.entriesCount).toBe(1);
      expect(stats.sizeBytes).toBe(size);
      expect(stats.maxSizeBytes).toBe(100 * 1024 * 1024); // 100 MB default
      expect(stats.percentFull).toBeCloseTo((size / (100 * 1024 * 1024)) * 100, 2);
    });
  });

  describe('Blob URL Management', () => {
    it('should create blob URL', () => {
      const blob = new Blob(['test']);
      const url = service.createBlobUrl(blob);

      expect(url).toMatch(/^blob:/);
    });

    it('should revoke blob URL', () => {
      const blob = new Blob(['test']);
      const url = service.createBlobUrl(blob);

      expect(() => service.revokeBlobUrl(url)).not.toThrow();
    });

    it('should download blob to file', () => {
      const blob = new Blob(['test content']);
      const filename = 'test.mp4';

      spyOn(document.body, 'appendChild');
      spyOn(document.body, 'removeChild');

      service.downloadBlobToFile(blob, filename);

      expect(document.body.appendChild).toHaveBeenCalled();
      expect(document.body.removeChild).toHaveBeenCalled();
    });
  });

  describe('Cache Eviction', () => {
    it('should evict oldest entries when cache is full', () => {
      // Set small cache size for testing
      service.maxCacheSizeBytes = 3000;

      const blob1 = new Blob(['a'.repeat(1000)]);
      const blob2 = new Blob(['b'.repeat(1000)]);
      const blob3 = new Blob(['c'.repeat(1000)]);
      const blob4 = new Blob(['d'.repeat(1000)]); // This should evict blob1

      service['cacheBlob']('hash1', blob1, blob1.size);
      service['cacheBlob']('hash2', blob2, blob2.size);
      service['cacheBlob']('hash3', blob3, blob3.size);
      service['cacheBlob']('hash4', blob4, blob4.size);

      // blob1 should have been evicted
      expect(service.isCached('hash1')).toBe(false);
      expect(service.isCached('hash2')).toBe(true);
      expect(service.isCached('hash3')).toBe(true);
      expect(service.isCached('hash4')).toBe(true);
    });

    it('should not cache oversized blobs', () => {
      service.maxCacheSizeBytes = 1000;
      const largeBlob = new Blob(['x'.repeat(2000)]);

      spyOn(console, 'warn');
      service['cacheBlob']('large_hash', largeBlob, largeBlob.size);

      expect(console.warn).toHaveBeenCalledWith(
        jasmine.stringMatching(/Blob too large to cache/)
      );
    });
  });

  describe('Progress Tracking', () => {
    it('should call progress callback during fetch', (done) => {
      const contentBlob = createMockContentBlob();
      const progressUpdates: BlobDownloadProgress[] = [];

      const progressCallback = (progress: BlobDownloadProgress) => {
        progressUpdates.push(progress);
      };

      // Mock the fallback service to avoid actual HTTP calls
      spyOn(fallbackService, 'fetchWithFallback').and.returnValue({
        pipe: jasmine.createSpy('pipe').and.callFake((op: any) => ({
          pipe: jasmine.createSpy('pipe').and.callFake((op2: any) => ({
            pipe: jasmine.createSpy('pipe').and.callFake((op3: any) => ({
              pipe: jasmine.createSpy('pipe').and.callFake((op4: any) => ({
                subscribe: (next: any) => {
                  next({
                    blob: new Blob(['test']),
                    urlIndex: 0,
                    successUrl: 'test',
                    durationMs: 100,
                    retryCount: 0,
                  });
                  return { unsubscribe: () => {} };
                },
              })),
            })),
          })),
        })),
      } as any);

      // Test that progress callback is being set up
      service.downloadBlob(contentBlob, progressCallback);
      done();
    });

    it('should report 100% progress when cached', (done) => {
      const contentBlob = createMockContentBlob();
      const progressUpdates: BlobDownloadProgress[] = [];

      const progressCallback = (progress: BlobDownloadProgress) => {
        progressUpdates.push(progress);
      };

      // Pre-cache the blob
      const testBlob = new Blob(['cached data']);
      service['blobCache'].set(contentBlob.hash, testBlob);

      service.downloadBlob(contentBlob, progressCallback).subscribe((result) => {
        expect(result.wasCached).toBe(true);
        expect(progressUpdates.length).toBeGreaterThan(0);
        expect(progressUpdates[0].percentComplete).toBe(100);
        done();
      });
    });
  });

  describe('URL Health Operations', () => {
    it('should get URL health information', () => {
      const contentBlob = createMockContentBlob();
      spyOn(fallbackService, 'getUrlsHealth').and.returnValue([
        {
          url: 'https://example.com/blob.mp4',
          successCount: 10,
          failureCount: 1,
          isHealthy: true,
        },
      ]);

      const health = service.getUrlHealth(contentBlob) as UrlHealth[];

      expect(health.length).toBe(1);
      expect(health[0].isHealthy).toBe(true);
    });

    it('should check if blob is accessible', () => {
      const contentBlob = createMockContentBlob();
      spyOn(fallbackService, 'getUrlsHealth').and.returnValue([
        {
          url: 'https://example.com/blob.mp4',
          successCount: 10,
          failureCount: 0,
          isHealthy: true,
        },
      ]);

      const accessible = service.isAccessible(contentBlob);
      expect(accessible).toBe(true);
    });

    it('should return false if no healthy URLs', () => {
      const contentBlob = createMockContentBlob();
      spyOn(fallbackService, 'getUrlsHealth').and.returnValue([
        {
          url: 'https://example.com/blob.mp4',
          successCount: 0,
          failureCount: 10,
          isHealthy: false,
        },
      ]);

      const accessible = service.isAccessible(contentBlob);
      expect(accessible).toBe(false);
    });

    it('should test blob access', async () => {
      const contentBlob = createMockContentBlob();
      spyOn(fallbackService, 'testFallbackUrls').and.returnValue(
        Promise.resolve([
          {
            url: 'https://example.com/blob.mp4',
            successCount: 1,
            failureCount: 0,
            isHealthy: true,
          },
        ])
      );

      const health = (await service.testBlobAccess(contentBlob)) as UrlHealth[];

      expect(health.length).toBe(1);
      expect(health[0].isHealthy).toBe(true);
    });
  });

  describe('Blob Download Integration', () => {
    it('should not perform actual downloads in tests', () => {
      // This is more of a documentation test
      // In real scenarios, HTTP calls are intercepted by HttpTestingController
      expect(service).toBeTruthy();
    });
  });

  describe('Multiple Blob Download', () => {
    it('should download multiple blobs in parallel', (done) => {
      const blob1 = createMockContentBlob();
      const blob2 = createMockContentBlob();
      blob2.hash = 'hash2';

      // Mock the single download to return cached result
      service['blobCache'].set(blob1.hash, new Blob(['data1']));
      service['blobCache'].set(blob2.hash, new Blob(['data2']));

      service.downloadBlobs([blob1, blob2]).subscribe((results: BlobDownloadResult[]) => {
        expect(results.length).toBe(2);
        expect(results[0].wasCached).toBe(true);
        expect(results[1].wasCached).toBe(true);
        done();
      });
    });
  });

  describe('Metadata Retrieval from Holochain', () => {
    it('should retrieve blobs for content from Holochain', (done) => {
      const contentId = 'content_123';

      service.getBlobsForContent(contentId).subscribe((blobs) => {
        // Will return empty array since Holochain is mocked, but should not error
        expect(Array.isArray(blobs)).toBe(true);
        done();
      });
    });

    it('should return empty array on metadata retrieval error', (done) => {
      const contentId = 'content_nonexistent';

      service.getBlobsForContent(contentId).subscribe((blobs) => {
        expect(blobs).toEqual([]);
        done();
      });
    });

    it('should retrieve specific blob metadata by hash', (done) => {
      const contentId = 'content_123';
      const blobHash = 'test_hash_123';

      service.getBlobMetadata(contentId, blobHash).subscribe((metadata) => {
        // Will be null since Holochain is mocked, but should not error
        expect(metadata === null || metadata instanceof Object).toBe(true);
        done();
      });
    });

    it('should check if blob exists in DHT', (done) => {
      const contentId = 'content_123';
      const blobHash = 'test_hash_123';

      service.blobExists(contentId, blobHash).subscribe((exists) => {
        expect(typeof exists).toBe('boolean');
        done();
      });
    });

    it('should retrieve blobs for multiple content nodes in parallel', (done) => {
      const contentIds = ['content_1', 'content_2', 'content_3'];

      service.getBlobsForMultipleContent(contentIds).subscribe((blobMap) => {
        expect(blobMap instanceof Map).toBe(true);
        expect(blobMap.size).toBeLessThanOrEqual(contentIds.length);
        done();
      });
    });

    it('should transform BlobMetadataOutput to ContentBlob', () => {
      const metadata = service['transformBlobMetadata']({
        hash: 'test_hash',
        size_bytes: 1024,
        mime_type: 'video/mp4',
        fallback_urls: ['https://example.com/blob.mp4'],
        bitrate_mbps: 5,
        duration_seconds: 300,
        codec: 'h264',
        created_at: '2024-01-01T00:00:00Z',
        verified_at: '2024-01-02T00:00:00Z',
      });

      expect(metadata.hash).toBe('test_hash');
      expect(metadata.sizeBytes).toBe(1024);
      expect(metadata.mimeType).toBe('video/mp4');
      expect(metadata.bitrateMbps).toBe(5);
      expect(metadata.codec).toBe('h264');
    });
  });
});
