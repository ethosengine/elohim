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
import { GovernanceSignalService } from '@app/elohim/services/governance-signal.service';
import { vi, Mock } from 'vitest';

describe('ContentViewerComponent', () => {
  let component: ContentViewerComponent;
  let fixture: ComponentFixture<ContentViewerComponent>;
  let affinityServiceSpy: any;
  let agentServiceSpy: any;
  let contentServiceSpy: any;
  let dataLoaderSpy: any;
  let trustBadgeServiceSpy: any;
  let governanceServiceSpy: any;
  let editorServiceSpy: any;
  let pathContextServiceSpy: any;
  let rendererRegistrySpy: any;
  let routerSpy: any;
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

    const affinitySpyObj = {
      getAffinity: vi.fn().mockReturnValue(0),
      trackView: vi.fn(),
      incrementAffinity: vi.fn(),
      setAffinity: vi.fn(),
      changes$: affinityChangesSubject.asObservable(),
      affinity$: of({}),
      getStats: vi.fn().mockReturnValue({ totalNodes: 0, averageAffinity: 0, engagedNodes: 0, distribution: {}, byCategory: new Map(), byType: new Map() }),
    };

    const agentSpyObj = { markContentSeen: vi.fn().mockReturnValue(of(undefined)) };
    const contentSpyObj = { getContainingPathsSummary: vi.fn().mockReturnValue(of([])) };
    const dataLoaderSpyObj = {
      getContent: vi.fn().mockReturnValue(of(mockContentNode)),
      getGovernanceState: vi.fn().mockReturnValue(of(null)),
    };
    const trustBadgeSpyObj = { getBadge: vi.fn().mockReturnValue(of(null)) };
    const governanceSpyObj = {
      getGovernanceState: vi.fn().mockReturnValue(of(null)),
      getChallengesForEntity: vi.fn().mockReturnValue(of([])),
      getDiscussionsForEntity: vi.fn().mockReturnValue(of([])),
    };
    const editorSpyObj = { canEdit: vi.fn().mockReturnValue(false) };
    const pathContextSpyObj = {
      startDetour: vi.fn(),
      returnToPath: vi.fn(),
      context$: pathContextSubject.asObservable(),
    };
    const rendererRegistrySpyObj = { getRenderer: vi.fn().mockReturnValue(null) };
    // Use a real Router (from provideRouter) so RouterLink directives work;
    // routerSpy is set after inject below
    const routerSpyObj = null;
    const seoServiceSpyObj = {
      updateForContent: vi.fn(),
      updateSeo: vi.fn(),
      setTitle: vi.fn(),
    };
    const emptyReactionCounts = { total: 0, byType: {}, supportive: 0, critical: 0, neutral: 0 };
    const governanceSignalSpyObj = {
      changes$: of(null),
      signalChanges$: of(null),
      onEntityUpdate: vi.fn().mockReturnValue(of(null)),
      getContentSignals: vi.fn().mockReturnValue(of(null)),
      getReactionCounts: vi.fn().mockReturnValue(of(emptyReactionCounts)),
      getReactions: vi.fn().mockReturnValue(of([])),
      recordReaction: vi.fn().mockReturnValue(of(true)),
      recordMediationProceed: vi.fn().mockReturnValue(of(true)),
      recordInteractiveCompletion: vi.fn().mockReturnValue(of(true)),
      checkAttestationTrigger: vi.fn().mockReturnValue(of(null)),
      recordLearningSignal: vi.fn().mockReturnValue(of(true)),
      getGraduatedFeedback: vi.fn().mockReturnValue(of([])),
      recordGraduatedFeedback: vi.fn().mockReturnValue(of(true)),
      getFeedbackStats: vi.fn().mockReturnValue(of(null)),
    };

    await TestBed.configureTestingModule({
      imports: [ContentViewerComponent],
      providers: [
        provideHttpClient(),
        provideRouter([]),
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
        { provide: GovernanceSignalService, useValue: governanceSignalSpyObj },
        {
          provide: ActivatedRoute,
          useValue: {
            snapshot: { paramMap: { get: vi.fn().mockReturnValue('test-content-1') } },
            paramMap: of({ get: (k: string) => k === 'id' ? 'test-content-1' : null }),
            params: of({ resourceId: 'test-content-1' }),
            queryParams: of({}),
          },
        },
      ],
    }).compileComponents();

    affinityServiceSpy = TestBed.inject(AffinityTrackingService);
    agentServiceSpy = TestBed.inject(AgentService);
    contentServiceSpy = TestBed.inject(ContentService);
    dataLoaderSpy = TestBed.inject(DataLoaderService);
    trustBadgeServiceSpy = TestBed.inject(TrustBadgeService);
    governanceServiceSpy = TestBed.inject(GovernanceService);
    editorServiceSpy = TestBed.inject(ContentEditorService);
    pathContextServiceSpy = TestBed.inject(PathContextService);
    rendererRegistrySpy = TestBed.inject(RendererRegistryService);
    routerSpy = TestBed.inject(Router);
    vi.spyOn(routerSpy, 'navigate').mockResolvedValue(true);

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
      expect(component.isLoading).toBe(false);
    }));

    it('should track view on content load', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(affinityServiceSpy.trackView).toHaveBeenCalledWith('test-content-1');
    }));

    it('should load related nodes', fakeAsync(() => {
      dataLoaderSpy.getContent.mockImplementation((id: string) => {
        if (id === 'test-content-1') return of(mockContentNode);
        if (id === 'related-1') return of(mockRelatedNode);
        return of(null as any);
      });

      fixture.detectChanges();
      tick();

      expect(component.relatedNodes.length).toBe(1);
    }));

    it('should handle content load error', fakeAsync(() => {
      dataLoaderSpy.getContent.mockReturnValue(throwError(() => new Error('Load failed')));

      fixture.detectChanges();
      tick();

      expect(component.error).toBe('Failed to load content');
      expect(component.isLoading).toBe(false);
    }));

    it('should handle content not found', fakeAsync(() => {
      dataLoaderSpy.getContent.mockReturnValue(of(null as any));

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
      affinityServiceSpy.getAffinity.mockReturnValue(0.65);
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
      component.handleAction({ route: '/some/route' });
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/some/route']);
    });

    it('should handle action without route gracefully', () => {
      // Actions without routes are no-ops
      expect(() => component.handleAction({})).not.toThrow();
      expect(routerSpy.navigate).not.toHaveBeenCalled();
    });
  });

  describe('containing paths', () => {
    it('should load containing paths', fakeAsync(() => {
      const mockPaths = [{ pathId: 'path-1', pathTitle: 'Path 1', stepIndex: 0 }];
      contentServiceSpy.getContainingPathsSummary.mockReturnValue(of(mockPaths));

      fixture.detectChanges();
      tick();

      expect(component.containingPaths).toEqual(mockPaths);
      expect(component.loadingPaths).toBe(false);
    }));
  });

  describe('cleanup', () => {
    it('should clean up on destroy', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      component.ngOnDestroy();

      // Should not throw errors
      expect(true).toBe(true);
    }));
  });

  describe('governance features', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should get governance status label', () => {
      component.governanceState = { status: 'community-reviewed' } as any;
      expect(component.getGovernanceStatusLabel()).toBe('Community Reviewed');

      component.governanceState = { status: 'challenged' } as any;
      expect(component.getGovernanceStatusLabel()).toBe('Under Challenge');

      component.governanceState = null;
      expect(component.getGovernanceStatusLabel()).toBe('Unreviewed');
    });

    it('should get governance status icon', () => {
      component.governanceState = { status: 'elohim-reviewed' } as any;
      expect(component.getGovernanceStatusIcon()).toBe('✓');

      component.governanceState = { status: 'challenged' } as any;
      expect(component.getGovernanceStatusIcon()).toBe('⚠️');

      component.governanceState = null;
      expect(component.getGovernanceStatusIcon()).toBe('❓');
    });

    it('should get SLA status for challenge', () => {
      const futureDate = new Date(Date.now() + 10 * 24 * 60 * 60 * 1000).toISOString(); // 10 days
      const nearDate = new Date(Date.now() + 2 * 24 * 60 * 60 * 1000).toISOString(); // 2 days
      const pastDate = new Date(Date.now() - 1 * 24 * 60 * 60 * 1000).toISOString(); // -1 day

      expect(component.getSlaStatus({ slaDeadline: futureDate } as any)).toBe('sla-on-track');
      expect(component.getSlaStatus({ slaDeadline: nearDate } as any)).toBe('sla-warning');
      expect(component.getSlaStatus({ slaDeadline: pastDate } as any)).toBe('sla-breached');
      expect(component.getSlaStatus({} as any)).toBe('unknown');
    });

    it('should get days remaining until deadline', () => {
      const futureDate = new Date(Date.now() + 5 * 24 * 60 * 60 * 1000).toISOString();
      const daysRemaining = component.getDaysRemaining(futureDate);
      expect(daysRemaining).toBeGreaterThanOrEqual(4);
      expect(daysRemaining).toBeLessThanOrEqual(6);

      expect(component.getDaysRemaining(undefined)).toBe(-1);
    });

    it('should format governance date', () => {
      const isoDate = '2025-01-15T10:30:00.000Z';
      const formatted = component.formatGovernanceDate(isoDate);
      expect(formatted).toContain('Jan');
      expect(formatted).toContain('15');
      expect(formatted).toContain('2025');
    });

    it('should handle invalid governance date', () => {
      expect(component.formatGovernanceDate('invalid-date')).toBe('Invalid Date');
      expect(component.formatGovernanceDate(undefined)).toBe('Unknown');
    });
  });

  describe('feedback profile mapping', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should map content types to feedback profiles', () => {
      expect(component.mapContentTypeToProfileType('epic')).toBe('learning-content');
      expect(component.mapContentTypeToProfileType('tutorial')).toBe('learning-content');
      expect(component.mapContentTypeToProfileType('research')).toBe('research-content');
      expect(component.mapContentTypeToProfileType('testimony')).toBe('personal-testimony');
      expect(component.mapContentTypeToProfileType('proposal')).toBe('governance-proposal');
      expect(component.mapContentTypeToProfileType('unknown')).toBe('learning-content');
    });
  });

  describe('path context and detours', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should track path context from service', fakeAsync(() => {
      const pathContext = {
        pathId: 'test-path',
        pathTitle: 'Test Path',
        stepIndex: 2,
        totalSteps: 10,
        returnRoute: ['path', 'test-path'],
        detourStack: [{ fromContentId: 'node-1', toContentId: 'test-content-1', detourType: 'graph-explore' as const, timestamp: new Date().toISOString() }],
      };

      pathContextSubject.next(pathContext);
      tick();

      expect(component.pathContext).toEqual(pathContext);
      expect(component.hasReturnPath).toBe(true);
    }));

    it('should detect no return path when detour stack is empty', fakeAsync(() => {
      const pathContext = {
        pathId: 'test-path',
        pathTitle: 'Test Path',
        stepIndex: 2,
        totalSteps: 10,
        detourStack: [],
      };

      pathContextSubject.next(pathContext);
      tick();

      expect(component.hasReturnPath).toBe(false);
    }));

    it('should return to path when hasReturnPath', () => {
      component.pathContext = {
        pathId: 'test-path',
        pathTitle: 'Test Path',
        stepIndex: 2,
        totalSteps: 10,
        returnRoute: ['/lamad/path', 'test-path', 'step', '2'],
        detourStack: [],
      };
      pathContextServiceSpy.returnToPath.mockReturnValue(['/lamad/path', 'test-path', 'step', '2']);

      component.returnToPath();

      expect(pathContextServiceSpy.returnToPath).toHaveBeenCalled();
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/path', 'test-path', 'step', '2']);
    });

    it('should not navigate if no return path', () => {
      component.pathContext = null;
      pathContextServiceSpy.returnToPath.mockReturnValue(null);

      component.returnToPath();

      expect(routerSpy.navigate).not.toHaveBeenCalled();
    });

    it('should track detour when selecting graph node', () => {
      component.pathContext = {
        pathId: 'test-path',
        pathTitle: 'Test Path',
        stepIndex: 2,
        totalSteps: 10,
        returnRoute: ['path', 'test-path'],
        detourStack: [],
      };
      component['nodeId'] = 'current-node';

      component.onGraphNodeSelected('related-node');

      expect(pathContextServiceSpy.startDetour).toHaveBeenCalledWith(
        expect.objectContaining({
          fromContentId: 'current-node',
          toContentId: 'related-node',
          detourType: 'related',
        })
      );
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/resource', 'related-node']);
    });

    it('should track detour when exploring in graph', () => {
      component.pathContext = {
        pathId: 'test-path',
        pathTitle: 'Test Path',
        stepIndex: 2,
        totalSteps: 10,
        returnRoute: ['path', 'test-path'],
        detourStack: [],
      };
      component['nodeId'] = 'current-node';

      component.exploreInGraph();

      expect(pathContextServiceSpy.startDetour).toHaveBeenCalledWith(
        expect.objectContaining({
          fromContentId: 'current-node',
          toContentId: 'current-node',
          detourType: 'graph-explore',
        })
      );
      expect(routerSpy.navigate).toHaveBeenCalledWith(
        ['/lamad/explore'],
        expect.objectContaining({
          queryParams: expect.objectContaining({
            focus: 'current-node',
            fromPath: 'test-path',
            returnStep: 2,
          }),
        })
      );
    });

    it('should not track detour if no path context', () => {
      component.pathContext = null;
      component['nodeId'] = 'current-node';

      component.onGraphNodeSelected('related-node');

      expect(pathContextServiceSpy.startDetour).not.toHaveBeenCalled();
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/resource', 'related-node']);
    });

    it('should not explore in graph if no nodeId', () => {
      component['nodeId'] = null;

      component.exploreInGraph();

      expect(routerSpy.navigate).not.toHaveBeenCalled();
    });
  });

  describe('focused view mode', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should toggle focused view on', () => {
      expect(component.isFocusedView).toBe(false);

      component.onFocusedViewToggle(true);

      expect(component.isFocusedView).toBe(true);
    });

    it('should toggle focused view off', () => {
      component.isFocusedView = true;

      component.onFocusedViewToggle(false);

      expect(component.isFocusedView).toBe(false);
    });

    it('should exit focused view on escape key', () => {
      component.isFocusedView = true;

      component.onEscapeKey();

      expect(component.isFocusedView).toBe(false);
    });

    it('should not exit if not in focused view', () => {
      component.isFocusedView = false;
      vi.spyOn(component, 'onFocusedViewToggle');

      component.onEscapeKey();

      expect(component.onFocusedViewToggle).not.toHaveBeenCalled();
    });
  });

  describe('renderer completion events', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
      component['nodeId'] = 'test-content-1';
    }));

    it('should handle renderer completion with passing score', () => {
      const event = {
        type: 'quiz',
        passed: true,
        score: 85,
        details: { attempts: 1 },
      };

      (component as any).onRendererComplete(event);

      expect(affinityServiceSpy.incrementAffinity).toHaveBeenCalledWith(
        'test-content-1',
        expect.any(Number)
      );
    });

    it('should handle renderer completion with failing score', () => {
      const event = {
        type: 'quiz',
        passed: false,
        score: 45,
        details: { attempts: 2 },
      };

      (component as any).onRendererComplete(event);

      expect(affinityServiceSpy.incrementAffinity).toHaveBeenCalledWith('test-content-1', 0.1);
    });

    it('should not handle completion if no nodeId', () => {
      component['nodeId'] = null;
      const event = {
        type: 'quiz',
        passed: true,
        score: 90,
        details: {},
      };

      (component as any).onRendererComplete(event);

      expect(affinityServiceSpy.incrementAffinity).not.toHaveBeenCalled();
    });
  });

  describe('loading states and errors', () => {
    it('should handle related nodes load error', fakeAsync(() => {
      dataLoaderSpy.getContent.mockImplementation((id: string) => {
        if (id === 'test-content-1') return of(mockContentNode);
        return throwError(() => new Error('Failed'));
      });

      fixture.detectChanges();
      tick();

      expect(component.relatedNodes).toEqual([]);
    }));

    it('should handle containing paths load error', fakeAsync(() => {
      contentServiceSpy.getContainingPathsSummary.mockReturnValue(
        throwError(() => new Error('Failed'))
      );

      fixture.detectChanges();
      tick();

      expect(component.loadingPaths).toBe(false);
      expect(component.containingPaths).toEqual([]);
    }));

    it('should handle trust badge load error', fakeAsync(() => {
      trustBadgeServiceSpy.getBadge.mockReturnValue(throwError(() => new Error('Failed')));

      fixture.detectChanges();
      tick();

      expect(component.isLoadingTrust).toBe(false);
      expect(component.trustBadge).toBeNull();
    }));

    it('should handle governance data load errors gracefully', fakeAsync(() => {
      governanceServiceSpy.getGovernanceState.mockReturnValue(
        throwError(() => new Error('Failed'))
      );
      governanceServiceSpy.getChallengesForEntity.mockReturnValue(
        throwError(() => new Error('Failed'))
      );

      fixture.detectChanges();
      tick();

      expect(component.isLoadingGovernance).toBe(false);
    }));
  });

  describe('empty related nodes', () => {
    it('should handle content with no related nodes', fakeAsync(() => {
      const nodeWithoutRelated = { ...mockContentNode, relatedNodeIds: [] };
      dataLoaderSpy.getContent.mockReturnValue(of(nodeWithoutRelated));

      fixture.detectChanges();
      tick();

      expect(component.relatedNodes).toEqual([]);
    }));

    it('should handle content with null related node IDs', fakeAsync(() => {
      const nodeWithNullRelated = { ...mockContentNode, relatedNodeIds: null as any };
      dataLoaderSpy.getContent.mockReturnValue(of(nodeWithNullRelated));

      fixture.detectChanges();
      tick();

      expect(component.relatedNodes).toEqual([]);
    }));

    it('should limit related nodes to 5', fakeAsync(() => {
      const nodeWithManyRelated = {
        ...mockContentNode,
        relatedNodeIds: ['r1', 'r2', 'r3', 'r4', 'r5', 'r6', 'r7'],
      };
      dataLoaderSpy.getContent.mockImplementation((id: string) => {
        if (id === 'test-content-1') return of(nodeWithManyRelated);
        return of(mockRelatedNode);
      });

      fixture.detectChanges();
      tick();

      // Should call getContent for max 5 related nodes + 1 for main content
      expect(dataLoaderSpy.getContent).toHaveBeenCalledTimes(6);
    }));
  });

  describe('content editor capability', () => {
    it('should check edit capability on content load', fakeAsync(() => {
      editorServiceSpy.canEdit.mockReturnValue(true);

      fixture.detectChanges();
      tick();

      expect(component.canEditContent).toBe(true);
      expect(editorServiceSpy.canEdit).toHaveBeenCalledWith(mockContentNode);
    }));

    it('should set canEdit to false when user cannot edit', fakeAsync(() => {
      editorServiceSpy.canEdit.mockReturnValue(false);

      fixture.detectChanges();
      tick();

      expect(component.canEditContent).toBe(false);
    }));
  });
});
