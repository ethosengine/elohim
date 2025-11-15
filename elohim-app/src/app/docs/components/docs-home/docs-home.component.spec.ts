import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { DocsHomeComponent } from './docs-home.component';
import { DocumentGraphService } from '../../services/document-graph.service';
import { DocumentGraph } from '../../models';

describe('DocsHomeComponent', () => {
  let component: DocsHomeComponent;
  let fixture: ComponentFixture<DocsHomeComponent>;
  let mockDocumentGraphService: jasmine.SpyObj<DocumentGraphService>;

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
    mockDocumentGraphService = jasmine.createSpyObj('DocumentGraphService', ['getGraph']);
    mockDocumentGraphService.getGraph.and.returnValue(mockGraph as DocumentGraph);

    await TestBed.configureTestingModule({
      imports: [DocsHomeComponent],
      providers: [
        { provide: DocumentGraphService, useValue: mockDocumentGraphService },
        provideRouter([])
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(DocsHomeComponent);
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
    mockDocumentGraphService.getGraph.and.returnValue(null);
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
