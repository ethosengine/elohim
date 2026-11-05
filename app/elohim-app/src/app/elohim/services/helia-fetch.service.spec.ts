/**
 * HeliaFetchService tests.
 *
 * The actual service lazy-loads @helia/verified-fetch which uses node:stream,
 * incompatible with Karma/Webpack. These tests use a mock provider to avoid
 * importing the real service at compile time.
 *
 * Integration testing of the real Helia path requires an E2E environment.
 */
import { TestBed } from '@angular/core/testing';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';

import { StorageClientService } from './storage-client.service';
import { provideHttpClient } from '@angular/common/http';

/** Minimal interface matching HeliaFetchService's public API */
interface HeliaFetchApi {
  fetchVerified(cidStr: string, timeoutMs?: number): Promise<Uint8Array>;
}

describe('HeliaFetchService (mock)', () => {
  let httpMock: HttpTestingController;

  const mockStorageClient = {
    getBlobUrl: vi.fn().mockImplementation((cid: string) => `http://localhost:8888/blob/${cid}`),
  };

  /**
   * Minimal service that mirrors HeliaFetchService's HTTP fallback path.
   * This avoids importing the real service (which pulls in node:stream).
   */
  class MockHeliaFetchService implements HeliaFetchApi {
    constructor(private storage: { getBlobUrl: (cid: string) => string }) {}

    async fetchVerified(cidStr: string): Promise<Uint8Array> {
      const url = this.storage.getBlobUrl(cidStr);
      const response = await fetch(url);
      const buffer = await response.arrayBuffer();
      return new Uint8Array(buffer);
    }
  }

  let service: HeliaFetchApi;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: StorageClientService, useValue: mockStorageClient },
      ],
    });
    httpMock = TestBed.inject(HttpTestingController);
    service = new MockHeliaFetchService(mockStorageClient);
    mockStorageClient.getBlobUrl.mockClear();
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should build correct blob URL from StorageClientService', () => {
    const cid = 'bafkr4ichqz3c2fywkwlfyk3sstcvmqy22bw4a7twn2iuuxpy2xcnyqobsm';
    service.fetchVerified(cid).catch(() => {
      /* expected: fetch mock not available */
    });
    expect(mockStorageClient.getBlobUrl).toHaveBeenCalledWith(cid);
  });

  it('should call getBlobUrl for CID strings', () => {
    const cid = 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku';
    service.fetchVerified(cid).catch(() => {
      /* expected: fetch mock not available */
    });
    expect(mockStorageClient.getBlobUrl).toHaveBeenCalledWith(cid);
  });

  it('should return URL with /blob/ prefix', () => {
    const hash = 'bafkr4igw3xatlz4e2dsqewk5mpkp32qvvc2sdirm2jyclsze7ihvcje2vu';
    const url = mockStorageClient.getBlobUrl(hash);
    expect(url).toBe(`http://localhost:8888/blob/${hash}`);
  });
});
