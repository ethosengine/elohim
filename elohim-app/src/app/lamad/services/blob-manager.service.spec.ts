import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { Injector } from '@angular/core';
import {
  BlobManagerService,
  BlobDownloadResult,
  BlobDownloadProgress,
  BlobMetadataOutput,
  BlobsForContentOutput,
} from './blob-manager.service';
import { BlobVerificationService } from './blob-verification.service';
import { BlobFallbackService, BlobFetchResult, UrlHealth } from './blob-fallback.service';
import { ContentBlob } from '../models/content-node.model';
import { of, throwError } from 'rxjs';

describe('BlobManagerService', () => {
  let service: BlobManagerService;
  let verificationService: BlobVerificationService;
  let fallbackService: BlobFallbackService;
  let injector: Injector;

  const createMockContentBlob = (): ContentBlob => ({
    hash: '0000000000000000000000000000000000000000000000000000000000000000',
    sizeBytes: 1024,
    mimeType: 'video/mp4',
    fallbackUrls: ['https://example.com/blob.mp4'],
    bitrateMbps: 5,
    durationSeconds: 300,
    codec: 'h264',
  });

  const createMockBlobFetchResult = (): BlobFetchResult => ({
    blob: new Blob(['test content']),
    urlIndex: 0,
    successUrl: 'https://example.com/blob.mp4',
    durationMs: 100,
    retryCount: 0,
  });

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [BlobManagerService, BlobVerificationService, BlobFallbackService],
    });
    service = TestBed.inject(BlobManagerService);
    verificationService = TestBed.inject(BlobVerificationService);
    fallbackService = TestBed.inject(BlobFallbackService);
    injector = TestBed.inject(Injector);
  });

  // =========================================================================
  // Service Creation & Initialization
  // =========================================================================

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should initialize with empty blob cache', () => {
    expect(service['blobCache'].size).toBe(0);
  });

  it('should initialize with zero cache size', () => {
    expect(service['cacheSize']).toBe(0);
  });

  it('should initialize with max cache size of 100 MB', () => {
    expect(service.maxCacheSizeBytes).toBe(100 * 1024 * 1024);
  });

  it('should initialize cache lock as resolved promise', async () => {
    const lock = service['cacheLock'];
    expect(lock).toBeInstanceOf(Promise);
    await expectAsync(Promise.resolve(lock)).toBeResolved();
  });

  it('should initialize storageClient as null', () => {
    expect(service['storageClient']).toBeNull();
  });

  // =========================================================================
  // Public Method Existence Tests
  // =========================================================================

  it('should have getBlobUrl method', () => {
    expect(typeof service.getBlobUrl).toBe('function');
  });

  it('should have connectionMode getter', () => {
    expect(typeof Object.getOwnPropertyDescriptor(Object.getPrototypeOf(service), 'connectionMode')?.get).toBe(
      'function'
    );
  });

  it('should have getPriorityUrls method', () => {
    expect(typeof service.getPriorityUrls).toBe('function');
  });

  it('should have downloadBlob method', () => {
    expect(typeof service.downloadBlob).toBe('function');
  });

  it('should have downloadBlobs method', () => {
    expect(typeof service.downloadBlobs).toBe('function');
  });

  it('should have isCached method', () => {
    expect(typeof service.isCached).toBe('function');
  });

  it('should have getCachedBlob method', () => {
    expect(typeof service.getCachedBlob).toBe('function');
  });

  it('should have removeFromCache method', () => {
    expect(typeof service.removeFromCache).toBe('function');
  });

  it('should have clearCache method', () => {
    expect(typeof service.clearCache).toBe('function');
  });

  it('should have getCacheStats method', () => {
    expect(typeof service.getCacheStats).toBe('function');
  });

  it('should have getUrlHealth method', () => {
    expect(typeof service.getUrlHealth).toBe('function');
  });

  it('should have isAccessible method', () => {
    expect(typeof service.isAccessible).toBe('function');
  });

  it('should have testBlobAccess method', () => {
    expect(typeof service.testBlobAccess).toBe('function');
  });

  it('should have createBlobUrl method', () => {
    expect(typeof service.createBlobUrl).toBe('function');
  });

  it('should have revokeBlobUrl method', () => {
    expect(typeof service.revokeBlobUrl).toBe('function');
  });

  it('should have downloadBlobToFile method', () => {
    expect(typeof service.downloadBlobToFile).toBe('function');
  });

  it('should have getBlobsForContent method', () => {
    expect(typeof service.getBlobsForContent).toBe('function');
  });

  it('should have getBlobMetadata method', () => {
    expect(typeof service.getBlobMetadata).toBe('function');
  });

  it('should have blobExists method', () => {
    expect(typeof service.blobExists).toBe('function');
  });

  it('should have getBlobsForMultipleContent method', () => {
    expect(typeof service.getBlobsForMultipleContent).toBe('function');
  });

  // =========================================================================
  // Cache Operations
  // =========================================================================

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

    it('should remove blob from cache', async () => {
      const hash = 'test_hash_123';
      const blob = new Blob(['test']);

      service['blobCache'].set(hash, blob);
      service['cacheSize'] = blob.size;

      await service.removeFromCache(hash);

      expect(service.isCached(hash)).toBe(false);
      expect(service['cacheSize']).toBe(0);
    });

    it('should clear entire cache', async () => {
      const blob1 = new Blob(['data1']);
      const blob2 = new Blob(['data2']);

      service['blobCache'].set('hash1', blob1);
      service['blobCache'].set('hash2', blob2);
      service['cacheSize'] = blob1.size + blob2.size;

      await service.clearCache();

      expect(service['blobCache'].size).toBe(0);
      expect(service['cacheSize']).toBe(0);
    });

    it('should track cache size', async () => {
      const hash1 = 'hash1';
      const hash2 = 'hash2';
      const blob1 = new Blob(['a'.repeat(1000)]);
      const blob2 = new Blob(['b'.repeat(500)]);

      await service['cacheBlob'](hash1, blob1, blob1.size);
      await service['cacheBlob'](hash2, blob2, blob2.size);

      const stats = service.getCacheStats();
      expect(stats.sizeBytes).toBe(blob1.size + blob2.size);
      expect(stats.entriesCount).toBe(2);
    });

    it('should report cache statistics', async () => {
      const blob = new Blob(['test']);
      const size = 1000;

      await service['cacheBlob']('hash1', blob, size);

      const stats = service.getCacheStats();
      expect(stats.entriesCount).toBe(1);
      expect(stats.sizeBytes).toBe(size);
      expect(stats.maxSizeBytes).toBe(100 * 1024 * 1024); // 100 MB default
      expect(stats.percentFull).toBeCloseTo((size / (100 * 1024 * 1024)) * 100, 2);
    });
  });

  // =========================================================================
  // Strategy-Aware Blob URL Methods
  // =========================================================================

  describe('Strategy-Aware Blob URL Methods', () => {
    it('should get blob URL', () => {
      const blobHash = 'test_hash_123';
      spyOn(service as any, 'getStorageClient').and.returnValue({
        getBlobUrl: jasmine.createSpy('getBlobUrl').and.returnValue('https://doorway.host/blob/test_hash_123'),
      });

      const url = service.getBlobUrl(blobHash);
      expect(url).toBe('https://doorway.host/blob/test_hash_123');
    });

    it('should return connection mode', () => {
      const mode = service.connectionMode;
      expect(['doorway', 'direct']).toContain(mode);
    });

    it('should get priority URLs with strategy URL first', () => {
      const contentBlob = createMockContentBlob();
      contentBlob.fallbackUrls = [
        'https://example.com/blob.mp4',
        'https://fallback.com/blob.mp4',
      ];

      spyOn(service as any, 'getStorageClient').and.returnValue({
        getBlobUrl: jasmine.createSpy('getBlobUrl').and.returnValue('https://strategy.host/blob'),
      });

      const urls = service.getPriorityUrls(contentBlob);
      expect(urls[0]).toBe('https://strategy.host/blob');
      expect(urls.length).toBe(3); // strategy + 2 fallbacks
    });

    it('should not duplicate strategy URL in priority URLs', () => {
      const contentBlob = createMockContentBlob();
      const strategyUrl = 'https://strategy.host/blob';
      contentBlob.fallbackUrls = [strategyUrl, 'https://fallback.com/blob.mp4'];

      spyOn(service as any, 'getStorageClient').and.returnValue({
        getBlobUrl: jasmine.createSpy('getBlobUrl').and.returnValue(strategyUrl),
      });

      const urls = service.getPriorityUrls(contentBlob);
      const strategyCount = urls.filter(u => u === strategyUrl).length;
      expect(strategyCount).toBe(1); // Should not be duplicated
    });

    it('should lazy inject StorageClientService', () => {
      expect(service['storageClient']).toBeNull();
      spyOn(injector, 'get').and.returnValue({
        getBlobUrl: jasmine.createSpy('getBlobUrl').and.returnValue('https://test.com'),
      });

      service['getStorageClient']();
      expect(injector.get).toHaveBeenCalled();
    });
  });

  // =========================================================================
  // Blob URL Management
  // =========================================================================

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
      const removeSpy = jasmine.createSpy('remove');
      spyOn(document, 'createElement').and.returnValue({
        set href(_: string) { /* noop */ },
        set download(_: string) { /* noop */ },
        click: jasmine.createSpy('click'),
        remove: removeSpy,
      } as unknown as HTMLElement);

      service.downloadBlobToFile(blob, filename);

      expect(document.body.appendChild).toHaveBeenCalled();
      expect(removeSpy).toHaveBeenCalled();
    });

    it('createBlobUrl should return string', () => {
      const blob = new Blob(['test']);
      const url = service.createBlobUrl(blob);
      expect(typeof url).toBe('string');
    });

    it('downloadBlobToFile should create anchor element with download attribute', () => {
      const blob = new Blob(['test']);
      const filename = 'download.mp4';

      const mockAnchor = document.createElement('a');
      spyOn(document, 'createElement').and.returnValue(mockAnchor);
      spyOn(document.body, 'appendChild');
      spyOn(document.body, 'removeChild');

      service.downloadBlobToFile(blob, filename);

      expect(mockAnchor.download).toBe(filename);
    });
  });

  describe('Cache Eviction', () => {
    it('should evict oldest entries when cache is full', async () => {
      // Set small cache size for testing
      service.maxCacheSizeBytes = 3000;

      const blob1 = new Blob(['a'.repeat(1000)]);
      const blob2 = new Blob(['b'.repeat(1000)]);
      const blob3 = new Blob(['c'.repeat(1000)]);
      const blob4 = new Blob(['d'.repeat(1000)]); // This should evict blob1

      await service['cacheBlob']('hash1', blob1, blob1.size);
      await service['cacheBlob']('hash2', blob2, blob2.size);
      await service['cacheBlob']('hash3', blob3, blob3.size);
      await service['cacheBlob']('hash4', blob4, blob4.size);

      // blob1 should have been evicted
      expect(service.isCached('hash1')).toBe(false);
      expect(service.isCached('hash2')).toBe(true);
      expect(service.isCached('hash3')).toBe(true);
      expect(service.isCached('hash4')).toBe(true);
    });

    it('should not cache oversized blobs', async () => {
      service.maxCacheSizeBytes = 1000;
      const largeBlob = new Blob(['x'.repeat(2000)]);

      spyOn(console, 'warn');
      await service['cacheBlob']('large_hash', largeBlob, largeBlob.size);

      expect(console.warn).toHaveBeenCalledWith(jasmine.stringMatching(/Blob too large to cache/));
    });
  });

  // =========================================================================
  // Observable Return Type Tests
  // =========================================================================

  describe('Observable Return Types', () => {
    it('getBlobsForContent should return Observable', () => {
      const contentId = 'content_123';
      const result = service.getBlobsForContent(contentId);

      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('getBlobMetadata should return Observable', () => {
      const contentId = 'content_123';
      const blobHash = 'hash_123';
      const result = service.getBlobMetadata(contentId, blobHash);

      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('blobExists should return Observable', () => {
      const contentId = 'content_123';
      const blobHash = 'hash_123';
      const result = service.blobExists(contentId, blobHash);

      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('getBlobsForMultipleContent should return Observable', () => {
      const contentIds = ['id1', 'id2'];
      const result = service.getBlobsForMultipleContent(contentIds);

      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('downloadBlob should return Observable', () => {
      const contentBlob = createMockContentBlob();
      const result = service.downloadBlob(contentBlob);

      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('downloadBlobs should return Observable', () => {
      const blobs = [createMockContentBlob()];
      const result = service.downloadBlobs(blobs);

      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });
  });

  // =========================================================================
  // Simple Input/Output Tests
  // =========================================================================

  describe('Simple Input/Output Tests', () => {
    it('isCached should return boolean for valid hash', () => {
      const result = service.isCached('any_hash');
      expect(typeof result).toBe('boolean');
    });

    it('getCachedBlob should return Blob or null', () => {
      const result = service.getCachedBlob('nonexistent');
      expect(result === null || result instanceof Blob).toBe(true);
    });

    it('getCacheStats should return stats object with required properties', () => {
      const stats = service.getCacheStats();

      expect(typeof stats.entriesCount).toBe('number');
      expect(typeof stats.sizeBytes).toBe('number');
      expect(typeof stats.maxSizeBytes).toBe('number');
      expect(typeof stats.percentFull).toBe('number');
    });

    it('createBlobUrl should accept Blob and return string', () => {
      const blob = new Blob(['test']);
      const url = service.createBlobUrl(blob);

      expect(typeof url).toBe('string');
      expect(url.length).toBeGreaterThan(0);
    });

    it('revokeBlobUrl should accept string URL', () => {
      const blob = new Blob(['test']);
      const url = service.createBlobUrl(blob);

      expect(() => service.revokeBlobUrl(url)).not.toThrow();
    });

    it('downloadBlobToFile should accept Blob and filename', () => {
      const blob = new Blob(['test']);
      const filename = 'test.txt';

      spyOn(document.body, 'appendChild');
      spyOn(document.body, 'removeChild');

      expect(() => service.downloadBlobToFile(blob, filename)).not.toThrow();
    });

    it('removeFromCache should accept hash string', async () => {
      await expectAsync(service.removeFromCache('test_hash')).toBeResolved();
    });

    it('clearCache should return resolved Promise', async () => {
      await expectAsync(service.clearCache()).toBeResolved();
    });

    it('testBlobAccess should accept ContentBlob and return Promise', async () => {
      const contentBlob = createMockContentBlob();
      spyOn(fallbackService, 'testFallbackUrls').and.returnValue(Promise.resolve([]));

      const result = service.testBlobAccess(contentBlob);
      expect(result).toBeInstanceOf(Promise);
    });

    it('isAccessible should accept ContentBlob and return boolean', () => {
      const contentBlob = createMockContentBlob();
      spyOn(fallbackService, 'getUrlsHealth').and.returnValue([]);

      const result = service.isAccessible(contentBlob);
      expect(typeof result).toBe('boolean');
    });

    it('getUrlHealth should accept ContentBlob and return array', () => {
      const contentBlob = createMockContentBlob();
      spyOn(fallbackService, 'getUrlsHealth').and.returnValue([]);

      const result = service.getUrlHealth(contentBlob);
      expect(Array.isArray(result)).toBe(true);
    });

    it('getPriorityUrls should accept ContentBlob and return string array', () => {
      const contentBlob = createMockContentBlob();
      spyOn(service as any, 'getStorageClient').and.returnValue({
        getBlobUrl: jasmine.createSpy('getBlobUrl').and.returnValue('https://test.com'),
      });

      const result = service.getPriorityUrls(contentBlob);
      expect(Array.isArray(result)).toBe(true);
      expect(result.every(item => typeof item === 'string')).toBe(true);
    });

    it('getBlobUrl should accept string and return string', () => {
      const hash = 'test_hash';
      spyOn(service as any, 'getStorageClient').and.returnValue({
        getBlobUrl: jasmine.createSpy('getBlobUrl').and.returnValue('https://test.com/blob'),
      });

      const result = service.getBlobUrl(hash);
      expect(typeof result).toBe('string');
    });
  });

  // =========================================================================
  // Holochain Metadata Retrieval - Simple Tests
  // =========================================================================

  describe('Holochain Metadata Retrieval', () => {
    it('should retrieve blobs for content', done => {
      const contentId = 'content_123';

      service.getBlobsForContent(contentId).subscribe(blobs => {
        expect(Array.isArray(blobs)).toBe(true);
        done();
      });
    });

    it('should return empty array on metadata retrieval error', done => {
      const contentId = 'content_nonexistent';

      service.getBlobsForContent(contentId).subscribe(blobs => {
        expect(blobs).toEqual([]);
        done();
      });
    });

    it('should retrieve specific blob metadata by hash', done => {
      const contentId = 'content_123';
      const blobHash = 'test_hash_123';

      service.getBlobMetadata(contentId, blobHash).subscribe(metadata => {
        expect(metadata === null || metadata instanceof Object).toBe(true);
        done();
      });
    });

    it('should check if blob exists in DHT', done => {
      const contentId = 'content_123';
      const blobHash = 'test_hash_123';

      service.blobExists(contentId, blobHash).subscribe(exists => {
        expect(typeof exists).toBe('boolean');
        done();
      });
    });

    it('should retrieve blobs for multiple content nodes', done => {
      const contentIds = ['content_1', 'content_2', 'content_3'];

      service.getBlobsForMultipleContent(contentIds).subscribe(blobMap => {
        expect(blobMap instanceof Map).toBe(true);
        expect(blobMap.size).toBeLessThanOrEqual(contentIds.length);
        done();
      });
    });

    it('should transform BlobMetadataOutput to ContentBlob', () => {
      const metadata: BlobMetadataOutput = {
        hash: 'test_hash',
        sizeBytes: 1024,
        mimeType: 'video/mp4',
        fallbackUrls: ['https://example.com/blob.mp4'],
        bitrateMbps: 5,
        durationSeconds: 300,
        codec: 'h264',
        createdAt: '2024-01-01T00:00:00Z',
        verifiedAt: '2024-01-02T00:00:00Z',
      };

      const result = service['transformBlobMetadata'](metadata);

      expect(result.hash).toBe('test_hash');
      expect(result.sizeBytes).toBe(1024);
      expect(result.mimeType).toBe('video/mp4');
      expect(result.bitrateMbps).toBe(5);
      expect(result.codec).toBe('h264');
    });

    it('should preserve all optional fields in transformation', () => {
      const metadata: BlobMetadataOutput = {
        hash: 'hash',
        sizeBytes: 100,
        mimeType: 'audio/mpeg',
        fallbackUrls: [],
        bitrateMbps: 320,
        durationSeconds: 180,
        codec: 'aac',
        createdAt: '2024-01-01T00:00:00Z',
        verifiedAt: '2024-01-02T00:00:00Z',
      };

      const result = service['transformBlobMetadata'](metadata);

      expect(result.durationSeconds).toBe(180);
      expect(result.createdAt).toBe('2024-01-01T00:00:00Z');
      expect(result.verifiedAt).toBe('2024-01-02T00:00:00Z');
    });
  });

  // =========================================================================
  // Cached Blob Download
  // =========================================================================

  describe('Cached Blob Download', () => {
    it('should report 100% progress when cached', done => {
      const contentBlob = createMockContentBlob();
      const progressUpdates: BlobDownloadProgress[] = [];

      const progressCallback = (progress: BlobDownloadProgress) => {
        progressUpdates.push(progress);
      };

      // Pre-cache the blob
      const testBlob = new Blob(['cached data']);
      service['blobCache'].set(contentBlob.hash, testBlob);

      service.downloadBlob(contentBlob, progressCallback).subscribe(result => {
        expect(result.wasCached).toBe(true);
        expect(progressUpdates.length).toBeGreaterThan(0);
        expect(progressUpdates[0].percentComplete).toBe(100);
        done();
      });
    });

    it('should return cached blob result with wasCached flag true', done => {
      const contentBlob = createMockContentBlob();
      const testBlob = new Blob(['cached']);
      service['blobCache'].set(contentBlob.hash, testBlob);

      service.downloadBlob(contentBlob).subscribe(result => {
        expect(result.wasCached).toBe(true);
        expect(result.blob).toBe(testBlob);
        expect(result.totalDurationMs).toBe(0);
        expect(result.fetch.successUrl).toBe('(cached)');
        done();
      });
    });

    it('downloadBlob should return BlobDownloadResult with all required properties', done => {
      const contentBlob = createMockContentBlob();
      const testBlob = new Blob(['cached']);
      service['blobCache'].set(contentBlob.hash, testBlob);

      service.downloadBlob(contentBlob).subscribe(result => {
        expect(result.blob).toBeDefined();
        expect(result.metadata).toBeDefined();
        expect(result.verification).toBeDefined();
        expect(result.fetch).toBeDefined();
        expect(typeof result.totalDurationMs).toBe('number');
        expect(typeof result.wasCached).toBe('boolean');
        done();
      });
    });
  });

  // =========================================================================
  // Progress Tracking - Complex async flows
  // =========================================================================

  describe('Progress Tracking', () => {
    it('should accept optional progress callback parameter', () => {
      const contentBlob = createMockContentBlob();
      service['blobCache'].set(contentBlob.hash, new Blob(['test']));

      // Should accept undefined callback
      expect(() => {
        service.downloadBlob(contentBlob, undefined);
      }).not.toThrow();

      // Should accept function callback
      expect(() => {
        service.downloadBlob(contentBlob, () => {});
      }).not.toThrow();
    });

    // TODO: Add async flow tests
    // - Mock fallbackService.fetchWithFallback properly
    // - Mock verificationService.verifyBlob properly
    // - Test progress callback invocation during fetch phase
    // - Test progress callback invocation during verification phase
    // - Test timing/duration calculation
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

    it('getUrlHealth should return array of UrlHealth objects', () => {
      const contentBlob = createMockContentBlob();
      spyOn(fallbackService, 'getUrlsHealth').and.returnValue([]);

      const result = service.getUrlHealth(contentBlob);
      expect(Array.isArray(result)).toBe(true);
    });

    it('isAccessible should use getUrlHealth internally', () => {
      const contentBlob = createMockContentBlob();
      spyOn(service, 'getUrlHealth').and.returnValue([]);

      service.isAccessible(contentBlob);
      expect(service.getUrlHealth).toHaveBeenCalledWith(contentBlob);
    });
  });

  // =========================================================================
  // Multiple Blob Download
  // =========================================================================

  describe('Multiple Blob Download', () => {
    it('should download multiple blobs in parallel', done => {
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

    it('downloadBlobs should accept array of ContentBlobs', () => {
      const blobs = [createMockContentBlob(), createMockContentBlob()];

      // Pre-cache to avoid async issues
      blobs.forEach((blob, i) => {
        service['blobCache'].set(blob.hash, new Blob([`data${i}`]));
      });

      const result = service.downloadBlobs(blobs);
      expect(typeof result.subscribe).toBe('function');
    });

    it('downloadBlobs should accept empty array', () => {
      const result = service.downloadBlobs([]);
      expect(typeof result.subscribe).toBe('function');
    });

    // TODO: Add async flow tests
    // - Test parallel execution timing
    // - Test error handling for individual blob failures
    // - Test progress callback aggregation across multiple downloads
  });

  // =========================================================================
  // Blob Download Integration - Complex async flows
  // =========================================================================

  describe('Blob Download Integration', () => {
    it('should not perform actual downloads in tests', () => {
      // This is more of a documentation test
      // In real scenarios, HTTP calls are intercepted by HttpTestingController
      expect(service).toBeTruthy();
    });

    // TODO: Add comprehensive mocks
    // - Mock BlobFallbackService.fetchWithFallback with proper Observable
    // - Mock BlobVerificationService.verifyBlob with proper Observable
    // - Test full download flow with real RxJS operators
    // - Test error scenarios (verification failure, fetch timeout)
  });

  // =========================================================================
  // Cache Lock & Concurrency
  // =========================================================================

  describe('Cache Lock & Concurrency', () => {
    it('should maintain cache lock promise chain', async () => {
      const hash1 = 'hash1';
      const hash2 = 'hash2';
      const blob1 = new Blob(['data1']);
      const blob2 = new Blob(['data2']);

      const lock1Promise = service['cacheBlob'](hash1, blob1, blob1.size);
      const lock2Promise = service['cacheBlob'](hash2, blob2, blob2.size);

      await Promise.all([lock1Promise, lock2Promise]);

      expect(service.isCached(hash1)).toBe(true);
      expect(service.isCached(hash2)).toBe(true);
    });

    // TODO: Add storage coordination tests
    // - Test race condition prevention with concurrent cache operations
    // - Test lock serialization during eviction
    // - Test cache stats accuracy under concurrent access
  });

  // =========================================================================
  // Connection Mode & Environment Detection
  // =========================================================================

  describe('Connection Mode & Environment Detection', () => {
    it('connectionMode getter should return valid mode', () => {
      const mode = service.connectionMode;
      expect(['doorway', 'direct']).toContain(mode);
    });

    it('getPriorityUrls should include strategy URL when not in fallbacks', () => {
      const contentBlob = createMockContentBlob();
      contentBlob.fallbackUrls = ['https://fallback1.com/blob'];

      spyOn(service as any, 'getStorageClient').and.returnValue({
        getBlobUrl: jasmine.createSpy('getBlobUrl').and.returnValue('https://strategy.com/blob'),
      });

      const urls = service.getPriorityUrls(contentBlob);
      expect(urls.includes('https://strategy.com/blob')).toBe(true);
    });
  });

  // =========================================================================
  // Metadata Retrieval from Holochain - Simple tests
  // =========================================================================

  describe('Metadata Retrieval from Holochain - Integration', () => {
    it('getBlobsForContent should return Observable of ContentBlob array', done => {
      const contentId = 'content_123';

      service.getBlobsForContent(contentId).subscribe(blobs => {
        expect(Array.isArray(blobs)).toBe(true);
        done();
      });
    });

    it('getBlobMetadata should return Observable of ContentBlob or null', done => {
      const contentId = 'content_123';
      const blobHash = 'test_hash_123';

      service.getBlobMetadata(contentId, blobHash).subscribe(metadata => {
        expect(metadata === null || metadata instanceof Object).toBe(true);
        done();
      });
    });

    it('blobExists should return Observable of boolean', done => {
      const contentId = 'content_123';
      const blobHash = 'test_hash_123';

      service.blobExists(contentId, blobHash).subscribe(exists => {
        expect(typeof exists).toBe('boolean');
        done();
      });
    });

    it('getBlobsForMultipleContent should return Observable of Map', done => {
      const contentIds = ['content_1', 'content_2', 'content_3'];

      service.getBlobsForMultipleContent(contentIds).subscribe(blobMap => {
        expect(blobMap instanceof Map).toBe(true);
        done();
      });
    });

    it('should return empty array on metadata retrieval error', done => {
      const contentId = 'content_nonexistent';

      service.getBlobsForContent(contentId).subscribe(blobs => {
        expect(blobs).toEqual([]);
        done();
      });
    });

    // TODO: Add async flow tests
    // - Mock HolochainClientService.callZome with proper success/failure paths
    // - Test Holochain DHT query error handling
    // - Test metadata transformation edge cases
    // - Test content ID parameter validation
  });
});
