import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { KnowledgeMapService } from './knowledge-map.service';
import { DataLoaderService } from './data-loader.service';
import { AgentService } from './agent.service';
import { SessionUserService } from './session-user.service';

describe('KnowledgeMapService', () => {
  let service: KnowledgeMapService;
  let dataLoaderMock: jasmine.SpyObj<DataLoaderService>;
  let agentServiceMock: jasmine.SpyObj<AgentService>;
  let sessionUserMock: jasmine.SpyObj<SessionUserService>;

  const mockContentIndex = {
    nodes: [
      {
        id: 'node-1',
        title: 'Test Node',
        contentType: 'concept',
        tags: ['test'],
        relatedNodeIds: ['node-2']
      },
      {
        id: 'node-2',
        title: 'Related Node',
        contentType: 'concept',
        tags: ['test'],
        relatedNodeIds: ['node-1']
      }
    ]
  };

  beforeEach(() => {
    dataLoaderMock = jasmine.createSpyObj('DataLoaderService', [
      'getContentIndex',
      'getContent',
      'getPaths'
    ]);
    agentServiceMock = jasmine.createSpyObj('AgentService', ['getCurrentAgent']);
    sessionUserMock = jasmine.createSpyObj('SessionUserService', [
      'getCurrentSession',
      'getAllPathProgress'
    ]);

    dataLoaderMock.getContentIndex.and.returnValue(of(mockContentIndex));
    dataLoaderMock.getPaths.and.returnValue(of([]));
    agentServiceMock.getCurrentAgent.and.returnValue(of(null));
    sessionUserMock.getCurrentSession.and.returnValue(null);
    sessionUserMock.getAllPathProgress.and.returnValue([]);

    TestBed.configureTestingModule({
      providers: [
        KnowledgeMapService,
        { provide: DataLoaderService, useValue: dataLoaderMock },
        { provide: AgentService, useValue: agentServiceMock },
        { provide: SessionUserService, useValue: sessionUserMock }
      ]
    });
    service = TestBed.inject(KnowledgeMapService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getDomainMap', () => {
    it('should return domain knowledge map', (done) => {
      service.getDomainMap('test-domain').subscribe(map => {
        expect(map).toBeDefined();
        expect(map.id).toContain('test-domain');
        expect(map.type).toBe('domain');
        done();
      });
    });

    it('should include nodes in map', (done) => {
      service.getDomainMap('test-domain').subscribe(map => {
        expect(map.nodes).toBeDefined();
        done();
      });
    });
  });

  describe('getPersonalMap', () => {
    it('should return personal knowledge map', (done) => {
      service.getPersonalMap('agent-1').subscribe(map => {
        expect(map).toBeDefined();
        expect(map.type).toBe('person');
        done();
      });
    });
  });

  describe('getCollectiveMap', () => {
    it('should return collective knowledge map', (done) => {
      service.getCollectiveMap('community-1').subscribe(map => {
        expect(map).toBeDefined();
        expect(map.type).toBe('collective');
        done();
      });
    });
  });

  describe('getMapNode', () => {
    it('should return node from map', (done) => {
      service.getMapNode('test-domain', 'node-1').subscribe(node => {
        expect(node).toBeDefined();
        done();
      });
    });
  });

  describe('getConnectedNodes', () => {
    it('should return connected nodes', (done) => {
      service.getConnectedNodes('test-domain', 'node-1').subscribe(nodes => {
        expect(nodes).toBeDefined();
        expect(Array.isArray(nodes)).toBe(true);
        done();
      });
    });
  });

  describe('getNodesByTag', () => {
    it('should filter nodes by tag', (done) => {
      service.getNodesByTag('test-domain', 'test').subscribe(nodes => {
        expect(nodes).toBeDefined();
        done();
      });
    });
  });

  describe('getNodesByType', () => {
    it('should filter nodes by content type', (done) => {
      service.getNodesByType('test-domain', 'concept').subscribe(nodes => {
        expect(nodes).toBeDefined();
        done();
      });
    });
  });

  describe('getMapStats', () => {
    it('should return map statistics', (done) => {
      service.getMapStats('test-domain').subscribe(stats => {
        expect(stats).toBeDefined();
        expect(stats.totalNodes).toBeDefined();
        done();
      });
    });
  });

  describe('searchMap', () => {
    it('should search within map', (done) => {
      service.searchMap('test-domain', 'test').subscribe(results => {
        expect(results).toBeDefined();
        expect(Array.isArray(results)).toBe(true);
        done();
      });
    });
  });
});
