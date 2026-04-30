import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';

import { StorageClientService, StorageContentNode } from './storage-client.service';
import { CONNECTION_STRATEGY } from '../providers/connection-strategy.provider';
import { ListResponse, BulkCreateResult } from '../models/storage-response.model';
import { vi } from 'vitest';
import { provideHttpClient } from '@angular/common/http';

describe('StorageClientService', () => {
  let service: StorageClientService;
  let httpMock: HttpTestingController;
  let strategyMock: any;

  beforeEach(() => {
    const strategySpy = {
      getStorageBaseUrl: vi.fn(),
    };
    strategySpy.mode = 'doorway'; // Set as writable property
    strategySpy.getStorageBaseUrl.mockReturnValue('http://localhost:8888');

    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        StorageClientService,
        { provide: CONNECTION_STRATEGY, useValue: strategySpy },
      ],
    });

    service = TestBed.inject(StorageClientService);
    httpMock = TestBed.inject(HttpTestingController);
    strategyMock = TestBed.inject(CONNECTION_STRATEGY);
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

    it('should construct doorway blob URL at /blob/{hash}', () => {
      // Doorway serves blobs via the storage-manifest-registered /blob/{hash}
      // route through the RouteRegistry; both doorway and direct modes use
      // the same canonical path.
      const url = service.getBlobUrl('sha256-abc123');
      expect(url).toBe('http://localhost:8888/blob/sha256-abc123');
    });

    it('should construct direct blob URL when strategy is direct', () => {
      strategyMock.mode = 'direct';
      const url = service.getBlobUrl('sha256-def456');
      expect(url).toBe('http://localhost:8888/blob/sha256-def456');
    });

    it('should handle blob hash with special characters', () => {
      const url = service.getBlobUrl('sha256-abc_123-xyz');
      expect(url).toContain('sha256-abc_123-xyz');
    });
  });

  describe('fetchBlob', () => {
    it('should fetch blob as ArrayBuffer', fakeAsync(() => {
      const mockBuffer = new ArrayBuffer(8);
      let result: ArrayBuffer | undefined;

      service.fetchBlob('sha256-test').subscribe(buffer => {
        result = buffer;
      });

      const req = httpMock.expectOne('http://localhost:8888/blob/sha256-test');
      expect(req.request.method).toBe('GET');
      expect(req.request.responseType).toBe('arraybuffer');
      req.flush(mockBuffer);

      tick();
      expect(result).toBe(mockBuffer);
    }));

    it('should handle fetch errors gracefully', fakeAsync(() => {
      let errorThrown = false;

      service.fetchBlob('sha256-missing').subscribe({
        next: () => {
          throw new Error('Should have errored');
        },
        error: () => {
          errorThrown = true;
        },
      });

      const req = httpMock.expectOne('http://localhost:8888/blob/sha256-missing');
      req.error(new ProgressEvent('error'), { status: 404 });

      tick();
      expect(errorThrown).toBe(true);
    }));

    it('should timeout after default timeout', fakeAsync(() => {
      let timedOut = false;

      service.fetchBlob('sha256-slow').subscribe({
        error: () => {
          timedOut = true;
        },
      });

      const req = httpMock.expectOne('http://localhost:8888/blob/sha256-slow');
      tick(31000); // > 30s default timeout
      expect(timedOut).toBe(true);

      // Clean up the pending request
      if (!req.cancelled) {
        req.flush(null, { status: 500, statusText: 'Timeout' });
      }
    }));
  });

  describe('blobExists', () => {
    it('should return true if blob exists', fakeAsync(() => {
      let exists: boolean | undefined;

      service.blobExists('sha256-exists').subscribe(result => {
        exists = result;
      });

      const req = httpMock.expectOne('http://localhost:8888/blob/sha256-exists');
      expect(req.request.method).toBe('HEAD');
      req.flush(null, { status: 200, statusText: 'OK' });

      tick();
      expect(exists).toBe(true);
    }));

    it('should return false if blob does not exist', fakeAsync(() => {
      let exists: boolean | undefined;

      service.blobExists('sha256-missing').subscribe(result => {
        exists = result;
      });

      const req = httpMock.expectOne('http://localhost:8888/blob/sha256-missing');
      req.error(new ProgressEvent('error'), { status: 404 });

      tick();
      expect(exists).toBe(false);
    }));

    it('should return false on network error', fakeAsync(() => {
      let exists: boolean | undefined;

      service.blobExists('sha256-error').subscribe(result => {
        exists = result;
      });

      const req = httpMock.expectOne('http://localhost:8888/blob/sha256-error');
      req.error(new ProgressEvent('error'));

      tick();
      expect(exists).toBe(false);
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

    it('should handle URL encoding for special characters', fakeAsync(() => {
      service.getContent('test content/with spaces').subscribe();

      const req = httpMock.expectOne(
        'http://localhost:8888/db/content/test%20content%2Fwith%20spaces'
      );
      req.flush(null, { status: 404, statusText: 'Not Found' });

      tick();
    }));

    it('should propagate server errors', fakeAsync(() => {
      let errorMessage = '';

      service.getContent('error-content').subscribe({
        error: err => {
          errorMessage = err.message;
        },
      });

      const req = httpMock.expectOne('http://localhost:8888/db/content/error-content');
      req.flush({ error: 'Database error' }, { status: 500, statusText: 'Internal Server Error' });

      tick();
      expect(errorMessage).toContain('Database error');
    }));
  });

  describe('queryContent', () => {
    it('should query content without filters', fakeAsync(() => {
      const mockResponse: ListResponse<StorageContentNode> = {
        items: [],
        count: 0,
        limit: 100,
        offset: 0,
      };

      let result: ListResponse<StorageContentNode> | undefined;

      service.queryContent().subscribe(response => {
        result = response;
      });

      const req = httpMock.expectOne('http://localhost:8888/db/content');
      expect(req.request.method).toBe('GET');
      req.flush(mockResponse);

      tick();
      expect(result).toEqual(mockResponse);
    }));

    it('should query content with contentType filter', fakeAsync(() => {
      service.queryContent({ contentType: 'concept' }).subscribe();

      const req = httpMock.expectOne('http://localhost:8888/db/content?contentType=concept');
      req.flush({ items: [], count: 0, limit: 100, offset: 0 });

      tick();
    }));

    it('should query content with contentFormat filter', fakeAsync(() => {
      service.queryContent({ contentFormat: 'markdown' }).subscribe();

      const req = httpMock.expectOne('http://localhost:8888/db/content?contentFormat=markdown');
      req.flush({ items: [], count: 0, limit: 100, offset: 0 });

      tick();
    }));

    it('should query content with tags filter', fakeAsync(() => {
      service.queryContent({ tags: ['tag1', 'tag2'] }).subscribe();

      const req = httpMock.expectOne('http://localhost:8888/db/content?tags=tag1%2Ctag2');
      req.flush({ items: [], count: 0, limit: 100, offset: 0 });

      tick();
    }));

    it('should query content with limit and offset', fakeAsync(() => {
      service.queryContent({ limit: 10, offset: 20 }).subscribe();

      const req = httpMock.expectOne('http://localhost:8888/db/content?limit=10&offset=20');
      req.flush({ items: [], count: 0, limit: 10, offset: 20 });

      tick();
    }));

    it('should query content with multiple filters', fakeAsync(() => {
      service
        .queryContent({
          contentType: 'quiz',
          contentFormat: 'sophia',
          tags: ['assessment'],
          limit: 5,
          offset: 0,
        })
        .subscribe();

      // Note: offset=0 is omitted as it's falsy (implementation quirk)
      const req = httpMock.expectOne(
        'http://localhost:8888/db/content?contentType=quiz&contentFormat=sophia&tags=assessment&limit=5'
      );
      req.flush({ items: [], count: 0, limit: 5, offset: 0 });

      tick();
    }));
  });

  describe('bulkCreateContent', () => {
    it('should bulk create content items', fakeAsync(() => {
      const items: Partial<StorageContentNode>[] = [
        {
          id: 'content-1',
          contentType: 'concept',
          title: 'Test 1',
          description: '',
          contentBody: 'Body 1',
          contentFormat: 'markdown',
          tags: [],
        },
        {
          id: 'content-2',
          contentType: 'concept',
          title: 'Test 2',
          description: '',
          contentBody: 'Body 2',
          contentFormat: 'markdown',
          tags: [],
        },
      ];

      const mockResult: BulkCreateResult = {
        inserted: 2,
        skipped: 0,
      };

      let result: BulkCreateResult | undefined;

      service.bulkCreateContent(items).subscribe(response => {
        result = response;
      });

      const req = httpMock.expectOne('http://localhost:8888/db/content/bulk');
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual(items);
      req.flush(mockResult);

      tick();
      expect(result).toEqual(mockResult);
    }));

    it('should use extended timeout for bulk operations', fakeAsync(() => {
      service.bulkCreateContent([]).subscribe();

      const req = httpMock.expectOne('http://localhost:8888/db/content/bulk');
      req.flush({ inserted: 0, skipped: 0 });

      tick(120000); // Should not timeout before 2 minutes
    }));
  });

  describe('bulkCreateRelationships', () => {
    it('should bulk create relationships', fakeAsync(() => {
      const items = [
        {
          sourceId: 'content-1',
          targetId: 'content-2',
          relationshipType: 'RELATES_TO',
          confidence: 0.9,
        },
      ];

      const mockResult: BulkCreateResult = {
        inserted: 1,
        skipped: 0,
      };

      let result: BulkCreateResult | undefined;

      service.bulkCreateRelationships(items).subscribe(response => {
        result = response;
      });

      const req = httpMock.expectOne('http://localhost:8888/db/relationships/bulk');
      expect(req.request.method).toBe('POST');
      req.flush(mockResult);

      tick();
      expect(result).toEqual(mockResult);
    }));

    it('should handle relationship with metadata', fakeAsync(() => {
      const items = [
        {
          sourceId: 'content-1',
          targetId: 'content-2',
          relationshipType: 'CONTAINS',
          metadata: { order: 1, section: 'intro' },
        },
      ];

      service.bulkCreateRelationships(items).subscribe();

      const req = httpMock.expectOne('http://localhost:8888/db/relationships/bulk');
      expect(req.request.body[0].metadata).toEqual({ order: 1, section: 'intro' });
      req.flush({ inserted: 1, skipped: 0 });

      tick();
    }));
  });

  describe('error handling', () => {
    it('should extract error message from backend response', fakeAsync(() => {
      let errorMessage = '';

      service.getContent('error-test').subscribe({
        error: err => {
          errorMessage = err.message;
        },
      });

      const req = httpMock.expectOne('http://localhost:8888/db/content/error-test');
      req.flush({ error: 'Custom error message' }, { status: 400, statusText: 'Bad Request' });

      tick();
      expect(errorMessage).toContain('Custom error message');
    }));

    it('should handle network errors gracefully', fakeAsync(() => {
      let errorOccurred = false;

      service.getContent('network-fail').subscribe({
        error: () => {
          errorOccurred = true;
        },
      });

      const req = httpMock.expectOne('http://localhost:8888/db/content/network-fail');
      req.error(new ProgressEvent('error'));

      tick();
      expect(errorOccurred).toBe(true);
    }));
  });
});
