import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute } from '@angular/router';
import { provideRouter } from '@angular/router';
import { DomSanitizer } from '@angular/platform-browser';
import { BehaviorSubject } from 'rxjs';
import { ModuleViewerComponent } from './module-viewer.component';
import { DocumentGraphService } from '../../services/document-graph.service';
import { EpicNode, FeatureNode, ScenarioNode, NodeType } from '../../models';

describe('ModuleViewerComponent', () => {
  let component: ModuleViewerComponent;
  let fixture: ComponentFixture<ModuleViewerComponent>;
  let mockDocumentGraphService: jasmine.SpyObj<DocumentGraphService>;
  let paramsSubject: BehaviorSubject<any>;

  const mockEpicNode: Partial<EpicNode> = {
    id: 'elohim-value-scanner-protocol',
    type: NodeType.EPIC,
    title: 'Value Scanner Protocol',
    description: 'Test epic description',
    tags: [],
    sourcePath: '',
    content: '',
    relatedNodeIds: [],
    metadata: {},
    featureIds: ['care-economy'],
    relatedEpicIds: [],
    markdownContent: '## Test Section\n\nTest content here.\n\n### Subsection\n\nMore test content.',
    sections: []
  };

  const mockFeatureNode: Partial<FeatureNode> = {
    id: 'care-economy',
    type: NodeType.FEATURE,
    title: 'Care Economy',
    description: 'Test feature description',
    tags: [],
    sourcePath: '',
    content: '',
    relatedNodeIds: [],
    metadata: {},
    category: 'value-scanner',
    epicIds: ['elohim-value-scanner-protocol'],
    scenarioIds: ['scenario-1'],
    featureDescription: 'Care economy feature',
    gherkinContent: 'Feature: Care Economy'
  };

  const mockScenarioNode: Partial<ScenarioNode> = {
    id: 'scenario-1',
    type: NodeType.SCENARIO,
    title: 'Test Scenario',
    description: 'Test scenario description',
    tags: [],
    sourcePath: '',
    content: '',
    relatedNodeIds: [],
    metadata: {},
    featureId: 'care-economy',
    scenarioType: 'scenario',
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

    // Set up default mock behaviors
    mockDocumentGraphService.getNodesByType.and.callFake((type: string) => {
      if (type === 'epic') return [mockEpicNode as any];
      if (type === 'feature') return [mockFeatureNode as any];
      return [];
    });
    mockDocumentGraphService.getNode.and.returnValue(mockScenarioNode as any);

    paramsSubject = new BehaviorSubject<any>({});

    const mockDomSanitizer = jasmine.createSpyObj('DomSanitizer', ['sanitize']);
    mockDomSanitizer.sanitize.and.returnValue('');

    await TestBed.configureTestingModule({
      imports: [ModuleViewerComponent],
      providers: [
        { provide: DocumentGraphService, useValue: mockDocumentGraphService },
        { provide: ActivatedRoute, useValue: { params: paramsSubject } },
        { provide: DomSanitizer, useValue: mockDomSanitizer },
        provideRouter([])
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(ModuleViewerComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load value-scanner module on init', async () => {
    fixture.detectChanges(); // Initialize component and set up subscription
    paramsSubject.next({ id: 'value-scanner' }); // Emit params to trigger loadModule
    fixture.detectChanges(); // Process changes from loadModule
    await fixture.whenStable();

    expect(component.moduleName).toBe('Value Scanner: Care Economy');
    expect(mockDocumentGraphService.getNodesByType).toHaveBeenCalledWith('epic');
    expect(mockDocumentGraphService.getNodesByType).toHaveBeenCalledWith('feature');
  });

  it('should parse epic sections correctly', async () => {
    fixture.detectChanges(); // Initialize component and set up subscription
    paramsSubject.next({ id: 'value-scanner' }); // Emit params to trigger loadModule
    fixture.detectChanges(); // Process changes from loadModule
    await fixture.whenStable();

    expect(component.interleavedSections.length).toBeGreaterThan(0);
    expect(component.interleavedSections.some(s => s.type === 'epic')).toBe(true);
  });

  it('should interleave scenarios with epic sections', async () => {
    fixture.detectChanges(); // Initialize component and set up subscription
    paramsSubject.next({ id: 'value-scanner' }); // Emit params to trigger loadModule
    fixture.detectChanges(); // Process changes from loadModule
    await fixture.whenStable();

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

  it('should handle missing epic gracefully', async () => {
    mockDocumentGraphService.getNodesByType.and.callFake(() => []);

    fixture.detectChanges(); // Initialize component and set up subscription
    paramsSubject.next({ id: 'value-scanner' }); // Emit params to trigger loadModule
    fixture.detectChanges(); // Process changes from loadModule
    await fixture.whenStable();

    expect(component.epic).toBeNull();
    expect(component.interleavedSections.length).toBe(0);
  });

  it('should handle missing feature gracefully', async () => {
    mockDocumentGraphService.getNodesByType.and.callFake((type: string) => {
      if (type === 'epic') return [mockEpicNode as any];
      return [];
    });

    fixture.detectChanges(); // Initialize component and set up subscription
    paramsSubject.next({ id: 'value-scanner' }); // Emit params to trigger loadModule
    fixture.detectChanges(); // Process changes from loadModule
    await fixture.whenStable();

    expect(component.feature).toBeNull();
    expect(component.scenarios.length).toBe(0);
  });
});
