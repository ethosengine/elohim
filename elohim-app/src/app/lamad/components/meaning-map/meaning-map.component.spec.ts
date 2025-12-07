import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { MeaningMapComponent } from './meaning-map.component';
import { provideHttpClient } from '@angular/common/http';
import { Router } from '@angular/router';
import { BehaviorSubject, of } from 'rxjs';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { AffinityTrackingService } from '@app/shared/services/affinity-tracking.service';
import { ContentNode } from '../../models/content-node.model';

describe('MeaningMapComponent', () => {
  let component: MeaningMapComponent;
  let fixture: ComponentFixture<MeaningMapComponent>;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let affinityServiceSpy: jasmine.SpyObj<AffinityTrackingService>;
  let routerSpy: jasmine.SpyObj<Router>;
  let affinitySubject: BehaviorSubject<Map<string, number>>;

  const mockNodes: ContentNode[] = [
    {
      id: 'node-1',
      title: 'Node 1',
      description: 'Description 1',
      contentType: 'epic',
      contentFormat: 'markdown',
      content: 'Content 1',
      tags: ['tag1'],
      relatedNodeIds: [],
      metadata: { category: 'core' }
    },
    {
      id: 'node-2',
      title: 'Node 2',
      description: 'Description 2',
      contentType: 'feature',
      contentFormat: 'markdown',
      content: 'Content 2',
      tags: ['tag2'],
      relatedNodeIds: [],
      metadata: { category: 'core' }
    },
    {
      id: 'node-3',
      title: 'Node 3',
      description: 'Description 3',
      contentType: 'scenario',
      contentFormat: 'markdown',
      content: 'Content 3',
      tags: ['tag3'],
      relatedNodeIds: [],
      metadata: { category: 'deployment' }
    }
  ];

  beforeEach(async () => {
    affinitySubject = new BehaviorSubject(new Map<string, number>());

    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', ['getContentIndex']);
    const affinitySpyObj = jasmine.createSpyObj('AffinityTrackingService', ['getAffinity', 'getStats'], {
      affinity$: affinitySubject.asObservable()
    });
    const routerSpyObj = jasmine.createSpyObj('Router', ['navigate']);

    await TestBed.configureTestingModule({
      imports: [MeaningMapComponent],
      providers: [
        provideHttpClient(),
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: AffinityTrackingService, useValue: affinitySpyObj },
        { provide: Router, useValue: routerSpyObj }
      ]
    }).compileComponents();

    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    affinityServiceSpy = TestBed.inject(AffinityTrackingService) as jasmine.SpyObj<AffinityTrackingService>;
    routerSpy = TestBed.inject(Router) as jasmine.SpyObj<Router>;

    // Default spy returns
    dataLoaderSpy.getContentIndex.and.returnValue(of({ nodes: mockNodes }));
    affinityServiceSpy.getAffinity.and.returnValue(0.5);
    affinityServiceSpy.getStats.and.returnValue({
      totalNodes: 3,
      averageAffinity: 0.5,
      engagedNodes: 2,
      distribution: { unseen: 0, low: 1, medium: 1, high: 1 },
      byCategory: new Map([
        ['core', { category: 'core', nodeCount: 2, engagedCount: 1, averageAffinity: 0.6 }],
        ['deployment', { category: 'deployment', nodeCount: 1, engagedCount: 1, averageAffinity: 0.4 }]
      ]),
      byType: new Map()
    });

    fixture = TestBed.createComponent(MeaningMapComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('initialization', () => {
    it('should load content nodes and build meaning map', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(dataLoaderSpy.getContentIndex).toHaveBeenCalled();
      expect(component.categories.length).toBeGreaterThan(0);
      expect(component.isLoading).toBeFalse();
    }));

    it('should calculate overall stats', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(component.overallStats.totalNodes).toBe(3);
      expect(component.overallStats.averageAffinity).toBe(0.5);
      expect(component.overallStats.engagedCount).toBe(2);
    }));

    it('should group nodes by category', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      const coreCategory = component.categories.find(c => c.name === 'core');
      const deploymentCategory = component.categories.find(c => c.name === 'deployment');

      expect(coreCategory).toBeTruthy();
      expect(coreCategory!.nodes.length).toBe(2);
      expect(deploymentCategory).toBeTruthy();
      expect(deploymentCategory!.nodes.length).toBe(1);
    }));

    it('should sort categories by average affinity', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      // deployment has lower affinity (0.4) than core (0.6)
      expect(component.categories[0].name).toBe('deployment');
    }));
  });

  describe('getAffinityLevel', () => {
    it('should return unseen for 0', () => {
      expect(component.getAffinityLevel(0)).toBe('unseen');
    });

    it('should return low for <= 0.33', () => {
      expect(component.getAffinityLevel(0.2)).toBe('low');
      expect(component.getAffinityLevel(0.33)).toBe('low');
    });

    it('should return medium for <= 0.66', () => {
      expect(component.getAffinityLevel(0.5)).toBe('medium');
      expect(component.getAffinityLevel(0.66)).toBe('medium');
    });

    it('should return high for > 0.66', () => {
      expect(component.getAffinityLevel(0.8)).toBe('high');
      expect(component.getAffinityLevel(1.0)).toBe('high');
    });
  });

  describe('toggleCategory', () => {
    it('should toggle category expansion', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      const category = component.categories[0];
      expect(category.expanded).toBeTrue();

      component.toggleCategory(category);
      expect(category.expanded).toBeFalse();

      component.toggleCategory(category);
      expect(category.expanded).toBeTrue();
    }));
  });

  describe('viewContent', () => {
    it('should navigate to content viewer', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      const node = { ...mockNodes[0], affinity: 0.5, affinityLevel: 'medium' as const };
      component.viewContent(node);

      expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/content', 'node-1']);
    }));
  });

  describe('getAffinityColorClass', () => {
    it('should return correct class for each level', () => {
      expect(component.getAffinityColorClass('unseen')).toBe('affinity-unseen');
      expect(component.getAffinityColorClass('low')).toBe('affinity-low');
      expect(component.getAffinityColorClass('medium')).toBe('affinity-medium');
      expect(component.getAffinityColorClass('high')).toBe('affinity-high');
    });
  });

  describe('getAffinityPercentage', () => {
    it('should return rounded percentage', () => {
      expect(component.getAffinityPercentage(0.756)).toBe(76);
      expect(component.getAffinityPercentage(0.5)).toBe(50);
      expect(component.getAffinityPercentage(0)).toBe(0);
    });
  });

  describe('getContentTypeDisplay', () => {
    it('should return display names for known types', () => {
      expect(component.getContentTypeDisplay('epic')).toBe('Epic');
      expect(component.getContentTypeDisplay('feature')).toBe('Feature');
      expect(component.getContentTypeDisplay('scenario')).toBe('Scenario');
    });

    it('should return raw type for unknown types', () => {
      expect(component.getContentTypeDisplay('unknown')).toBe('unknown');
    });
  });

  describe('getContentTypeIcon', () => {
    it('should return icons for known types', () => {
      expect(component.getContentTypeIcon('epic')).toBe('📖');
      expect(component.getContentTypeIcon('feature')).toBe('⚙️');
      expect(component.getContentTypeIcon('scenario')).toBe('✓');
    });

    it('should return default icon for unknown types', () => {
      expect(component.getContentTypeIcon('unknown')).toBe('•');
    });
  });

  describe('cleanup', () => {
    it('should clean up on destroy', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      component.ngOnDestroy();

      // Should not throw errors
      expect(true).toBeTrue();
    }));
  });

  describe('uncategorized nodes', () => {
    it('should group nodes without category as uncategorized', fakeAsync(() => {
      const nodesWithoutCategory: ContentNode[] = [
        {
          id: 'node-4',
          title: 'Node 4',
          description: 'No category',
          contentType: 'concept',
          contentFormat: 'markdown',
          content: 'Content',
          tags: [],
          relatedNodeIds: [],
          metadata: {}
        }
      ];

      dataLoaderSpy.getContentIndex.and.returnValue(of({ nodes: nodesWithoutCategory }));
      affinityServiceSpy.getStats.and.returnValue({
        totalNodes: 1,
        averageAffinity: 0,
        engagedNodes: 0,
        distribution: { unseen: 1, low: 0, medium: 0, high: 0 },
        byCategory: new Map([['uncategorized', { category: 'uncategorized', nodeCount: 1, engagedCount: 0, averageAffinity: 0 }]]),
        byType: new Map()
      });

      fixture.detectChanges();
      tick();

      const uncategorized = component.categories.find(c => c.name === 'uncategorized');
      expect(uncategorized).toBeTruthy();
      expect(uncategorized!.displayName).toBe('Other');
    }));
  });
});
