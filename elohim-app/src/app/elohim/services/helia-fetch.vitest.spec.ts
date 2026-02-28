/**
 * Vitest spike — service test with vi.fn() replacing jasmine.createSpy.
 *
 * Proves: TestBed, vi.fn(), mockClear(), basic assertions.
 */
import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';

import { StorageClientService } from './storage-client.service';

/** Minimal interface matching HeliaFetchService's public API */
interface HeliaFetchApi {
  fetchVerified(cidStr: string, timeoutMs?: number): Promise<Uint8Array>;
}

describe('HeliaFetchService (vitest spike)', () => {
  let httpMock: HttpTestingController;

  const mockStorageClient = {
    getBlobUrl: vi.fn((cid: string) => `http://localhost:8888/store/${cid}`),
  };

  /**
   * Minimal service mirroring HeliaFetchService's HTTP fallback path.
   * Avoids importing the real service (which pulls in node:stream).
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
    const cid = 'sha256-test1234test1234test1234test1234test1234test1234test1234test1234';
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

  it('should return URL with /store/ prefix', () => {
    const hash = 'sha256-abc123def456abc123def456abc123def456abc123def456abc123def456abc1';
    const url = mockStorageClient.getBlobUrl(hash);
    expect(url).toBe(`http://localhost:8888/store/${hash}`);
  });
});
