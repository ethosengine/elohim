import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { BlobStreamingService, StreamingProgress, BandwidthProbeResult, QualityRecommendation } from './blob-streaming.service';
import { ContentBlob, ContentBlobVariant } from '../models/content-node.model';

describe('BlobStreamingService', () => {
  let service: BlobStreamingService;
  let httpMock: HttpTestingController;

  const createMockContentBlob = (withVariants = false): ContentBlob => ({
    hash: 'test_hash_1080p',
    sizeBytes: 10 * 1024 * 1024, // 10 MB
    mimeType: 'video/mp4',
    fallbackUrls: ['https://example.com/blob.mp4'],
    bitrateMbps: 5,
    durationSeconds: 600,
    codec: 'h264',
    ...(withVariants && {
      variants: [
        {
          hash: 'test_hash_480p',
          label: '480p',
          bitrateMbps: 1.5,
          height: 480,
          width: 854,
          sizeBytes: 50000000,
          fallbackUrls: ['http://cdn.example.com/480p.mp4'],
        },
        {
          hash: 'test_hash_720p',
          label: '720p',
          bitrateMbps: 3,
          height: 720,
          width: 1280,
          sizeBytes: 100000000,
          fallbackUrls: ['http://cdn.example.com/720p.mp4'],
        },
        {
          hash: 'test_hash_1080p',
          label: '1080p',
          bitrateMbps: 5,
          height: 1080,
          width: 1920,
          sizeBytes: 200000000,
          fallbackUrls: ['http://cdn.example.com/1080p.mp4'],
        },
      ],
    }),
  });

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [BlobStreamingService],
    });
    service = TestBed.inject(BlobStreamingService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
    service.clearBandwidthCache();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Chunked Download', () => {
    it('should check Range request support', (done) => {
      const url = 'https://example.com/blob.mp4';

      service['checkRangeSupport'](url).then((supported) => {
        expect(typeof supported).toBe('boolean');
        done();
      });

      const req = httpMock.expectOne(url);
      expect(req.request.headers.has('Range')).toBe(true);
      req.flush(null, { status: 206, statusText: 'Partial Content' });
    });

    it('should fallback to single request if Range not supported', (done) => {
      const url = 'https://example.com/blob.mp4';
      const blob = createMockContentBlob();
      const testData = new Uint8Array(1024);

      service.downloadInChunks(blob, url).subscribe((result) => {
        expect(result.size).toBeGreaterThan(0);
        done();
      });

      // Range check returns 200 (Range not supported)
      const rangeReq = httpMock.expectOne(url);
      rangeReq.flush(null, { status: 200, statusText: 'OK' });

      // Single request
      const downloadReq = httpMock.expectOne(url);
      downloadReq.flush(testData);
    });
  });

  describe('Bandwidth Probing', () => {
    it('should measure bandwidth', async () => {
      const url = 'https://example.com/probe.bin';
      const probeData = new Uint8Array(1024 * 1024); // 1 MB

      const probePromise = service.probeBandwidth(url, 1024 * 1024);

      const req = httpMock.expectOne(url);
      req.flush(probeData);

      const result = await probePromise;
      expect(result.averageSpeedMbps).toBeGreaterThan(0);
      expect(result.probeDataSize).toBe(1024 * 1024);
    });

    it('should cache bandwidth probe results', async () => {
      const url = 'https://example.com/probe.bin';
      const probeData = new Uint8Array(1024 * 1024);

      // First probe
      let probePromise = service.probeBandwidth(url);
      let req = httpMock.expectOne(url);
      req.flush(probeData);
      const result1 = await probePromise;

      // Second probe should be cached (no HTTP call)
      probePromise = service.probeBandwidth(url);
      const result2 = await probePromise;

      // Results should be identical (cached)
      expect(result1.averageSpeedMbps).toBe(result2.averageSpeedMbps);

      // No additional HTTP request
      httpMock.expectNone(url);
    });

    it('should return conservative/optimistic speed estimates', async () => {
      const url = 'https://example.com/probe.bin';
      const probeData = new Uint8Array(1024 * 1024);

      const probePromise = service.probeBandwidth(url);
      const req = httpMock.expectOne(url);
      req.flush(probeData);

      const result = await probePromise;
      expect(result.minSpeedMbps).toBeLessThan(result.averageSpeedMbps);
      expect(result.maxSpeedMbps).toBeGreaterThan(result.averageSpeedMbps);
    });
  });

  describe('Quality Recommendation', () => {
    it('should recommend quality based on bandwidth', () => {
      const blob = createMockContentBlob(true);

      // Slow connection: 2 Mbps -> should get 480p
      let recommendation = service.recommendQuality(blob, 2);
      expect(recommendation.variant).toBe('480p');

      // Fast connection: 10 Mbps -> should get 1080p
      recommendation = service.recommendQuality(blob, 10);
      expect(recommendation.variant).toBe('1080p');

      // Very fast connection: 50 Mbps -> should get highest available
      recommendation = service.recommendQuality(blob, 50);
      expect(['720p', '1080p']).toContain(recommendation.variant);
    });

    it('should not recommend quality higher than available bandwidth', () => {
      const blob = createMockContentBlob(true);

      // Very slow connection: 0.5 Mbps
      const recommendation = service.recommendQuality(blob, 0.5);

      // Should recommend lowest quality
      expect(recommendation.variant).toBe('480p');
      // Bitrate should be <= 0.4 Mbps (80% headroom)
      expect(recommendation.bitrateMbps!).toBeLessThanOrEqual(0.5 * 0.8);
    });

    it('should handle single-bitrate blobs', () => {
      const blob = createMockContentBlob(false); // No variants

      const recommendation = service.recommendQuality(blob, 5);

      expect(recommendation.variant).toBe('default');
      expect(recommendation.bitrateMbps).toBe(blob.bitrateMbps);
    });

    it('should provide reasoning score for quality choice', () => {
      const blob = createMockContentBlob(true);

      const recommendation = service.recommendQuality(blob, 10);

      expect(recommendation.reasoningScore).toBeGreaterThan(0);
      expect(recommendation.reasoningScore).toBeLessThanOrEqual(1);
    });
  });

  describe('HLS Playlist Generation', () => {
    it('should generate HLS playlist for multi-variant stream', () => {
      const blob = createMockContentBlob(true);
      const playlist = service.generateHlsPlaylist(blob, 'https://cdn.example.com/blob');

      expect(playlist).toContain('#EXTM3U');
      expect(playlist).toContain('#EXT-X-VERSION:3');
      expect(playlist).toContain('480p');
      expect(playlist).toContain('720p');
      expect(playlist).toContain('1080p');
      expect(playlist).toContain('#EXT-X-ENDLIST');
    });

    it('should generate HLS playlist for single-bitrate stream', () => {
      const blob = createMockContentBlob(false);
      const playlist = service.generateHlsPlaylist(blob, 'https://cdn.example.com/blob');

      expect(playlist).toContain('#EXTM3U');
      expect(playlist).toContain('#EXTINF');
      expect(playlist).toContain('segment-');
      expect(playlist).toContain('#EXT-X-ENDLIST');
    });

    it('should include correct target duration in playlist', () => {
      const blob = createMockContentBlob(true);
      blob.durationSeconds = 600; // 10 minutes

      const playlist = service.generateHlsPlaylist(blob, 'https://cdn.example.com/blob');

      // Target duration should be based on segment duration (10 seconds)
      expect(playlist).toContain('#EXT-X-TARGETDURATION:');
      expect(playlist).toContain('60'); // ceil(600/10)
    });

    it('should generate correct segment count', () => {
      const blob = createMockContentBlob(false);
      blob.durationSeconds = 100; // 100 seconds / 10 second segments = 10 segments

      const playlist = service.generateHlsPlaylist(blob, 'https://cdn.example.com/blob');

      // Count segment lines
      const segmentLines = playlist.split('\n').filter((line) => line.startsWith('segment-'));
      expect(segmentLines.length).toBe(10);
    });
  });

  describe('DASH MPD Generation', () => {
    it('should generate DASH MPD for multi-variant stream', () => {
      const blob = createMockContentBlob(true);
      const mpd = service.generateDashMpd(blob, 'https://cdn.example.com/blob');

      expect(mpd).toContain('<?xml');
      expect(mpd).toContain('<MPD');
      expect(mpd).toContain('</MPD>');
      expect(mpd).toContain('<AdaptationSet');
      expect(mpd).toContain('<Representation');
      expect(mpd).toContain('480p');
      expect(mpd).toContain('720p');
      expect(mpd).toContain('1080p');
    });

    it('should include correct bandwidth in representations', () => {
      const blob = createMockContentBlob(true);
      const mpd = service.generateDashMpd(blob, 'https://cdn.example.com/blob');

      // Check for bandwidth values (in bits per second)
      expect(mpd).toContain('bandwidth="1500000"'); // 1.5 Mbps * 1,000,000
      expect(mpd).toContain('bandwidth="3000000"'); // 3 Mbps * 1,000,000
      expect(mpd).toContain('bandwidth="5000000"'); // 5 Mbps * 1,000,000
    });

    it('should include correct resolution attributes', () => {
      const blob = createMockContentBlob(true);
      const mpd = service.generateDashMpd(blob, 'https://cdn.example.com/blob');

      // 720p = 1280x720
      expect(mpd).toContain('width="1280"');
      expect(mpd).toContain('height="720"');

      // 1080p = 1920x1080
      expect(mpd).toContain('width="1920"');
      expect(mpd).toContain('height="1080"');
    });

    it('should format duration as ISO 8601', () => {
      const blob = createMockContentBlob(false);
      blob.durationSeconds = 3725; // 1 hour, 2 minutes, 5 seconds

      const mpd = service.generateDashMpd(blob, 'https://cdn.example.com/blob');

      expect(mpd).toContain('PT1H2M5S');
    });
  });

  describe('Chunk Size and Parallelization', () => {
    it('should use configurable chunk size', () => {
      service.chunkSizeBytes = 10 * 1024 * 1024; // 10 MB
      expect(service.chunkSizeBytes).toBe(10 * 1024 * 1024);
    });

    it('should limit parallel chunks', () => {
      service.maxParallelChunks = 8;
      expect(service.maxParallelChunks).toBe(8);
    });

    it('should use configurable chunk timeout', () => {
      service.chunkTimeoutMs = 60000; // 60 seconds
      expect(service.chunkTimeoutMs).toBe(60000);
    });
  });

  describe('Error Handling', () => {
    it('should handle bandwidth probe failure', async () => {
      const url = 'https://example.com/probe.bin';

      const probePromise = service.probeBandwidth(url);

      const req = httpMock.expectOne(url);
      req.error(new ErrorEvent('Network error'));

      try {
        await probePromise;
        fail('should have thrown');
      } catch (error: any) {
        expect(error.message).toContain('Bandwidth probe failed');
      }
    });
  });

  describe('Cache Management', () => {
    it('should clear bandwidth cache', async () => {
      const url = 'https://example.com/probe.bin';
      const probeData = new Uint8Array(1024 * 1024);

      // Populate cache
      let probePromise = service.probeBandwidth(url);
      let req = httpMock.expectOne(url);
      req.flush(probeData);
      await probePromise;

      // Clear cache
      service.clearBandwidthCache();

      // Next probe should hit network again
      probePromise = service.probeBandwidth(url);
      req = httpMock.expectOne(url);
      req.flush(probeData);
      await probePromise;
    });

    it('should expire cached bandwidth after 10 minutes', async () => {
      const url = 'https://example.com/probe.bin';
      const probeData = new Uint8Array(1024 * 1024);

      // Populate cache
      let probePromise = service.probeBandwidth(url);
      let req = httpMock.expectOne(url);
      req.flush(probeData);
      const result1 = await probePromise;

      // Manually expire cache by manipulating timestamp
      service['bandwidthCache'].forEach((value) => {
        value.timestamp = Date.now() - 11 * 60 * 1000; // 11 minutes ago
      });

      // Next probe should hit network again (cache expired)
      probePromise = service.probeBandwidth(url);
      req = httpMock.expectOne(url);
      req.flush(probeData);
      const result2 = await probePromise;

      // Results should be similar but from different probes
      expect(result1.averageSpeedMbps).toBeCloseTo(result2.averageSpeedMbps, 0);
    });
  });

  describe('Chunk Reassembly Validation', () => {
    it('should detect missing chunks', () => {
      const chunks = new Map<number, Uint8Array>();
      chunks.set(0, new Uint8Array(1024));
      chunks.set(2, new Uint8Array(1024));
      // Chunk 1 is missing

      const chunkErrors = new Map<number, string>();
      chunkErrors.set(1, 'Network timeout');

      const validation = service['validateChunks'](chunks, 3, 3072, chunkErrors);

      expect(validation.isValid).toBe(false);
      expect(validation.missingChunkIndices).toContain(1);
      expect(validation.totalChunks).toBe(3);
      expect(validation.successfulChunks).toBe(2);
    });

    it('should detect size mismatch', () => {
      const chunks = new Map<number, Uint8Array>();
      chunks.set(0, new Uint8Array(1024));
      chunks.set(1, new Uint8Array(512)); // Only 512 bytes instead of 1024

      const chunkErrors = new Map<number, string>();

      const validation = service['validateChunks'](chunks, 2, 2048, chunkErrors);

      expect(validation.isValid).toBe(false);
      expect(validation.expectedSizeBytes).toBe(2048);
      expect(validation.actualSizeBytes).toBe(1536);
    });

    it('should validate complete and correct chunks', () => {
      const chunks = new Map<number, Uint8Array>();
      chunks.set(0, new Uint8Array(1024));
      chunks.set(1, new Uint8Array(1024));
      chunks.set(2, new Uint8Array(1024));

      const chunkErrors = new Map<number, string>();

      const validation = service['validateChunks'](chunks, 3, 3072, chunkErrors);

      expect(validation.isValid).toBe(true);
      expect(validation.successfulChunks).toBe(3);
      expect(validation.missingChunkIndices.length).toBe(0);
      expect(validation.failedChunkIndices.length).toBe(0);
    });

    it('should format validation error message for missing chunks', () => {
      const validation: any = {
        isValid: false,
        totalChunks: 5,
        successfulChunks: 3,
        missingChunkIndices: [1, 3],
        failedChunkIndices: [1],
        chunkErrors: new Map([[1, 'Connection reset']]),
        expectedSizeBytes: 5120,
        actualSizeBytes: 3072,
      };

      const message = service.formatValidationError(validation);

      expect(message).toContain('Missing chunks: [1, 3]');
      expect(message).toContain('Failed chunks: 1');
      expect(message).toContain('Size mismatch');
    });

    it('should format validation success message', () => {
      const validation: any = {
        isValid: true,
        totalChunks: 5,
        successfulChunks: 5,
        missingChunkIndices: [],
        failedChunkIndices: [],
        chunkErrors: new Map(),
        expectedSizeBytes: 5120,
        actualSizeBytes: 5120,
      };

      const message = service.formatValidationError(validation);

      expect(message).toBe('All chunks downloaded successfully');
    });

    it('should track chunk download errors', () => {
      const chunks = new Map<number, Uint8Array>();
      chunks.set(0, new Uint8Array(1024));
      chunks.set(2, new Uint8Array(1024));

      const chunkErrors = new Map<number, string>();
      chunkErrors.set(1, 'Timeout after 30s');
      chunkErrors.set(3, 'HTTP 404: Not Found');

      const validation = service['validateChunks'](chunks, 4, 4096, chunkErrors);

      expect(validation.failedChunkIndices).toEqual([1, 3]);
      expect(validation.chunkErrors.get(1)).toBe('Timeout after 30s');
      expect(validation.chunkErrors.get(3)).toBe('HTTP 404: Not Found');
    });
  });
});
