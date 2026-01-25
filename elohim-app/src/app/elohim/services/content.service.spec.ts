import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { of, throwError } from 'rxjs';

import { ContentService, ContentFilters, PathFilters } from './content.service';
import { StorageClientService } from './storage-client.service';
import { ELOHIM_CLIENT, ElohimClient } from '../providers/elohim-client.provider';
import { ContentNode } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';

/**
 * Unit tests for ContentService
 *
 * Tests content fetching, blob resolution, caching, and query operations.
 */
describe('ContentService', () => {
  let service: ContentService;
  let httpMock: HttpTestingController;
  let mockClient: jasmine.SpyObj<ElohimClient>;
  let mockStorageClient: jasmine.SpyObj<StorageClientService>;

  const mockContentNode = {
    id: 'test-content-1',
    contentType: 'article',
    title: 'Test Article',
    description: 'A test description',
    contentBody: '# Hello World',
    contentFormat: 'markdown',
    tags: ['test', 'demo'],
    relatedNodeIds: [],
    reach: 'commons',
  };

  const mockPathData = {
    id: 'test-path-1',
    title: 'Test Learning Path',
    description: 'A test path',
    version: '1.0.0',
    difficulty: 'beginner',
    visibility: 'public',
    pathType: 'course',
    tags: ['learning'],
    createdBy: 'user-1',
    steps: [],
    chapters: [],
  };

  beforeEach(() => {
    mockClient = jasmine.createSpyObj('ElohimClient', [
      'get',
      'query',
      'getBatch',
      'fetch',
      'supportsOffline',
      'backpressure',
    ]);

    mockStorageClient = jasmine.createSpyObj('StorageClientService', ['getBlobUrl']);
    mockStorageClient.getBlobUrl.and.callFake((hash: string) => `/blob/${hash}`);

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        ContentService,
        { provide: ELOHIM_CLIENT, useValue: mockClient },
        { provide: StorageClientService, useValue: mockStorageClient },
      ],
    });

    service = TestBed.inject(ContentService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ===========================================================================
  // getContent Tests
  // ===========================================================================

  describe('getContent', () => {
    it('should fetch content by ID', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.resolve(mockContentNode));

      let result: ContentNode | null = null;
      service.getContent('test-content-1').subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result!.id).toBe('test-content-1');
      expect(result!.title).toBe('Test Article');
      expect(mockClient.get).toHaveBeenCalledWith('content', 'test-content-1');
    }));

    it('should return null for missing content', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.resolve(null));

      let result: ContentNode | null = undefined as unknown as ContentNode | null;
      service.getContent('non-existent').subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeNull();
    }));

    it('should cache content and return cached value on subsequent calls', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.resolve(mockContentNode));

      // First call
      let result1: ContentNode | null = null;
      service.getContent('test-content-1').subscribe(content => {
        result1 = content;
      });
      tick();

      // Second call (should use cache)
      let result2: ContentNode | null = null;
      service.getContent('test-content-1').subscribe(content => {
        result2 = content;
      });
      tick();

      expect(result1).toBeTruthy();
      expect(result2).toBeTruthy();
      expect(result1!.id).toBe(result2!.id);
      // Client should only be called once due to caching
      expect(mockClient.get).toHaveBeenCalledTimes(1);
    }));

    it('should resolve sha256: blob references', fakeAsync(() => {
      const contentWithBlobRef = {
        ...mockContentNode,
        contentBody: 'sha256:abc123def456',
        blobCid: 'sha256:abc123def456',
      };
      mockClient.get.and.returnValue(Promise.resolve(contentWithBlobRef));

      let result: ContentNode | null = null;
      service.getContent('blob-content').subscribe(content => {
        result = content;
      });
      tick();

      // Should make HTTP request for blob
      const req = httpMock.expectOne('/blob/sha256-abc123def456');
      expect(req.request.method).toBe('GET');
      req.flush('# Blob Content Here');
      tick();

      expect(result).toBeTruthy();
      expect(result!.content).toBe('# Blob Content Here');
    }));

    it('should resolve sha256- blob references', fakeAsync(() => {
      const contentWithBlobRef = {
        ...mockContentNode,
        contentBody: 'sha256-abc123def456',
      };
      mockClient.get.and.returnValue(Promise.resolve(contentWithBlobRef));

      let result: ContentNode | null = null;
      service.getContent('blob-content-2').subscribe(content => {
        result = content;
      });
      tick();

      const req = httpMock.expectOne('/blob/sha256-abc123def456');
      req.flush('# More Blob Content');
      tick();

      expect(result).toBeTruthy();
      expect(result!.content).toBe('# More Blob Content');
    }));

    it('should fetch blob when contentBody is empty but blobCid exists', fakeAsync(() => {
      const contentWithBlobCid = {
        ...mockContentNode,
        contentBody: '',
        blobCid: 'sha256-xyz789',
      };
      mockClient.get.and.returnValue(Promise.resolve(contentWithBlobCid));

      let result: ContentNode | null = null;
      service.getContent('sparse-content').subscribe(content => {
        result = content;
      });
      tick();

      const req = httpMock.expectOne('/blob/sha256-xyz789');
      req.flush('# Sparse Content');
      tick();

      expect(result).toBeTruthy();
      expect(result!.content).toBe('# Sparse Content');
    }));

    it('should fall back gracefully when blob fetch fails', fakeAsync(() => {
      const contentWithBlobRef = {
        ...mockContentNode,
        contentBody: 'sha256:failblob',
      };
      mockClient.get.and.returnValue(Promise.resolve(contentWithBlobRef));

      let result: ContentNode | null = null;
      service.getContent('fail-blob-content').subscribe(content => {
        result = content;
      });
      tick();

      const req = httpMock.expectOne('/blob/sha256-failblob');
      req.error(new ProgressEvent('error'), { status: 404 });
      tick();

      // Should still return content without blob
      expect(result).toBeTruthy();
      expect(result!.id).toBe('test-content-1');
    }));

    it('should handle client errors gracefully', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.reject(new Error('Network error')));

      let result: ContentNode | null = undefined as unknown as ContentNode | null;
      service.getContent('error-content').subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeNull();
    }));

    it('should parse JSON for structured formats', fakeAsync(() => {
      const sophiaContent = {
        id: 'sophia-quiz-1',
        contentType: 'quiz',
        title: 'Sophia Quiz',
        contentBody: '{"questions":[{"id":"q1","text":"What is 2+2?"}]}',
        contentFormat: 'sophia-quiz-json',
        tags: [],
      };
      mockClient.get.and.returnValue(Promise.resolve(sophiaContent));

      let result: ContentNode | null = null;
      service.getContent('sophia-quiz-1').subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeTruthy();
      expect(typeof result!.content).toBe('object');
      expect((result!.content as any).questions).toBeDefined();
      expect((result!.content as any).questions[0].id).toBe('q1');
    }));

    it('should handle html5-app format content', fakeAsync(() => {
      const html5Content = {
        id: 'html5-app-1',
        contentType: 'simulation',
        title: 'Interactive Demo',
        contentBody: '{"appId":"demo-app","entryPoint":"index.html"}',
        contentFormat: 'html5-app',
        tags: [],
      };
      mockClient.get.and.returnValue(Promise.resolve(html5Content));

      let result: ContentNode | null = null;
      service.getContent('html5-app-1').subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeTruthy();
      expect(typeof result!.content).toBe('object');
      expect((result!.content as any).appId).toBe('demo-app');
    }));

    it('should not parse non-structured formats as JSON', fakeAsync(() => {
      const markdownContent = {
        ...mockContentNode,
        contentBody: '{"not":"json-parsed"}',
        contentFormat: 'markdown',
      };
      mockClient.get.and.returnValue(Promise.resolve(markdownContent));

      let result: ContentNode | null = null;
      service.getContent('markdown-json').subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeTruthy();
      expect(typeof result!.content).toBe('string');
      expect(result!.content).toBe('{"not":"json-parsed"}');
    }));
  });

  // ===========================================================================
  // queryContent Tests
  // ===========================================================================

  describe('queryContent', () => {
    it('should query content with filters', fakeAsync(() => {
      mockClient.query.and.returnValue(Promise.resolve([mockContentNode]));

      let result: ContentNode[] = [];
      service.queryContent({ tags: ['test'], limit: 10 }).subscribe(content => {
        result = content;
      });
      tick();

      expect(result.length).toBe(1);
      expect(result[0].id).toBe('test-content-1');
    }));

    it('should filter by contentType locally', fakeAsync(() => {
      const items = [
        { ...mockContentNode, id: '1', contentType: 'article' },
        { ...mockContentNode, id: '2', contentType: 'quiz' },
        { ...mockContentNode, id: '3', contentType: 'article' },
      ];
      mockClient.query.and.returnValue(Promise.resolve(items));

      let result: ContentNode[] = [];
      service
        .queryContent({ contentType: 'article' as any } as ContentFilters)
        .subscribe(content => {
          result = content;
        });
      tick();

      expect(result.length).toBe(2);
      expect(result.every(c => (c.contentType as string) === 'article')).toBeTrue();
    }));

    it('should filter by reach locally', fakeAsync(() => {
      const items = [
        { ...mockContentNode, id: '1', reach: 'commons' },
        { ...mockContentNode, id: '2', reach: 'private' },
        { ...mockContentNode, id: '3', reach: 'commons' },
      ];
      mockClient.query.and.returnValue(Promise.resolve(items));

      let result: ContentNode[] = [];
      service.queryContent({ reach: 'commons' }).subscribe(content => {
        result = content;
      });
      tick();

      expect(result.length).toBe(2);
      expect(result.every(c => c.reach === 'commons')).toBeTrue();
    }));

    it('should return empty array on error', fakeAsync(() => {
      mockClient.query.and.returnValue(Promise.reject(new Error('Query failed')));

      let result: ContentNode[] = [];
      service.queryContent({ search: 'test' }).subscribe(content => {
        result = content;
      });
      tick();

      expect(result.length).toBe(0);
    }));
  });

  // ===========================================================================
  // batchGetContent Tests
  // ===========================================================================

  describe('batchGetContent', () => {
    it('should batch fetch multiple content nodes', fakeAsync(() => {
      const batchResult = new Map([
        ['id-1', { ...mockContentNode, id: 'id-1' }],
        ['id-2', { ...mockContentNode, id: 'id-2' }],
      ]);
      mockClient.getBatch.and.returnValue(Promise.resolve(batchResult));

      let result: Map<string, ContentNode> | null = null;
      service.batchGetContent(['id-1', 'id-2']).subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result!.size).toBe(2);
      expect(result!.get('id-1')!.id).toBe('id-1');
    }));

    it('should return empty map on error', fakeAsync(() => {
      mockClient.getBatch.and.returnValue(Promise.reject(new Error('Batch failed')));

      let result: Map<string, ContentNode> = new Map();
      service.batchGetContent(['id-1']).subscribe(content => {
        result = content;
      });
      tick();

      expect(result.size).toBe(0);
    }));
  });

  // ===========================================================================
  // searchContent Tests
  // ===========================================================================

  describe('searchContent', () => {
    it('should search content with query string', fakeAsync(() => {
      mockClient.query.and.returnValue(Promise.resolve([mockContentNode]));

      let result: ContentNode[] = [];
      service.searchContent('hello').subscribe(content => {
        result = content;
      });
      tick();

      expect(result.length).toBe(1);
      expect(mockClient.query).toHaveBeenCalledWith(
        jasmine.objectContaining({
          search: 'hello',
          limit: 50,
        })
      );
    }));

    it('should respect custom limit', fakeAsync(() => {
      mockClient.query.and.returnValue(Promise.resolve([]));

      service.searchContent('test', 25).subscribe();
      tick();

      expect(mockClient.query).toHaveBeenCalledWith(
        jasmine.objectContaining({
          limit: 25,
        })
      );
    }));
  });

  // ===========================================================================
  // Path Operations Tests
  // ===========================================================================

  describe('getPath', () => {
    it('should fetch path by ID', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.resolve(mockPathData));

      let result: LearningPath | null = null;
      service.getPath('test-path-1').subscribe(path => {
        result = path;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result!.id).toBe('test-path-1');
      expect(result!.title).toBe('Test Learning Path');
    }));

    it('should return null for missing path', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.resolve(null));

      let result: LearningPath | null = undefined as unknown as LearningPath | null;
      service.getPath('non-existent').subscribe(path => {
        result = path;
      });
      tick();

      expect(result).toBeNull();
    }));

    it('should cache paths', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.resolve(mockPathData));

      // First call
      service.getPath('test-path-1').subscribe();
      tick();

      // Second call
      service.getPath('test-path-1').subscribe();
      tick();

      expect(mockClient.get).toHaveBeenCalledTimes(1);
    }));

    it('should transform nested path response format', fakeAsync(() => {
      const nestedResponse = {
        id: 'test-path-1', // Required by ContentReadable type
        path: mockPathData,
        chapters: [
          {
            id: 'ch-1',
            title: 'Chapter 1',
            steps: [{ id: 'step-1', title: 'Step 1', orderIndex: 0 }],
          },
        ],
        ungroupedSteps: [],
      };
      mockClient.get.and.returnValue(Promise.resolve(nestedResponse as any));

      let result: LearningPath | null = null;
      service.getPath('nested-path').subscribe(path => {
        result = path;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result!.chapters?.length).toBe(1);
      expect(result!.chapters?.[0]?.title).toBe('Chapter 1');
    }));
  });

  describe('queryPaths', () => {
    it('should query paths with filters', fakeAsync(() => {
      mockClient.query.and.returnValue(Promise.resolve([mockPathData]));

      let result: LearningPath[] = [];
      service.queryPaths({ visibility: 'public', limit: 10 }).subscribe(paths => {
        result = paths;
      });
      tick();

      expect(result.length).toBe(1);
    }));

    it('should filter by difficulty locally', fakeAsync(() => {
      const items = [
        { ...mockPathData, id: '1', difficulty: 'beginner' },
        { ...mockPathData, id: '2', difficulty: 'advanced' },
      ];
      mockClient.query.and.returnValue(Promise.resolve(items));

      let result: LearningPath[] = [];
      service.queryPaths({ difficulty: 'beginner' }).subscribe(paths => {
        result = paths;
      });
      tick();

      expect(result.length).toBe(1);
      expect(result[0].difficulty).toBe('beginner');
    }));
  });

  describe('getAllPaths', () => {
    it('should get all public paths', fakeAsync(() => {
      mockClient.query.and.returnValue(Promise.resolve([mockPathData]));

      let result: LearningPath[] = [];
      service.getAllPaths().subscribe(paths => {
        result = paths;
      });
      tick();

      expect(result.length).toBe(1);
    }));
  });

  // ===========================================================================
  // Relationship Operations Tests
  // ===========================================================================

  describe('getRelationships', () => {
    it('should fetch relationships for content', fakeAsync(() => {
      mockClient.fetch.and.returnValue(
        Promise.resolve({
          items: [
            {
              id: 'rel-1',
              sourceId: 'content-1',
              targetId: 'content-2',
              relationshipType: 'RELATES_TO',
              confidence: 0.9,
            },
          ],
        })
      );

      let result: any[] = [];
      service.getRelationships('content-1').subscribe(rels => {
        result = rels;
      });
      tick();

      expect(result.length).toBe(1);
      expect(result[0].relationshipType).toBe('RELATES_TO');
    }));

    it('should return empty array on error', fakeAsync(() => {
      mockClient.fetch.and.returnValue(Promise.reject(new Error('Failed')));

      let result: any[] = [];
      service.getRelationships('content-1').subscribe(rels => {
        result = rels;
      });
      tick();

      expect(result.length).toBe(0);
    }));
  });

  describe('getContentGraph', () => {
    it('should fetch content graph', fakeAsync(() => {
      mockClient.fetch.and.returnValue(
        Promise.resolve({
          rootId: 'root-1',
          related: [],
          totalNodes: 1,
        })
      );

      let result: any = null;
      service.getContentGraph('root-1').subscribe(graph => {
        result = graph;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result.rootId).toBe('root-1');
    }));
  });

  // ===========================================================================
  // Knowledge Map Operations Tests
  // ===========================================================================

  describe('getKnowledgeMap', () => {
    it('should fetch knowledge map by ID', fakeAsync(() => {
      mockClient.fetch.and.returnValue(
        Promise.resolve({
          id: 'map-1',
          mapType: 'domain',
          title: 'Test Map',
          ownerId: 'user-1',
          subjectType: 'domain',
          subjectId: 'math',
          subjectName: 'Mathematics',
          visibility: 'public',
          overallAffinity: 0.75,
        })
      );

      let result: any = null;
      service.getKnowledgeMap('map-1').subscribe(map => {
        result = map;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result.id).toBe('map-1');
      expect(result.mapType).toBe('domain');
    }));
  });

  describe('queryKnowledgeMaps', () => {
    it('should query knowledge maps with filters', fakeAsync(() => {
      mockClient.fetch.and.returnValue(
        Promise.resolve({
          items: [{ id: 'map-1', mapType: 'domain', title: 'Map' }],
        })
      );

      let result: any[] = [];
      service.queryKnowledgeMaps({ ownerId: 'user-1' }).subscribe(maps => {
        result = maps;
      });
      tick();

      expect(result.length).toBe(1);
    }));
  });

  // ===========================================================================
  // Path Extension Operations Tests
  // ===========================================================================

  describe('getPathExtension', () => {
    it('should fetch path extension by ID', fakeAsync(() => {
      mockClient.fetch.and.returnValue(
        Promise.resolve({
          id: 'ext-1',
          basePathId: 'path-1',
          basePathVersion: '1.0.0',
          extendedBy: 'user-1',
          title: 'My Extension',
          visibility: 'private',
        })
      );

      let result: any = null;
      service.getPathExtension('ext-1').subscribe(ext => {
        result = ext;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result.id).toBe('ext-1');
    }));
  });

  // ===========================================================================
  // Cache Management Tests
  // ===========================================================================

  describe('cache management', () => {
    it('should clear all caches', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.resolve(mockContentNode));

      // Populate cache
      service.getContent('test-1').subscribe();
      tick();

      // Clear cache
      service.clearCache();

      // Next call should hit client again
      service.getContent('test-1').subscribe();
      tick();

      expect(mockClient.get).toHaveBeenCalledTimes(2);
    }));

    it('should invalidate specific content', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.resolve(mockContentNode));

      // Populate cache
      service.getContent('test-1').subscribe();
      tick();

      // Invalidate specific item
      service.invalidateContent('test-1');

      // Next call should hit client again
      service.getContent('test-1').subscribe();
      tick();

      expect(mockClient.get).toHaveBeenCalledTimes(2);
    }));

    it('should invalidate specific path', fakeAsync(() => {
      mockClient.get.and.returnValue(Promise.resolve(mockPathData));

      // Populate cache
      service.getPath('path-1').subscribe();
      tick();

      // Invalidate specific item
      service.invalidatePath('path-1');

      // Next call should hit client again
      service.getPath('path-1').subscribe();
      tick();

      expect(mockClient.get).toHaveBeenCalledTimes(2);
    }));
  });

  // ===========================================================================
  // Client Info Tests
  // ===========================================================================

  describe('client info', () => {
    it('should check offline support', () => {
      mockClient.supportsOffline.and.returnValue(true);

      expect(service.supportsOffline()).toBeTrue();
      expect(mockClient.supportsOffline).toHaveBeenCalled();
    });

    it('should check backpressure', async () => {
      mockClient.backpressure.and.returnValue(Promise.resolve(0.5));

      const result = await service.backpressure();

      expect(result).toBe(0.5);
      expect(mockClient.backpressure).toHaveBeenCalled();
    });
  });

  // ===========================================================================
  // URL Resolution Tests
  // ===========================================================================

  describe('URL resolution', () => {
    it('should resolve blob URLs using storage client', fakeAsync(() => {
      const contentWithThumbnail = {
        ...mockContentNode,
        thumbnailUrl: 'sha256-thumbnail123',
      };
      mockClient.get.and.returnValue(Promise.resolve(contentWithThumbnail));

      let result: any = null;
      service.getContent('with-thumbnail').subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result.thumbnailUrl).toBe('/blob/sha256-thumbnail123');
      expect(mockStorageClient.getBlobUrl).toHaveBeenCalledWith('sha256-thumbnail123');
    }));

    it('should pass through full URLs unchanged', fakeAsync(() => {
      const contentWithFullUrl = {
        ...mockContentNode,
        thumbnailUrl: 'https://example.com/image.jpg',
      };
      mockClient.get.and.returnValue(Promise.resolve(contentWithFullUrl));

      let result: any = null;
      service.getContent('with-full-url').subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result.thumbnailUrl).toBe('https://example.com/image.jpg');
    }));

    it('should handle /blob/ prefix in thumbnail URLs', fakeAsync(() => {
      const contentWithBlobPath = {
        ...mockContentNode,
        thumbnailUrl: '/blob/sha256-abc',
      };
      mockClient.get.and.returnValue(Promise.resolve(contentWithBlobPath));

      let result: any = null;
      service.getContent('with-blob-path').subscribe(content => {
        result = content;
      });
      tick();

      expect(result).toBeTruthy();
      expect(mockStorageClient.getBlobUrl).toHaveBeenCalledWith('sha256-abc');
    }));
  });
});
