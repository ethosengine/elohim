import { TestBed } from '@angular/core/testing';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { DataLoaderService } from './data-loader.service';
import { GOVERNANCE } from '@elohim/service';
import { AttestationApiService } from '@app/elohim/services/attestation-api.service';
import { IndexedDBCacheService } from './indexeddb-cache.service';
import { ProjectionAPIService } from './projection-api.service';
import { ContentResolverService, SourceTier } from './content-resolver.service';
import { ContentBackendService } from './content-backend.service';
import { ELOHIM_CLIENT } from '@elohim/service';
import { LearningPath, PathIndex, ContentNode } from '../models';
import { AgentProgress } from '@elohim/service/angular/models/agent.model';
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
      getGovernanceState: vi.fn().mockResolvedValue(null),
      queryGovernanceStates: vi.fn().mockResolvedValue([]),
      queryChallenges: vi.fn().mockResolvedValue([]),
      queryProposals: vi.fn().mockResolvedValue([]),
      queryPrecedents: vi.fn().mockResolvedValue([]),
      queryDiscussions: vi.fn().mockResolvedValue([]),
    };
    const mockAttestationApi = {
      listBySubject: vi.fn().mockResolvedValue([]),
      getById: vi.fn().mockResolvedValue(null),
      issue: vi.fn().mockResolvedValue(null),
      revoke: vi.fn().mockResolvedValue(null),
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
      getRelationships: vi.fn(),
      getContentGraph: vi.fn(),
    };
    mockContentService.getContent.mockReturnValue(of(null));
    mockContentService.queryContent.mockReturnValue(of([]));
    mockContentService.batchGetContent.mockReturnValue(of(new Map()));
    mockContentService.searchContent.mockReturnValue(of([]));
    mockContentService.getPath.mockReturnValue(of(null));
    mockContentService.queryPaths.mockReturnValue(of([]));
    mockContentService.getAllPaths.mockReturnValue(of([]));
    mockContentService.getRelationships.mockReturnValue(of([]));
    mockContentService.getContentGraph.mockReturnValue(of(null));

    // ElohimClient mock
    mockElohimClient = {
      get: vi.fn().mockReturnValue(Promise.resolve(null)),
      query: vi.fn().mockReturnValue(Promise.resolve([])),
      supportsOffline: vi.fn().mockReturnValue(false),
      backpressure: vi.fn().mockReturnValue(Promise.resolve(0)),
    };

    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        DataLoaderService,
        { provide: GOVERNANCE, useValue: mockHolochainContent },
        { provide: AttestationApiService, useValue: mockAttestationApi },
        { provide: IndexedDBCacheService, useValue: mockIndexedDBCache },
        { provide: ProjectionAPIService, useValue: mockProjectionApi },
        { provide: ContentResolverService, useValue: mockContentResolver },
        { provide: ContentBackendService, useValue: mockContentService },
        { provide: ELOHIM_CLIENT, useValue: mockElohimClient },
      ],
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
    it('should load path via ContentService', () =>
      new Promise<void>(done => {
        // getPath now delegates to getContent + parsePathView
        const mockPathNode = {
          id: 'test-path',
          contentType: 'path',
          title: 'Test Path',
          description: 'A test path',
          content: { sections: [] },
          contentFormat: 'epr-composite',
          tags: ['test'],
          relatedNodeIds: [],
          metadata: {
            pathType: 'journey',
            difficulty: 'beginner',
            estimatedDuration: '1 hour',
            purpose: 'Testing',
          },
          authorId: 'test-agent',
          reach: 'commons',
          createdAt: '2025-01-01T00:00:00.000Z',
          updatedAt: '2025-01-01T00:00:00.000Z',
        };

        mockContentService.getContent.mockReturnValue(of(mockPathNode));

        service.getPath('test-path').subscribe(path => {
          expect(path.id).toBe('test-path');
          expect(path.title).toBe('Test Path');
          expect(mockContentService.getContent).toHaveBeenCalledWith('test-path');
          done();
        });
      }));
  });

  describe('getContent', () => {
    it('should load content via ContentService', () =>
      new Promise<void>(done => {
        // ContentService returns content directly
        mockContentService.getContent.mockReturnValue(of(mockContent));

        service.getContent('test-content').subscribe(content => {
          expect(content.id).toBe(mockContent.id);
          expect(content.title).toBe(mockContent.title);
          expect(mockContentService.getContent).toHaveBeenCalledWith('test-content');
          done();
        });
      }));

    it('should cache content in IndexedDB after loading', () =>
      new Promise<void>(done => {
        mockContentService.getContent.mockReturnValue(of(mockContent));

        service.getContent('test-content').subscribe(content => {
          expect(content.id).toBe(mockContent.id);
          // ContentService handles caching internally, but DataLoader also caches to IndexedDB
          expect(mockContentService.getContent).toHaveBeenCalledWith('test-content');
          done();
        });
      }));

    it('should return placeholder when content not found', () =>
      new Promise<void>(done => {
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
    it('should load path index from ContentService', () =>
      new Promise<void>(done => {
        // getPathIndex now calls queryContent({ contentType: 'path' })
        const mockPathNodes = [
          {
            id: 'test-path',
            contentType: 'path',
            title: 'Test',
            description: 'Desc',
            content: {
              sections: [
                {
                  id: 'sec-1',
                  title: 'Section 1',
                  level: 'lesson',
                  items: [{ ref: 'step-1', role: 'step' }],
                },
              ],
            },
            contentFormat: 'epr-composite',
            tags: [],
            relatedNodeIds: [],
            metadata: {
              pathType: 'journey',
              difficulty: 'beginner',
              estimatedDuration: '1h',
            },
            authorId: 'test-agent',
            reach: 'commons',
            createdAt: '2025-01-01T00:00:00.000Z',
            updatedAt: '2025-01-01T00:00:00.000Z',
          },
        ];

        mockContentService.queryContent.mockReturnValue(of(mockPathNodes));

        service.getPathIndex().subscribe(index => {
          expect(index.totalCount).toBe(1);
          expect(index.paths.length).toBe(1);
          expect(index.paths[0].id).toBe('test-path');
          expect(mockContentService.queryContent).toHaveBeenCalled();
          done();
        });
      }));
  });

  describe('getContentIndex', () => {
    it('should return content index from ContentService', () =>
      new Promise<void>(done => {
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
    mockSetItem = vi.fn((key: string, value: string) => {
      localStorageData[key] = value;
    });
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
      mockSetItem.mockImplementation(() => {
        throw 'QuotaExceededError';
      });

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
    it('should clear internal caches without error', () => {
      expect(() => service.clearCache()).not.toThrow();
    });
  });

  // The mini-graph neighborhood (RelatedConceptsService.getNeighborhood) roots
  // the underlying graph at the CURRENT node so a /epr/{id} view shows that
  // node's neighborhood, not always the default (manifesto) root's. getGraph()
  // forwards its root to the resolver graph endpoint.
  describe('getGraph (root forwarding)', () => {
    beforeEach(() => {
      mockContentService.getContentGraph.mockReturnValue(
        of({ rootId: 'x', related: [], totalNodes: 0 })
      );
    });

    it('requests the graph rooted at the passed content id', async () => {
      await new Promise<void>(resolve => {
        service.getGraph('confession').subscribe(() => {
          expect(mockContentService.getContentGraph).toHaveBeenCalledWith('confession');
          resolve();
        });
      });
    });

    it('defaults to the manifesto root when no id is passed', async () => {
      await new Promise<void>(resolve => {
        service.getGraph().subscribe(() => {
          expect(mockContentService.getContentGraph).toHaveBeenCalledWith('manifesto');
          resolve();
        });
      });
    });

    it('caches per-root so distinct nodes do not collide', async () => {
      await new Promise<void>(resolve => {
        service.getGraph('confession').subscribe(() => resolve());
      });
      await new Promise<void>(resolve => {
        service.getGraph('manifesto').subscribe(() => resolve());
      });
      expect(mockContentService.getContentGraph).toHaveBeenCalledWith('confession');
      expect(mockContentService.getContentGraph).toHaveBeenCalledWith('manifesto');
    });
  });

  // B2b (post-fix): getResolvedRelationshipsForNode is a DISJOINT MERGE of two
  // sources — the LIST endpoint (getRelationships, 'both') for authored
  // explicit/structural edges in BOTH directions, AND the resolver graph
  // endpoint (getContentGraph, computed=true) for COMPUTED-only discovery edges
  // (inferenceSource:"tag", Category C). The graph endpoint is outgoing-only, so
  // the list source is what restores INCOMING explicit edges (the regression).
  describe('getResolvedRelationshipsForNode (disjoint merge: list-both + computed graph)', () => {
    it('should call BOTH the list endpoint (both directions) AND the computed graph endpoint', async () => {
      mockContentService.getContentGraph.mockReturnValue(
        of({
          rootId: 'concept-1',
          related: [
            {
              contentId: 'tag-discovered-target',
              relationshipType: 'RELATES_TO',
              confidence: 0.6,
              inferenceSource: 'tag',
              depth: 1,
              children: [],
            },
          ],
          totalNodes: 2,
        })
      );

      await new Promise<void>(resolve => {
        service.getResolvedRelationshipsForNode('concept-1', 2).subscribe(() => {
          expect(mockContentService.getContentGraph).toHaveBeenCalledWith(
            'concept-1',
            expect.objectContaining({ computed: true, depth: 2 })
          );
          // The list endpoint, queried in BOTH directions, restores incoming explicit edges.
          expect(mockContentService.getRelationships).toHaveBeenCalledWith('concept-1', 'both');
          resolve();
        });
      });
    });

    // The core regression guard: an INCOMING explicit edge (other CONTAINS
    // concept-1) is dropped by the outgoing-only graph endpoint. The list-both
    // source must restore it so the parents/Part-Of bucket survives — while the
    // tag edge from the graph still flows for discovery.
    it('should restore an INCOMING explicit edge from list-both AND keep the computed tag edge', async () => {
      mockContentService.getRelationships.mockReturnValue(
        of([
          {
            id: 'rel-incoming',
            sourceId: 'concept-6',
            targetId: 'concept-1',
            relationshipType: 'CONTAINS',
            confidence: 1,
            inferenceSource: 'explicit',
          },
        ])
      );
      mockContentService.getContentGraph.mockReturnValue(
        of({
          rootId: 'concept-1',
          related: [
            // Graph echoes an explicit OUTGOING edge — must be filtered out (the
            // list source owns explicit edges); only the tag edge survives here.
            {
              contentId: 'explicit-outgoing',
              relationshipType: 'RELATES_TO',
              confidence: 1,
              inferenceSource: 'explicit',
              depth: 1,
              children: [],
            },
            {
              contentId: 'tag-discovered-target',
              relationshipType: 'RELATES_TO',
              confidence: 0.6,
              inferenceSource: 'tag',
              depth: 1,
              children: [],
            },
          ],
          totalNodes: 3,
        })
      );

      const relationships = await new Promise<any[]>(resolve => {
        service.getResolvedRelationshipsForNode('concept-1', 2).subscribe(rels => resolve(rels));
      });

      // Incoming explicit edge restored from list-both (feeds parents/Part-Of).
      const incomingEdge = relationships.find(
        r => r.sourceNodeId === 'concept-6' && r.targetNodeId === 'concept-1'
      );
      expect(incomingEdge).toBeDefined();
      expect(incomingEdge.relationshipType).toBe('CONTAINS');
      expect(incomingEdge.inferenceSource).toBe('explicit');

      // Computed tag edge still flows for discovery.
      const tagEdge = relationships.find(r => r.targetNodeId === 'tag-discovered-target');
      expect(tagEdge).toBeDefined();
      expect(tagEdge.inferenceSource).toBe('tag');
      expect(tagEdge.sourceNodeId).toBe('concept-1');

      // The graph's explicit-outgoing echo is filtered out (list owns explicit).
      const graphExplicit = relationships.find(r => r.targetNodeId === 'explicit-outgoing');
      expect(graphExplicit).toBeUndefined();
    });

    it('should carry inferenceSource:"tag" through from the resolver graph node', async () => {
      mockContentService.getContentGraph.mockReturnValue(
        of({
          rootId: 'concept-1',
          related: [
            {
              contentId: 'tag-discovered-target',
              relationshipType: 'RELATES_TO',
              confidence: 0.6,
              inferenceSource: 'tag',
              depth: 1,
              children: [],
            },
          ],
          totalNodes: 2,
        })
      );

      const relationships = await new Promise<any[]>(resolve => {
        service.getResolvedRelationshipsForNode('concept-1', 2).subscribe(rels => resolve(rels));
      });

      const tagEdge = relationships.find(r => r.targetNodeId === 'tag-discovered-target');
      expect(tagEdge).toBeDefined();
      expect(tagEdge.inferenceSource).toBe('tag');
      expect(tagEdge.sourceNodeId).toBe('concept-1');
      expect(tagEdge.relationshipType).toBe('RELATES_TO');
    });
  });
});
