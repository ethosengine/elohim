/**
 * NetworkPostureService Tests
 *
 * Coverage focus:
 * - Service creation
 * - get() returns posture data on success
 * - get() returns null on HTTP error (graceful fallback)
 */

import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { firstValueFrom } from 'rxjs';

import { NetworkPostureService, type NetworkPostureView } from './network-posture.service';
import { StorageClientService } from './storage-client.service';

const MOCK_POSTURE: NetworkPostureView = {
  totalPeers: 42,
  activePeers: 38,
  stalePeers: 4,
  alwaysOnPeers: 12,
  householdsReciprocating: 9,
  computeAvailable: true,
  storagePressure: 0.35,
  computedAt: '2026-04-19T10:00:00.000Z',
};

describe('NetworkPostureService', () => {
  let service: NetworkPostureService;
  let httpMock: HttpTestingController;
  let storageClientMock: { getStorageBaseUrl: () => string };

  beforeEach(() => {
    storageClientMock = { getStorageBaseUrl: () => 'http://localhost:8888' };

    TestBed.configureTestingModule({
      providers: [
        NetworkPostureService,
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: StorageClientService, useValue: storageClientMock },
      ],
    });

    service = TestBed.inject(NetworkPostureService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  // =========================================================================
  // Creation
  // =========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should have a get() method', () => {
      expect(typeof service.get).toBe('function');
    });
  });

  // =========================================================================
  // get() — success path
  // =========================================================================

  describe('get() — success', () => {
    it('should return posture data when endpoint responds', async () => {
      const promise = firstValueFrom(service.get());
      httpMock.expectOne('http://localhost:8888/api/v1/network/posture').flush(MOCK_POSTURE);
      const posture = await promise;
      expect(posture).toEqual(MOCK_POSTURE);
    });

    it('should hit the correct URL using storage base URL', async () => {
      const promise = firstValueFrom(service.get());
      const req = httpMock.expectOne('http://localhost:8888/api/v1/network/posture');
      req.flush(MOCK_POSTURE);
      await promise;
      expect(req.request.url).toContain('/api/v1/network/posture');
    });

    it('returned posture should contain totalPeers', async () => {
      const promise = firstValueFrom(service.get());
      httpMock.expectOne('http://localhost:8888/api/v1/network/posture').flush(MOCK_POSTURE);
      const posture = await promise;
      expect(posture?.totalPeers).toBe(42);
    });

    it('returned posture should contain activePeers', async () => {
      const promise = firstValueFrom(service.get());
      httpMock.expectOne('http://localhost:8888/api/v1/network/posture').flush(MOCK_POSTURE);
      const posture = await promise;
      expect(posture?.activePeers).toBe(38);
    });

    it('returned posture should contain computeAvailable flag', async () => {
      const promise = firstValueFrom(service.get());
      httpMock.expectOne('http://localhost:8888/api/v1/network/posture').flush(MOCK_POSTURE);
      const posture = await promise;
      expect(posture?.computeAvailable).toBe(true);
    });
  });

  // =========================================================================
  // get() — error / graceful fallback
  // =========================================================================

  describe('get() — graceful fallback', () => {
    it('should return null on 404 (endpoint not yet deployed)', async () => {
      const promise = firstValueFrom(service.get());
      httpMock
        .expectOne('http://localhost:8888/api/v1/network/posture')
        .flush('Not Found', { status: 404, statusText: 'Not Found' });
      const posture = await promise;
      expect(posture).toBeNull();
    });

    it('should return null on 500 server error', async () => {
      const promise = firstValueFrom(service.get());
      httpMock
        .expectOne('http://localhost:8888/api/v1/network/posture')
        .flush('Server Error', { status: 500, statusText: 'Internal Server Error' });
      const posture = await promise;
      expect(posture).toBeNull();
    });

    it('should return null on network error', async () => {
      const promise = firstValueFrom(service.get());
      httpMock
        .expectOne('http://localhost:8888/api/v1/network/posture')
        .error(new ProgressEvent('network error'));
      const posture = await promise;
      expect(posture).toBeNull();
    });

    it('should not throw when endpoint is unavailable', async () => {
      await expect(async () => {
        const promise = firstValueFrom(service.get());
        httpMock
          .expectOne('http://localhost:8888/api/v1/network/posture')
          .flush('Gone', { status: 410, statusText: 'Gone' });
        await promise;
      }).not.toThrow();
    });
  });

  // =========================================================================
  // get() — uses storage base URL
  // =========================================================================

  describe('get() — base URL delegation', () => {
    it('should use the base URL from StorageClientService', async () => {
      storageClientMock.getStorageBaseUrl = () => 'https://doorway.elohim.host';

      const promise = firstValueFrom(service.get());
      const req = httpMock.expectOne('https://doorway.elohim.host/api/v1/network/posture');
      req.flush(MOCK_POSTURE);
      await promise;
      expect(req.request.url).toContain('doorway.elohim.host');
    });
  });
});
