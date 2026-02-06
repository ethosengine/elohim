import {
  HttpClientTestingModule,
  HttpTestingController,
} from '@angular/common/http/testing';
import { TestBed, fakeAsync, tick } from '@angular/core/testing';

import {
  DoorwayClientService,
  VerifyBlobResponse,
  CustodianInfo,
  BestCustodianResponse,
} from './doorway-client.service';

/**
 * Unit tests for DoorwayClientService
 *
 * Tests blob retrieval, streaming URLs, verification, custodian selection,
 * and health/status endpoints.
 *
 * Coverage:
 * - Service creation and initialization
 * - Method existence for all public methods
 * - Observable return types
 * - Simple input/output validation
 * - Property initialization
 *
 * Future enhancements:
 * - Add async flow tests (timeout/retry behavior)
 * - Add comprehensive mocks for complex scenarios
 * - Add HTTP integration tests for edge cases
 */
describe('DoorwayClientService', () => {
  let service: DoorwayClientService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [DoorwayClientService],
    });

    service = TestBed.inject(DoorwayClientService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  // ===========================================================================
  // Service Creation & Initialization Tests
  // ===========================================================================

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should be provided as singleton (providedIn: root)', () => {
    const service1 = TestBed.inject(DoorwayClientService);
    const service2 = TestBed.inject(DoorwayClientService);
    expect(service1).toBe(service2);
  });

  // ===========================================================================
  // Method Existence Tests - All Public Methods
  // ===========================================================================

  describe('method existence', () => {
    it('should have getBlob method', () => {
      expect(typeof service.getBlob).toBe('function');
    });

    it('should have getChunk method', () => {
      expect(typeof service.getChunk).toBe('function');
    });

    it('should have getHlsManifestUrl method', () => {
      expect(typeof service.getHlsManifestUrl).toBe('function');
    });

    it('should have getHlsVariantUrl method', () => {
      expect(typeof service.getHlsVariantUrl).toBe('function');
    });

    it('should have getDashManifestUrl method', () => {
      expect(typeof service.getDashManifestUrl).toBe('function');
    });

    it('should have getChunkUrl method', () => {
      expect(typeof service.getChunkUrl).toBe('function');
    });

    it('should have verifyBlob method', () => {
      expect(typeof service.verifyBlob).toBe('function');
    });

    it('should have verifyBlobData method', () => {
      expect(typeof service.verifyBlobData).toBe('function');
    });

    it('should have getCustodiansForBlob method', () => {
      expect(typeof service.getCustodiansForBlob).toBe('function');
    });

    it('should have getBestCustodianUrl method', () => {
      expect(typeof service.getBestCustodianUrl).toBe('function');
    });

    it('should have fetchHlsManifest method', () => {
      expect(typeof service.fetchHlsManifest).toBe('function');
    });

    it('should have fetchDashManifest method', () => {
      expect(typeof service.fetchDashManifest).toBe('function');
    });

    it('should have checkHealth method', () => {
      expect(typeof service.checkHealth).toBe('function');
    });

    it('should have getStatus method', () => {
      expect(typeof service.getStatus).toBe('function');
    });

    it('should have setBaseUrl method', () => {
      expect(typeof service.setBaseUrl).toBe('function');
    });

    it('should have getBaseUrl method', () => {
      expect(typeof service.getBaseUrl).toBe('function');
    });

    it('should have setDefaultTimeout method', () => {
      expect(typeof service.setDefaultTimeout).toBe('function');
    });

    it('should have setMaxRetries method', () => {
      expect(typeof service.setMaxRetries).toBe('function');
    });
  });

  // ===========================================================================
  // Observable Return Type Tests
  // ===========================================================================

  describe('observable return types', () => {
    it('getBlob should return Observable', () => {
      const result = service.getBlob('test-hash');
      expect(result.subscribe).toBeDefined();
    });

    it('getChunk should return Observable', () => {
      const result = service.getChunk('test-hash', 0);
      expect(result.subscribe).toBeDefined();
    });

    it('verifyBlob should return Observable', () => {
      const result = service.verifyBlob({ expectedHash: 'test' });
      expect(result.subscribe).toBeDefined();
    });

    it('verifyBlobData should return Observable', () => {
      const data = new Uint8Array([1, 2, 3]);
      const result = service.verifyBlobData(data, 'test');
      expect(result.subscribe).toBeDefined();
    });

    it('getCustodiansForBlob should return Observable', () => {
      const result = service.getCustodiansForBlob('test-hash');
      expect(result.subscribe).toBeDefined();
    });

    it('getBestCustodianUrl should return Observable', () => {
      const result = service.getBestCustodianUrl('test-hash');
      expect(result.subscribe).toBeDefined();
    });

    it('fetchHlsManifest should return Observable', () => {
      const result = service.fetchHlsManifest('test-content');
      expect(result.subscribe).toBeDefined();
    });

    it('fetchDashManifest should return Observable', () => {
      const result = service.fetchDashManifest('test-content');
      expect(result.subscribe).toBeDefined();
    });

    it('checkHealth should return Observable', () => {
      const result = service.checkHealth();
      expect(result.subscribe).toBeDefined();
    });

    it('getStatus should return Observable', () => {
      const result = service.getStatus();
      expect(result.subscribe).toBeDefined();
    });
  });

  // ===========================================================================
  // Property Initialization Tests
  // ===========================================================================

  describe('property initialization', () => {
    it('should initialize with default base URL from environment', () => {
      const baseUrl = service.getBaseUrl();
      expect(typeof baseUrl).toBe('string');
    });

    it('should allow setting custom base URL', () => {
      service.setBaseUrl('https://test.example.com');
      expect(service.getBaseUrl()).toBe('https://test.example.com');
    });

    it('should allow setting timeout to positive number', () => {
      service.setDefaultTimeout(60000);
      // No direct getter, but method should execute without error
      expect(() => service.setDefaultTimeout(60000)).not.toThrow();
    });

    it('should allow setting retries to positive number', () => {
      service.setMaxRetries(5);
      // No direct getter, but method should execute without error
      expect(() => service.setMaxRetries(5)).not.toThrow();
    });

    it('should accept zero for retries', () => {
      expect(() => service.setMaxRetries(0)).not.toThrow();
    });

    it('should accept zero timeout', () => {
      expect(() => service.setDefaultTimeout(0)).not.toThrow();
    });
  });

  // ===========================================================================
  // Configuration Tests
  // ===========================================================================

  describe('configuration', () => {
    it('should have default base URL', () => {
      // Default is empty string or from environment
      const baseUrl = service.getBaseUrl();
      expect(typeof baseUrl).toBe('string');
    });

    it('should set and get base URL', () => {
      service.setBaseUrl('https://doorway.example.com');
      expect(service.getBaseUrl()).toBe('https://doorway.example.com');
    });

    it('should set default timeout', () => {
      // No direct way to verify, but should not throw
      expect(() => service.setDefaultTimeout(60000)).not.toThrow();
    });

    it('should set max retries', () => {
      expect(() => service.setMaxRetries(5)).not.toThrow();
    });
  });

  // ===========================================================================
  // Blob Retrieval Tests
  // ===========================================================================

  describe('getBlob', () => {
    it('should fetch blob by hash', fakeAsync(() => {
      const mockBlob = new ArrayBuffer(1024);

      let result: ArrayBuffer | null = null;
      service.getBlob('sha256-abc123').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne('/api/blob/sha256-abc123');
      expect(req.request.method).toBe('GET');
      expect(req.request.responseType).toBe('arraybuffer');
      req.flush(mockBlob);
      tick();

      expect(result).toBeTruthy();
      expect(result!.byteLength).toBe(1024);
    }));

    it('should support byte range requests', fakeAsync(() => {
      const mockBlob = new ArrayBuffer(512);

      service.getBlob('sha256-abc123', { start: 0, end: 511 }).subscribe();

      const req = httpMock.expectOne('/api/blob/sha256-abc123');
      expect(req.request.headers.get('Range')).toBe('bytes=0-511');
      req.flush(mockBlob);
      tick();
    }));

    it('should handle blob fetch errors', fakeAsync(() => {
      // Set retries to 0 for this test to avoid multiple requests
      service.setMaxRetries(0);

      let error: Error | null = null;
      service.getBlob('sha256-notfound').subscribe({
        error: err => {
          error = err;
        },
      });

      const req = httpMock.expectOne('/api/blob/sha256-notfound');
      req.error(new ProgressEvent('error'), { status: 404, statusText: 'Not Found' });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('getBlob failed');
    }));
  });

  describe('getChunk', () => {
    it('should fetch chunk by hash and index', fakeAsync(() => {
      const mockChunk = new ArrayBuffer(256);

      let result: ArrayBuffer | null = null;
      service.getChunk('sha256-abc123', 5).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne('/api/stream/chunk/sha256-abc123/5');
      expect(req.request.method).toBe('GET');
      expect(req.request.responseType).toBe('arraybuffer');
      req.flush(mockChunk);
      tick();

      expect(result).toBeTruthy();
    }));

    it('should handle chunk fetch errors', fakeAsync(() => {
      // Set retries to 0 for this test to avoid multiple requests
      service.setMaxRetries(0);

      let error: Error | null = null;
      service.getChunk('sha256-abc123', 999).subscribe({
        error: err => {
          error = err;
        },
      });

      const req = httpMock.expectOne('/api/stream/chunk/sha256-abc123/999');
      req.error(new ProgressEvent('error'), { status: 416, statusText: 'Range Not Satisfiable' });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('getChunk failed');
    }));
  });

  // ===========================================================================
  // Streaming URL Tests
  // ===========================================================================

  describe('streaming URLs', () => {
    beforeEach(() => {
      service.setBaseUrl('https://doorway.example.com');
    });

    it('should generate HLS manifest URL', () => {
      const url = service.getHlsManifestUrl('content-123');
      expect(url).toBe('https://doorway.example.com/api/stream/hls/content-123');
    });

    it('should encode special characters in HLS manifest URL', () => {
      const url = service.getHlsManifestUrl('content/with spaces');
      expect(url).toBe('https://doorway.example.com/api/stream/hls/content%2Fwith%20spaces');
    });

    it('should generate HLS variant URL', () => {
      const url = service.getHlsVariantUrl('content-123', '720p');
      expect(url).toBe('https://doorway.example.com/api/stream/hls/content-123/720p.m3u8');
    });

    it('should generate DASH manifest URL', () => {
      const url = service.getDashManifestUrl('content-123');
      expect(url).toBe('https://doorway.example.com/api/stream/dash/content-123');
    });

    it('should generate chunk URL', () => {
      const url = service.getChunkUrl('sha256-abc', 10);
      expect(url).toBe('https://doorway.example.com/api/stream/chunk/sha256-abc/10');
    });
  });

  // ===========================================================================
  // Manifest Fetching Tests
  // ===========================================================================

  describe('fetchHlsManifest', () => {
    it('should fetch HLS manifest content', fakeAsync(() => {
      const manifestContent = '#EXTM3U\n#EXT-X-VERSION:3\n...';

      let result = '';
      service.fetchHlsManifest('content-123').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne('/api/stream/hls/content-123');
      expect(req.request.method).toBe('GET');
      expect(req.request.responseType).toBe('text');
      req.flush(manifestContent);
      tick();

      expect(result).toBe(manifestContent);
    }));

    it('should handle manifest fetch errors', fakeAsync(() => {
      let error: Error | null = null;
      service.fetchHlsManifest('not-found').subscribe({
        error: err => {
          error = err;
        },
      });

      const req = httpMock.expectOne('/api/stream/hls/not-found');
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('fetchHlsManifest failed');
    }));
  });

  describe('fetchDashManifest', () => {
    it('should fetch DASH MPD manifest', fakeAsync(() => {
      const mpdContent = '<?xml version="1.0"?><MPD>...</MPD>';

      let result = '';
      service.fetchDashManifest('content-123').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne('/api/stream/dash/content-123');
      expect(req.request.method).toBe('GET');
      req.flush(mpdContent);
      tick();

      expect(result).toBe(mpdContent);
    }));
  });

  // ===========================================================================
  // Verification Tests
  // ===========================================================================

  describe('verifyBlob', () => {
    it('should verify blob with expected hash', fakeAsync(() => {
      const mockResponse: VerifyBlobResponse = {
        isValid: true,
        computedHash: 'abc123',
        expectedHash: 'abc123',
        sizeBytes: 1024,
        durationMs: 15,
      };

      let result: VerifyBlobResponse | null = null;
      service
        .verifyBlob({
          expectedHash: 'abc123',
          dataBase64: 'SGVsbG8gV29ybGQ=',
        })
        .subscribe(data => {
          result = data;
        });

      const req = httpMock.expectOne('/api/blob/verify');
      expect(req.request.method).toBe('POST');
      expect(req.request.body.expected_hash).toBe('abc123');
      expect(req.request.body.data_base64).toBe('SGVsbG8gV29ybGQ=');
      req.flush(mockResponse);
      tick();

      expect(result).toBeTruthy();
      expect(result!.isValid).toBeTrue();
      expect(result!.computedHash).toBe('abc123');
    }));

    it('should verify blob with fetch URL', fakeAsync(() => {
      const mockResponse: VerifyBlobResponse = {
        isValid: false,
        computedHash: 'xyz789',
        expectedHash: 'abc123',
        sizeBytes: 2048,
        durationMs: 50,
        error: 'Hash mismatch',
      };

      let result: VerifyBlobResponse | null = null;
      service
        .verifyBlob({
          expectedHash: 'abc123',
          fetchUrl: 'https://source.com/blob',
          contentId: 'content-1',
        })
        .subscribe(data => {
          result = data;
        });

      const req = httpMock.expectOne('/api/blob/verify');
      expect(req.request.body.fetch_url).toBe('https://source.com/blob');
      expect(req.request.body.content_id).toBe('content-1');
      req.flush(mockResponse);
      tick();

      expect(result!.isValid).toBeFalse();
      expect(result!.error).toBe('Hash mismatch');
    }));

    it('should handle verification errors', fakeAsync(() => {
      let error: Error | null = null;
      service
        .verifyBlob({
          expectedHash: 'abc123',
        })
        .subscribe({
          error: err => {
            error = err;
          },
        });

      const req = httpMock.expectOne('/api/blob/verify');
      req.error(new ProgressEvent('error'), { status: 500 });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('verifyBlob failed');
    }));
  });

  describe('verifyBlobData', () => {
    it('should verify blob data from Uint8Array', fakeAsync(() => {
      const mockResponse: VerifyBlobResponse = {
        isValid: true,
        computedHash: 'abc123',
        expectedHash: 'abc123',
        sizeBytes: 5,
        durationMs: 5,
      };

      const data = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"

      let result: VerifyBlobResponse | null = null;
      service.verifyBlobData(data, 'abc123').subscribe(resp => {
        result = resp;
      });

      const req = httpMock.expectOne('/api/blob/verify');
      expect(req.request.body.data_base64).toBe('SGVsbG8='); // base64 of "Hello"
      req.flush(mockResponse);
      tick();

      expect(result!.isValid).toBeTrue();
    }));

    it('should verify blob data from ArrayBuffer', fakeAsync(() => {
      const mockResponse: VerifyBlobResponse = {
        isValid: true,
        computedHash: 'def456',
        expectedHash: 'def456',
        sizeBytes: 3,
        durationMs: 3,
      };

      const buffer = new ArrayBuffer(3);
      const view = new Uint8Array(buffer);
      view[0] = 65;
      view[1] = 66;
      view[2] = 67; // "ABC"

      let result: VerifyBlobResponse | null = null;
      service.verifyBlobData(buffer, 'def456').subscribe(resp => {
        result = resp;
      });

      const req = httpMock.expectOne('/api/blob/verify');
      expect(req.request.body.data_base64).toBe('QUJD'); // base64 of "ABC"
      req.flush(mockResponse);
      tick();

      expect(result!.isValid).toBeTrue();
    }));
  });

  // ===========================================================================
  // Custodian Selection Tests
  // ===========================================================================

  describe('getCustodiansForBlob', () => {
    it('should fetch custodians for blob', fakeAsync(() => {
      const mockCustodians: CustodianInfo[] = [
        {
          agentId: 'agent-1',
          baseUrl: 'https://custodian1.example.com',
          bandwidthMbps: 100,
          latencyMs: 20,
          uptimeRatio: 0.99,
          region: 'us-east',
          score: 95,
        },
        {
          agentId: 'agent-2',
          baseUrl: 'https://custodian2.example.com',
          bandwidthMbps: 50,
          latencyMs: 50,
          uptimeRatio: 0.95,
          score: 70,
        },
      ];

      let result: CustodianInfo[] | null = null;
      service.getCustodiansForBlob('sha256-abc123').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne('/api/custodian/blob/sha256-abc123');
      expect(req.request.method).toBe('GET');
      req.flush({ custodians: mockCustodians });
      tick();

      expect(result).toBeTruthy();
      expect(result!.length).toBe(2);
      expect(result![0].agentId).toBe('agent-1');
      expect(result![0].score).toBe(95);
    }));

    it('should encode hash in URL', fakeAsync(() => {
      service.getCustodiansForBlob('sha256/special+hash').subscribe();

      const req = httpMock.expectOne('/api/custodian/blob/sha256%2Fspecial%2Bhash');
      req.flush({ custodians: [] });
      tick();
    }));

    it('should handle custodian fetch errors', fakeAsync(() => {
      let error: Error | null = null;
      service.getCustodiansForBlob('sha256-notfound').subscribe({
        error: err => {
          error = err;
        },
      });

      const req = httpMock.expectOne('/api/custodian/blob/sha256-notfound');
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('getCustodiansForBlob failed');
    }));
  });

  describe('getBestCustodianUrl', () => {
    it('should get best custodian URL', fakeAsync(() => {
      const mockResponse: BestCustodianResponse = {
        url: 'https://best.custodian.com/blob/sha256-abc',
        custodian: {
          agentId: 'agent-best',
          baseUrl: 'https://best.custodian.com',
          bandwidthMbps: 200,
          latencyMs: 10,
          uptimeRatio: 0.999,
          region: 'us-west',
          score: 99,
        },
        fallbackUrls: [
          'https://fallback1.com/blob/sha256-abc',
          'https://fallback2.com/blob/sha256-abc',
        ],
      };

      let result: BestCustodianResponse | null = null;
      service.getBestCustodianUrl('sha256-abc').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne('/api/custodian/blob/sha256-abc/best');
      expect(req.request.method).toBe('GET');
      req.flush(mockResponse);
      tick();

      expect(result).toBeTruthy();
      expect(result!.url).toContain('best.custodian.com');
      expect(result!.fallbackUrls.length).toBe(2);
    }));

    it('should include preferred region parameter', fakeAsync(() => {
      service.getBestCustodianUrl('sha256-abc', 'eu-west').subscribe();

      const req = httpMock.expectOne(
        request =>
          request.url.includes('/api/custodian/blob/sha256-abc/best') &&
          request.params.get('region') === 'eu-west'
      );
      req.flush({
        url: 'https://eu.custodian.com/blob/sha256-abc',
        custodian: {
          agentId: 'agent-eu',
          baseUrl: 'https://eu.custodian.com',
          bandwidthMbps: 100,
          latencyMs: 30,
          uptimeRatio: 0.98,
          score: 85,
        },
        fallbackUrls: [],
      });
      tick();
    }));

    it('should handle best custodian fetch errors', fakeAsync(() => {
      let error: Error | null = null;
      service.getBestCustodianUrl('sha256-notfound').subscribe({
        error: err => {
          error = err;
        },
      });

      const req = httpMock.expectOne('/api/custodian/blob/sha256-notfound/best');
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('getBestCustodianUrl failed');
    }));
  });

  // ===========================================================================
  // Health & Status Tests
  // ===========================================================================

  describe('checkHealth', () => {
    it('should check doorway health', fakeAsync(() => {
      let result: { status: string } | null = null;
      service.checkHealth().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne('/health');
      expect(req.request.method).toBe('GET');
      req.flush({ status: 'healthy' });
      tick();

      expect(result).toBeTruthy();
      expect(result!.status).toBe('healthy');
    }));

    it('should handle health check errors', fakeAsync(() => {
      let error: Error | null = null;
      service.checkHealth().subscribe({
        error: err => {
          error = err;
        },
      });

      const req = httpMock.expectOne('/health');
      req.error(new ProgressEvent('error'), { status: 503 });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('checkHealth failed');
    }));
  });

  describe('getStatus', () => {
    it('should get doorway status', fakeAsync(() => {
      const mockStatus = {
        version: '1.0.0',
        uptime: 3600,
        connections: 42,
        cacheHitRate: 0.85,
      };

      let result: Record<string, unknown> | null = null;
      service.getStatus().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne('/status');
      expect(req.request.method).toBe('GET');
      req.flush(mockStatus);
      tick();

      expect(result).toBeTruthy();
      expect(result!['version']).toBe('1.0.0');
      expect(result!['uptime']).toBe(3600);
    }));

    it('should handle status fetch errors', fakeAsync(() => {
      let error: Error | null = null;
      service.getStatus().subscribe({
        error: err => {
          error = err;
        },
      });

      const req = httpMock.expectOne('/status');
      req.error(new ProgressEvent('error'), { status: 500 });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('getStatus failed');
    }));
  });

  // ===========================================================================
  // Base URL Integration Tests
  // ===========================================================================

  describe('base URL integration', () => {
    it('should use configured base URL for all requests', fakeAsync(() => {
      service.setBaseUrl('https://custom-doorway.example.com');

      service.getBlob('sha256-test').subscribe();
      const req = httpMock.expectOne('https://custom-doorway.example.com/api/blob/sha256-test');
      req.flush(new ArrayBuffer(0));
      tick();
    }));

    it('should work with empty base URL (same-origin)', fakeAsync(() => {
      service.setBaseUrl('');

      service.checkHealth().subscribe();
      const req = httpMock.expectOne('/health');
      req.flush({ status: 'ok' });
      tick();
    }));
  });
});
