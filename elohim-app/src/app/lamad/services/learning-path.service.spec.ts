import { TestBed } from '@angular/core/testing';
import { LearningPathService } from './learning-path.service';
import { DocumentGraphService } from './document-graph.service';
import { BehaviorSubject } from 'rxjs';
import { DocumentGraph } from '../models/document-graph.model';
import { ContentNode } from '../models/content-node.model';

describe('LearningPathService', () => {
  let service: LearningPathService;
  let graphServiceMock: any;
  let graphSubject: BehaviorSubject<DocumentGraph | null>;

  const mockNodes: ContentNode[] = [
    {
      id: 'manifesto',
      title: 'Manifesto',
      contentType: 'epic',
      description: '',
      tags: [],
      sourcePath: '',
      relatedNodeIds: [],
      metadata: { category: 'core' },
      content: '',
      contentFormat: 'markdown'
    },
    {
      id: 'elohim-observer-protocol',
      title: 'Observer Protocol',
      contentType: 'epic',
      description: '',
      tags: [],
      sourcePath: '',
      relatedNodeIds: [],
      metadata: { category: 'observer' },
      content: '',
      contentFormat: 'markdown'
    },
    {
      id: 'unknown-node',
      title: 'Unknown',
      contentType: 'feature',
      description: '',
      tags: [],
      sourcePath: '',
      relatedNodeIds: [],
      metadata: { category: 'test' },
      content: '',
      contentFormat: 'gherkin'
    }
  ];

  const mockGraph: any = {
    nodes: new Map(mockNodes.map(n => [n.id, n])),
    relationships: new Map()
  };

  beforeEach(() => {
    graphSubject = new BehaviorSubject<DocumentGraph | null>(null);
    graphServiceMock = {
      graph$: graphSubject.asObservable()
    };

    TestBed.configureTestingModule({
      providers: [
        LearningPathService,
        { provide: DocumentGraphService, useValue: graphServiceMock }
      ]
    });
    service = TestBed.inject(LearningPathService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should initialize path with matching nodes', () => {
    // Manually set the path with the epic nodes
    const epicNodes = mockNodes.filter(n => n.contentType === 'epic');
    service.setPath(epicNodes);

    const path = service.getPath();
    expect(path.length).toBeGreaterThan(0);
    expect(path.some(p => p.node.id === 'manifesto')).toBeTrue();
    expect(path.some(p => p.node.id === 'unknown-node')).toBeFalse();
  });

  it('should get next node', () => {
    const epicNodes = mockNodes.filter(n => n.contentType === 'epic');
    service.setPath(epicNodes);

    const next = service.getNextNode('manifesto');
    expect(next).toBeTruthy();
    expect(next?.node.id).toBe('elohim-observer-protocol');
  });

  it('should return null if next node does not exist (end of path)', () => {
    const epicNodes = mockNodes.filter(n => n.contentType === 'epic');
    service.setPath(epicNodes);

    const lastNode = service.getPath()[service.getPath().length - 1];
    const next = service.getNextNode(lastNode.node.id);
    expect(next).toBeNull();
  });

  it('should return null if next node does not exist (unknown node)', () => {
    const epicNodes = mockNodes.filter(n => n.contentType === 'epic');
    service.setPath(epicNodes);

    const next = service.getNextNode('unknown-node');
    expect(next).toBeNull();
  });

  it('should get previous node', () => {
    const epicNodes = mockNodes.filter(n => n.contentType === 'epic');
    service.setPath(epicNodes);

    const prev = service.getPreviousNode('elohim-observer-protocol');
    expect(prev).toBeTruthy();
    expect(prev?.node.id).toBe('manifesto');
  });

  it('should return null if previous node does not exist (start of path)', () => {
    const epicNodes = mockNodes.filter(n => n.contentType === 'epic');
    service.setPath(epicNodes);

    const prev = service.getPreviousNode('manifesto');
    expect(prev).toBeNull();
  });

  it('should get node position', () => {
    const epicNodes = mockNodes.filter(n => n.contentType === 'epic');
    service.setPath(epicNodes);

    expect(service.getNodePosition('manifesto')).toBe(0);
    expect(service.getNodePosition('unknown-node')).toBe(-1);
  });

  it('should check if node is in path', () => {
    const epicNodes = mockNodes.filter(n => n.contentType === 'epic');
    service.setPath(epicNodes);

    expect(service.isInPath('manifesto')).toBeTrue();
    expect(service.isInPath('unknown-node')).toBeFalse();
  });

  it('should calculate path progress', () => {
    const epicNodes = mockNodes.filter(n => n.contentType === 'epic');
    service.setPath(epicNodes);

    // Path should contain 2 nodes: manifesto and elohim-observer-protocol
    const affinityMap = new Map<string, number>();
    affinityMap.set('manifesto', 0.5); // Engaged
    affinityMap.set('elohim-observer-protocol', 0.5); // Engaged

    const progress = service.getPathProgress(affinityMap);
    expect(progress).toBe(50); // Average affinity is 0.5, so 50%
  });

  it('should handle empty graph', () => {
    service.setPath([]);
    expect(service.getPath().length).toBe(0);
  });

  it('should handle path progress with empty path', () => {
    service.setPath([]);
    expect(service.getPathProgress(new Map())).toBe(0);
  });
});
