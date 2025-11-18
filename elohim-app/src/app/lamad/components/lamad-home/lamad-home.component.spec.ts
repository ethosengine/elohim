import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { BehaviorSubject } from 'rxjs';
import { LamadHomeComponent } from './lamad-home.component';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { DocumentGraph } from '../../models';

describe('LamadHomeComponent', () => {
  let component: LamadHomeComponent;
  let fixture: ComponentFixture<LamadHomeComponent>;
  let mockDocumentGraphService: any;
  let mockAffinityService: any;
  let graphSubject: BehaviorSubject<DocumentGraph | null>;
  let changesSubject: BehaviorSubject<void>;

  const mockGraph: Partial<DocumentGraph> = {
    nodes: new Map(),
    relationships: new Map(),
    nodesByType: {
      epics: new Map([
        ['epic-1', {
          id: 'epic-1',
          type: 'epic' as const,
          title: 'Test Epic',
          description: 'Test description',
          tags: [],
          sourcePath: '',
          content: '',
          relatedNodeIds: [],
          metadata: {},
          category: 'observer',
          featureIds: [],
          relatedEpicIds: [],
          markdownContent: '',
          sections: []
        }]
      ]),
      features: new Map([
        ['feature-1', {
          id: 'feature-1',
          type: 'feature' as const,
          title: 'Test Feature',
          description: 'Test description',
          tags: [],
          sourcePath: '',
          content: '',
          relatedNodeIds: [],
          metadata: {},
          category: 'value-scanner',
          epicIds: [],
          scenarioIds: [],
          featureDescription: 'Test',
          gherkinContent: ''
        }]
      ]),
      scenarios: new Map()
    },
    nodesByTag: new Map(),
    nodesByCategory: new Map(),
    adjacency: new Map(),
    reverseAdjacency: new Map(),
    metadata: {
      nodeCount: 2,
      relationshipCount: 0,
      lastBuilt: new Date(),
      sources: {
        epicPath: '',
        featurePath: ''
      },
      stats: {
        epicCount: 1,
        featureCount: 1,
        scenarioCount: 0,
        averageConnectionsPerNode: 0
      }
    }
  } as DocumentGraph;

  beforeEach(async () => {
    graphSubject = new BehaviorSubject<DocumentGraph | null>(mockGraph as DocumentGraph);
    changesSubject = new BehaviorSubject<void>(undefined);

    mockDocumentGraphService = {
      getGraph: jasmine.createSpy('getGraph').and.returnValue(mockGraph as DocumentGraph),
      graph$: graphSubject.asObservable()
    };

    mockAffinityService = {
      getStats: jasmine.createSpy('getStats').and.returnValue({
        totalNodes: 2,
        engagedNodes: 1,
        averageAffinity: 0.5,
        distribution: {
          unseen: 1,
          low: 0,
          medium: 0,
          high: 1
        }
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

  it('should load graph data on init', () => {
    fixture.detectChanges();

    expect(component.epics.length).toBe(1);
    expect(component.features.length).toBe(1);
    expect(component.stats.epicCount).toBe(1);
  });

  it('should handle null graph gracefully', () => {
    graphSubject.next(null);
    fixture.detectChanges();

    expect(component.epics.length).toBe(0);
    expect(component.features.length).toBe(0);
  });

  it('should return correct epic category', () => {
    const epic = { category: 'observer' } as any;
    expect(component.getEpicCategory(epic)).toBe('observer');

    const epicNoCategory = {} as any;
    expect(component.getEpicCategory(epicNoCategory)).toBe('general');
  });

  it('should return correct feature category', () => {
    const feature = { category: 'value-scanner' } as any;
    expect(component.getFeatureCategory(feature)).toBe('value-scanner');

    const featureNoCategory = {} as any;
    expect(component.getFeatureCategory(featureNoCategory)).toBe('general');
  });

  it('should return correct category icons', () => {
    expect(component.getCategoryIcon('observer')).toBe('👁️');
    expect(component.getCategoryIcon('value-scanner')).toBe('🔍');
    expect(component.getCategoryIcon('autonomous-entity')).toBe('🤖');
    expect(component.getCategoryIcon('social')).toBe('🌐');
    expect(component.getCategoryIcon('deployment')).toBe('🚀');
    expect(component.getCategoryIcon('unknown')).toBe('📄');
  });
});
