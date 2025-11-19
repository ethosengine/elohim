import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { BehaviorSubject } from 'rxjs';
import { LamadHomeComponent } from './lamad-home.component';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { DocumentGraph, EpicNode, FeatureNode, ScenarioNode } from '../../models';

describe('LamadHomeComponent', () => {
  let component: LamadHomeComponent;
  let fixture: ComponentFixture<LamadHomeComponent>;
  let mockDocumentGraphService: any;
  let mockAffinityService: any;
  let graphSubject: BehaviorSubject<DocumentGraph | null>;
  let changesSubject: BehaviorSubject<any>;

  const mockEpicNode: EpicNode = {
    id: 'epic-social-medium-v1',
    type: 'epic' as const,
    title: 'Social Medium',
    description: 'Building a decentralized social platform',
    tags: ['social', 'platform'],
    sourcePath: '/epics/social-medium.md',
    content: '# Social Medium Epic',
    relatedNodeIds: [],
    metadata: { category: 'social' },
    category: 'social',
    featureIds: ['feature-affinity-tracking', 'feature-content-graph'],
    relatedEpicIds: [],
    markdownContent: '# Social Medium Epic',
    sections: []
  };

  const mockFeatureNode: FeatureNode = {
    id: 'feature-affinity-tracking',
    type: 'feature' as const,
    title: 'Affinity Tracking',
    description: 'Track user engagement and affinity',
    tags: ['engagement'],
    sourcePath: '/features/affinity-tracking.feature',
    content: 'Feature: Affinity Tracking',
    relatedNodeIds: [],
    metadata: { category: 'social' },
    category: 'social',
    epicId: 'epic-social-medium-v1',
    scenarioIds: [],
    gherkinContent: 'Feature: Affinity Tracking'
  };

  const mockScenarioNode: ScenarioNode = {
    id: 'scenario-track-view',
    type: 'scenario' as const,
    title: 'Track Content View',
    description: 'Automatically track when user views content',
    tags: ['tracking'],
    sourcePath: '/features/affinity-tracking.feature',
    content: 'Scenario: Track Content View',
    relatedNodeIds: [],
    metadata: {},
    featureId: 'feature-affinity-tracking',
    gherkinContent: 'Scenario: Track Content View'
  };

  const mockGraph: Partial<DocumentGraph> = {
    nodes: new Map([
      ['epic-social-medium-v1', mockEpicNode],
      ['feature-affinity-tracking', mockFeatureNode],
      ['scenario-track-view', mockScenarioNode]
    ]),
    relationships: new Map(),
    nodesByType: {
      epics: new Map([['epic-social-medium-v1', mockEpicNode]]),
      features: new Map([['feature-affinity-tracking', mockFeatureNode]]),
      scenarios: new Map([['scenario-track-view', mockScenarioNode]])
    },
    nodesByTag: new Map(),
    nodesByCategory: new Map(),
    adjacency: new Map(),
    reverseAdjacency: new Map(),
    metadata: {
      nodeCount: 3,
      relationshipCount: 0,
      lastBuilt: new Date(),
      sources: {
        epicPath: '/epics',
        featurePath: '/features'
      },
      stats: {
        epicCount: 1,
        featureCount: 1,
        scenarioCount: 1,
        averageConnectionsPerNode: 0.5
      }
    }
  } as DocumentGraph;

  beforeEach(async () => {
    graphSubject = new BehaviorSubject<DocumentGraph | null>(mockGraph as DocumentGraph);
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
        totalNodes: 3,
        engagedNodes: 2,
        averageAffinity: 0.6,
        distribution: {
          unseen: 1,
          low: 0,
          medium: 1,
          high: 1
        },
        byCategory: new Map(),
        byType: new Map()
      }),
      changes$: changesSubject.asObservable()
    };

    await TestBed.configureTestingModule({
      imports: [LamadHomeComponent],
      providers: [
        { provide: DocumentGraphService, useValue: mockDocumentGraphService },
        { provide: AffinityTrackingService, useValue: mockAffinityService },
        provideRouter([])
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(LamadHomeComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load epics from graph on init', () => {
    fixture.detectChanges();

    expect(component.epics.length).toBe(1);
    expect(component.epics[0].title).toBe('Social Medium');
    expect(component.epics[0].id).toBe('epic-social-medium-v1');
  });

  it('should load features from graph on init', () => {
    fixture.detectChanges();

    expect(component.features.length).toBe(1);
    expect(component.features[0].title).toBe('Affinity Tracking');
  });

  it('should load scenarios from graph on init', () => {
    fixture.detectChanges();

    expect(component.scenarios.length).toBe(1);
    expect(component.scenarios[0].title).toBe('Track Content View');
  });

  it('should load stats from graph metadata', () => {
    fixture.detectChanges();

    expect(component.stats).toBeTruthy();
    expect(component.stats.epicCount).toBe(1);
    expect(component.stats.featureCount).toBe(1);
    expect(component.stats.scenarioCount).toBe(1);
    expect(component.stats.averageConnectionsPerNode).toBe(0.5);
  });

  it('should load affinity stats on init', () => {
    fixture.detectChanges();

    expect(component.affinityStats).toBeTruthy();
    expect(component.affinityStats?.totalNodes).toBe(3);
    expect(component.affinityStats?.engagedNodes).toBe(2);
    expect(component.affinityStats?.averageAffinity).toBe(0.6);
  });

  it('should update affinity stats when affinity changes', () => {
    fixture.detectChanges();

    const newStats = {
      totalNodes: 3,
      engagedNodes: 3,
      averageAffinity: 0.8,
      distribution: {
        unseen: 0,
        low: 0,
        medium: 1,
        high: 2
      },
      byCategory: new Map(),
      byType: new Map()
    };
    mockAffinityService.getStats.and.returnValue(newStats);

    changesSubject.next({ nodeId: 'epic-social-medium-v1', newValue: 0.9 });

    expect(component.affinityStats?.engagedNodes).toBe(3);
    expect(component.affinityStats?.averageAffinity).toBe(0.8);
  });

  it('should handle null graph gracefully', () => {
    graphSubject.next(null);
    fixture.detectChanges();

    expect(component.epics.length).toBe(0);
    expect(component.features.length).toBe(0);
    expect(component.scenarios.length).toBe(0);
  });

  it('should get epic category', () => {
    fixture.detectChanges();
    const epic = component.epics[0];

    const category = component.getEpicCategory(epic);

    expect(category).toBe('social');
  });

  it('should return "general" for epic without category', () => {
    const epicWithoutCategory: EpicNode = {
      ...mockEpicNode,
      category: undefined
    };

    const category = component.getEpicCategory(epicWithoutCategory);

    expect(category).toBe('general');
  });

  it('should get feature category', () => {
    fixture.detectChanges();
    const feature = component.features[0];

    const category = component.getFeatureCategory(feature);

    expect(category).toBe('social');
  });

  it('should return "general" for feature without category', () => {
    const featureWithoutCategory: FeatureNode = {
      ...mockFeatureNode,
      category: undefined
    };

    const category = component.getFeatureCategory(featureWithoutCategory);

    expect(category).toBe('general');
  });

  it('should return correct category icons', () => {
    expect(component.getCategoryIcon('observer')).toBe('👁️');
    expect(component.getCategoryIcon('value-scanner')).toBe('🔍');
    expect(component.getCategoryIcon('autonomous-entity')).toBe('🤖');
    expect(component.getCategoryIcon('social')).toBe('🌐');
    expect(component.getCategoryIcon('deployment')).toBe('🚀');
    expect(component.getCategoryIcon('general')).toBe('📄');
  });

  it('should return default icon for unknown category', () => {
    expect(component.getCategoryIcon('unknown')).toBe('📄');
  });

  it('should unsubscribe on destroy', () => {
    fixture.detectChanges();
    const destroySpy = spyOn(component['destroy$'], 'next');
    const completeSpy = spyOn(component['destroy$'], 'complete');

    component.ngOnDestroy();

    expect(destroySpy).toHaveBeenCalled();
    expect(completeSpy).toHaveBeenCalled();
  });
});
