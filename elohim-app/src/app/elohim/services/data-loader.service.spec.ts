import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { of } from 'rxjs';

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
    const childLoggerSpy = jasmine.createSpyObj('Logger', ['debug', 'warn', 'error']);
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
});
