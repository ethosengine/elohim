import { ComponentFixture, TestBed } from '@angular/core/testing';
import { EpicContentPanesComponent } from './epic-content-panes.component';
import { ActivatedRoute, Router } from '@angular/router';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { BehaviorSubject, of } from 'rxjs';
import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { AffinityCircleComponent } from '../affinity-circle/affinity-circle.component';

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

  const mockEpic = {
    id: 'epic-1',
    title: 'Test Epic',
    description: 'Description',
    featureIds: ['feat-1'],
    relatedEpicIds: ['epic-2'],
    markdownContent: '# Title',
    authors: [],
    version: '1.0',
    category: 'core'
  };

  const mockFeature = {
    id: 'feat-1',
    title: 'Test Feature',
    scenarioIds: ['scen-1'],
    testStatus: { status: 'passed' },
    featureDescription: 'Desc',
    category: 'tech'
  };

  const mockScenario = {
    id: 'scen-1',
    title: 'Test Scenario',
    steps: [],
    testStatus: { status: 'passed' },
    scenarioType: 'e2e'
  };

  const mockRelatedEpic = {
    id: 'epic-2',
    title: 'Related Epic',
    featureIds: [],
    description: 'Desc',
    category: 'core'
  };

  const mockGraph = {
    nodesByType: {
      epics: new Map<string, any>([['epic-1', mockEpic], ['epic-2', mockRelatedEpic]]),
      features: new Map<string, any>([['feat-1', mockFeature]]),
      scenarios: new Map<string, any>([['scen-1', mockScenario]])
    }
  };

  beforeEach(async () => {
    routerSpy = jasmine.createSpyObj('Router', ['navigate']);
    routeParamsSubject = new BehaviorSubject({ id: 'epic-1' });
    graphSubject = new BehaviorSubject(mockGraph as any);
    affinitySubject = new BehaviorSubject({ affinity: { 'epic-1': 0.5 } });

    graphServiceMock = {
      graph$: graphSubject.asObservable()
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
    expect(component.epic).toEqual(mockEpic as any);
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
    component.viewFeature(mockFeature as any);
    expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/content', 'feat-1']);
  });

  it('should navigate to scenario', () => {
    component.viewScenario(mockScenario as any);
    expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/content', 'scen-1']);
  });

  it('should navigate to related epic', () => {
    component.viewRelatedEpic(mockRelatedEpic as any);
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
    const cls = component.getFeatureStatusClass(mockFeature as any);
    expect(cls).toBe('status-passed');
    
    const unknown = component.getFeatureStatusClass({} as any);
    expect(unknown).toBe('status-unknown');
  });
});
