import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { ContentService } from './content.service';
import { DataLoaderService } from './data-loader.service';
import { AgentService } from './agent.service';
import { ContentNode } from '../models/content-node.model';
import { LearningPath } from '../models/learning-path.model';

describe('ContentService', () => {
  let service: ContentService;
  let dataLoaderMock: jasmine.SpyObj<DataLoaderService>;
  let agentServiceMock: jasmine.SpyObj<AgentService>;

  const mockContentNode: ContentNode = {
    id: 'test-node',
    title: 'Test Content',
    description: 'A test content node',
    contentType: 'epic',
    contentFormat: 'markdown',
    content: '# Test Content',
    tags: ['test'],
    relatedNodeIds: ['related-1', 'related-2'],
    metadata: {},
    reach: 'commons'
  };

  const mockPath: LearningPath = {
    id: 'test-path',
    title: 'Test Path',
    description: 'A test learning path',
    steps: [
      { nodeId: 'test-node', stepIndex: 0, narrative: 'First step' }
    ],
    metadata: {}
  };

  beforeEach(() => {
    dataLoaderMock = jasmine.createSpyObj('DataLoaderService', [
      'getContent',
      'getPaths',
      'getContentIndex'
    ]);
    agentServiceMock = jasmine.createSpyObj('AgentService', [
      'getCurrentAgent',
      'getAgentReach'
    ]);

    dataLoaderMock.getContent.and.returnValue(of(mockContentNode));
    dataLoaderMock.getPaths.and.returnValue(of([mockPath]));
    dataLoaderMock.getContentIndex.and.returnValue(of({ nodes: [mockContentNode] }));
    agentServiceMock.getCurrentAgent.and.returnValue(of(null));
    agentServiceMock.getAgentReach.and.returnValue(of('commons'));

    TestBed.configureTestingModule({
      providers: [
        ContentService,
        { provide: DataLoaderService, useValue: dataLoaderMock },
        { provide: AgentService, useValue: agentServiceMock }
      ]
    });
    service = TestBed.inject(ContentService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getContent', () => {
    it('should return content by ID', (done) => {
      service.getContent('test-node').subscribe(content => {
        expect(content).toBeTruthy();
        expect(content.id).toBe('test-node');
        expect(content.title).toBe('Test Content');
        done();
      });
    });

    it('should call dataLoader.getContent', (done) => {
      service.getContent('test-node').subscribe(() => {
        expect(dataLoaderMock.getContent).toHaveBeenCalledWith('test-node');
        done();
      });
    });
  });

  describe('getRelatedResourceIds', () => {
    it('should return related resource IDs', (done) => {
      service.getRelatedResourceIds('test-node').subscribe(ids => {
        expect(ids).toEqual(['related-1', 'related-2']);
        done();
      });
    });
  });

  describe('getContainingPaths', () => {
    it('should return paths containing the resource', (done) => {
      service.getContainingPaths('test-node').subscribe(pathRefs => {
        expect(pathRefs.length).toBeGreaterThan(0);
        expect(pathRefs[0].path.id).toBe('test-path');
        expect(pathRefs[0].stepIndex).toBe(0);
        done();
      });
    });

    it('should return empty array for resource not in any path', (done) => {
      dataLoaderMock.getPaths.and.returnValue(of([{
        ...mockPath,
        steps: [{ nodeId: 'other-node', stepIndex: 0, narrative: 'Other' }]
      }]));

      service.getContainingPaths('test-node').subscribe(pathRefs => {
        expect(pathRefs.length).toBe(0);
        done();
      });
    });
  });

  describe('getContentWithAccess', () => {
    it('should return accessible content for commons reach', (done) => {
      service.getContentWithAccess('test-node').subscribe(result => {
        expect(result.canAccess).toBe(true);
        expect(result.content).toBeDefined();
        expect(result.content?.id).toBe('test-node');
        done();
      });
    });

    it('should handle not found content', (done) => {
      dataLoaderMock.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getContentWithAccess('nonexistent').subscribe(result => {
        expect(result.canAccess).toBe(false);
        expect(result.reason).toBe('not-found');
        done();
      });
    });
  });

  describe('searchContent', () => {
    it('should search content by query', (done) => {
      service.searchContent('test').subscribe(results => {
        expect(results).toBeDefined();
        done();
      });
    });
  });

  describe('getContentByType', () => {
    it('should filter content by type', (done) => {
      service.getContentByType('epic').subscribe(results => {
        expect(results).toBeDefined();
        results.forEach(node => {
          expect(node.contentType).toBe('epic');
        });
        done();
      });
    });
  });

  describe('getContentByTag', () => {
    it('should filter content by tag', (done) => {
      service.getContentByTag('test').subscribe(results => {
        expect(results).toBeDefined();
        done();
      });
    });
  });
});
