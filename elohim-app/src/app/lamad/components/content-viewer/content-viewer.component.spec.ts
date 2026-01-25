import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { ContentViewerComponent } from './content-viewer.component';
import { ActivatedRoute, Router, provideRouter } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { of, Subject, throwError } from 'rxjs';
import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { ContentService } from '../../services/content.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { TrustBadgeService } from '@app/elohim/services/trust-badge.service';
import { GovernanceService } from '@app/elohim/services/governance.service';
import { ContentEditorService } from '../../content-io/services/content-editor.service';
import { PathContextService } from '../../services/path-context.service';
import { SeoService } from '../../../services/seo.service';
import { RendererRegistryService } from '../../renderers/renderer-registry.service';
import { ContentNode } from '../../models/content-node.model';

describe('ContentViewerComponent', () => {
  let component: ContentViewerComponent;
  let fixture: ComponentFixture<ContentViewerComponent>;
  let affinityServiceSpy: jasmine.SpyObj<AffinityTrackingService>;
  let agentServiceSpy: jasmine.SpyObj<AgentService>;
  let contentServiceSpy: jasmine.SpyObj<ContentService>;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let trustBadgeServiceSpy: jasmine.SpyObj<TrustBadgeService>;
  let governanceServiceSpy: jasmine.SpyObj<GovernanceService>;
  let editorServiceSpy: jasmine.SpyObj<ContentEditorService>;
  let pathContextServiceSpy: jasmine.SpyObj<PathContextService>;
  let rendererRegistrySpy: jasmine.SpyObj<RendererRegistryService>;
  let routerSpy: jasmine.SpyObj<Router>;
  let affinityChangesSubject: Subject<any>;
  let pathContextSubject: Subject<any>;

  const mockContentNode: ContentNode = {
    id: 'test-content-1',
    title: 'Test Content',
    description: 'Test description',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Test Content',
    tags: ['test'],
    relatedNodeIds: ['related-1'],
    metadata: { category: 'test-category', authors: ['Author 1'], version: '1.0' },
  };

  const mockRelatedNode: ContentNode = {
    id: 'related-1',
    title: 'Related Content',
    description: 'Related description',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Related',
    tags: ['related'],
    relatedNodeIds: [],
    metadata: {},
  };

  beforeEach(async () => {
    affinityChangesSubject = new Subject();
    pathContextSubject = new Subject();

    const affinitySpyObj = jasmine.createSpyObj(
      'AffinityTrackingService',
      ['getAffinity', 'trackView', 'incrementAffinity', 'setAffinity'],
      { changes$: affinityChangesSubject.asObservable() }
    );

    const agentSpyObj = jasmine.createSpyObj('AgentService', ['markContentSeen']);
    const contentSpyObj = jasmine.createSpyObj('ContentService', ['getContainingPathsSummary']);
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', [
      'getContent',
      'getGovernanceState',
    ]);
    const trustBadgeSpyObj = jasmine.createSpyObj('TrustBadgeService', ['getBadge']);
    const governanceSpyObj = jasmine.createSpyObj('GovernanceService', [
      'getGovernanceState',
      'getChallengesForEntity',
      'getDiscussionsForEntity',
    ]);
    const editorSpyObj = jasmine.createSpyObj('ContentEditorService', ['canEdit']);
    const pathContextSpyObj = jasmine.createSpyObj(
      'PathContextService',
      ['startDetour', 'returnToPath'],
      { context$: pathContextSubject.asObservable() }
    );
    const rendererRegistrySpyObj = jasmine.createSpyObj('RendererRegistryService', ['getRenderer']);
    const routerSpyObj = jasmine.createSpyObj('Router', ['navigate']);
    const seoServiceSpyObj = jasmine.createSpyObj('SeoService', [
      'updateForContent',
      'updateSeo',
      'setTitle',
    ]);

    await TestBed.configureTestingModule({
      imports: [ContentViewerComponent],
      providers: [
        provideHttpClient(),
        provideRouter([]), // Provide empty routes for RouterLink support
        {
          provide: ActivatedRoute,
          useValue: {
            params: of({ resourceId: 'test-content-1' }),
          },
        },
        { provide: AffinityTrackingService, useValue: affinitySpyObj },
        { provide: AgentService, useValue: agentSpyObj },
        { provide: ContentService, useValue: contentSpyObj },
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: TrustBadgeService, useValue: trustBadgeSpyObj },
        { provide: GovernanceService, useValue: governanceSpyObj },
        { provide: ContentEditorService, useValue: editorSpyObj },
        { provide: PathContextService, useValue: pathContextSpyObj },
        { provide: RendererRegistryService, useValue: rendererRegistrySpyObj },
        { provide: SeoService, useValue: seoServiceSpyObj },
      ],
    }).compileComponents();

    affinityServiceSpy = TestBed.inject(
      AffinityTrackingService
    ) as jasmine.SpyObj<AffinityTrackingService>;
    agentServiceSpy = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;
    contentServiceSpy = TestBed.inject(ContentService) as jasmine.SpyObj<ContentService>;
    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    trustBadgeServiceSpy = TestBed.inject(TrustBadgeService) as jasmine.SpyObj<TrustBadgeService>;
    governanceServiceSpy = TestBed.inject(GovernanceService) as jasmine.SpyObj<GovernanceService>;
    editorServiceSpy = TestBed.inject(ContentEditorService) as jasmine.SpyObj<ContentEditorService>;
    pathContextServiceSpy = TestBed.inject(
      PathContextService
    ) as jasmine.SpyObj<PathContextService>;
    rendererRegistrySpy = TestBed.inject(
      RendererRegistryService
    ) as jasmine.SpyObj<RendererRegistryService>;
    routerSpy = TestBed.inject(Router) as any; // Use real router from provideRouter
    spyOn(routerSpy, 'navigate');

    // Default spy returns
    affinityServiceSpy.getAffinity.and.returnValue(0.5);
    agentServiceSpy.markContentSeen.and.returnValue(of(undefined));
    dataLoaderSpy.getContent.and.returnValue(of(mockContentNode));
    dataLoaderSpy.getGovernanceState.and.returnValue(of(null));
    contentServiceSpy.getContainingPathsSummary.and.returnValue(of([]));
    trustBadgeServiceSpy.getBadge.and.returnValue(of(null as any));
    governanceServiceSpy.getGovernanceState.and.returnValue(of(null));
    governanceServiceSpy.getChallengesForEntity.and.returnValue(of([]));
    governanceServiceSpy.getDiscussionsForEntity.and.returnValue(of([]));
    editorServiceSpy.canEdit.and.returnValue(false);
    rendererRegistrySpy.getRenderer.and.returnValue(null);

    fixture = TestBed.createComponent(ContentViewerComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('initialization', () => {
    it('should load content on init', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(dataLoaderSpy.getContent).toHaveBeenCalledWith('test-content-1');
      expect(component.node).toEqual(mockContentNode);
      expect(component.isLoading).toBeFalse();
    }));

    it('should track view on content load', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(affinityServiceSpy.trackView).toHaveBeenCalledWith('test-content-1');
    }));

    it('should load related nodes', fakeAsync(() => {
      dataLoaderSpy.getContent.and.callFake((id: string) => {
        if (id === 'test-content-1') return of(mockContentNode);
        if (id === 'related-1') return of(mockRelatedNode);
        return of(null as any);
      });

      fixture.detectChanges();
      tick();

      expect(component.relatedNodes.length).toBe(1);
    }));

    it('should handle content load error', fakeAsync(() => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Load failed')));

      fixture.detectChanges();
      tick();

      expect(component.error).toBe('Failed to load content');
      expect(component.isLoading).toBeFalse();
    }));

    it('should handle content not found', fakeAsync(() => {
      dataLoaderSpy.getContent.and.returnValue(of(null as any));

      fixture.detectChanges();
      tick();

      expect(component.error).toBe('Content not found');
    }));
  });

  describe('affinity tracking', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should update affinity on changes', fakeAsync(() => {
      affinityChangesSubject.next({ nodeId: 'test-content-1', newValue: 0.8 });
      tick();

      expect(component.affinity).toBe(0.8);
    }));

    it('should ignore affinity changes for other nodes', fakeAsync(() => {
      component.affinity = 0.5;
      affinityChangesSubject.next({ nodeId: 'other-node', newValue: 0.9 });
      tick();

      expect(component.affinity).toBe(0.5);
    }));

    it('should adjust affinity', () => {
      component.adjustAffinity(0.1);
      expect(affinityServiceSpy.incrementAffinity).toHaveBeenCalledWith('test-content-1', 0.1);
    });

    it('should set affinity', () => {
      component.setAffinity(0.75);
      expect(affinityServiceSpy.setAffinity).toHaveBeenCalledWith('test-content-1', 0.75);
    });
  });

  describe('tabs', () => {
    it('should start on content tab', () => {
      expect(component.activeTab).toBe('content');
    });

    it('should switch tabs', () => {
      component.setActiveTab('trust');
      expect(component.activeTab).toBe('trust');

      component.setActiveTab('governance');
      expect(component.activeTab).toBe('governance');

      component.setActiveTab('network');
      expect(component.activeTab).toBe('network');
    });
  });

  describe('navigation', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should navigate to related content', () => {
      component.viewRelatedContent(mockRelatedNode);
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/content', 'related-1']);
    });

    it('should navigate to path', () => {
      component.navigateToPath('path-1', 2);
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/path', 'path-1', 'step', 2]);
    });

    it('should navigate back to home', () => {
      component.backToHome();
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad']);
    });
  });

  describe('affinity level', () => {
    it('should return unseen for 0 affinity', () => {
      component.affinity = 0;
      expect(component.getAffinityLevel()).toBe('unseen');
    });

    it('should return low for affinity <= 0.33', () => {
      component.affinity = 0.2;
      expect(component.getAffinityLevel()).toBe('low');
    });

    it('should return medium for affinity <= 0.66', () => {
      component.affinity = 0.5;
      expect(component.getAffinityLevel()).toBe('medium');
    });

    it('should return high for affinity > 0.66', () => {
      component.affinity = 0.8;
      expect(component.getAffinityLevel()).toBe('high');
    });
  });

  describe('affinity percentage', () => {
    it('should calculate percentage correctly', () => {
      component.affinity = 0.75;
      expect(component.getAffinityPercentage()).toBe(75);
    });

    it('should round percentage', () => {
      component.affinity = 0.333;
      expect(component.getAffinityPercentage()).toBe(33);
    });
  });

  describe('content type display', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should return empty string when no node', () => {
      component.node = null;
      expect(component.getContentTypeDisplay()).toBe('');
    });

    it('should return display name for known types', () => {
      component.node = { ...mockContentNode, contentType: 'epic' };
      expect(component.getContentTypeDisplay()).toBe('Epic');

      component.node = { ...mockContentNode, contentType: 'feature' };
      expect(component.getContentTypeDisplay()).toBe('Feature');

      component.node = { ...mockContentNode, contentType: 'scenario' };
      expect(component.getContentTypeDisplay()).toBe('Scenario');
    });

    it('should return raw type for unknown types', () => {
      component.node = { ...mockContentNode, contentType: 'unknown' as any };
      expect(component.getContentTypeDisplay()).toBe('unknown');
    });
  });

  describe('content type icon', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should return empty string when no node', () => {
      component.node = null;
      expect(component.getContentTypeIcon()).toBe('');
    });

    it('should return icon for known types', () => {
      component.node = { ...mockContentNode, contentType: 'epic' };
      expect(component.getContentTypeIcon()).toBe('📖');
    });

    it('should return default icon for unknown types', () => {
      component.node = { ...mockContentNode, contentType: 'unknown' as any };
      expect(component.getContentTypeIcon()).toBe('📄');
    });
  });

  describe('getStringContent', () => {
    it('should return string content as-is', () => {
      expect(component.getStringContent('test content')).toBe('test content');
    });

    it('should stringify object content', () => {
      const obj = { key: 'value' };
      const result = component.getStringContent(obj);
      expect(result).toContain('"key"');
      expect(result).toContain('"value"');
    });
  });

  describe('related node affinity', () => {
    it('should get affinity for related node', () => {
      affinityServiceSpy.getAffinity.and.returnValue(0.65);
      expect(component.getRelatedNodeAffinity('some-node')).toBe(65);
    });
  });

  describe('metadata accessors', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should get metadata category', () => {
      expect(component.getMetadataCategory()).toBe('test-category');
    });

    it('should return null when no category', () => {
      component.node = { ...mockContentNode, metadata: {} };
      expect(component.getMetadataCategory()).toBeNull();
    });

    it('should get metadata authors', () => {
      expect(component.getMetadataAuthors()).toBe('Author 1');
    });

    it('should join multiple authors', () => {
      component.node = { ...mockContentNode, metadata: { authors: ['Author 1', 'Author 2'] } };
      expect(component.getMetadataAuthors()).toBe('Author 1, Author 2');
    });

    it('should return null when no authors', () => {
      component.node = { ...mockContentNode, metadata: {} };
      expect(component.getMetadataAuthors()).toBeNull();
    });

    it('should get metadata version', () => {
      expect(component.getMetadataVersion()).toBe('1.0');
    });

    it('should return null when no version', () => {
      component.node = { ...mockContentNode, metadata: {} };
      expect(component.getMetadataVersion()).toBeNull();
    });
  });

  describe('action handling', () => {
    it('should navigate on action with route', () => {
      component.handleAction({ route: '/some/route', label: 'Action' });
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/some/route']);
    });

    it('should handle action without route gracefully', () => {
      // Actions without routes are no-ops
      expect(() => component.handleAction({ label: 'No Route Action' })).not.toThrow();
      expect(routerSpy.navigate).not.toHaveBeenCalled();
    });
  });

  describe('containing paths', () => {
    it('should load containing paths', fakeAsync(() => {
      const mockPaths = [{ pathId: 'path-1', pathTitle: 'Path 1', stepIndex: 0 }];
      contentServiceSpy.getContainingPathsSummary.and.returnValue(of(mockPaths));

      fixture.detectChanges();
      tick();

      expect(component.containingPaths).toEqual(mockPaths);
      expect(component.loadingPaths).toBeFalse();
    }));
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
});
