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
    it('POSTs to /api/v1/pins with bare headRef (epr: prefix stripped) and returns "peer"', async () => {
      storageMock.connectionMode = 'direct';

      const resultPromise = service.download('epr:test-content-1');

      const req = httpMock.expectOne('http://localhost:8888/api/v1/pins');
      expect(req.request.method).toBe('POST');
      // The epr: prefix must be stripped so headRef matches the bare content.id
      // used by the reconcile→dispatch→completion loop.
      expect(req.request.body).toEqual({ headRef: 'test-content-1', kind: 'item' });
      req.flush({});

      const result = await resultPromise;
      expect(result).toBe('peer');
    });

    it('strips the "epr:" prefix when building the peer pin POST body', async () => {
      storageMock.connectionMode = 'direct';

      const resultPromise = service.download('epr:some-doc');

      const req = httpMock.expectOne('http://localhost:8888/api/v1/pins');
      expect(req.request.body).toEqual({ headRef: 'some-doc', kind: 'item' });
      req.flush({});

      await resultPromise;
    });
  });

  describe('download() — browser path', () => {
    it('fetches the content URL and returns "browser"', async () => {
      storageMock.connectionMode = 'doorway';

      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{}'));

      const result = await service.download('epr:test-content-2');

      expect(fetchSpy).toHaveBeenCalledWith('http://localhost:8888/db/content/test-content-2');
      expect(result).toBe('browser');

      fetchSpy.mockRestore();
    });

    it('strips the "epr:" prefix when building the content URL', async () => {
      storageMock.connectionMode = 'doorway';

      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{}'));

      await service.download('epr:some-doc');
      expect(fetchSpy).toHaveBeenCalledWith('http://localhost:8888/db/content/some-doc');

      fetchSpy.mockRestore();
    });

    it('rejects when the browser cache-warm gets a non-ok status', async () => {
      storageMock.connectionMode = 'doorway';

      const fetchSpy = vi
        .spyOn(globalThis, 'fetch')
        .mockResolvedValue(new Response('Not Found', { status: 404 }));

      await expect(service.download('epr:missing')).rejects.toThrow('404');

      fetchSpy.mockRestore();
    });
  });

  describe('pinAsPeer()', () => {
    it('POSTs an item pin with provide:true to /api/v1/pins (epr: prefix stripped)', async () => {
      storageMock.connectionMode = 'direct';

      const resultPromise = service.pinAsPeer('epr:strawberry-guide');

      const req = httpMock.expectOne('http://localhost:8888/api/v1/pins');
      expect(req.request.method).toBe('POST');
      // pin-as-peer is a provide pin: same own-node pins API, provide flag set so
      // the provide reconciler authors a replicates-commons commitment for it.
      expect(req.request.body).toEqual({
        headRef: 'strawberry-guide',
        kind: 'item',
        provide: true,
      });
      req.flush({});

      await expect(resultPromise).resolves.toBeUndefined();
    });

    it('rejects when called on a browser (non-peer) node', async () => {
      storageMock.connectionMode = 'doorway';
      await expect(service.pinAsPeer('epr:strawberry-guide')).rejects.toThrow('peer');
      // No HTTP call must have been made.
      httpMock.expectNone('http://localhost:8888/api/v1/pins');
    });
  });

  describe('pullStatus$()', () => {
    it('polls GET /api/v1/pins/{eprId}/pull and maps the wire shape', async () => {
      const id = 'strawberry-guide';
      const emissions: Array<unknown> = [];
      const sub = service.pullStatus$('epr:strawberry-guide').subscribe(v => emissions.push(v));

      // timer(0, …) leads on the next macrotask; let it fire before asserting.
      await new Promise(resolve => setTimeout(resolve, 0));

      // First poll fires immediately (timer(0, …) leads).
      const req = httpMock.expectOne(`http://localhost:8888/api/v1/pins/${id}/pull`);
      expect(req.request.method).toBe('GET');
      req.flush({
        eprId: id,
        total: 3,
        fetched: 1,
        pending: 2,
        failed: 0,
        caughtUp: false,
      });

      expect(emissions).toEqual([{ total: 3, fetched: 1, pending: 2, failed: 0, caughtUp: false }]);
      sub.unsubscribe();
    });

    it('null-guards: emits a waiting state (caughtUp null, never complete) when the endpoint errors', async () => {
      const emissions: Array<unknown> = [];
      const sub = service.pullStatus$('epr:strawberry-guide').subscribe(v => emissions.push(v));

      // timer(0, …) leads on the next macrotask; let it fire before asserting.
      await new Promise(resolve => setTimeout(resolve, 0));

      const req = httpMock.expectOne('http://localhost:8888/api/v1/pins/strawberry-guide/pull');
      req.flush('boom', { status: 503, statusText: 'Service Unavailable' });

      // On error the stream must NOT die and must NOT claim caughtUp — it emits a
      // null-guarded waiting state (caughtUp:null) so the UI keeps polling.
      expect(emissions).toEqual([
        { total: null, fetched: 0, pending: 0, failed: 0, caughtUp: null },
      ]);
      sub.unsubscribe();
    });
  });
});
