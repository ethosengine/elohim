import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { vi } from 'vitest';

import { AcquisitionService } from './acquisition.service';
import { StorageClientService } from './storage-client.service';

describe('AcquisitionService', () => {
  let service: AcquisitionService;
  let httpMock: HttpTestingController;
  let storageMock: { connectionMode: string; getStorageBaseUrl: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    storageMock = {
      connectionMode: 'doorway',
      getStorageBaseUrl: vi.fn().mockReturnValue('http://localhost:8888'),
    };

    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        AcquisitionService,
        { provide: StorageClientService, useValue: storageMock },
      ],
    });

    service = TestBed.inject(AcquisitionService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  describe('capability()', () => {
    it('returns "peer" when storage connectionMode is "direct"', () => {
      storageMock.connectionMode = 'direct';
      expect(service.capability()).toBe('peer');
    });

    it('returns "browser" when storage connectionMode is "doorway"', () => {
      storageMock.connectionMode = 'doorway';
      expect(service.capability()).toBe('browser');
    });
  });

  describe('download() — peer path', () => {
    it('POSTs to /api/v1/pins with {headRef, kind:"item"} and returns "peer"', async () => {
      storageMock.connectionMode = 'direct';

      const resultPromise = service.download('epr:test-content-1');

      const req = httpMock.expectOne('http://localhost:8888/api/v1/pins');
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual({ headRef: 'epr:test-content-1', kind: 'item' });
      req.flush({});

      const result = await resultPromise;
      expect(result).toBe('peer');
    });
  });

  describe('download() — browser path', () => {
    it('fetches the content URL and returns "browser"', async () => {
      storageMock.connectionMode = 'doorway';

      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{}'));

      const result = await service.download('epr:test-content-2');

      expect(fetchSpy).toHaveBeenCalledWith(
        'http://localhost:8888/db/content/test-content-2',
      );
      expect(result).toBe('browser');

      fetchSpy.mockRestore();
    });

    it('strips the "epr:" prefix when building the content URL', async () => {
      storageMock.connectionMode = 'doorway';

      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{}'));

      await service.download('epr:some-doc');
      expect(fetchSpy).toHaveBeenCalledWith(
        'http://localhost:8888/db/content/some-doc',
      );

      fetchSpy.mockRestore();
    });
  });
});
