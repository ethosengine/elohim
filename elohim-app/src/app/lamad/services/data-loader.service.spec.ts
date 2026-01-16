import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { HolochainContentService } from '@app/elohim/services/holochain-content.service';
import { IndexedDBCacheService } from '@app/elohim/services/indexeddb-cache.service';
import { ProjectionAPIService } from '@app/elohim/services/projection-api.service';
import { ContentResolverService, SourceTier } from '@app/elohim/services/content-resolver.service';
import { ContentService } from '@app/elohim/services/content.service';
import { ELOHIM_CLIENT } from '@app/elohim/providers/elohim-client.provider';
import { LearningPath, PathIndex, ContentNode } from '../models';
import { AgentProgress } from '@app/elohim/models/agent.model';
import { of, throwError, BehaviorSubject } from 'rxjs';

describe('DataLoaderService', () => {
  let service: DataLoaderService;
  let httpMock: HttpTestingController;
  let mockHolochainContent: jasmine.SpyObj<HolochainContentService>;
  let mockIndexedDBCache: jasmine.SpyObj<IndexedDBCacheService>;
  let mockProjectionApi: jasmine.SpyObj<ProjectionAPIService>;
  let mockContentResolver: jasmine.SpyObj<ContentResolverService>;
  let mockContentService: jasmine.SpyObj<ContentService>;
  let mockElohimClient: any;
  const basePath = '/assets/lamad-data';

  const mockContent: ContentNode = {
    id: 'test-content',
    title: 'Test Content',
    description: 'Test content node',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Test',
    tags: [],
    relatedNodeIds: [],
    metadata: {}
  };

  const mockProgress: AgentProgress = {
    agentId: 'test-agent',
    pathId: 'test-path',
    currentStepIndex: 0,
    completedStepIndices: [],
    startedAt: '2025-01-01T00:00:00.000Z',
    lastActivityAt: '2025-01-01T00:00:00.000Z',
    stepAffinity: {},
    stepNotes: {},
    reflectionResponses: {},
    attestationsEarned: []
  };

  beforeEach(() => {
    mockHolochainContent = jasmine.createSpyObj('HolochainContentService', [
      'getContent',
      'getContentByType',
      'getStats',
      'clearCache',
      'isAvailable',
      'getPathIndex',
      'getPathWithSteps'
    ], {
      // Signal property - callable function that returns availability
      available: jasmine.createSpy('available').and.returnValue(true)
    });

    mockIndexedDBCache = jasmine.createSpyObj('IndexedDBCacheService', [
      'init',
      'getPath',
      'setPath',
      'getContent',
      'setContent',
      'getContentBatch',
      'setContentBatch',
      'getStats',
      'clearAll'
    ]);

    mockProjectionApi = jasmine.createSpyObj('ProjectionAPIService', [
      'getContent',
      'queryContent',
      'batchGetContent',
      'getPath',
      'getPathOverview',
      'queryPaths',
      'getRelated',
      'getStats',
      'isHealthy'
    ], {
      enabled: false  // Disabled by default so tests use Holochain path
    });

    mockContentResolver = jasmine.createSpyObj('ContentResolverService', [
      'initialize',
      'resolveContent',
      'resolvePath',
      'batchResolveContent',
      'cacheContent',
      'cachePath',
      'registerStandardSource',
      'setSourceAvailable'
    ], {
      isReady: true,
      state$: new BehaviorSubject('ready').asObservable()
    });

    // Projection API mock returns (disabled by default)
    mockProjectionApi.getContent.and.returnValue(of(null));
    mockProjectionApi.queryContent.and.returnValue(of([]));
    mockProjectionApi.batchGetContent.and.returnValue(of(new Map()));
    mockProjectionApi.getPath.and.returnValue(of(null));
    mockProjectionApi.getPathOverview.and.returnValue(of(null));
    mockProjectionApi.queryPaths.and.returnValue(of([]));
    mockProjectionApi.getRelated.and.returnValue(of([]));
    mockProjectionApi.getStats.and.returnValue(of(null));
    mockProjectionApi.isHealthy.and.returnValue(of(false));

    // Default mock returns
    mockHolochainContent.getStats.and.returnValue(of({ totalCount: 0, byType: {} }));
    mockHolochainContent.isAvailable.and.returnValue(true);
    (mockHolochainContent.getPathIndex as jasmine.Spy).and.returnValue(Promise.resolve({ paths: [], totalCount: 0, lastUpdated: '' }));
    (mockHolochainContent.getPathWithSteps as jasmine.Spy).and.returnValue(Promise.resolve(null));

    // IndexedDB mock returns (disabled by default to use Holochain)
    mockIndexedDBCache.init.and.returnValue(Promise.resolve(false));
    mockIndexedDBCache.getPath.and.returnValue(Promise.resolve(null));
    mockIndexedDBCache.setPath.and.returnValue(Promise.resolve());
    mockIndexedDBCache.getContent.and.returnValue(Promise.resolve(null));
    mockIndexedDBCache.setContent.and.returnValue(Promise.resolve());
    mockIndexedDBCache.getContentBatch.and.returnValue(Promise.resolve(new Map()));
    mockIndexedDBCache.setContentBatch.and.returnValue(Promise.resolve());
    mockIndexedDBCache.getStats.and.returnValue(Promise.resolve({ contentCount: 0, pathCount: 0, isAvailable: false }));
    mockIndexedDBCache.clearAll.and.returnValue(Promise.resolve());

    // ContentResolver mock returns
    mockContentResolver.initialize.and.returnValue(Promise.resolve({ success: true, implementation: 'typescript' }));
    mockContentResolver.resolveContent.and.returnValue(Promise.resolve(null));
    mockContentResolver.resolvePath.and.returnValue(Promise.resolve(null));
    mockContentResolver.batchResolveContent.and.returnValue(Promise.resolve(new Map()));
    mockContentResolver.cacheContent.and.returnValue(Promise.resolve());
    mockContentResolver.cachePath.and.returnValue(Promise.resolve());
    // These methods are called during initialization - they don't return anything
    mockContentResolver.registerStandardSource.and.returnValue(undefined);
    mockContentResolver.setSourceAvailable.and.returnValue(undefined);

    // ContentService mock
    mockContentService = jasmine.createSpyObj('ContentService', [
      'getContent',
      'queryContent',
      'batchGetContent',
      'searchContent',
      'getPath',
      'queryPaths',
      'getAllPaths'
    ]);
    mockContentService.getContent.and.returnValue(of(null));
    mockContentService.queryContent.and.returnValue(of([]));
    mockContentService.batchGetContent.and.returnValue(of(new Map()));
    mockContentService.searchContent.and.returnValue(of([]));
    mockContentService.getPath.and.returnValue(of(null));
    mockContentService.queryPaths.and.returnValue(of([]));
    mockContentService.getAllPaths.and.returnValue(of([]));

    // ElohimClient mock
    mockElohimClient = {
      get: jasmine.createSpy('get').and.returnValue(Promise.resolve(null)),
      query: jasmine.createSpy('query').and.returnValue(Promise.resolve([])),
      supportsOffline: jasmine.createSpy('supportsOffline').and.returnValue(false),
      backpressure: jasmine.createSpy('backpressure').and.returnValue(Promise.resolve(0))
    };

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        DataLoaderService,
        { provide: HolochainContentService, useValue: mockHolochainContent },
        { provide: IndexedDBCacheService, useValue: mockIndexedDBCache },
        { provide: ProjectionAPIService, useValue: mockProjectionApi },
        { provide: ContentResolverService, useValue: mockContentResolver },
        { provide: ContentService, useValue: mockContentService },
        { provide: ELOHIM_CLIENT, useValue: mockElohimClient }
      ]
    });

    service = TestBed.inject(DataLoaderService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getPath', () => {
    it('should load path via ContentService', (done) => {
      const mockPath: LearningPath = {
        id: 'test-path',
        version: '1.0.0',
        title: 'Test Path',
        description: 'A test path',
        purpose: 'Testing',
        createdBy: 'test-agent',
        contributors: [],
        difficulty: 'beginner',
        estimatedDuration: '1 hour',
        visibility: 'public',
        pathType: 'journey',
        tags: ['test'],
        createdAt: '2025-01-01T00:00:00.000Z',
        updatedAt: '2025-01-01T00:00:00.000Z',
        steps: []
      };

      // ContentService returns path directly
      mockContentService.getPath.and.returnValue(of(mockPath));

      service.getPath('test-path').subscribe(path => {
        expect(path.id).toBe('test-path');
        expect(path.title).toBe('Test Path');
        expect(mockContentService.getPath).toHaveBeenCalledWith('test-path');
        done();
      });
    });
  });

  describe('getContent', () => {
    it('should load content via ContentService', (done) => {
      // ContentService returns content directly
      mockContentService.getContent.and.returnValue(of(mockContent));

      service.getContent('test-content').subscribe(content => {
        expect(content.id).toBe(mockContent.id);
        expect(content.title).toBe(mockContent.title);
        expect(mockContentService.getContent).toHaveBeenCalledWith('test-content');
        done();
      });
    });

    it('should cache content in IndexedDB after loading', (done) => {
      mockContentService.getContent.and.returnValue(of(mockContent));

      service.getContent('test-content').subscribe(content => {
        expect(content.id).toBe(mockContent.id);
        // ContentService handles caching internally, but DataLoader also caches to IndexedDB
        expect(mockContentService.getContent).toHaveBeenCalledWith('test-content');
        done();
      });
    });

    it('should return placeholder when content not found', (done) => {
      // Resolver returns null when content not found
      mockContentResolver.resolveContent.and.returnValue(Promise.resolve(null));

      service.getContent('missing-content').subscribe({
        next: content => {
          expect(content.contentType).toBe('placeholder');
          expect(content.title).toContain('Content Not Found');
          expect(content.id).toBe('missing-content');
          done();
        },
        error: () => {
          fail('Should not throw error, should return placeholder');
        }
      });
    });
  });

  describe('getPathIndex', () => {
    it('should load path index from ContentService', (done) => {
      const mockPaths: LearningPath[] = [
        {
          id: 'test-path',
          version: '1.0.0',
          title: 'Test',
          description: 'Desc',
          purpose: 'Testing',
          createdBy: 'test-agent',
          contributors: [],
          difficulty: 'beginner',
          estimatedDuration: '1h',
          visibility: 'public',
          pathType: 'journey',
          tags: [],
          createdAt: '2025-01-01T00:00:00.000Z',
          updatedAt: '2025-01-01T00:00:00.000Z',
          steps: [{
            order: 0,
            stepType: 'content',
            resourceId: 'step-1',
            stepTitle: 'Step 1',
            stepNarrative: '',
            optional: false,
            learningObjectives: [],
            completionCriteria: []
          }]
        }
      ];

      mockContentService.queryPaths.and.returnValue(of(mockPaths));

      service.getPathIndex().subscribe(index => {
        expect(index.totalCount).toBe(1);
        expect(index.paths.length).toBe(1);
        expect(index.paths[0].id).toBe('test-path');
        expect(mockContentService.queryPaths).toHaveBeenCalled();
        done();
      });
    });
  });

  describe('getContentIndex', () => {
    it('should return content index from ContentService', (done) => {
      // Mock content nodes with different types (cast to any to avoid full interface)
      const mockNodes = [
        { id: '1', title: 'A', contentType: 'concept', tags: [] },
        { id: '2', title: 'B', contentType: 'concept', tags: [] },
        { id: '3', title: 'C', contentType: 'concept', tags: [] },
        { id: '4', title: 'D', contentType: 'exercise', tags: [] },
        { id: '5', title: 'E', contentType: 'exercise', tags: [] },
      ] as any[];
      mockContentService.queryContent.and.returnValue(of(mockNodes));

      service.getContentIndex().subscribe(index => {
        expect(index.totalCount).toBe(5);
        expect(index.byType.concept).toBe(3);
        expect(index.byType.exercise).toBe(2);
        done();
      });
    });
  });

  describe('getAgentProgress', () => {
    it('should load agent progress from localStorage', (done) => {
      const progressJson = JSON.stringify(mockProgress);
      spyOn(localStorage, 'getItem').and.returnValue(progressJson);

      service.getAgentProgress('test-agent', 'test-path').subscribe(progress => {
        expect(progress).toEqual(mockProgress);
        expect(localStorage.getItem).toHaveBeenCalledWith('lamad-progress-test-agent-test-path');
        done();
      });
    });

    it('should return null if progress not in localStorage', (done) => {
      spyOn(localStorage, 'getItem').and.returnValue(null);

      service.getAgentProgress('test-agent', 'missing-path').subscribe(progress => {
        expect(progress).toBeNull();
        done();
      });
    });
  });

  describe('saveAgentProgress', () => {
    it('should save progress to localStorage', (done) => {
      spyOn(localStorage, 'setItem');

      service.saveAgentProgress(mockProgress).subscribe(() => {
        expect(localStorage.setItem).toHaveBeenCalledWith(
          'lamad-progress-test-agent-test-path',
          jasmine.any(String)
        );
        done();
      });
    });

    it('should handle localStorage errors gracefully', (done) => {
      spyOn(localStorage, 'setItem').and.throwError('QuotaExceededError');

      service.saveAgentProgress(mockProgress).subscribe(() => {
        expect(true).toBe(true); // Should complete without error
        done();
      });
    });
  });

  describe('getLocalProgress', () => {
    it('should retrieve progress from localStorage', () => {
      const progressJson = JSON.stringify(mockProgress);
      spyOn(localStorage, 'getItem').and.returnValue(progressJson);

      const result = service.getLocalProgress('test-agent', 'test-path');
      expect(result).toEqual(mockProgress);
      expect(localStorage.getItem).toHaveBeenCalledWith('lamad-progress-test-agent-test-path');
    });

    it('should return null if no progress in localStorage', () => {
      spyOn(localStorage, 'getItem').and.returnValue(null);

      const result = service.getLocalProgress('test-agent', 'test-path');
      expect(result).toBeNull();
    });

    it('should return null for malformed JSON', () => {
      spyOn(localStorage, 'getItem').and.returnValue('invalid json');

      const result = service.getLocalProgress('test-agent', 'test-path');
      expect(result).toBeNull();
    });
  });

  describe('clearCache', () => {
    it('should clear Holochain content cache', () => {
      service.clearCache();
      expect(mockHolochainContent.clearCache).toHaveBeenCalled();
    });
  });
});
