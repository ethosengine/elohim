import { ComponentFixture, TestBed } from '@angular/core/testing';
import { EpicContentPanesComponent } from './epic-content-panes.component';
import { ActivatedRoute, Router } from '@angular/router';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { BehaviorSubject, of } from 'rxjs';
import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { AffinityCircleComponent } from '../affinity-circle/affinity-circle.component';
import { ContentNode } from '../../models/content-node.model';

// Mock AffinityCircleComponent
@Component({
  selector: 'app-affinity-circle',
  standalone: true,
  template: ''
})
class MockAffinityCircleComponent {
  @Input() affinity: number = 0;
  @Input() size: number = 100;
}

describe('EpicContentPanesComponent', () => {
  let component: EpicContentPanesComponent;
  let fixture: ComponentFixture<EpicContentPanesComponent>;
  let routerSpy: jasmine.SpyObj<Router>;
  let graphServiceMock: any;
  let affinityServiceMock: any;
  let routeParamsSubject: BehaviorSubject<any>;
  let graphSubject: BehaviorSubject<any>;
  let affinitySubject: BehaviorSubject<any>;

  const mockEpic: ContentNode = {
    id: 'epic-1',
    title: 'Test Epic',
    contentType: 'epic',
    description: 'Description',
    content: '# Title',
    contentFormat: 'markdown',
    tags: [],
    sourcePath: 'epic-1.md',
    relatedNodeIds: ['feat-1', 'epic-2'],
    metadata: {
      featureIds: ['feat-1'],
      relatedEpicIds: ['epic-2'],
      markdownContent: '# Title',
      authors: [],
      version: '1.0',
      category: 'core'
    }
  };

  const mockFeature: ContentNode = {
    id: 'feat-1',
    title: 'Test Feature',
    contentType: 'feature',
    description: 'Desc',
    content: 'Feature content',
    contentFormat: 'gherkin',
    tags: [],
    sourcePath: 'feat-1.feature',
    relatedNodeIds: ['epic-1', 'scen-1'],
    metadata: {
      scenarioIds: ['scen-1'],
      testStatus: { status: 'passed' },
      featureDescription: 'Desc',
      category: 'tech'
    }
  };

  const mockScenario: ContentNode = {
    id: 'scen-1',
    title: 'Test Scenario',
    contentType: 'scenario',
    description: 'Scenario desc',
    content: 'Scenario content',
    contentFormat: 'gherkin',
    tags: [],
    sourcePath: 'feat-1.feature',
    relatedNodeIds: ['feat-1'],
    metadata: {
      steps: [],
      testStatus: { status: 'passed' },
      scenarioType: 'e2e'
    }
  };

  const mockRelatedEpic: ContentNode = {
    id: 'epic-2',
    title: 'Related Epic',
    contentType: 'epic',
    description: 'Desc',
    content: '# Related',
    contentFormat: 'markdown',
    tags: [],
    sourcePath: 'epic-2.md',
    relatedNodeIds: ['epic-1'],
    metadata: {
      featureIds: [],
      category: 'core'
    }
  };

  const mockGraph = {
    nodes: new Map<string, ContentNode>([
      ['epic-1', mockEpic],
      ['epic-2', mockRelatedEpic],
      ['feat-1', mockFeature],
      ['scen-1', mockScenario]
    ]),
    relationships: new Map(),
    adjacency: new Map([
      ['epic-1', new Set(['feat-1', 'epic-2'])],
      ['feat-1', new Set(['epic-1', 'scen-1'])],
      ['scen-1', new Set(['feat-1'])],
      ['epic-2', new Set(['epic-1'])]
    ])
  };

  beforeEach(async () => {
    routerSpy = jasmine.createSpyObj('Router', ['navigate']);
    routeParamsSubject = new BehaviorSubject({ id: 'epic-1' });
    graphSubject = new BehaviorSubject(mockGraph as any);
    affinitySubject = new BehaviorSubject({ affinity: { 'epic-1': 0.5 } });

    graphServiceMock = {
      graph$: graphSubject.asObservable(),
      getRelatedNodes: jasmine.createSpy('getRelatedNodes').and.callFake((nodeId: string) => {
        const relatedIds = mockGraph.adjacency.get(nodeId);
        if (!relatedIds) return [];
        return Array.from(relatedIds).map(id => mockGraph.nodes.get(id)).filter((n): n is ContentNode => n !== undefined);
      })
    };

    affinityServiceMock = {
      affinity$: affinitySubject.asObservable(),
      getAffinity: jasmine.createSpy('getAffinity').and.returnValue(0.5),
      setAffinity: jasmine.createSpy('setAffinity'),
      incrementAffinity: jasmine.createSpy('incrementAffinity')
    };

    await TestBed.configureTestingModule({
      imports: [EpicContentPanesComponent, CommonModule],
      providers: [
        { provide: Router, useValue: routerSpy },
        { provide: ActivatedRoute, useValue: { params: routeParamsSubject.asObservable() } },
        { provide: DocumentGraphService, useValue: graphServiceMock },
        { provide: AffinityTrackingService, useValue: affinityServiceMock }
      ]
    })
    .overrideComponent(EpicContentPanesComponent, {
      remove: { imports: [AffinityCircleComponent] },
      add: { imports: [MockAffinityCircleComponent] }
    })
    .compileComponents();

    fixture = TestBed.createComponent(EpicContentPanesComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load epic and related content', () => {
    expect(component.epic).toEqual(mockEpic);
    expect(component.features.length).toBe(1);
    expect(component.scenarios.length).toBe(1);
    expect(component.relatedEpics.length).toBe(1);
  });

  it('should update tab counts', () => {
    const featureTab = component.tabs.find(t => t.id === 'features');
    expect(featureTab?.count).toBe(1);
  });

  it('should track affinity', () => {
    expect(component.affinity).toBe(0.5);
  });

  it('should switch tabs', () => {
    component.selectTab('features');
    expect(component.activeTab).toBe('features');
  });

  it('should navigate to feature', () => {
    component.viewFeature(mockFeature);
    expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/content', 'feat-1']);
  });

  it('should navigate to scenario', () => {
    component.viewScenario(mockScenario);
    expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/content', 'scen-1']);
  });

  it('should navigate to related epic', () => {
    component.viewRelatedEpic(mockRelatedEpic);
    expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/content', 'epic-2']);
  });

  it('should handle affinity controls', () => {
    component.increaseAffinity();
    expect(affinityServiceMock.incrementAffinity).toHaveBeenCalledWith('epic-1', 0.2);

    component.decreaseAffinity();
    expect(affinityServiceMock.incrementAffinity).toHaveBeenCalledWith('epic-1', -0.2);

    component.markMastered();
    expect(affinityServiceMock.setAffinity).toHaveBeenCalledWith('epic-1', 1.0);
  });

  it('should render markdown', () => {
    const html = component.renderMarkdown('# Hello');
    expect(html).toContain('<h1>Hello</h1>');
  });

  it('should get feature status class', () => {
    const cls = component.getFeatureStatusClass(mockFeature);
    expect(cls).toBe('status-passed');

    const unknownFeature: ContentNode = { ...mockFeature, metadata: {} };
    const unknown = component.getFeatureStatusClass(unknownFeature);
    expect(unknown).toBe('status-unknown');
  });
});
