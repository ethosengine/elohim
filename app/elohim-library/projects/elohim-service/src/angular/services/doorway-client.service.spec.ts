// @vitest-environment jsdom
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { firstValueFrom } from 'rxjs';

import {
  DoorwayClientService,
  VerifyBlobResponse,
  CustodianInfo,
  BestCustodianResponse,
} from './doorway-client.service';
import { ELOHIM_ENV } from '../../env/elohim-env';

/**
 * Unit tests for DoorwayClientService (migrated to @elohim/service, Slice 2.1b).
 *
 * Tests blob retrieval, streaming URLs, verification, custodian selection,
 * and health/status endpoints.
 */
describe('DoorwayClientService', () => {
  let service: DoorwayClientService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        DoorwayClientService,
        { provide: ELOHIM_ENV, useValue: { production: false, doorwayUrl: '' } },
      ],
    });

    service = TestBed.inject(DoorwayClientService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should be provided as singleton (providedIn: root)', () => {
    const service1 = TestBed.inject(DoorwayClientService);
    const service2 = TestBed.inject(DoorwayClientService);
    expect(service1).toBe(service2);
  });

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

  describe('property initialization', () => {
    it('should initialize with base URL from ELOHIM_ENV', () => {
      const baseUrl = service.getBaseUrl();
      expect(typeof baseUrl).toBe('string');
    });

    it('should allow setting custom base URL', () => {
      service.setBaseUrl('https://test.example.com');
      expect(service.getBaseUrl()).toBe('https://test.example.com');
    });

    it('should allow setting timeout to positive number', () => {
      expect(() => service.setDefaultTimeout(60000)).not.toThrow();
    });

    it('should allow setting retries to positive number', () => {
      expect(() => service.setMaxRetries(5)).not.toThrow();
    });

    it('should accept zero for retries', () => {
      expect(() => service.setMaxRetries(0)).not.toThrow();
    });

    it('should accept zero timeout', () => {
      expect(() => service.setDefaultTimeout(0)).not.toThrow();
    });
  });

  describe('configuration', () => {
    it('should have default base URL', () => {
      const baseUrl = service.getBaseUrl();
      expect(typeof baseUrl).toBe('string');
    });

    it('should set and get base URL', () => {
      service.setBaseUrl('https://doorway.example.com');
      expect(service.getBaseUrl()).toBe('https://doorway.example.com');
    });

    it('should set default timeout', () => {
      expect(() => service.setDefaultTimeout(60000)).not.toThrow();
    });

    it('should set max retries', () => {
      expect(() => service.setMaxRetries(5)).not.toThrow();
    });
  });

  describe('getBlob', () => {
    it('should fetch blob by hash', async () => {
      const mockBlob = new ArrayBuffer(1024);

      const promise = firstValueFrom(service.getBlob('sha256-abc123'));

      const req = httpMock.expectOne('/blob/sha256-abc123');
      expect(req.request.method).toBe('GET');
      expect(req.request.responseType).toBe('arraybuffer');
      req.flush(mockBlob);

      const result = await promise;
      expect(result).toBeTruthy();
      expect(result.byteLength).toBe(1024);
    });

    it('should support byte range requests', async () => {
      const mockBlob = new ArrayBuffer(512);

      const promise = firstValueFrom(service.getBlob('sha256-abc123', { start: 0, end: 511 }));

      const req = httpMock.expectOne('/blob/sha256-abc123');
      expect(req.request.headers.get('Range')).toBe('bytes=0-511');
      req.flush(mockBlob);

      await promise;
    });

    it('should handle blob fetch errors', async () => {
      service.setMaxRetries(0);

      const promise = firstValueFrom(service.getBlob('sha256-notfound'));

      const req = httpMock.expectOne('/blob/sha256-notfound');
      req.error(new ProgressEvent('error'), { status: 404, statusText: 'Not Found' });

      await expect(promise).rejects.toThrow('getBlob failed');
    });
  });

  describe('streaming URLs', () => {
    beforeEach(() => {
      service.setBaseUrl('https://doorway.example.com');
    });

    it('should generate HLS manifest URL', () => {
      const url = service.getHlsManifestUrl('content-123');
      expect(url).toBe('https://doorway.example.com/api/stream/hls/content-123');
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

  describe('verifyBlob', () => {
    it('should verify blob with expected hash', async () => {
      const mockResponse: VerifyBlobResponse = {
        isValid: true,
        computedHash: 'abc123',
        expectedHash: 'abc123',
        sizeBytes: 1024,
        durationMs: 15,
      };

      const promise = firstValueFrom(
        service.verifyBlob({
          expectedHash: 'abc123',
          dataBase64: 'SGVsbG8gV29ybGQ=',
        }),
      );

      const req = httpMock.expectOne('/api/blob/verify');
      expect(req.request.method).toBe('POST');
      expect(req.request.body.expected_hash).toBe('abc123');
      req.flush(mockResponse);

      const result = await promise;
      expect(result).toBeTruthy();
      expect(result.isValid).toBe(true);
    });
  });

  describe('getCustodiansForBlob', () => {
    it('should fetch custodians for blob', async () => {
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
      ];

      const promise = firstValueFrom(service.getCustodiansForBlob('sha256-abc123'));

      const req = httpMock.expectOne('/api/custodian/blob/sha256-abc123');
      expect(req.request.method).toBe('GET');
      req.flush({ custodians: mockCustodians });

      const result = await promise;
      expect(result).toBeTruthy();
      expect(result.length).toBe(1);
      expect(result[0].agentId).toBe('agent-1');
    });
  });

  describe('checkHealth', () => {
    it('should check doorway health', async () => {
      const promise = firstValueFrom(service.checkHealth());

      const req = httpMock.expectOne('/health');
      expect(req.request.method).toBe('GET');
      req.flush({ status: 'healthy' });

      const result = await promise;
      expect(result).toBeTruthy();
      expect((result as { status: string }).status).toBe('healthy');
    });
  });

  describe('base URL integration', () => {
    it('should use configured base URL for all requests', async () => {
      service.setBaseUrl('https://custom-doorway.example.com');

      const promise = firstValueFrom(service.getBlob('sha256-test'));
      const req = httpMock.expectOne('https://custom-doorway.example.com/blob/sha256-test');
      req.flush(new ArrayBuffer(0));
      await promise;
    });

    it('should work with empty base URL (same-origin)', async () => {
      service.setBaseUrl('');

      const promise = firstValueFrom(service.checkHealth());
      const req = httpMock.expectOne('/health');
      req.flush({ status: 'ok' });
      await promise;
    });
  });
});
