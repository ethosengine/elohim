import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { of, throwError } from 'rxjs';

import { ContentNode } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';
import { AgentProgress } from '../models/agent.model';
import { DataLoaderService } from './data-loader.service';
import { HolochainContentService } from './holochain-content.service';
import { IndexedDBCacheService } from './indexeddb-cache.service';
import { ProjectionAPIService } from './projection-api.service';
import { ContentResolverService } from './content-resolver.service';
import { ContentService } from './content.service';
import { LoggerService } from './logger.service';

describe('DataLoaderService', () => {
  let service: DataLoaderService;
  let holochainMock: jasmine.SpyObj<HolochainContentService>;
  let idbMock: jasmine.SpyObj<IndexedDBCacheService>;
  let projectionApiMock: jasmine.SpyObj<ProjectionAPIService>;
  let contentResolverMock: jasmine.SpyObj<ContentResolverService>;
  let contentServiceMock: jasmine.SpyObj<ContentService>;
  let loggerMock: jasmine.SpyObj<LoggerService>;

  beforeEach(() => {
    const holochainSpy = jasmine.createSpyObj('HolochainContentService', [
      'isAvailable',
      'clearCache',
      'prefetchRelatedContent',
    ]);
    const idbSpy = jasmine.createSpyObj('IndexedDBCacheService', [
      'init',
      'getStats',
      'setPath',
      'setContent',
      'setContentBatch',
      'clearAll',
      'getContent',
      'getPath',
      'getContentBatch',
      'getMetadata',
      'setMetadata',
    ]);
    const projectionApiSpy = jasmine.createSpyObj(
      'ProjectionAPIService',
      ['getPathOverview'],
      { enabled: false }
    );
    const contentResolverSpy = jasmine.createSpyObj('ContentResolverService', [
      'initialize',
      'registerStandardSource',
      'setSourceAvailable',
    ]);
    const contentServiceSpy = jasmine.createSpyObj('ContentService', [
      'getPath',
      'getContent',
      'batchGetContent',
      'queryContent',
      'queryPaths',
    ]);
    const loggerSpy = jasmine.createSpyObj('LoggerService', ['createChild']);

    // Setup default return values
    holochainSpy.isAvailable.and.returnValue(false);
    idbSpy.init.and.returnValue(Promise.resolve(false));
    idbSpy.getStats.and.returnValue(
      Promise.resolve({ contentCount: 0, pathCount: 0, isAvailable: false })
    );
    idbSpy.setPath.and.returnValue(Promise.resolve());
    idbSpy.setContent.and.returnValue(Promise.resolve());
    idbSpy.setContentBatch.and.returnValue(Promise.resolve());
    idbSpy.clearAll.and.returnValue(Promise.resolve());
    idbSpy.getContent.and.returnValue(Promise.resolve(null));
    idbSpy.getPath.and.returnValue(Promise.resolve(null));
    idbSpy.getContentBatch.and.returnValue(Promise.resolve(new Map()));
    idbSpy.getMetadata.and.returnValue(Promise.resolve(null));
    idbSpy.setMetadata.and.returnValue(Promise.resolve());

    contentResolverSpy.initialize.and.returnValue(
      Promise.resolve({ success: true, implementation: 'typescript' })
    );
    contentResolverSpy.registerStandardSource.and.returnValue(undefined);
    contentResolverSpy.setSourceAvailable.and.returnValue(undefined);

    contentServiceSpy.getPath.and.returnValue(of(null));
    contentServiceSpy.getContent.and.returnValue(of(null));
    contentServiceSpy.batchGetContent.and.returnValue(of(new Map()));
    contentServiceSpy.queryContent.and.returnValue(of([]));
    contentServiceSpy.queryPaths.and.returnValue(of([]));

    // Logger mock returns itself for createChild
    const childLoggerSpy = jasmine.createSpyObj('Logger', ['debug', 'info', 'warn', 'error']);
    loggerSpy.createChild.and.returnValue(childLoggerSpy);

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        DataLoaderService,
        { provide: HolochainContentService, useValue: holochainSpy },
        { provide: IndexedDBCacheService, useValue: idbSpy },
        { provide: ProjectionAPIService, useValue: projectionApiSpy },
        { provide: ContentResolverService, useValue: contentResolverSpy },
        { provide: ContentService, useValue: contentServiceSpy },
        { provide: LoggerService, useValue: loggerSpy },
      ],
    });

    service = TestBed.inject(DataLoaderService);
    holochainMock = TestBed.inject(HolochainContentService) as jasmine.SpyObj<HolochainContentService>;
    idbMock = TestBed.inject(IndexedDBCacheService) as jasmine.SpyObj<IndexedDBCacheService>;
    projectionApiMock = TestBed.inject(ProjectionAPIService) as jasmine.SpyObj<ProjectionAPIService>;
    contentResolverMock = TestBed.inject(
      ContentResolverService
    ) as jasmine.SpyObj<ContentResolverService>;
    contentServiceMock = TestBed.inject(ContentService) as jasmine.SpyObj<ContentService>;
    loggerMock = TestBed.inject(LoggerService) as jasmine.SpyObj<LoggerService>;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have getPath method', () => {
    expect(service.getPath).toBeDefined();
    expect(typeof service.getPath).toBe('function');
  });

  it('should have getContent method', () => {
    expect(service.getContent).toBeDefined();
    expect(typeof service.getContent).toBe('function');
  });

  it('should have batchGetContent method', () => {
    expect(service.batchGetContent).toBeDefined();
    expect(typeof service.batchGetContent).toBe('function');
  });

  it('should have prefetchContent method', () => {
    expect(service.prefetchContent).toBeDefined();
    expect(typeof service.prefetchContent).toBe('function');
  });

  it('should have getPathWithPrefetch method', () => {
    expect(service.getPathWithPrefetch).toBeDefined();
    expect(typeof service.getPathWithPrefetch).toBe('function');
  });

  it('should have checkReadiness method', () => {
    expect(service.checkReadiness).toBeDefined();
    expect(typeof service.checkReadiness).toBe('function');
  });

  it('should have getContentIndex method', () => {
    expect(service.getContentIndex).toBeDefined();
    expect(typeof service.getContentIndex).toBe('function');
  });

  it('should have getPathIndex method', () => {
    expect(service.getPathIndex).toBeDefined();
    expect(typeof service.getPathIndex).toBe('function');
  });

  it('should have getAgent method', () => {
    expect(service.getAgent).toBeDefined();
    expect(typeof service.getAgent).toBe('function');
  });

  it('should have getAttestations method', () => {
    expect(service.getAttestations).toBeDefined();
    expect(typeof service.getAttestations).toBe('function');
  });

  it('should have getGraph method', () => {
    expect(service.getGraph).toBeDefined();
    expect(typeof service.getGraph).toBe('function');
  });

  it('should have clearCache method', () => {
    expect(service.clearCache).toBeDefined();
    expect(typeof service.clearCache).toBe('function');
  });

  it('should have getCacheStats method', () => {
    expect(service.getCacheStats).toBeDefined();
    expect(typeof service.getCacheStats).toBe('function');
  });

  // ==========================================================================
  // Offline-First Read Fallback
  // ==========================================================================

  describe('offline-first read fallback', () => {
    const mockContent: ContentNode = {
      id: 'test-content-1',
      contentType: 'concept',
      title: 'Cached Content',
      description: 'From IDB cache',
      content: '# Cached',
      contentFormat: 'markdown',
      tags: ['test'],
      relatedNodeIds: [],
      metadata: {},
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
    };

    const mockPath: LearningPath = {
      id: 'test-path-1',
      version: '1.0.0',
      title: 'Cached Path',
      description: 'From IDB cache',
      purpose: '',
      difficulty: 'beginner',
      estimatedDuration: '1h',
      tags: ['test'],
      steps: [],
      createdBy: 'agent-1',
      contributors: [],
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
      visibility: 'public',
    };

    beforeEach(() => {
      // Enable IDB for these tests
      (service as any).idbInitialized = true;
    });

    describe('getContent', () => {
      it('should fall back to IDB cache when network fails', fakeAsync(() => {
        contentServiceMock.getContent.and.returnValue(
          throwError(() => new Error('Network error'))
        );
        idbMock.getContent.and.returnValue(Promise.resolve(mockContent));

        let result: ContentNode | undefined;
        service.getContent('test-content-1').subscribe(content => {
          result = content;
        });
        tick(); // flush Promise microtask

        expect(result?.id).toBe('test-content-1');
        expect(result?.title).toBe('Cached Content');
        expect(idbMock.getContent).toHaveBeenCalledWith('test-content-1');
      }));

      it('should return placeholder when both network and IDB fail', fakeAsync(() => {
        contentServiceMock.getContent.and.returnValue(
          throwError(() => new Error('Network error'))
        );
        idbMock.getContent.and.returnValue(Promise.resolve(null));

        let result: ContentNode | undefined;
        service.getContent('missing-id').subscribe(content => {
          result = content;
        });
        tick();

        expect(result?.contentType).toBe('placeholder');
      }));

      it('should return placeholder when IDB not initialized', fakeAsync(() => {
        (service as any).idbInitialized = false;
        contentServiceMock.getContent.and.returnValue(
          throwError(() => new Error('Network error'))
        );

        let result: ContentNode | undefined;
        service.getContent('test-id').subscribe(content => {
          result = content;
        });
        tick();

        expect(result?.contentType).toBe('placeholder');
        expect(idbMock.getContent).not.toHaveBeenCalled();
      }));

      it('should return placeholder when IDB itself throws', fakeAsync(() => {
        contentServiceMock.getContent.and.returnValue(
          throwError(() => new Error('Network error'))
        );
        idbMock.getContent.and.returnValue(Promise.reject(new Error('IDB corrupt')));

        let result: ContentNode | undefined;
        service.getContent('test-id').subscribe(content => {
          result = content;
        });
        tick();

        expect(result?.contentType).toBe('placeholder');
      }));
    });

    describe('getPath', () => {
      it('should fall back to IDB cache when network fails', fakeAsync(() => {
        contentServiceMock.getPath.and.returnValue(
          throwError(() => new Error('Network timeout'))
        );
        idbMock.getPath.and.returnValue(Promise.resolve(mockPath));

        let result: LearningPath | undefined;
        service.getPath('test-path-1').subscribe(path => {
          result = path;
        });
        tick();

        expect(result?.id).toBe('test-path-1');
        expect(result?.title).toBe('Cached Path');
        expect(idbMock.getPath).toHaveBeenCalledWith('test-path-1');
      }));

      it('should re-throw when path not found (404, not connectivity)', fakeAsync(() => {
        contentServiceMock.getPath.and.returnValue(of(null));

        let error: Error | undefined;
        service.getPath('missing-path').subscribe({
          error: err => {
            error = err;
          },
        });
        tick();

        expect(error?.message).toContain('Path not found');
        expect(idbMock.getPath).not.toHaveBeenCalled();
      }));

      it('should re-throw when IDB also misses', fakeAsync(() => {
        contentServiceMock.getPath.and.returnValue(
          throwError(() => new Error('Network timeout'))
        );
        idbMock.getPath.and.returnValue(Promise.resolve(null));

        let error: Error | undefined;
        service.getPath('uncached-path').subscribe({
          error: (err: Error) => {
            error = err;
          },
        });
        tick();

        expect(error).toBeDefined();
        expect(idbMock.getPath).toHaveBeenCalledWith('uncached-path');
      }));

      it('should not attempt IDB when IDB not initialized', fakeAsync(() => {
        (service as any).idbInitialized = false;
        contentServiceMock.getPath.and.returnValue(
          throwError(() => new Error('Network timeout'))
        );

        let error: Error | undefined;
        service.getPath('test-path').subscribe({
          error: (err: Error) => {
            error = err;
          },
        });
        tick();

        expect(error).toBeDefined();
        expect(idbMock.getPath).not.toHaveBeenCalled();
      }));
    });

    describe('batchGetContent', () => {
      it('should fall back to IDB cache when network fails', fakeAsync(() => {
        contentServiceMock.batchGetContent.and.returnValue(
          throwError(() => new Error('Network error'))
        );
        const cachedMap = new Map<string, ContentNode>([['id-a', mockContent]]);
        idbMock.getContentBatch.and.returnValue(Promise.resolve(cachedMap));

        let result: Map<string, ContentNode> | undefined;
        service.batchGetContent(['id-a', 'id-b']).subscribe(r => {
          result = r;
        });
        tick();

        expect(result?.get('id-a')?.title).toBe('Cached Content');
        expect(result?.get('id-b')?.contentType).toBe('placeholder');
      }));

      it('should fill placeholders for IDB cache misses', fakeAsync(() => {
        contentServiceMock.batchGetContent.and.returnValue(
          throwError(() => new Error('Network error'))
        );
        idbMock.getContentBatch.and.returnValue(Promise.resolve(new Map()));

        let result: Map<string, ContentNode> | undefined;
        service.batchGetContent(['id-x', 'id-y']).subscribe(r => {
          result = r;
        });
        tick();

        expect(result?.get('id-x')?.contentType).toBe('placeholder');
        expect(result?.get('id-y')?.contentType).toBe('placeholder');
        expect(result?.size).toBe(2);
      }));

      it('should return all placeholders when IDB not initialized', fakeAsync(() => {
        (service as any).idbInitialized = false;
        contentServiceMock.batchGetContent.and.returnValue(
          throwError(() => new Error('Network error'))
        );

        let result: Map<string, ContentNode> | undefined;
        service.batchGetContent(['id-1', 'id-2']).subscribe(r => {
          result = r;
        });
        tick();

        expect(result?.size).toBe(2);
        expect(result?.get('id-1')?.contentType).toBe('placeholder');
        expect(idbMock.getContentBatch).not.toHaveBeenCalled();
      }));
    });
  });

  // ==========================================================================
  // Agent Progress: localStorage → IndexedDB Migration
  // ==========================================================================

  describe('agent progress IDB migration', () => {
    const mockProgress: AgentProgress = {
      agentId: 'agent-1',
      pathId: 'path-1',
      currentStepIndex: 3,
      completedStepIndices: [0, 1, 2],
      startedAt: '2026-01-01T00:00:00Z',
      lastActivityAt: '2026-02-01T00:00:00Z',
      stepAffinity: { 0: 0.8, 1: 0.6, 2: 0.9 },
      stepNotes: {},
      reflectionResponses: {},
      attestationsEarned: [],
    };

    beforeEach(() => {
      (service as any).idbInitialized = true;
    });

    afterEach(() => {
      // Clean up localStorage entries from tests
      localStorage.removeItem('lamad-progress-agent-1-path-1');
    });

    describe('getAgentProgress', () => {
      it('should read from IDB when available', fakeAsync(() => {
        idbMock.getMetadata.and.returnValue(Promise.resolve(mockProgress));

        let result: AgentProgress | null | undefined;
        service.getAgentProgress('agent-1', 'path-1').subscribe(p => {
          result = p;
        });
        tick();

        expect(result?.currentStepIndex).toBe(3);
        expect(idbMock.getMetadata).toHaveBeenCalledWith('progress-agent-1-path-1');
      }));

      it('should fall back to localStorage when IDB misses', fakeAsync(() => {
        idbMock.getMetadata.and.returnValue(Promise.resolve(null));
        localStorage.setItem(
          'lamad-progress-agent-1-path-1',
          JSON.stringify(mockProgress)
        );

        let result: AgentProgress | null | undefined;
        service.getAgentProgress('agent-1', 'path-1').subscribe(p => {
          result = p;
        });
        tick();

        expect(result?.currentStepIndex).toBe(3);
        // Should trigger migration to IDB
        expect(idbMock.setMetadata).toHaveBeenCalledWith(
          'progress-agent-1-path-1',
          jasmine.objectContaining({ agentId: 'agent-1' })
        );
      }));

      it('should return null when neither IDB nor localStorage has data', fakeAsync(() => {
        idbMock.getMetadata.and.returnValue(Promise.resolve(null));

        let result: AgentProgress | null | undefined;
        service.getAgentProgress('agent-1', 'path-1').subscribe(p => {
          result = p;
        });
        tick();

        expect(result).toBeNull();
      }));

      it('should use localStorage directly when IDB not initialized', () => {
        (service as any).idbInitialized = false;
        localStorage.setItem(
          'lamad-progress-agent-1-path-1',
          JSON.stringify(mockProgress)
        );

        let result: AgentProgress | null | undefined;
        service.getAgentProgress('agent-1', 'path-1').subscribe(p => {
          result = p;
        });

        expect(result?.currentStepIndex).toBe(3);
        expect(idbMock.getMetadata).not.toHaveBeenCalled();
      });

      it('should fall back to localStorage when IDB throws', fakeAsync(() => {
        idbMock.getMetadata.and.returnValue(
          Promise.reject(new Error('IDB error'))
        );
        localStorage.setItem(
          'lamad-progress-agent-1-path-1',
          JSON.stringify(mockProgress)
        );

        let result: AgentProgress | null | undefined;
        service.getAgentProgress('agent-1', 'path-1').subscribe(p => {
          result = p;
        });
        tick();

        expect(result?.currentStepIndex).toBe(3);
      }));
    });

    describe('saveAgentProgress', () => {
      it('should dual-write to localStorage and IDB', () => {
        service.saveAgentProgress(mockProgress).subscribe();

        const lsData = localStorage.getItem('lamad-progress-agent-1-path-1');
        expect(lsData).toBeTruthy();
        expect(JSON.parse(lsData!).currentStepIndex).toBe(3);

        expect(idbMock.setMetadata).toHaveBeenCalledWith(
          'progress-agent-1-path-1',
          mockProgress
        );
      });

      it('should only write localStorage when IDB not initialized', () => {
        (service as any).idbInitialized = false;

        service.saveAgentProgress(mockProgress).subscribe();

        const lsData = localStorage.getItem('lamad-progress-agent-1-path-1');
        expect(lsData).toBeTruthy();
        expect(idbMock.setMetadata).not.toHaveBeenCalled();
      });
    });
  });
});
