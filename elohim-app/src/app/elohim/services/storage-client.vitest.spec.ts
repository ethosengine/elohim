/**
 * Vitest spike — HTTP service test with provideHttpClientTesting + fakeAsync/tick.
 *
 * Proves: HttpTestingController, fakeAsync, tick, vi.fn() for createSpyObj,
 * provideHttpClient/provideHttpClientTesting replacing HttpClientTestingModule.
 *
 * Uses global describe/it (not imported from vitest) because fakeAsync
 * requires Zone.js-patched globals from @analogjs/vitest-angular/setup-zone.
 */
import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { vi } from 'vitest';

import { StorageClientService, StorageContentNode, StoragePath } from './storage-client.service';
import { CONNECTION_STRATEGY } from '../providers/connection-strategy.provider';
import { ListResponse } from '../models/storage-response.model';

describe('StorageClientService (vitest spike)', () => {
  let service: StorageClientService;
  let httpMock: HttpTestingController;
  let strategyMock: { getStorageBaseUrl: ReturnType<typeof vi.fn>; mode: string };

  beforeEach(() => {
    strategyMock = {
      getStorageBaseUrl: vi.fn().mockReturnValue('http://localhost:8888'),
      mode: 'doorway',
    };

    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        StorageClientService,
        { provide: CONNECTION_STRATEGY, useValue: strategyMock },
      ],
    });

    service = TestBed.inject(StorageClientService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('connectionMode', () => {
    it('should return mode from strategy', () => {
      expect(service.connectionMode).toBe('doorway');
    });

    it('should reflect direct mode', () => {
      strategyMock.mode = 'direct';
      expect(service.connectionMode).toBe('direct');
    });
  });

  describe('getStorageBaseUrl', () => {
    it('should call strategy to get base URL', () => {
      const url = service.getStorageBaseUrl();
      expect(strategyMock.getStorageBaseUrl).toHaveBeenCalled();
      expect(url).toBe('http://localhost:8888');
    });
  });

  describe('getBlobUrl', () => {
    it('should return empty string for empty hash', () => {
      expect(service.getBlobUrl('')).toBe('');
    });

    it('should construct doorway blob URL', () => {
      const url = service.getBlobUrl('sha256-abc123');
      expect(url).toBe('http://localhost:8888/api/blob/sha256-abc123');
    });

    it('should construct direct blob URL when strategy is direct', () => {
      strategyMock.mode = 'direct';
      const url = service.getBlobUrl('sha256-def456');
      expect(url).toBe('http://localhost:8888/blob/sha256-def456');
    });
  });

  describe('fetchBlob', () => {
    it('should fetch blob as ArrayBuffer', fakeAsync(() => {
      const mockBuffer = new ArrayBuffer(8);
      let result: ArrayBuffer | undefined;

      service.fetchBlob('sha256-test').subscribe(buffer => {
        result = buffer;
      });

      const req = httpMock.expectOne('http://localhost:8888/api/blob/sha256-test');
      expect(req.request.method).toBe('GET');
      expect(req.request.responseType).toBe('arraybuffer');
      req.flush(mockBuffer);

      tick();
      expect(result).toBe(mockBuffer);
    }));

    it('should handle fetch errors gracefully', fakeAsync(() => {
      let errorThrown = false;

      service.fetchBlob('sha256-missing').subscribe({
        error: () => {
          errorThrown = true;
        },
      });

      const req = httpMock.expectOne('http://localhost:8888/api/blob/sha256-missing');
      req.error(new ProgressEvent('error'), { status: 404 });

      tick();
      expect(errorThrown).toBe(true);
    }));
  });

  describe('getContent', () => {
    it('should fetch content by ID', fakeAsync(() => {
      const mockContent: StorageContentNode = {
        id: 'test-content',
        contentType: 'concept',
        title: 'Test Content',
        description: 'Test description',
        contentBody: 'Test body',
        contentFormat: 'markdown',
        blobHash: null,
        blobCid: null,
        metadataJson: null,
        tags: ['test'],
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      };

      let result: StorageContentNode | null | undefined;

      service.getContent('test-content').subscribe(content => {
        result = content;
      });

      const req = httpMock.expectOne('http://localhost:8888/db/content/test-content');
      expect(req.request.method).toBe('GET');
      req.flush(mockContent);

      tick();
      expect(result).toEqual(mockContent);
    }));

    it('should return null for 404', fakeAsync(() => {
      let result: StorageContentNode | null | undefined;

      service.getContent('nonexistent').subscribe(content => {
        result = content;
      });

      const req = httpMock.expectOne('http://localhost:8888/db/content/nonexistent');
      req.flush(null, { status: 404, statusText: 'Not Found' });

      tick();
      expect(result).toBeNull();
    }));
  });

  describe('getAllPaths', () => {
    it('should fetch all paths', fakeAsync(() => {
      const mockResponse: ListResponse<StoragePath> = {
        items: [
          {
            id: 'path-1',
            version: '1.0',
            title: 'Path 1',
            description: 'First path',
            difficulty: 'beginner',
            estimatedDuration: '1 hour',
            pathType: 'course',
            thumbnailUrl: null,
            thumbnailBlobHash: null,
            metadataJson: null,
            tags: [],
          },
        ],
        count: 1,
        limit: 100,
        offset: 0,
      };

      let result: ListResponse<StoragePath> | undefined;

      service.getAllPaths().subscribe(response => {
        result = response;
      });

      const req = httpMock.expectOne('http://localhost:8888/db/paths');
      expect(req.request.method).toBe('GET');
      req.flush(mockResponse);

      tick();
      expect(result).toEqual(mockResponse);
      expect(result!.items.length).toBe(1);
    }));
  });
});
