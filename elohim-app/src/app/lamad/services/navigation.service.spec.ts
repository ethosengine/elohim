import { TestBed } from '@angular/core/testing';
import { Router, NavigationEnd } from '@angular/router';
import { Subject } from 'rxjs';
import { NavigationService, ViewMode, NavigationContext } from './navigation.service';
import { DocumentGraphService } from './document-graph.service';
import { ContentNode, ContentGraph } from '../models/content-node.model';

describe('NavigationService', () => {
  let service: NavigationService;
  let mockRouter: jasmine.SpyObj<Router>;
  let mockGraphService: jasmine.SpyObj<DocumentGraphService>;
  let routerEventsSubject: Subject<any>;

  const mockNode: ContentNode = {
    id: 'test-epic',
    contentType: 'epic',
    title: 'Test Epic',
    description: 'Test epic description',
    content: 'Test content',
    contentFormat: 'markdown',
    tags: [],
    sourcePath: '/test/path',
    relatedNodeIds: ['feature-1', 'feature-2'],
    metadata: {}
  };

  const mockFeatureNode: ContentNode = {
    id: 'feature-1',
    contentType: 'feature',
    title: 'Test Feature',
    description: 'Test feature description',
    content: 'Test content',
    contentFormat: 'markdown',
    tags: [],
    sourcePath: '/test/path',
    relatedNodeIds: ['scenario-1'],
    metadata: {}
  };

  const mockScenarioNode: ContentNode = {
    id: 'scenario-1',
    contentType: 'scenario',
    title: 'Test Scenario',
    description: 'Test scenario description',
    content: 'Test content',
    contentFormat: 'markdown',
    tags: [],
    sourcePath: '/test/path',
    relatedNodeIds: [],
    metadata: {}
  };

  const mockGraph: ContentGraph = {
    nodes: new Map([
      ['test-epic', mockNode],
      ['feature-1', mockFeatureNode],
      ['scenario-1', mockScenarioNode]
    ]),
    relationships: new Map(),
    nodesByType: new Map([
      ['epic', new Set(['test-epic'])],
      ['feature', new Set(['feature-1'])],
      ['scenario', new Set(['scenario-1'])]
    ]),
    nodesByTag: new Map(),
    nodesByCategory: new Map(),
    adjacency: new Map(),
    reverseAdjacency: new Map(),
    metadata: {
      nodeCount: 3,
      relationshipCount: 0,
      lastUpdated: new Date(),
      version: '1.0'
    }
  };

  beforeEach(() => {
    routerEventsSubject = new Subject();
    mockRouter = jasmine.createSpyObj('Router', ['navigate', 'parseUrl'], {
      events: routerEventsSubject.asObservable(),
      url: '/lamad'
    });

    mockGraphService = jasmine.createSpyObj('DocumentGraphService', [
      'getGraph',
      'getNode',
      'getNodesByType'
    ]);

    TestBed.configureTestingModule({
      providers: [
        NavigationService,
        { provide: Router, useValue: mockRouter },
        { provide: DocumentGraphService, useValue: mockGraphService }
      ]
    });

    service = TestBed.inject(NavigationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('navigateTo', () => {
    it('should navigate to a node without parent path', () => {
      service.navigateTo('epic', 'test-epic');
      expect(mockRouter.navigate).toHaveBeenCalledWith(
        ['/lamad/epic:test-epic'],
        { queryParams: {} }
      );
    });

    it('should navigate to a node with parent path', () => {
      service.navigateTo('feature', 'test-feature', {
        parentPath: 'epic:test-epic'
      });
      expect(mockRouter.navigate).toHaveBeenCalledWith(
        ['/lamad/epic:test-epic:feature:test-feature'],
        { queryParams: {} }
      );
    });

    it('should navigate with query params', () => {
      service.navigateTo('epic', 'test-epic', {
        queryParams: { target: 'some-target' }
      });
      expect(mockRouter.navigate).toHaveBeenCalledWith(
        ['/lamad/epic:test-epic'],
        { queryParams: { target: 'some-target' } }
      );
    });
  });

  describe('navigateToCollection', () => {
    it('should navigate to a collection without parent path', () => {
      service.navigateToCollection('feature');
      expect(mockRouter.navigate).toHaveBeenCalledWith(
        ['/lamad/feature'],
        { queryParams: {} }
      );
    });

    it('should navigate to a collection with parent path', () => {
      service.navigateToCollection('feature', 'epic:test-epic');
      expect(mockRouter.navigate).toHaveBeenCalledWith(
        ['/lamad/epic:test-epic:feature'],
        { queryParams: {} }
      );
    });

    it('should navigate with query params', () => {
      service.navigateToCollection('feature', undefined, { step: 1 });
      expect(mockRouter.navigate).toHaveBeenCalledWith(
        ['/lamad/feature'],
        { queryParams: { step: 1 } }
      );
    });
  });

  describe('navigateToHome', () => {
    it('should navigate to home', () => {
      service.navigateToHome();
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad']);
    });
  });

  describe('navigateUp', () => {
    it('should navigate to home when no context', () => {
      service.navigateUp();
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad']);
    });

    it('should navigate to home from collection view with no segments', () => {
      const context: NavigationContext = {
        pathSegments: [],
        viewMode: ViewMode.COLLECTION,
        currentNode: null,
        collectionType: 'epic',
        children: [],
        parent: null,
        queryParams: {}
      };

      spyOn<any>(service['contextSubject'], 'value').and.returnValue(context);
      service.navigateUp();
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad']);
    });

    it('should navigate to home from node view with single segment', () => {
      const context: NavigationContext = {
        pathSegments: [{
          type: 'epic',
          id: 'test-epic',
          node: mockNode,
          urlSegment: 'epic:test-epic'
        }],
        viewMode: ViewMode.NODE,
        currentNode: mockNode,
        collectionType: null,
        children: [],
        parent: null,
        queryParams: {}
      };

      spyOn<any>(service['contextSubject'], 'value').and.returnValue(context);
      service.navigateUp();
      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad']);
    });
  });

  describe('getCurrentContext', () => {
    it('should return null initially', () => {
      expect(service.getCurrentContext()).toBeNull();
    });
  });

  describe('parsePathSegments', () => {
    beforeEach(() => {
      mockGraphService.getGraph.and.returnValue(mockGraph);
      mockGraphService.getNode.and.callFake((id: string) => mockGraph.nodes.get(id));
    });

    it('should return home context for empty path', () => {
      const context = service.parsePathSegments('', {});
      expect(context).not.toBeNull();
      expect(context?.viewMode).toBe(ViewMode.HOME);
      expect(context?.pathSegments).toEqual([]);
      expect(context?.currentNode).toBeNull();
    });

    it('should return null when graph is not available', () => {
      mockGraphService.getGraph.and.returnValue(null);
      const context = service.parsePathSegments('epic:test-epic', {});
      expect(context).toBeNull();
    });

    it('should parse node view path', () => {
      const context = service.parsePathSegments('epic:test-epic', {});
      expect(context).not.toBeNull();
      expect(context?.viewMode).toBe(ViewMode.NODE);
      expect(context?.pathSegments.length).toBe(1);
      expect(context?.currentNode?.id).toBe('test-epic');
    });

    it('should parse collection view path', () => {
      const context = service.parsePathSegments('epic:test-epic:feature', {});
      expect(context).not.toBeNull();
      expect(context?.viewMode).toBe(ViewMode.COLLECTION);
      expect(context?.collectionType).toBe('feature');
    });

    it('should return null for invalid node ID', () => {
      mockGraphService.getNode.and.returnValue(undefined);
      const context = service.parsePathSegments('epic:invalid-id', {});
      expect(context).toBeNull();
    });

    it('should include query params in context', () => {
      const queryParams = { target: 'some-target', step: 1 };
      const context = service.parsePathSegments('epic:test-epic', queryParams);
      expect(context?.queryParams).toEqual(queryParams);
    });
  });

  describe('nodeToUrlSegment', () => {
    it('should convert node to URL segment', () => {
      const segment = service.nodeToUrlSegment(mockNode);
      expect(segment).toBe('epic:test-epic');
    });
  });

  describe('getBreadcrumbs', () => {
    it('should return home breadcrumb for home view', () => {
      const context: NavigationContext = {
        pathSegments: [],
        viewMode: ViewMode.HOME,
        currentNode: null,
        collectionType: null,
        children: [],
        parent: null,
        queryParams: {}
      };

      const breadcrumbs = service.getBreadcrumbs(context);
      expect(breadcrumbs.length).toBe(1);
      expect(breadcrumbs[0].label).toBe('Home');
    });

    it('should include path segments in breadcrumbs', () => {
      const context: NavigationContext = {
        pathSegments: [{
          type: 'epic',
          id: 'test-epic',
          node: mockNode,
          urlSegment: 'epic:test-epic'
        }],
        viewMode: ViewMode.NODE,
        currentNode: mockNode,
        collectionType: null,
        children: [],
        parent: null,
        queryParams: {}
      };

      const breadcrumbs = service.getBreadcrumbs(context);
      expect(breadcrumbs.length).toBe(2);
      expect(breadcrumbs[1].label).toBe('Test Epic');
      expect(breadcrumbs[1].typeLabel).toBe('Epic');
    });

    it('should add collection to breadcrumbs in collection view', () => {
      const context: NavigationContext = {
        pathSegments: [{
          type: 'epic',
          id: 'test-epic',
          node: mockNode,
          urlSegment: 'epic:test-epic'
        }],
        viewMode: ViewMode.COLLECTION,
        currentNode: mockNode,
        collectionType: 'feature',
        children: [],
        parent: null,
        queryParams: {}
      };

      const breadcrumbs = service.getBreadcrumbs(context);
      expect(breadcrumbs.length).toBe(3);
      expect(breadcrumbs[2].label).toBe('Features');
    });
  });

  describe('getSuggestedNext', () => {
    it('should return sorted children', () => {
      const nodeA: ContentNode = {
        id: 'b',
        contentType: 'feature',
        title: 'B Feature',
        description: 'B Feature description',
        content: 'Test content',
        contentFormat: 'markdown',
        tags: [],
        sourcePath: '/test/path',
        relatedNodeIds: [],
        metadata: {}
      };

      const nodeB: ContentNode = {
        id: 'a',
        contentType: 'feature',
        title: 'A Feature',
        description: 'A Feature description',
        content: 'Test content',
        contentFormat: 'markdown',
        tags: [],
        sourcePath: '/test/path',
        relatedNodeIds: [],
        metadata: {}
      };

      const context: NavigationContext = {
        pathSegments: [],
        viewMode: ViewMode.NODE,
        currentNode: mockNode,
        collectionType: null,
        children: [nodeA, nodeB],
        parent: null,
        queryParams: {}
      };

      const suggested = service.getSuggestedNext(context);
      expect(suggested.length).toBe(2);
      expect(suggested[0].title).toBe('A Feature');
      expect(suggested[1].title).toBe('B Feature');
    });
  });
});
