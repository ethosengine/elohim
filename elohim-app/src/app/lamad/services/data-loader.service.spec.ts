import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { HolochainContentService } from '@app/elohim/services/holochain-content.service';
import { IndexedDBCacheService } from '@app/elohim/services/indexeddb-cache.service';
import { ProjectionAPIService } from '@app/elohim/services/projection-api.service';
import { ContentResolverService, SourceTier } from '@app/elohim/services/content-resolver.service';
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
    ]);

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
      'cacheContent',
      'cachePath'
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
    mockHolochainContent.getStats.and.returnValue(of({ total_count: 0, by_type: {} }));
    mockHolochainContent.isAvailable.and.returnValue(true);
    (mockHolochainContent.getPathIndex as jasmine.Spy).and.returnValue(Promise.resolve({ paths: [], total_count: 0, last_updated: '' }));
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
    mockContentResolver.cacheContent.and.returnValue(Promise.resolve());
    mockContentResolver.cachePath.and.returnValue(Promise.resolve());

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        DataLoaderService,
        { provide: HolochainContentService, useValue: mockHolochainContent },
        { provide: IndexedDBCacheService, useValue: mockIndexedDBCache },
        { provide: ProjectionAPIService, useValue: mockProjectionApi },
        { provide: ContentResolverService, useValue: mockContentResolver }
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
    it('should load path via ContentResolver', (done) => {
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

      // ContentResolver returns path with resolution metadata
      mockContentResolver.resolvePath.and.returnValue(Promise.resolve({
        data: mockPath,
        sourceId: 'conductor',
        tier: SourceTier.Authoritative,
        durationMs: 50
      }));

      service.getPath('test-path').subscribe(path => {
        expect(path.id).toBe('test-path');
        expect(path.title).toBe('Test Path');
        expect(mockContentResolver.resolvePath).toHaveBeenCalledWith('test-path');
        done();
      });
    });
  });

  describe('getContent', () => {
    it('should load content via ContentResolver', (done) => {
      // ContentResolver returns content with resolution metadata
      mockContentResolver.resolveContent.and.returnValue(Promise.resolve({
        data: mockContent,
        sourceId: 'conductor',
        tier: SourceTier.Authoritative,
        durationMs: 50
      }));

      service.getContent('test-content').subscribe(content => {
        expect(content).toEqual(mockContent);
        expect(mockContentResolver.resolveContent).toHaveBeenCalledWith('test-content');
        done();
      });
    });

    it('should cache content requests in memory', (done) => {
      mockContentResolver.resolveContent.and.returnValue(Promise.resolve({
        data: mockContent,
        sourceId: 'conductor',
        tier: SourceTier.Authoritative,
        durationMs: 50
      }));

      service.getContent('test-content').subscribe();
      service.getContent('test-content').subscribe(content => {
        expect(content).toEqual(mockContent);
        // Should only call resolver once due to in-memory caching
        expect(mockContentResolver.resolveContent).toHaveBeenCalledTimes(1);
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
    it('should load path index from Holochain', (done) => {
      const mockHcPathIndex = {
        paths: [
          { id: 'test-path', title: 'Test', description: 'Desc', difficulty: 'beginner', estimated_duration: '1h', step_count: 3, tags: [] }
        ],
        total_count: 1,
        last_updated: '2025-01-01T00:00:00.000Z',
      };

      (mockHolochainContent.getPathIndex as jasmine.Spy).and.returnValue(Promise.resolve(mockHcPathIndex));

      service.getPathIndex().subscribe(index => {
        expect(index.totalCount).toBe(1);
        expect(index.paths.length).toBe(1);
        expect(index.paths[0].id).toBe('test-path');
        done();
      });
    });
  });

  describe('getContentIndex', () => {
    it('should return stats from Holochain', (done) => {
      mockHolochainContent.getStats.and.returnValue(of({ total_count: 5, by_type: { concept: 3, exercise: 2 } }));

      service.getContentIndex().subscribe(index => {
        expect(index.totalCount).toBe(5);
        expect(index.byType).toEqual({ concept: 3, exercise: 2 });
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
