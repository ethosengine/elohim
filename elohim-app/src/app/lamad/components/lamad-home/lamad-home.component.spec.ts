import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { BehaviorSubject } from 'rxjs';
import { LamadHomeComponent } from './lamad-home.component';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { LearningPathService } from '../../services/learning-path.service';
import { DocumentGraph } from '../../models';
import { ContentNode } from '../../models/content-node.model';

describe('LamadHomeComponent', () => {
  let component: LamadHomeComponent;
  let fixture: ComponentFixture<LamadHomeComponent>;
  let mockDocumentGraphService: any;
  let mockAffinityService: any;
  let mockLearningPathService: any;
  let graphSubject: BehaviorSubject<DocumentGraph | null>;
  let pathSubject: BehaviorSubject<any[]>;
  let affinitySubject: BehaviorSubject<any>;
  let changesSubject: BehaviorSubject<any>;

  const mockContentNode: ContentNode = {
    id: 'manifesto.md',
    contentType: 'epic',
    title: 'Elohim Manifesto',
    description: 'The founding vision',
    content: '# Test Content',
    contentFormat: 'markdown',
    tags: [],
    relatedNodeIds: [],
    metadata: {
      category: 'vision'
    }
  };

  const mockPathNodes = [
    {
      node: mockContentNode,
      order: 0,
      depth: 0,
      category: 'vision'
    }
  ];

  const mockGraph: Partial<DocumentGraph> = {
    nodes: new Map([
      ['manifesto.md', {
        id: 'manifesto.md',
        type: 'epic' as const,
        title: 'Elohim Manifesto',
        description: 'The founding vision',
        tags: [],
        sourcePath: '',
        content: '# Test Content',
        relatedNodeIds: [],
        metadata: { category: 'vision' },
        category: 'vision',
        featureIds: [],
        relatedEpicIds: [],
        markdownContent: '# Test Content',
        sections: []
      }]
    ]),
    relationships: new Map(),
    nodesByType: {
      epics: new Map(),
      features: new Map(),
      scenarios: new Map()
    },
    nodesByTag: new Map(),
    nodesByCategory: new Map(),
    adjacency: new Map(),
    reverseAdjacency: new Map(),
    metadata: {
      nodeCount: 1,
      relationshipCount: 0,
      lastBuilt: new Date(),
      sources: {
        epicPath: '',
        featurePath: ''
      },
      stats: {
        epicCount: 1,
        featureCount: 0,
        scenarioCount: 0,
        averageConnectionsPerNode: 0
      }
    }
  } as DocumentGraph;

  beforeEach(async () => {
    graphSubject = new BehaviorSubject<DocumentGraph | null>(mockGraph as DocumentGraph);
    pathSubject = new BehaviorSubject<any[]>(mockPathNodes);
    affinitySubject = new BehaviorSubject<any>({});
    changesSubject = new BehaviorSubject<any>(null);

    mockDocumentGraphService = {
      getGraph: jasmine.createSpy('getGraph').and.returnValue(mockGraph as DocumentGraph),
      graph$: graphSubject.asObservable()
    };

    mockAffinityService = {
      getAffinity: jasmine.createSpy('getAffinity').and.returnValue(0.5),
      trackView: jasmine.createSpy('trackView'),
      incrementAffinity: jasmine.createSpy('incrementAffinity'),
      setAffinity: jasmine.createSpy('setAffinity'),
      getStats: jasmine.createSpy('getStats').and.returnValue({
        totalNodes: 1,
        engagedNodes: 1,
        averageAffinity: 0.5,
        distribution: {
          unseen: 0,
          low: 0,
          medium: 1,
          high: 0
        },
        byCategory: new Map(),
        byType: new Map()
      }),
      affinity$: affinitySubject.asObservable(),
      changes$: changesSubject.asObservable()
    };

    mockLearningPathService = {
      path$: pathSubject.asObservable(),
      getPath: jasmine.createSpy('getPath').and.returnValue(mockPathNodes),
      getNextNode: jasmine.createSpy('getNextNode').and.returnValue(null),
      getPreviousNode: jasmine.createSpy('getPreviousNode').and.returnValue(null),
      getNodePosition: jasmine.createSpy('getNodePosition').and.returnValue(0),
      isInPath: jasmine.createSpy('isInPath').and.returnValue(true),
      getPathProgress: jasmine.createSpy('getPathProgress').and.returnValue(50)
    };

    await TestBed.configureTestingModule({
      imports: [LamadHomeComponent],
      providers: [
        { provide: DocumentGraphService, useValue: mockDocumentGraphService },
        { provide: AffinityTrackingService, useValue: mockAffinityService },
        { provide: LearningPathService, useValue: mockLearningPathService },
        provideRouter([])
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(LamadHomeComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load learning path on init', () => {
    fixture.detectChanges();

    expect(component.pathNodes.length).toBe(1);
    expect(component.pathNodes[0].node.title).toBe('Elohim Manifesto');
  });

  it('should select first node on init', () => {
    fixture.detectChanges();

    expect(component.selectedNode).toBeTruthy();
    expect(component.selectedNode?.id).toBe('manifesto.md');
  });

  it('should load affinity stats', () => {
    fixture.detectChanges();

    expect(component.affinityStats).toBeTruthy();
    expect(component.affinityStats?.totalNodes).toBe(1);
    expect(component.affinityStats?.engagedNodes).toBe(1);
  });

  it('should handle null graph gracefully', () => {
    graphSubject.next(null);
    fixture.detectChanges();

    expect(component.isLoading).toBe(true);
  });

  it('should return correct affinity level', () => {
    expect(component.getAffinityLevel(0)).toBe('unseen');
    expect(component.getAffinityLevel(0.2)).toBe('low');
    expect(component.getAffinityLevel(0.5)).toBe('medium');
    expect(component.getAffinityLevel(0.8)).toBe('high');
  });

  it('should return correct affinity percentage', () => {
    expect(component.getAffinityPercentage(0.5)).toBe(50);
    expect(component.getAffinityPercentage(0.75)).toBe(75);
    expect(component.getAffinityPercentage(1.0)).toBe(100);
  });

  it('should return correct content type icons', () => {
    expect(component.getContentTypeIcon('epic')).toBe('📖');
    expect(component.getContentTypeIcon('feature')).toBe('⚙️');
    expect(component.getContentTypeIcon('scenario')).toBe('✓');
    expect(component.getContentTypeIcon('unknown')).toBe('📄');
  });

  it('should return correct category display names', () => {
    expect(component.getCategoryDisplay('vision')).toBe('Vision');
    expect(component.getCategoryDisplay('core')).toBe('Core Concepts');
    expect(component.getCategoryDisplay('advanced')).toBe('Advanced');
    expect(component.getCategoryDisplay('systemic')).toBe('Systemic View');
    expect(component.getCategoryDisplay('implementation')).toBe('Implementation');
    expect(component.getCategoryDisplay('technical')).toBe('Technical');
  });

  it('should select node and track view', () => {
    fixture.detectChanges();
    const node = mockContentNode;

    component.selectNode(node);

    expect(component.selectedNode).toBe(node);
    expect(mockAffinityService.trackView).toHaveBeenCalledWith(node.id);
  });

  it('should toggle graph expansion', () => {
    fixture.detectChanges();
    const initialState = component.isGraphExpanded;

    component.toggleGraph();

    expect(component.isGraphExpanded).toBe(!initialState);
  });

  it('should adjust affinity', () => {
    fixture.detectChanges();
    component.selectedNode = mockContentNode;

    component.adjustAffinity(0.1);

    expect(mockAffinityService.incrementAffinity).toHaveBeenCalledWith(mockContentNode.id, 0.1);
  });

  it('should navigate to next node', () => {
    fixture.detectChanges();
    component.selectedNode = mockContentNode;
    const nextNode = { ...mockContentNode, id: 'next-node' };
    mockLearningPathService.getNextNode.and.returnValue({ node: nextNode, order: 1, depth: 0, category: 'core' });

    component.goToNext();

    expect(mockLearningPathService.getNextNode).toHaveBeenCalledWith(mockContentNode.id);
  });

  it('should navigate to previous node', () => {
    fixture.detectChanges();
    component.selectedNode = mockContentNode;
    const prevNode = { ...mockContentNode, id: 'prev-node' };
    mockLearningPathService.getPreviousNode.and.returnValue({ node: prevNode, order: 0, depth: 0, category: 'vision' });

    component.goToPrevious();

    expect(mockLearningPathService.getPreviousNode).toHaveBeenCalledWith(mockContentNode.id);
  });

  it('should check if has next node', () => {
    fixture.detectChanges();
    component.selectedNode = mockContentNode;

    const hasNext = component.hasNext();

    expect(mockLearningPathService.getNextNode).toHaveBeenCalled();
    expect(hasNext).toBe(false);
  });

  it('should check if has previous node', () => {
    fixture.detectChanges();
    component.selectedNode = mockContentNode;

    const hasPrev = component.hasPrevious();

    expect(mockLearningPathService.getPreviousNode).toHaveBeenCalled();
    expect(hasPrev).toBe(false);
  });

  it('should render markdown content', () => {
    const markdown = '# Heading\n**Bold** text';
    const html = component.renderMarkdown(markdown);

    expect(html).toContain('<h1>Heading</h1>');
    expect(html).toContain('<strong>Bold</strong>');
  });

  it('should render gherkin content', () => {
    const gherkin = 'Feature: Test\n@tag\nScenario: Test scenario';
    const html = component.renderGherkin(gherkin);

    expect(html).toContain('gherkin-keyword');
    expect(html).toContain('gherkin-tag');
  });
});
