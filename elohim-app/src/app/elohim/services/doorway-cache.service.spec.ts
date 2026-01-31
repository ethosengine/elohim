import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';

import { DoorwayCacheService } from './doorway-cache.service';

/**
 * Comprehensive tests for DoorwayCacheService
 *
 * Tests:
 * - Generic cache API methods (get, query)
 * - Typed convenience methods
 * - Cache hit/miss scenarios
 * - Error handling and graceful fallbacks
 * - Timeout behavior
 * - Query parameter handling
 * - Health checks
 */
describe('DoorwayCacheService', () => {
  let service: DoorwayCacheService;
  let httpMock: HttpTestingController;

  const mockContent = {
    id: 'content-1',
    contentType: 'article',
    title: 'Test Article',
    description: 'A test article',
    contentBody: 'Article content',
    contentFormat: 'markdown',
    tags: ['test', 'article'],
  };

  const mockPath = {
    id: 'path-1',
    version: '1.0.0',
    title: 'Test Path',
    description: 'A test learning path',
    difficulty: 'beginner',
    pathType: 'course',
    tags: ['learning', 'test'],
  };

  const mockRelationship = {
    id: 'rel-1',
    sourceId: 'content-1',
    targetId: 'content-2',
    relationshipType: 'RELATES_TO',
    confidence: 0.95,
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [DoorwayCacheService],
    });

    service = TestBed.inject(DoorwayCacheService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  // ===========================================================================
  // Service Creation
  // ===========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should be provided as singleton', () => {
      const service1 = TestBed.inject(DoorwayCacheService);
      const service2 = TestBed.inject(DoorwayCacheService);
      expect(service1).toBe(service2);
    });
  });

  // ===========================================================================
  // Generic Cache Methods
  // ===========================================================================

  describe('get', () => {
    it('should fetch document by type and ID', fakeAsync(() => {
      let result = null;
      service.get('Content', 'content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/api/v1/cache/Content/content-1')
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockContent);
      tick();

      expect(result).toEqual(mockContent as any);
    }));

    it('should handle 404 errors gracefully', fakeAsync(() => {
      let result = undefined;
      service.get('Content', 'not-found').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/not-found'));
      req.error(new ProgressEvent('error'), { status: 404, statusText: 'Not Found' });
      tick();

      expect(result).toBeNull();
    }));

    it('should handle 500 errors gracefully', fakeAsync(() => {
      let result = undefined;
      service.get('Content', 'content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/content-1'));
      req.error(new ProgressEvent('error'), { status: 500, statusText: 'Internal Server Error' });
      tick();

      expect(result).toBeNull();
    }));

    it('should handle timeout errors', fakeAsync(() => {
      let result = undefined;
      service.get('Content', 'content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/content-1'));
      // Don't respond, let it timeout
      tick(6000); // Default timeout is 5000ms

      expect(result).toBeNull();
    }));

    it('should encode special characters in type and ID', fakeAsync(() => {
      service.get('Content Type', 'content/with/slashes').subscribe();

      const req = httpMock.expectOne(request =>
        request.url.includes('Content%20Type/content/with/slashes')
      );
      req.flush(mockContent);
      tick();
    }));
  });

  describe('query', () => {
    it('should query documents by type', fakeAsync(() => {
      let result: any[] = [];
      service.query('Content').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/api/v1/cache/Content')
      );
      expect(req.request.method).toBe('GET');
      req.flush([mockContent]);
      tick();

      expect(result.length).toBe(1);
      expect(result[0]).toEqual(mockContent);
    }));

    it('should query with limit parameter', fakeAsync(() => {
      service.query('Content', { limit: 50 }).subscribe();

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/cache/Content') &&
          request.params.get('limit') === '50'
        );
      });
      req.flush([mockContent]);
      tick();
    }));

    it('should query with skip parameter', fakeAsync(() => {
      service.query('Content', { skip: 10 }).subscribe();

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/cache/Content') &&
          request.params.get('skip') === '10'
        );
      });
      req.flush([mockContent]);
      tick();
    }));

    it('should query with both limit and skip', fakeAsync(() => {
      service.query('Content', { limit: 20, skip: 5 }).subscribe();

      const req = httpMock.expectOne(request => {
        return (
          request.params.get('limit') === '20' &&
          request.params.get('skip') === '5'
        );
      });
      req.flush([mockContent]);
      tick();
    }));

    it('should handle empty results', fakeAsync(() => {
      let result: any[] | null = null;
      service.query('Content').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/cache/Content'));
      req.flush([]);
      tick();

      expect((result as any)?.length).toBe(0);
    }));

    it('should handle query errors gracefully', fakeAsync(() => {
      let result: any[] | null = null;
      service.query('Content').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/cache/Content'));
      req.error(new ProgressEvent('error'), { status: 500 });
      tick();

      expect((result as any)?.length).toBe(0);
    }));

    it('should handle network errors', fakeAsync(() => {
      let result: any[] | null = null;
      service.query('Content').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/cache/Content'));
      req.error(new ProgressEvent('error'), { status: 0, statusText: 'Network Error' });
      tick();

      expect((result as any)?.length).toBe(0);
    }));
  });

  // ===========================================================================
  // Typed Convenience Methods - Content
  // ===========================================================================

  describe('getContent', () => {
    it('should fetch content by ID', fakeAsync(() => {
      let result = null;
      service.getContent('content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/cache/Content/content-1'));
      req.flush(mockContent);
      tick();

      expect(result).toEqual(mockContent as any);
    }));

    it('should handle content not found', fakeAsync(() => {
      let result = undefined;
      service.getContent('not-found').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/not-found'));
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();

      expect(result).toBeNull();
    }));
  });

  describe('getAllContent', () => {
    it('should fetch all content with default limit', fakeAsync(() => {
      let result: any[] = [];
      service.getAllContent().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/cache/Content') &&
          request.params.get('limit') === '1000'
        );
      });
      req.flush([mockContent]);
      tick();

      expect(result.length).toBe(1);
    }));

    it('should accept custom limit', fakeAsync(() => {
      service.getAllContent(500).subscribe();

      const req = httpMock.expectOne(request =>
        request.params.get('limit') === '500'
      );
      req.flush([mockContent]);
      tick();
    }));

    it('should handle large result sets', fakeAsync(() => {
      const largeDataset = Array.from({ length: 100 }, (_, i) => ({
        ...mockContent,
        id: `content-${i}`,
      }));

      let result: any[] = [];
      service.getAllContent().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/cache/Content'));
      req.flush(largeDataset);
      tick();

      expect(result.length).toBe(100);
    }));
  });

  // ===========================================================================
  // Typed Convenience Methods - Paths
  // ===========================================================================

  describe('getPath', () => {
    it('should fetch path by ID', fakeAsync(() => {
      let result = null;
      service.getPath('path-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/cache/LearningPath/path-1')
      );
      req.flush(mockPath);
      tick();

      expect(result).toEqual(mockPath as any);
    }));

    it('should handle path not found', fakeAsync(() => {
      let result = undefined;
      service.getPath('not-found').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/LearningPath/not-found'));
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();

      expect(result).toBeNull();
    }));
  });

  describe('getAllPaths', () => {
    it('should fetch all paths with default limit', fakeAsync(() => {
      let result: any[] = [];
      service.getAllPaths().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/cache/LearningPath') &&
          request.params.get('limit') === '100'
        );
      });
      req.flush([mockPath]);
      tick();

      expect(result.length).toBe(1);
    }));

    it('should accept custom limit', fakeAsync(() => {
      service.getAllPaths(50).subscribe();

      const req = httpMock.expectOne(request =>
        request.params.get('limit') === '50'
      );
      req.flush([mockPath]);
      tick();
    }));
  });

  // ===========================================================================
  // Typed Convenience Methods - Relationships
  // ===========================================================================

  describe('getAllRelationships', () => {
    it('should fetch all relationships with default limit', fakeAsync(() => {
      let result: any[] = [];
      service.getAllRelationships().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/cache/Relationship') &&
          request.params.get('limit') === '1000'
        );
      });
      req.flush([mockRelationship]);
      tick();

      expect(result.length).toBe(1);
    }));

    it('should accept custom limit', fakeAsync(() => {
      service.getAllRelationships(500).subscribe();

      const req = httpMock.expectOne(request =>
        request.params.get('limit') === '500'
      );
      req.flush([mockRelationship]);
      tick();
    }));
  });

  // ===========================================================================
  // Health Check
  // ===========================================================================

  describe('isHealthy', () => {
    it('should return true when health endpoint responds', fakeAsync(() => {
      let result = false;
      service.isHealthy().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/health'));
      expect(req.request.method).toBe('GET');
      expect(req.request.responseType).toBe('text');
      req.flush('OK');
      tick();

      expect(result).toBe(true);
    }));

    it('should return false on error', fakeAsync(() => {
      let result = true;
      service.isHealthy().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/health'));
      req.error(new ProgressEvent('error'), { status: 503 });
      tick();

      expect(result).toBe(false);
    }));

    it('should return false on timeout', fakeAsync(() => {
      let result = true;
      service.isHealthy().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/health'));
      // Don't respond, let it timeout (2000ms for health check)
      tick(3000);

      expect(result).toBe(false);
    }));

    it('should return false on network error', fakeAsync(() => {
      let result = true;
      service.isHealthy().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/health'));
      req.error(new ProgressEvent('error'), { status: 0, statusText: 'Network Error' });
      tick();

      expect(result).toBe(false);
    }));
  });

  // ===========================================================================
  // URL Building & API Key
  // ===========================================================================

  describe('URL building', () => {
    it('should build URLs with proper base path', fakeAsync(() => {
      service.get('Content', 'test-id').subscribe();

      const req = httpMock.expectOne(request => {
        return request.url.includes('/api/v1/cache/Content/test-id');
      });
      req.flush(mockContent);
      tick();
    }));

    it('should handle query parameters correctly', fakeAsync(() => {
      service.query('Content', { limit: 10, skip: 5 }).subscribe();

      const req = httpMock.expectOne(request => {
        const params = request.params;
        return params.get('limit') === '10' && params.get('skip') === '5';
      });
      req.flush([mockContent]);
      tick();
    }));

    it('should not add query params when options are empty', fakeAsync(() => {
      service.query('Content', {}).subscribe();

      const req = httpMock.expectOne(request => {
        return request.params.keys().length === 0;
      });
      req.flush([mockContent]);
      tick();
    }));
  });

  // ===========================================================================
  // Cache Scenarios
  // ===========================================================================

  describe('cache hit/miss scenarios', () => {
    it('should handle cache hit (200 OK)', fakeAsync(() => {
      let result = null;
      service.get('Content', 'cached-item').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/cached-item'));
      req.flush(mockContent, { status: 200, statusText: 'OK' });
      tick();

      expect(result).toEqual(mockContent as any);
    }));

    it('should handle cache miss (404 Not Found)', fakeAsync(() => {
      let result = undefined;
      service.get('Content', 'not-cached').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/not-cached'));
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();

      expect(result).toBeNull();
    }));

    it('should handle cache expired (return empty on query)', fakeAsync(() => {
      let result: any[] | null = null;
      service.query('Content').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/cache/Content'));
      req.flush([]);
      tick();

      expect((result as any)?.length).toBe(0);
    }));
  });

  // ===========================================================================
  // Error Handling Edge Cases
  // ===========================================================================

  describe('error handling edge cases', () => {
    it('should handle malformed JSON response', fakeAsync(() => {
      let result = null;
      service.get('Content', 'malformed').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/malformed'));
      req.error(new ProgressEvent('error'), {
        status: 500,
        statusText: 'Internal Server Error',
      });
      tick();

      expect(result).toBeNull();
    }));

    it('should handle unexpected error types', fakeAsync(() => {
      let result = null;
      service.get('Content', 'test').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/test'));
      req.error(new ProgressEvent('unknown'), { status: 418, statusText: "I'm a teapot" });
      tick();

      expect(result).toBeNull();
    }));

    it('should not log 404 errors (expected for missing content)', fakeAsync(() => {
      spyOn(console, 'error');

      service.get('Content', 'not-found').subscribe();

      const req = httpMock.expectOne(request => request.url.includes('/Content/not-found'));
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();

      // The service should not log 404s as they are expected
      expect(console.error).not.toHaveBeenCalled();
    }));
  });

  // ===========================================================================
  // Multiple Concurrent Requests
  // ===========================================================================

  describe('concurrent requests', () => {
    it('should handle multiple get requests in parallel', fakeAsync(() => {
      const results: any[] = [];

      service.get('Content', 'content-1').subscribe(data => results.push(data));
      service.get('Content', 'content-2').subscribe(data => results.push(data));
      service.get('LearningPath', 'path-1').subscribe(data => results.push(data));

      const requests = httpMock.match(() => true);
      expect(requests.length).toBe(3);

      requests[0].flush(mockContent);
      requests[1].flush({ ...mockContent, id: 'content-2' });
      requests[2].flush(mockPath);
      tick();

      expect(results.length).toBe(3);
    }));

    it('should handle mixed success and error responses', fakeAsync(() => {
      const results: any[] = [];

      service.get('Content', 'success').subscribe(data => results.push(data));
      service.get('Content', 'not-found').subscribe(data => results.push(data));
      service.query('Content').subscribe(data => results.push(data));

      const requests = httpMock.match(() => true);
      expect(requests.length).toBe(3);

      requests[0].flush(mockContent);
      requests[1].error(new ProgressEvent('error'), { status: 404 });
      requests[2].error(new ProgressEvent('error'), { status: 500 });
      tick();

      expect(results.length).toBe(3);
      expect(results[0]).toEqual(mockContent);
      expect(results[1]).toBeNull();
      expect(results[2]).toEqual([]);
    }));
  });

  // ===========================================================================
  // Type Safety & Generic Typing
  // ===========================================================================

  describe('type safety', () => {
    it('should preserve type information with generics', fakeAsync(() => {
      interface CustomContent {
        id: string;
        customField: string;
      }

      let result: CustomContent | null = null;
      service.get<CustomContent>('Content', 'custom-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/custom-1'));
      req.flush({ id: 'custom-1', customField: 'custom-value' });
      tick();

      expect((result as any)?.customField).toBe('custom-value');
    }));

    it('should work with typed convenience methods', fakeAsync(() => {
      interface TypedContent {
        id: string;
        title: string;
      }

      let result: TypedContent | null = null;
      service.getContent<TypedContent>('content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/content-1'));
      req.flush({ id: 'content-1', title: 'Typed Content' });
      tick();

      expect((result as any)?.title).toBe('Typed Content');
    }));
  });

  // ===========================================================================
  // Pagination Scenarios
  // ===========================================================================

  describe('pagination', () => {
    it('should support basic pagination with limit and skip', fakeAsync(() => {
      const allItems = Array.from({ length: 50 }, (_, i) => ({
        ...mockContent,
        id: `content-${i}`,
      }));

      let page1: any[] = [];
      let page2: any[] = [];

      service.query('Content', { limit: 10, skip: 0 }).subscribe(data => {
        page1 = data;
      });

      const req1 = httpMock.expectOne(request =>
        request.params.get('limit') === '10'
      );
      req1.flush(allItems.slice(0, 10));
      tick();

      service.query('Content', { limit: 10, skip: 10 }).subscribe(data => {
        page2 = data;
      });

      const req2 = httpMock.expectOne(request =>
        request.params.get('limit') === '10' && request.params.get('skip') === '10'
      );
      req2.flush(allItems.slice(10, 20));
      tick();

      expect(page1.length).toBe(10);
      expect(page2.length).toBe(10);
      expect(page1[0].id).toBe('content-0');
      expect(page2[0].id).toBe('content-10');
    }));

    it('should handle last page with fewer items', fakeAsync(() => {
      const lastPage = [mockContent, { ...mockContent, id: 'content-2' }];

      let result: any[] = [];
      service.query('Content', { limit: 10, skip: 90 }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request =>
        request.params.get('limit') === '10' && request.params.get('skip') === '90'
      );
      req.flush(lastPage);
      tick();

      expect(result.length).toBe(2);
    }));
  });
});
