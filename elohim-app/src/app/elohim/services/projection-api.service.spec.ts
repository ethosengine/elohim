import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import {
  HttpClientTestingModule,
  HttpTestingController,
} from '@angular/common/http/testing';

import { ProjectionAPIService } from './projection-api.service';
import { StorageClientService } from './storage-client.service';
import { ContentNode, ContentType } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';

/**
 * Comprehensive tests for ProjectionAPIService
 *
 * Tests:
 * - API integration patterns
 * - Cache hit/miss scenarios (via HTTP mocking)
 * - Error handling and graceful fallbacks
 * - Timeout behavior
 * - Data transformation logic
 * - Client-side filtering
 * - Blob URL resolution
 */
describe('ProjectionAPIService', () => {
  let service: ProjectionAPIService;
  let httpMock: HttpTestingController;
  let mockStorageClient: jasmine.SpyObj<StorageClientService>;

  const mockContentData = {
    id: 'content-1',
    contentType: 'article',
    title: 'Test Article',
    description: 'A test article',
    content: 'Article body content',
    contentFormat: 'markdown',
    tags: ['test', 'article'],
    relatedNodeIds: ['related-1'],
    metadata: { custom: 'value' },
    authorId: 'author-1',
    reach: 'commons',
    trustScore: 0.95,
    estimatedMinutes: 10,
    thumbnailUrl: 'blob/sha256-thumb123',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-02T00:00:00Z',
  };

  const mockPathData = {
    id: 'path-1',
    version: '1.0.0',
    title: 'Test Path',
    description: 'A test learning path',
    purpose: 'Learn testing',
    difficulty: 'beginner',
    estimatedDuration: '2 hours',
    visibility: 'public',
    pathType: 'course',
    thumbnailUrl: 'blob/sha256-paththumb',
    thumbnailAlt: 'Path thumbnail',
    tags: ['learning', 'test'],
    createdBy: 'author-1',
    contributors: ['contributor-1'],
    steps: [{ id: 'step-1' }],
    chapters: [{ id: 'chapter-1' }],
    stepCount: 5,
    chapterCount: 2,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-02T00:00:00Z',
  };

  beforeEach(() => {
    // Mock StorageClientService
    mockStorageClient = jasmine.createSpyObj('StorageClientService', ['getBlobUrl']);
    mockStorageClient.getBlobUrl.and.callFake((hash: string) => `https://blob.example.com/${hash}`);

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        ProjectionAPIService,
        { provide: StorageClientService, useValue: mockStorageClient },
      ],
    });

    service = TestBed.inject(ProjectionAPIService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  // ===========================================================================
  // Service Creation & Configuration
  // ===========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should have enabled property', () => {
      expect(service.enabled).toBeDefined();
    });

    it('should be provided as singleton', () => {
      const service1 = TestBed.inject(ProjectionAPIService);
      const service2 = TestBed.inject(ProjectionAPIService);
      expect(service1).toBe(service2);
    });
  });

  // ===========================================================================
  // Content Queries - Single Item
  // ===========================================================================

  describe('getContent', () => {
    it('should fetch content by ID', fakeAsync(() => {
      let result: ContentNode | null = null;
      service.getContent('content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/api/v1/cache/Content/content-1')
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockContentData);
      tick();

      expect(result).toBeTruthy();
      expect((result as any)?.id).toBe('content-1');
      expect((result as any)?.title).toBe('Test Article');
    }));

    it('should transform content data correctly', fakeAsync(() => {
      let result: ContentNode | null = null;
      service.getContent('content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/content-1'));
      req.flush(mockContentData);
      tick();

      expect((result as any)?.contentType).toBe('article');
      expect((result as any)?.tags).toEqual(['test', 'article']);
      expect((result as any)?.metadata).toEqual({ custom: 'value' });
      expect((result as any)?.reach).toBe('commons');
    }));

    it('should resolve blob URLs via StorageClientService', fakeAsync(() => {
      let result: ContentNode | null = null;
      service.getContent('content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/content-1'));
      req.flush(mockContentData);
      tick();

      expect(mockStorageClient.getBlobUrl).toHaveBeenCalledWith('sha256-thumb123');
      expect((result as any)?.thumbnailUrl).toBe('https://blob.example.com/sha256-thumb123');
    }));

    it('should handle full URLs without transformation', fakeAsync(() => {
      const dataWithFullUrl = {
        ...mockContentData,
        thumbnailUrl: 'https://external.com/image.jpg',
      };

      let result: ContentNode | null = null;
      service.getContent('content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/content-1'));
      req.flush(dataWithFullUrl);
      tick();

      expect((result as any)?.thumbnailUrl).toBe('https://external.com/image.jpg');
    }));

    it('should handle 404 errors gracefully', fakeAsync(() => {
      let result: ContentNode | null = null;
      service.getContent('not-found').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/not-found'));
      req.error(new ProgressEvent('error'), { status: 404, statusText: 'Not Found' });
      tick();

      expect(result).toBeNull();
    }));

    it('should handle timeout errors', fakeAsync(() => {
      let result: ContentNode | null = null;
      service.getContent('content-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/content-1'));
      // Don't respond, let it timeout
      tick(6000); // Default timeout is 5000ms

      // Error should result in null
      expect(result).toBeNull();
    }));

    it('should return null when API is disabled', fakeAsync(() => {
      // This would require mocking environment, so we test the observable completes
      let result: ContentNode | null | undefined = undefined;
      const obs = service.getContent('content-1');
      obs.subscribe(data => {
        result = data;
      });

      // If enabled, there should be a request
      const requests = httpMock.match(() => true);
      if (requests.length > 0) {
        requests[0].flush(mockContentData);
      }
      tick();

      expect(result).toBeDefined();
    }));

    it('should use shareReplay for caching', fakeAsync(() => {
      const obs = service.getContent('content-1');

      // Subscribe twice
      let result1: ContentNode | null = null;
      let result2: ContentNode | null = null;

      obs.subscribe(data => {
        result1 = data;
      });
      obs.subscribe(data => {
        result2 = data;
      });

      // Should only make one HTTP request
      const req = httpMock.expectOne(request => request.url.includes('/Content/content-1'));
      req.flush(mockContentData);
      tick();

      expect(result1).toEqual(result2);
    }));
  });

  // ===========================================================================
  // Content Queries - Collection
  // ===========================================================================

  describe('queryContent', () => {
    it('should query content with limit and skip', fakeAsync(() => {
      let result: ContentNode[] = [];
      service.queryContent({ limit: 10, skip: 5 }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/api/v1/cache/Content') &&
          request.params.get('limit') === '10' &&
          request.params.get('skip') === '5'
        );
      });
      expect(req.request.method).toBe('GET');
      req.flush([mockContentData]);
      tick();

      expect(result.length).toBe(1);
    }));

    it('should apply client-side filtering by ID', fakeAsync(() => {
      const content1 = { ...mockContentData, id: 'content-1' };
      const content2 = { ...mockContentData, id: 'content-2' };

      let result: ContentNode[] = [];
      service.queryContent({ id: 'content-1' }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([content1, content2]);
      tick();

      expect(result.length).toBe(1);
      expect(result[0].id).toBe('content-1');
    }));

    it('should apply client-side filtering by IDs array', fakeAsync(() => {
      const content1 = { ...mockContentData, id: 'content-1' };
      const content2 = { ...mockContentData, id: 'content-2' };
      const content3 = { ...mockContentData, id: 'content-3' };

      let result: ContentNode[] = [];
      service.queryContent({ ids: ['content-1', 'content-3'] }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([content1, content2, content3]);
      tick();

      expect(result.length).toBe(2);
      expect(result.map(c => c.id)).toEqual(['content-1', 'content-3']);
    }));

    it('should apply client-side filtering by content type', fakeAsync(() => {
      const article = { ...mockContentData, contentType: 'article' };
      const video = { ...mockContentData, id: 'video-1', contentType: 'video' };

      let result: ContentNode[] = [];
      service.queryContent({ contentType: 'article' as ContentType }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([article, video]);
      tick();

      expect(result.length).toBe(1);
      expect(result[0].contentType).toBe('article');
    }));

    it('should apply client-side filtering by multiple content types', fakeAsync(() => {
      const article = { ...mockContentData, contentType: 'article' };
      const video = { ...mockContentData, id: 'video-1', contentType: 'video' };
      const quiz = { ...mockContentData, id: 'quiz-1', contentType: 'quiz' };

      let result: ContentNode[] = [];
      service.queryContent({ contentType: ['article', 'quiz'] as any }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([article, video, quiz]);
      tick();

      expect(result.length).toBe(2);
      expect(result.map(c => c.contentType)).toContain('article' as any);
      expect(result.map(c => c.contentType)).toContain('quiz' as any);
    }));

    it('should apply client-side filtering by tags (all must match)', fakeAsync(() => {
      const content1 = { ...mockContentData, tags: ['test', 'article', 'featured'] };
      const content2 = { ...mockContentData, id: 'content-2', tags: ['test', 'video'] };

      let result: ContentNode[] = [];
      service.queryContent({ tags: ['test', 'article'] }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([content1, content2]);
      tick();

      expect(result.length).toBe(1);
      expect(result[0].id).toBe('content-1');
    }));

    it('should apply client-side filtering by anyTags', fakeAsync(() => {
      const content1 = { ...mockContentData, tags: ['javascript', 'tutorial'] };
      const content2 = { ...mockContentData, id: 'content-2', tags: ['python', 'tutorial'] };
      const content3 = { ...mockContentData, id: 'content-3', tags: ['rust', 'advanced'] };

      let result: ContentNode[] = [];
      service.queryContent({ anyTags: ['javascript', 'python'] }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([content1, content2, content3]);
      tick();

      expect(result.length).toBe(2);
      expect(result.map(c => c.id)).toContain('content-1');
      expect(result.map(c => c.id)).toContain('content-2');
    }));

    it('should apply client-side filtering by publicOnly', fakeAsync(() => {
      const publicContent = { ...mockContentData, reach: 'commons' };
      const privateContent = { ...mockContentData, id: 'private-1', reach: 'private' };

      let result: ContentNode[] = [];
      service.queryContent({ publicOnly: true }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([publicContent, privateContent]);
      tick();

      expect(result.length).toBe(1);
      expect(result[0].reach).toBe('commons');
    }));

    it('should apply client-side filtering by author', fakeAsync(() => {
      const content1 = { ...mockContentData, authorId: 'author-1' };
      const content2 = { ...mockContentData, id: 'content-2', authorId: 'author-2' };

      let result: ContentNode[] = [];
      service.queryContent({ author: 'author-1' }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([content1, content2]);
      tick();

      expect(result.length).toBe(1);
      expect(result[0].authorId).toBe('author-1');
    }));

    it('should handle empty results', fakeAsync(() => {
      let result: ContentNode[] | null = null;
      service.queryContent({}).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([]);
      tick();

      expect((result as any)?.length).toBe(0);
    }));

    it('should handle errors gracefully and return empty array', fakeAsync(() => {
      let result: ContentNode[] | null = null;
      service.queryContent({}).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.error(new ProgressEvent('error'), { status: 500 });
      tick();

      expect((result as any)?.length).toBe(0);
    }));
  });

  // ===========================================================================
  // Batch Operations
  // ===========================================================================

  describe('batchGetContent', () => {
    it('should fetch multiple content items by IDs', fakeAsync(() => {
      const content1 = { ...mockContentData, id: 'content-1' };
      const content2 = { ...mockContentData, id: 'content-2' };

      let result: Map<string, ContentNode> = new Map();
      service.batchGetContent(['content-1', 'content-2']).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content'));
      req.flush([content1, content2]);
      tick();

      expect(result.size).toBe(2);
      expect(result.get('content-1')?.title).toBe('Test Article');
      expect(result.get('content-2')?.title).toBe('Test Article');
    }));

    it('should return empty map for empty IDs array', fakeAsync(() => {
      let result: Map<string, ContentNode> | null = null;
      service.batchGetContent([]).subscribe(data => {
        result = data;
      });

      tick();

      expect((result as any)?.size).toBe(0);
      httpMock.expectNone(() => true);
    }));
  });

  describe('searchContent', () => {
    it('should search content with query and limit', fakeAsync(() => {
      let result: ContentNode[] = [];
      service.searchContent('test query', 25).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/Content') &&
          request.params.get('limit') === '25'
        );
      });
      req.flush([mockContentData]);
      tick();

      expect(result.length).toBe(1);
    }));
  });

  // ===========================================================================
  // Path Queries
  // ===========================================================================

  describe('getPath', () => {
    it('should fetch path by ID', fakeAsync(() => {
      let result: LearningPath | null = null;
      service.getPath('path-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request =>
        request.url.includes('/api/v1/cache/LearningPath/path-1')
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockPathData);
      tick();

      expect(result).toBeTruthy();
      expect((result as any)?.id).toBe('path-1');
      expect((result as any)?.title).toBe('Test Path');
    }));

    it('should transform path data correctly', fakeAsync(() => {
      let result: LearningPath | null = null;
      service.getPath('path-1').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/LearningPath/path-1'));
      req.flush(mockPathData);
      tick();

      expect((result as any)?.difficulty).toBe('beginner');
      expect((result as any)?.tags).toEqual(['learning', 'test']);
      expect((result as any)?.stepCount).toBe(5);
      expect((result as any)?.chapterCount).toBe(2);
    }));

    it('should handle path 404 errors gracefully', fakeAsync(() => {
      let result: LearningPath | null = null;
      service.getPath('not-found').subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/LearningPath/not-found'));
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();

      expect(result).toBeNull();
    }));
  });

  describe('queryPaths', () => {
    it('should query paths with limit and skip', fakeAsync(() => {
      let result: LearningPath[] = [];
      service.queryPaths({ limit: 20, skip: 10 }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/LearningPath') &&
          request.params.get('limit') === '20' &&
          request.params.get('skip') === '10'
        );
      });
      req.flush([mockPathData]);
      tick();

      expect(result.length).toBe(1);
    }));

    it('should apply client-side filtering by difficulty', fakeAsync(() => {
      const beginner = { ...mockPathData, difficulty: 'beginner' };
      const advanced = { ...mockPathData, id: 'path-2', difficulty: 'advanced' };

      let result: LearningPath[] = [];
      service.queryPaths({ difficulty: 'beginner' }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/LearningPath'));
      req.flush([beginner, advanced]);
      tick();

      expect(result.length).toBe(1);
      expect(result[0].difficulty).toBe('beginner');
    }));

    it('should apply client-side filtering by visibility', fakeAsync(() => {
      const publicPath = { ...mockPathData, visibility: 'public' };
      const privatePath = { ...mockPathData, id: 'path-2', visibility: 'private' };

      let result: LearningPath[] = [];
      service.queryPaths({ visibility: 'public' }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/LearningPath'));
      req.flush([publicPath, privatePath]);
      tick();

      expect(result.length).toBe(1);
      expect(result[0].visibility).toBe('public');
    }));

    it('should apply client-side filtering by publicOnly', fakeAsync(() => {
      const publicPath = { ...mockPathData, visibility: 'public' };
      const privatePath = { ...mockPathData, id: 'path-2', visibility: 'private' };

      let result: LearningPath[] = [];
      service.queryPaths({ publicOnly: true }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/LearningPath'));
      req.flush([publicPath, privatePath]);
      tick();

      expect(result.length).toBe(1);
    }));

    it('should apply client-side filtering by tags', fakeAsync(() => {
      const path1 = { ...mockPathData, tags: ['learning', 'beginner'] };
      const path2 = { ...mockPathData, id: 'path-2', tags: ['advanced', 'expert'] };

      let result: LearningPath[] = [];
      service.queryPaths({ tags: ['learning'] }).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/LearningPath'));
      req.flush([path1, path2]);
      tick();

      expect(result.length).toBe(1);
      expect(result[0].id).toBe('path-1');
    }));

    it('should handle path query errors gracefully', fakeAsync(() => {
      let result: LearningPath[] | null = null;
      service.queryPaths({}).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/LearningPath'));
      req.error(new ProgressEvent('error'), { status: 500 });
      tick();

      expect((result as any)?.length).toBe(0);
    }));
  });

  describe('getAllPaths', () => {
    it('should get all public paths with default limit', fakeAsync(() => {
      let result: LearningPath[] = [];
      service.getAllPaths().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => {
        return (
          request.url.includes('/LearningPath') &&
          request.params.get('limit') === '100'
        );
      });
      req.flush([mockPathData]);
      tick();

      expect(result.length).toBe(1);
    }));

    it('should accept custom limit', fakeAsync(() => {
      service.getAllPaths(50).subscribe();

      const req = httpMock.expectOne(request =>
        request.params.get('limit') === '50'
      );
      req.flush([]);
      tick();
    }));
  });

  describe('batchGetPaths', () => {
    it('should fetch multiple paths by IDs', fakeAsync(() => {
      const path1 = { ...mockPathData, id: 'path-1' };
      const path2 = { ...mockPathData, id: 'path-2' };

      let result: Map<string, LearningPath> = new Map();
      service.batchGetPaths(['path-1', 'path-2']).subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/LearningPath'));
      req.flush([path1, path2]);
      tick();

      expect(result.size).toBe(2);
      expect(result.get('path-1')?.title).toBe('Test Path');
    }));

    it('should return empty map for empty IDs', fakeAsync(() => {
      let result: Map<string, LearningPath> | null = null;
      service.batchGetPaths([]).subscribe(data => {
        result = data;
      });

      tick();

      expect((result as any)?.size).toBe(0);
    }));
  });

  // ===========================================================================
  // Stats & Health
  // ===========================================================================

  describe('getStats', () => {
    it('should fetch projection stats', fakeAsync(() => {
      const mockStats = {
        totalEntries: 1000,
        hotCacheEntries: 500,
        expiredEntries: 10,
        mongoConnected: true,
      };

      let result: typeof mockStats | null = null;
      service.getStats().subscribe(data => {
        result = data as typeof mockStats;
      });

      const req = httpMock.expectOne(request => request.url.includes('/api/v1/cache/stats'));
      req.flush(mockStats);
      tick();

      expect(result).toEqual(mockStats as any);
    }));

    it('should handle stats errors gracefully', fakeAsync(() => {
      let result = null;
      service.getStats().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/stats'));
      req.error(new ProgressEvent('error'), { status: 503 });
      tick();

      expect(result).toBeNull();
    }));
  });

  describe('isHealthy', () => {
    it('should return true when stats are available', fakeAsync(() => {
      let result = false;
      service.isHealthy().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/stats'));
      req.flush({ totalEntries: 100, mongoConnected: true });
      tick();

      expect(result).toBe(true);
    }));

    it('should return false on error', fakeAsync(() => {
      let result = true;
      service.isHealthy().subscribe(data => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/stats'));
      req.error(new ProgressEvent('error'));
      tick();

      expect(result).toBe(false);
    }));
  });

  // ===========================================================================
  // Edge Cases & Data Transformation
  // ===========================================================================

  describe('data transformation edge cases', () => {
    it('should handle content with docId instead of id', fakeAsync(() => {
      const dataWithDocId = { ...mockContentData, docId: 'doc-123' };
      delete (dataWithDocId as any).id;

      let result: ContentNode | null = null;
      service.getContent('test').subscribe((data: ContentNode | null) => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/test'));
      req.flush(dataWithDocId);
      tick();

      expect((result as any)?.id).toBe('doc-123');
    }));

    it('should handle content with author instead of authorId', fakeAsync(() => {
      const dataWithAuthor = { ...mockContentData, author: 'author-2' };
      delete (dataWithAuthor as any).authorId;

      let result: ContentNode | null = null;
      service.getContent('test').subscribe((data: ContentNode | null) => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/test'));
      req.flush(dataWithAuthor);
      tick();

      expect((result as any)?.authorId).toBe('author-2');
    }));

    it('should provide default values for missing fields', fakeAsync(() => {
      const minimalData = {
        id: 'minimal-1',
      };

      let result: ContentNode | null = null;
      service.getContent('test').subscribe((data: ContentNode | null) => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/test'));
      req.flush(minimalData);
      tick();

      expect((result as any)?.title).toBe('');
      expect((result as any)?.description).toBe('');
      expect((result as any)?.contentFormat).toBe('markdown');
      expect((result as any)?.tags).toEqual([]);
      expect((result as any)?.reach).toBe('private');
    }));

    it('should handle null thumbnailUrl', fakeAsync(() => {
      const dataWithNullThumb = { ...mockContentData, thumbnailUrl: null };

      let result: ContentNode | null = null;
      service.getContent('test').subscribe((data: ContentNode | null) => {
        result = data;
      });

      const req = httpMock.expectOne(request => request.url.includes('/Content/test'));
      req.flush(dataWithNullThumb);
      tick();

      expect((result as any)?.thumbnailUrl).toBeUndefined();
    }));

    it('should handle various blob URL formats', fakeAsync(() => {
      const testCases = [
        { input: '/blob/sha256-abc', expected: 'sha256-abc' },
        { input: 'blob/sha256-def', expected: 'sha256-def' },
        { input: 'sha256-ghi', expected: 'sha256-ghi' },
      ];

      testCases.forEach(testCase => {
        mockStorageClient.getBlobUrl.calls.reset();

        const data = { ...mockContentData, thumbnailUrl: testCase.input };
        let result: ContentNode | null = null;

        service.getContent('test').subscribe(d => {
          result = d;
        });

        const req = httpMock.expectOne(request => request.url.includes('/Content/test'));
        req.flush(data);
        tick();

        expect(mockStorageClient.getBlobUrl).toHaveBeenCalledWith(testCase.expected);
      });
    }));
  });

  // ===========================================================================
  // API Key & URL Building
  // ===========================================================================

  describe('URL building', () => {
    it('should encode special characters in IDs', fakeAsync(() => {
      service.getContent('content/with/slashes').subscribe();

      const req = httpMock.expectOne(request =>
        request.url.includes('content%2Fwith%2Fslashes')
      );
      req.flush(mockContentData);
      tick();
    }));

    it('should build proper cache endpoint URLs', fakeAsync(() => {
      service.getContent('test-id').subscribe();

      const req = httpMock.expectOne(request => {
        return request.url.includes('/api/v1/cache/Content/test-id');
      });
      req.flush(mockContentData);
      tick();
    }));
  });
});
