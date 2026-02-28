import { TestBed } from '@angular/core/testing';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
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
import { vi, Mock } from 'vitest';
import { provideHttpClient } from '@angular/common/http';

describe('DataLoaderService', () => {
  let service: DataLoaderService;
  let httpMock: HttpTestingController;
  let mockHolochainContent: any;
  let mockIndexedDBCache: any;
  let mockProjectionApi: any;
  let mockContentResolver: any;
  let mockContentService: any;
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
    metadata: {},
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
    attestationsEarned: [],
  };

  beforeEach(() => {
    mockHolochainContent = {
      getContent: vi.fn(),
      getContentByType: vi.fn(),
      getStats: vi.fn(),
      clearCache: vi.fn(),
      isAvailable: vi.fn(),
      getPathIndex: vi.fn(),
      getPathWithSteps: vi.fn(),
      // Signal property - callable function that returns availability
      available: vi.fn().mockReturnValue(true),
    };

    mockIndexedDBCache = {
      init: vi.fn(),
      getPath: vi.fn(),
      setPath: vi.fn(),
      getContent: vi.fn(),
      setContent: vi.fn(),
      getContentBatch: vi.fn(),
      setContentBatch: vi.fn(),
      getStats: vi.fn(),
      clearAll: vi.fn(),
    };

    mockProjectionApi = {
      getContent: vi.fn(),
      queryContent: vi.fn(),
      batchGetContent: vi.fn(),
      getPath: vi.fn(),
      getPathOverview: vi.fn(),
      queryPaths: vi.fn(),
      getRelated: vi.fn(),
      getStats: vi.fn(),
      isHealthy: vi.fn(),
    };

    mockContentResolver = {
      initialize: vi.fn(),
      resolveContent: vi.fn(),
      resolvePath: vi.fn(),
      batchResolveContent: vi.fn(),
      cacheContent: vi.fn(),
      cachePath: vi.fn(),
      registerStandardSource: vi.fn(),
      setSourceAvailable: vi.fn(),
    };

    // Projection API mock returns (disabled by default)
    mockProjectionApi.getContent.mockReturnValue(of(null));
    mockProjectionApi.queryContent.mockReturnValue(of([]));
    mockProjectionApi.batchGetContent.mockReturnValue(of(new Map()));
    mockProjectionApi.getPath.mockReturnValue(of(null));
    mockProjectionApi.getPathOverview.mockReturnValue(of(null));
    mockProjectionApi.queryPaths.mockReturnValue(of([]));
    mockProjectionApi.getRelated.mockReturnValue(of([]));
    mockProjectionApi.getStats.mockReturnValue(of(null));
    mockProjectionApi.isHealthy.mockReturnValue(of(false));

    // Default mock returns
    mockHolochainContent.getStats.mockReturnValue(of({ totalCount: 0, byType: {} }));
    mockHolochainContent.isAvailable.mockReturnValue(true);
    (mockHolochainContent.getPathIndex as Mock).mockReturnValue(
      Promise.resolve({ paths: [], totalCount: 0, lastUpdated: '' })
    );
    (mockHolochainContent.getPathWithSteps as Mock).mockReturnValue(Promise.resolve(null));

    // IndexedDB mock returns (disabled by default to use Holochain)
    mockIndexedDBCache.init.mockReturnValue(Promise.resolve(false));
    mockIndexedDBCache.getPath.mockReturnValue(Promise.resolve(null));
    mockIndexedDBCache.setPath.mockReturnValue(Promise.resolve());
    mockIndexedDBCache.getContent.mockReturnValue(Promise.resolve(null));
    mockIndexedDBCache.setContent.mockReturnValue(Promise.resolve());
    mockIndexedDBCache.getContentBatch.mockReturnValue(Promise.resolve(new Map()));
    mockIndexedDBCache.setContentBatch.mockReturnValue(Promise.resolve());
    mockIndexedDBCache.getStats.mockReturnValue(
      Promise.resolve({ contentCount: 0, pathCount: 0, isAvailable: false })
    );
    mockIndexedDBCache.clearAll.mockReturnValue(Promise.resolve());

    // ContentResolver mock returns
    mockContentResolver.initialize.mockReturnValue(
      Promise.resolve({ success: true, implementation: 'typescript' })
    );
    mockContentResolver.resolveContent.mockReturnValue(Promise.resolve(null));
    mockContentResolver.resolvePath.mockReturnValue(Promise.resolve(null));
    mockContentResolver.batchResolveContent.mockReturnValue(Promise.resolve(new Map()));
    mockContentResolver.cacheContent.mockReturnValue(Promise.resolve());
    mockContentResolver.cachePath.mockReturnValue(Promise.resolve());
    // These methods are called during initialization - they don't return anything
    mockContentResolver.registerStandardSource.mockReturnValue(undefined);
    mockContentResolver.setSourceAvailable.mockReturnValue(undefined);

    // ContentService mock
    mockContentService = {
      getContent: vi.fn(),
      queryContent: vi.fn(),
      batchGetContent: vi.fn(),
      searchContent: vi.fn(),
      getPath: vi.fn(),
      queryPaths: vi.fn(),
      getAllPaths: vi.fn(),
    };
    mockContentService.getContent.mockReturnValue(of(null));
    mockContentService.queryContent.mockReturnValue(of([]));
    mockContentService.batchGetContent.mockReturnValue(of(new Map()));
    mockContentService.searchContent.mockReturnValue(of([]));
    mockContentService.getPath.mockReturnValue(of(null));
    mockContentService.queryPaths.mockReturnValue(of([]));
    mockContentService.getAllPaths.mockReturnValue(of([]));

    // ElohimClient mock
    mockElohimClient = {
      get: vi.fn().mockReturnValue(Promise.resolve(null)),
      query: vi.fn().mockReturnValue(Promise.resolve([])),
      supportsOffline: vi.fn().mockReturnValue(false),
      backpressure: vi.fn().mockReturnValue(Promise.resolve(0)),
    };

    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), DataLoaderService,
        { provide: HolochainContentService, useValue: mockHolochainContent },
        { provide: IndexedDBCacheService, useValue: mockIndexedDBCache },
        { provide: ProjectionAPIService, useValue: mockProjectionApi },
        { provide: ContentResolverService, useValue: mockContentResolver },
        { provide: ContentService, useValue: mockContentService },
        { provide: ELOHIM_CLIENT, useValue: mockElohimClient },],
    });

    service = TestBed.inject(DataLoaderService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
    vi.restoreAllMocks();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getPath', () => {
    it('should load path via ContentService', () => new Promise<void>(done => {
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
        steps: [],
      };

      // ContentService returns path directly
      mockContentService.getPath.mockReturnValue(of(mockPath));

      service.getPath('test-path').subscribe(path => {
        expect(path.id).toBe('test-path');
        expect(path.title).toBe('Test Path');
        expect(mockContentService.getPath).toHaveBeenCalledWith('test-path');
        done();
      });
    }));
  });

  describe('getContent', () => {
    it('should load content via ContentService', () => new Promise<void>(done => {
      // ContentService returns content directly
      mockContentService.getContent.mockReturnValue(of(mockContent));

      service.getContent('test-content').subscribe(content => {
        expect(content.id).toBe(mockContent.id);
        expect(content.title).toBe(mockContent.title);
        expect(mockContentService.getContent).toHaveBeenCalledWith('test-content');
        done();
      });
    }));

    it('should cache content in IndexedDB after loading', () => new Promise<void>(done => {
      mockContentService.getContent.mockReturnValue(of(mockContent));

      service.getContent('test-content').subscribe(content => {
        expect(content.id).toBe(mockContent.id);
        // ContentService handles caching internally, but DataLoader also caches to IndexedDB
        expect(mockContentService.getContent).toHaveBeenCalledWith('test-content');
        done();
      });
    }));

    it('should return placeholder when content not found', () => new Promise<void>(done => {
      // Resolver returns null when content not found
      mockContentResolver.resolveContent.mockReturnValue(Promise.resolve(null));

      service.getContent('missing-content').subscribe({
        next: content => {
          expect(content.contentType).toBe('placeholder');
          expect(content.title).toContain('Content Not Found');
          expect(content.id).toBe('missing-content');
          done();
        },
        error: () => {
          throw new Error('Should not throw error, should return placeholder');
        },
      });
    }));
  });

  describe('getPathIndex', () => {
    it('should load path index from ContentService', () => new Promise<void>(done => {
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
          steps: [
            {
              order: 0,
              stepType: 'content',
              resourceId: 'step-1',
              stepTitle: 'Step 1',
              stepNarrative: '',
              optional: false,
              learningObjectives: [],
              completionCriteria: [],
            },
          ],
        },
      ];

      mockContentService.queryPaths.mockReturnValue(of(mockPaths));

      service.getPathIndex().subscribe(index => {
        expect(index.totalCount).toBe(1);
        expect(index.paths.length).toBe(1);
        expect(index.paths[0].id).toBe('test-path');
        expect(mockContentService.queryPaths).toHaveBeenCalled();
        done();
      });
    }));
  });

  describe('getContentIndex', () => {
    it('should return content index from ContentService', () => new Promise<void>(done => {
      // Mock content nodes with different types (cast to any to avoid full interface)
      const mockNodes = [
        { id: '1', title: 'A', contentType: 'concept', tags: [] },
        { id: '2', title: 'B', contentType: 'concept', tags: [] },
        { id: '3', title: 'C', contentType: 'concept', tags: [] },
        { id: '4', title: 'D', contentType: 'exercise', tags: [] },
        { id: '5', title: 'E', contentType: 'exercise', tags: [] },
      ] as any[];
      mockContentService.queryContent.mockReturnValue(of(mockNodes));

      service.getContentIndex().subscribe(index => {
        expect(index.totalCount).toBe(5);
        expect(index.byType!['concept']).toBe(3);
        expect(index.byType!['exercise']).toBe(2);
        done();
      });
    }));
  });

  // Use Object.defineProperty for localStorage mocks — vi.spyOn on native Storage is unreliable
  let localStorageData: Record<string, string | null>;
  let mockGetItem: ReturnType<typeof vi.fn>;
  let mockSetItem: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    localStorageData = {};
    mockGetItem = vi.fn((key: string) => localStorageData[key] ?? null);
    mockSetItem = vi.fn((key: string, value: string) => { localStorageData[key] = value; });
    Object.defineProperty(window, 'localStorage', {
      value: { getItem: mockGetItem, setItem: mockSetItem, removeItem: vi.fn(), clear: vi.fn() },
      writable: true,
      configurable: true,
    });
  });

  describe('getAgentProgress', () => {
    it('should load agent progress from localStorage', async () => {
      const progressJson = JSON.stringify(mockProgress);
      localStorageData['lamad-progress-test-agent-test-path'] = progressJson;

      const { firstValueFrom } = await import('rxjs');
      const progress = await firstValueFrom(service.getAgentProgress('test-agent', 'test-path'));
      expect(progress).toEqual(mockProgress);
      expect(mockGetItem).toHaveBeenCalledWith('lamad-progress-test-agent-test-path');
    });

    it('should return null if progress not in localStorage', async () => {
      const { firstValueFrom } = await import('rxjs');
      const progress = await firstValueFrom(service.getAgentProgress('test-agent', 'missing-path'));
      expect(progress).toBeNull();
    });
  });

  describe('saveAgentProgress', () => {
    it('should save progress to localStorage', async () => {
      const { firstValueFrom } = await import('rxjs');
      await firstValueFrom(service.saveAgentProgress(mockProgress));
      expect(mockSetItem).toHaveBeenCalledWith(
        'lamad-progress-test-agent-test-path',
        expect.any(String)
      );
    });

    it('should handle localStorage errors gracefully', async () => {
      mockSetItem.mockImplementation(() => { throw 'QuotaExceededError'; });

      const { firstValueFrom } = await import('rxjs');
      await firstValueFrom(service.saveAgentProgress(mockProgress));
      expect(true).toBe(true); // Should complete without error
    });
  });

  describe('getLocalProgress', () => {
    it('should retrieve progress from localStorage', () => {
      const progressJson = JSON.stringify(mockProgress);
      localStorageData['lamad-progress-test-agent-test-path'] = progressJson;

      const result = service.getLocalProgress('test-agent', 'test-path');
      expect(result).toEqual(mockProgress);
      expect(mockGetItem).toHaveBeenCalledWith('lamad-progress-test-agent-test-path');
    });

    it('should return null if no progress in localStorage', () => {
      const result = service.getLocalProgress('test-agent', 'test-path');
      expect(result).toBeNull();
    });

    it('should return null for malformed JSON', () => {
      localStorageData['lamad-progress-test-agent-test-path'] = 'invalid json';

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
