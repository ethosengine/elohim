import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute } from '@angular/router';
import { provideRouter } from '@angular/router';
import { of } from 'rxjs';
import { ModuleViewerComponent } from './module-viewer.component';
import { DocumentGraphService } from '../../services/document-graph.service';
import { EpicNode, FeatureNode, ScenarioNode } from '../../models';

describe('ModuleViewerComponent', () => {
  let component: ModuleViewerComponent;
  let fixture: ComponentFixture<ModuleViewerComponent>;
  let mockDocumentGraphService: jasmine.SpyObj<DocumentGraphService>;
  let mockActivatedRoute: any;

  const mockEpicNode: Partial<EpicNode> = {
    id: 'elohim-value-scanner-protocol',
    type: 'epic' as const,
    title: 'Value Scanner Protocol',
    description: 'Test epic description',
    tags: [],
    metadata: {},
    featureIds: ['care-economy'],
    relatedEpicIds: [],
    markdownContent: '## Test Section\n\nTest content here.\n\n### Subsection\n\nMore test content.',
    sections: []
  };

  const mockFeatureNode: Partial<FeatureNode> = {
    id: 'care-economy',
    type: 'feature' as const,
    title: 'Care Economy',
    description: 'Test feature description',
    tags: [],
    metadata: {},
    category: 'value-scanner',
    epicIds: ['elohim-value-scanner-protocol'],
    scenarioIds: ['scenario-1'],
    featureDescription: 'Care economy feature',
    gherkinContent: 'Feature: Care Economy'
  };

  const mockScenarioNode: Partial<ScenarioNode> = {
    id: 'scenario-1',
    type: 'scenario' as const,
    title: 'Test Scenario',
    description: 'Test scenario description',
    tags: [],
    metadata: {},
    featureId: 'care-economy',
    steps: [
      { keyword: 'Given', text: 'a test condition' },
      { keyword: 'When', text: 'an action occurs' },
      { keyword: 'Then', text: 'a result is expected' }
    ]
  };

  beforeEach(async () => {
    mockDocumentGraphService = jasmine.createSpyObj('DocumentGraphService', [
      'getNodesByType',
      'getNode'
    ]);

    mockActivatedRoute = {
      params: of({ id: 'value-scanner' })
    };

    await TestBed.configureTestingModule({
      imports: [ModuleViewerComponent],
      providers: [
        { provide: DocumentGraphService, useValue: mockDocumentGraphService },
        { provide: ActivatedRoute, useValue: mockActivatedRoute },
        provideRouter([])
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(ModuleViewerComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load value-scanner module on init', () => {
    mockDocumentGraphService.getNodesByType.and.returnValue([mockEpicNode as any]);
    mockDocumentGraphService.getNode.and.returnValue(mockScenarioNode as any);

    fixture.detectChanges();

    expect(component.moduleName).toBe('Value Scanner: Care Economy');
    expect(mockDocumentGraphService.getNodesByType).toHaveBeenCalledWith('epic');
    expect(mockDocumentGraphService.getNodesByType).toHaveBeenCalledWith('feature');
  });

  it('should parse epic sections correctly', () => {
    mockDocumentGraphService.getNodesByType.and.callFake((type: string) => {
      if (type === 'epic') return [mockEpicNode as any];
      if (type === 'feature') return [mockFeatureNode as any];
      return [];
    });
    mockDocumentGraphService.getNode.and.returnValue(mockScenarioNode as any);

    fixture.detectChanges();

    expect(component.interleavedSections.length).toBeGreaterThan(0);
    expect(component.interleavedSections.some(s => s.type === 'epic')).toBe(true);
  });

  it('should interleave scenarios with epic sections', () => {
    mockDocumentGraphService.getNodesByType.and.callFake((type: string) => {
      if (type === 'epic') return [mockEpicNode as any];
      if (type === 'feature') return [mockFeatureNode as any];
      return [];
    });
    mockDocumentGraphService.getNode.and.returnValue(mockScenarioNode as any);

    fixture.detectChanges();

    const hasScenarios = component.interleavedSections.some(s => s.type === 'scenario');
    expect(hasScenarios).toBe(true);
  });

  it('should return correct CSS class for step keywords', () => {
    expect(component.getStepKeywordClass('Given')).toBe('step-given');
    expect(component.getStepKeywordClass('When')).toBe('step-when');
    expect(component.getStepKeywordClass('Then')).toBe('step-then');
    expect(component.getStepKeywordClass('And')).toBe('step-and');
    expect(component.getStepKeywordClass('But')).toBe('step-and');
  });

  it('should handle missing epic gracefully', () => {
    mockDocumentGraphService.getNodesByType.and.returnValue([]);

    fixture.detectChanges();

    expect(component.epic).toBeNull();
    expect(component.interleavedSections.length).toBe(0);
  });

  it('should handle missing feature gracefully', () => {
    mockDocumentGraphService.getNodesByType.and.callFake((type: string) => {
      if (type === 'epic') return [mockEpicNode as any];
      return [];
    });

    fixture.detectChanges();

    expect(component.feature).toBeNull();
    expect(component.scenarios.length).toBe(0);
  });
});
