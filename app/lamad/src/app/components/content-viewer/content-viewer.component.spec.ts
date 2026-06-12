import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { ContentViewerComponent } from './content-viewer.component';
import { ExplorationSidebarComponent } from '../exploration-sidebar/exploration-sidebar.component';
import { ActivatedRoute, Router, provideRouter } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { of, Subject, throwError } from 'rxjs';
import { LAMAD_AGENT, type ILamadAgent } from '../../interfaces/agent.interface';
import {
  LAMAD_AFFINITY_TRACKING,
  LAMAD_EPR_NAV,
  LAMAD_EPR_RESOLVER,
  LAMAD_GOVERNANCE_SIGNAL,
  LAMAD_GOVERNANCE,
} from '../../interfaces/cross-pillar.interface';
import { ContentService } from '../../services/content.service';
import { DataLoaderService } from '../../services/data-loader.service';
import { TrustBadgeService } from '../../services/trust-badge.service';
import { ContentEditorService } from '../../content-io/services/content-editor.service';
import { PathContextService } from '../../services/path-context.service';
import { SeoService } from '../../shared/services/seo.service';
import { RendererRegistryService } from '../../renderers/renderer-registry.service';
import { ContentNode } from '../../models/content-node.model';
import { StewardshipAllocationService } from '../../services/stewardship-allocation.service';
import { SignalHarnessService } from '../../services/signal-harness.service';
import { HouseholdResilienceService } from '../../services/household-resilience.service';
import { ResilienceService as LibResilienceService } from '@elohim/service/public-api';
import { LAMAD_STORAGE_CLIENT } from '../../interfaces/storage.interface';
import { vi, Mock } from 'vitest';
import { AttentionTrackerService, EVENT_API, AGENT_CONTEXT } from '@elohim/rea-runtime';
import { GovernanceApiService } from '@elohim/service';

// Mock the shared exploration sidebar. The standalone content-viewer delegates
// its primary relation surface (mini-graph + related concepts + explore button)
// to this component now. The real sidebar pulls RelatedConceptsService ->
// DataLoaderService -> ElohimClient when rendered, so it is swapped for a mock
// here (mirroring lesson-view's B4 override); the content-viewer spec only
// verifies the sidebar is rendered and wired. The sidebar's internal
// composition is covered by exploration-sidebar.component.spec.ts.
@Component({ selector: 'app-exploration-sidebar', standalone: true, template: '' })
class MockExplorationSidebarComponent {
  @Input() contentId!: string;
  @Input() collapsible = true;
  @Input() open = false;
  @Output() openChange = new EventEmitter<boolean>();
  @Input() compact = true;
  @Input() relatedLimit = 4;
  @Input() graphDepth = 1;
  @Input() graphHeight = 180;
  @Output() exploreContent = new EventEmitter<string>();
  @Output() exploreInGraph = new EventEmitter<void>();
}

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
  let stewardshipServiceSpy: any;
  let householdResilienceServiceSpy: any;
  let governanceApiSpy: any;
  let eprNavSpy: any;
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
      getStats: vi.fn().mockReturnValue({
        totalNodes: 0,
        averageAffinity: 0,
        engagedNodes: 0,
        distribution: {},
        byCategory: new Map(),
        byType: new Map(),
      }),
    };

    const agentSpyObj = {
      markContentSeen: vi.fn().mockReturnValue(of(undefined)),
      getCurrentAgentId: vi.fn().mockReturnValue('test-agent-id'),
    };
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

    const stewardshipSpyObj = {
      getContentStewardship: vi.fn().mockReturnValue(of({
        contentId: 'test-content-1',
        allocations: [],
        totalAllocation: 0,
        hasDisputes: false,
        primarySteward: null,
      })),
    };

    const householdResilienceSpyObj = {
      get: vi.fn().mockReturnValue(of({
        contentId: 'test-content-1',
        householdsStewarding: 2,
        householdsReciprocated: 1,
        protectionStatus: 'protected',
        details: { stewardHouseholds: ['h1', 'h2'], onlinePeerCount: 4, healthScore: 0.9 },
      })),
    };

    // Wave D: mock the substrate views that the Lit primitives receive.
    // getMechanismSelection/getAccumulationStatus replace the old orchestration services.
    const governanceApiSpyObj = {
      getMechanismSelection: vi.fn().mockResolvedValue({
        entityType: 'content',
        entityId: 'test-content-1',
        level: 1,
        mechanism: 'reactions',
        renderTarget: 'angular',
        contextMenuOnly: false,
        allowReactions: true,
        allowGraduatedFeedback: false,
        activeProposalId: null,
        activeProposalMechanism: null,
        policyManifestCid: null,
        computedAt: new Date().toISOString(),
      }),
      getAccumulationStatus: vi.fn().mockResolvedValue({
        entityType: 'content',
        entityId: 'test-content-1',
        totalSignals: 0,
        uniqueParticipants: 0,
        consensusStrength: 0,
        status: 'pending',
        readyForSensemaking: false,
        controversyDetected: false,
        settled: false,
        policyManifestCid: null,
        computedAt: new Date().toISOString(),
      }),
    };

    const libResilienceSpyObj = {
      getSnapshot: vi.fn().mockReturnValue(of({
        contentId: 'test-content-1',
        stewardingCollectives: 2,
        commitmentBackedCollectives: 1,
        diversityScore: 0.8,
        regionalDistribution: { local: 1, regional: 1, global: 0, unknown: 0 },
        placementGaps: [],
        protectionStatus: 'protected',
      })),
      listPlacementGaps: vi.fn().mockReturnValue(of({ items: [] })),
    };

    await TestBed.configureTestingModule({
      imports: [ContentViewerComponent],
      providers: [
        provideHttpClient(),
        provideRouter([]),
        {
          provide: LAMAD_STORAGE_CLIENT,
          useValue: {
            getBlobUrl: (h: string) => `https://test/blob/${h}`,
            getStorageBaseUrl: () => 'https://test',
          },
        },
        { provide: LAMAD_AFFINITY_TRACKING, useValue: affinitySpyObj },
        { provide: LAMAD_AGENT, useValue: agentSpyObj },
        { provide: ContentService, useValue: contentSpyObj },
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: TrustBadgeService, useValue: trustBadgeSpyObj },
        { provide: LAMAD_GOVERNANCE, useValue: governanceSpyObj },
        { provide: ContentEditorService, useValue: editorSpyObj },
        { provide: PathContextService, useValue: pathContextSpyObj },
        { provide: RendererRegistryService, useValue: rendererRegistrySpyObj },
        { provide: SeoService, useValue: seoServiceSpyObj },
        { provide: LAMAD_GOVERNANCE_SIGNAL, useValue: governanceSignalSpyObj },
        { provide: StewardshipAllocationService, useValue: stewardshipSpyObj },
        { provide: HouseholdResilienceService, useValue: householdResilienceSpyObj },
        { provide: LibResilienceService, useValue: libResilienceSpyObj },
        { provide: SignalHarnessService, useValue: { onRendererComplete: vi.fn().mockResolvedValue(undefined) } },
        { provide: AttentionTrackerService, useValue: { trackContentView: vi.fn(), trackContentLeave: vi.fn(), getSessionViewedIds: vi.fn().mockReturnValue(new Set()) } },
        { provide: GovernanceApiService, useValue: governanceApiSpyObj },
        { provide: EVENT_API, useValue: { createEconomicEvent: vi.fn().mockReturnValue(of(null)), getEconomicEvents: vi.fn().mockReturnValue(of([])) } },
        { provide: AGENT_CONTEXT, useValue: { getCurrentAgentId: vi.fn().mockReturnValue('test-agent-id') } },
        {
          provide: LAMAD_EPR_RESOLVER,
          useValue: {
            resolveEprHead: vi.fn().mockReturnValue(of({
              version: 1,
              id: 'test-content-1',
              content: 'bafk-test',
              lamad: { title: 'Test', contentType: 'concept' },
              shefa: {},
              qahal: {},
              relationships: [{ type: 'PREREQUISITE', target: 'foo' }],
            })),
            resolveUrl: vi.fn().mockReturnValue({ ref: {}, url: '', route: null }),
            resolve: vi.fn().mockReturnValue(of(null)),
            resolveInContext: vi.fn().mockReturnValue({ route: null, href: '/epr/foo', resolution: 'standalone' }),
            resolveBlobUrl: vi.fn().mockReturnValue(''),
          },
        },
        {
          provide: ActivatedRoute,
          useValue: {
            snapshot: { paramMap: { get: vi.fn().mockReturnValue('test-content-1') } },
            paramMap: of({ get: (k: string) => (k === 'id' ? 'test-content-1' : null) }),
            params: of({ resourceId: 'test-content-1' }),
            queryParams: of({}),
          },
        },
        {
          provide: LAMAD_EPR_NAV,
          useValue: { navigate: vi.fn(), ownsPath: vi.fn(() => true), recordHandoff: vi.fn() },
        },
      ],
    })
      // Swap the real exploration sidebar for a mock (shallow render). The real
      // sidebar's data-fetching chain (RelatedConceptsService -> DataLoaderService
      // -> ElohimClient) is out of scope here; its composition has its own spec.
      .overrideComponent(ContentViewerComponent, {
        remove: { imports: [ExplorationSidebarComponent] },
        add: { imports: [MockExplorationSidebarComponent] },
      })
      .compileComponents();

    affinityServiceSpy = TestBed.inject(LAMAD_AFFINITY_TRACKING);
    agentServiceSpy = TestBed.inject(LAMAD_AGENT);
    contentServiceSpy = TestBed.inject(ContentService);
    dataLoaderSpy = TestBed.inject(DataLoaderService);
    trustBadgeServiceSpy = TestBed.inject(TrustBadgeService);
    governanceServiceSpy = TestBed.inject(LAMAD_GOVERNANCE);
    editorServiceSpy = TestBed.inject(ContentEditorService);
    pathContextServiceSpy = TestBed.inject(PathContextService);
    rendererRegistrySpy = TestBed.inject(RendererRegistryService);
    routerSpy = TestBed.inject(Router);
    eprNavSpy = TestBed.inject(LAMAD_EPR_NAV);
    stewardshipServiceSpy = TestBed.inject(StewardshipAllocationService);
    householdResilienceServiceSpy = TestBed.inject(HouseholdResilienceService);
    governanceApiSpy = TestBed.inject(GovernanceApiService);
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

    it('renders the shared exploration sidebar as a pinned rail rooted at the node id', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      const sidebar = fixture.debugElement.query(
        el => el.name === 'app-exploration-sidebar',
      );
      expect(sidebar).toBeTruthy();
      const sidebarInstance = sidebar.componentInstance as MockExplorationSidebarComponent;
      expect(sidebarInstance.contentId).toBe('test-content-1');
      expect(sidebarInstance.collapsible).toBe(false);
    }));

    it('re-emits sidebar exploreContent through the detour-aware navigation', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      const sidebar = fixture.debugElement.query(
        el => el.name === 'app-exploration-sidebar',
      );
      const sidebarInstance = sidebar.componentInstance as MockExplorationSidebarComponent;

      sidebarInstance.exploreContent.emit('neighbor-1');
      expect(eprNavSpy.navigate).toHaveBeenCalledWith('/epr/neighbor-1');
    }));

    it('wires sidebar exploreInGraph to the graph explorer', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      const sidebar = fixture.debugElement.query(
        el => el.name === 'app-exploration-sidebar',
      );
      const sidebarInstance = sidebar.componentInstance as MockExplorationSidebarComponent;

      sidebarInstance.exploreInGraph.emit();
      expect(routerSpy.navigate).toHaveBeenCalledWith(
        ['/explore'],
        expect.objectContaining({
          queryParams: expect.objectContaining({ focus: 'test-content-1' }),
        }),
      );
    }));

    it('does NOT render the retired inline related-content grid', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      const grid = fixture.nativeElement.querySelector('.related-section');
      expect(grid).toBeFalsy();
    }));

    it('does NOT render a duplicate mini-graph in the Network tab', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      component.setActiveTab('network');
      fixture.detectChanges();

      const miniGraph = fixture.nativeElement.querySelector('app-mini-graph');
      expect(miniGraph).toBeFalsy();
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
      expect(eprNavSpy.navigate).toHaveBeenCalledWith('/epr/related-1');
    });

    it('should navigate to path', () => {
      component.navigateToPath('path-1', 2);
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/path', 'path-1', 'step', 2]);
    });

    it('should navigate back to home', () => {
      component.backToHome();
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/']);
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
      expect(eprNavSpy.navigate).toHaveBeenCalledWith('/some/route');
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
        detourStack: [
          {
            fromContentId: 'node-1',
            toContentId: 'test-content-1',
            detourType: 'graph-explore' as const,
            timestamp: new Date().toISOString(),
          },
        ],
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
        returnRoute: ['/path', 'test-path', 'step', '2'],
        detourStack: [],
      };
      pathContextServiceSpy.returnToPath.mockReturnValue(['/path', 'test-path', 'step', '2']);

      component.returnToPath();

      expect(pathContextServiceSpy.returnToPath).toHaveBeenCalled();
      expect(eprNavSpy.navigate).toHaveBeenCalledWith(['/path', 'test-path', 'step', '2']);
    });

    it('should not navigate if no return path', () => {
      component.pathContext = null;
      pathContextServiceSpy.returnToPath.mockReturnValue(null);

      component.returnToPath();

      expect(eprNavSpy.navigate).not.toHaveBeenCalled();
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
      expect(eprNavSpy.navigate).toHaveBeenCalledWith('/epr/related-node');
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
        ['/explore'],
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
      expect(eprNavSpy.navigate).toHaveBeenCalledWith('/epr/related-node');
    });

    it('should not explore in graph if no nodeId', () => {
      component['nodeId'] = null;

      component.exploreInGraph();

      expect(routerSpy.navigate).not.toHaveBeenCalled();
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


  describe('Content Flags', () => {
    it('should return empty array when node has no flags', () => {
      component.node = { ...mockContentNode, flags: undefined };
      expect(component.getFlags()).toEqual([]);
    });

    it('should return flags when node has flags', () => {
      const flags = [
        { type: 'disputed' as const, reason: 'Factual accuracy questioned', flaggedAt: '2026-03-01' },
      ];
      component.node = { ...mockContentNode, flags };
      expect(component.getFlags()).toEqual(flags);
    });

    it('should return correct flag label', () => {
      expect(component.getFlagLabel('disputed')).toBe('Disputed');
      expect(component.getFlagLabel('outdated')).toBe('Outdated');
      expect(component.getFlagLabel('appeal-pending')).toBe('Appeal Pending');
      expect(component.getFlagLabel('under-review')).toBe('Under Review');
      expect(component.getFlagLabel('partial-revocation')).toBe('Partial Revocation');
    });

    it('should return correct flag CSS class', () => {
      expect(component.getFlagClass('disputed')).toBe('flag-tag flag-disputed');
      expect(component.getFlagClass('outdated')).toBe('flag-tag flag-outdated');
      expect(component.getFlagClass('under-review')).toBe('flag-tag flag-under-review');
    });
  });

  describe('Stewardship in Trust Tab', () => {
    it('should load stewardship data when content loads', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      expect(stewardshipServiceSpy.getContentStewardship).toHaveBeenCalledWith('test-content-1');
    }));

    it('should store stewardship data on component', fakeAsync(() => {
      const mockStewardship = {
        contentId: 'test-content-1',
        allocations: [{
          steward: { id: 's1', displayName: 'Alice', presenceState: 'active' },
          id: 'alloc-1',
          appId: 'test',
          contentId: 'test-content-1',
          stewardPresenceId: 'sp-1',
          allocationRatio: 0.6,
          allocationMethod: 'manual',
          contributionType: 'author',
          contributionEvidence: null,
          governanceState: 'active',
          disputeId: null,
          disputeReason: null,
          disputedAt: null,
          disputedBy: null,
          negotiationSessionId: null,
          elohimRatifiedAt: null,
          elohimRatifierId: null,
          effectiveFrom: '2026-01-01',
          effectiveUntil: null,
          supersededBy: null,
          recognitionAccumulated: 42.5,
          lastRecognitionAt: '2026-03-20',
          note: null,
          metadata: null,
          createdAt: '2026-01-01',
          updatedAt: '2026-03-20',
          dhtAnchorHash: null,
        }],
        totalAllocation: 0.6,
        hasDisputes: false,
        primarySteward: null,
      };
      stewardshipServiceSpy.getContentStewardship.mockReturnValue(of(mockStewardship));
      fixture.detectChanges();
      tick();
      expect(component.stewardship).toEqual(mockStewardship);
    }));

    it('should handle empty stewardship gracefully', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      expect(component.stewardship).toBeTruthy();
      expect(component.stewardship!.allocations).toEqual([]);
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

  describe('Wave D — governance substrate views bound to Lit primitives', () => {
    it('fetches mechanism selection and accumulation status when content loads', fakeAsync(async () => {
      fixture.detectChanges();
      tick();
      // Allow the async loadGovernanceViews() promise to resolve
      await fixture.whenStable();

      expect(governanceApiSpy.getMechanismSelection).toHaveBeenCalledWith('content', 'test-content-1');
      expect(governanceApiSpy.getAccumulationStatus).toHaveBeenCalledWith('content', 'test-content-1');
    }));

    it('stores mechanism selection and accumulation status on component after load', fakeAsync(async () => {
      fixture.detectChanges();
      tick();
      await fixture.whenStable();

      expect(component.mechanismSelection).toEqual({ level: 1, renderTarget: 'angular' });
      expect(component.accumulationStatus).toEqual({
        readyForSensemaking: false,
        controversyDetected: false,
        settled: false,
      });
    }));

    it('degrades gracefully when governance API fails', async () => {
      governanceApiSpy.getMechanismSelection.mockRejectedValue(new Error('Network error'));
      governanceApiSpy.getAccumulationStatus.mockRejectedValue(new Error('Network error'));

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.mechanismSelection).toBeNull();
      expect(component.accumulationStatus).toBeNull();
      expect(component.isLoadingGovernanceViews).toBe(false);
    });
  });

  describe('EPR relationships', () => {
    it('renders the EPR relationships panel when the head has relationships', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      const panel = fixture.nativeElement.querySelector('[data-testid="viewer-relationships-panel"]');
      expect(panel).toBeTruthy();
    }));

    it('mounts the real EprRelationshipsPanelComponent with the relationships from the head', fakeAsync(() => {
      const eprResolverSpy = TestBed.inject(LAMAD_EPR_RESOLVER);
      (eprResolverSpy.resolveEprHead as Mock).mockReturnValue(of({
        version: 1,
        id: 'test-content-1',
        content: 'bafk-test',
        lamad: { title: 'Test', contentType: 'concept' },
        shefa: {},
        qahal: {},
        relationships: [
          { type: 'PREREQUISITE', target: 'feedback-loops' },
          { type: 'TEACHES', target: 'mental-models' },
        ],
      }));

      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      // The mounted panel exposes its own internal testids (asserted by the
      // epr-link-navigation a2o scenarios), proving the real Angular component
      // is rendered — not a CUSTOM_ELEMENTS_SCHEMA-swallowed phantom tag.
      const innerPanel = fixture.nativeElement.querySelector(
        '[data-testid="epr-relationships-panel"]'
      );
      expect(innerPanel).toBeTruthy();

      const groups = fixture.nativeElement.querySelectorAll('[data-testid="epr-rel-group"]');
      expect(groups.length).toBe(2);

      const cards = fixture.nativeElement.querySelectorAll(
        '[data-testid="epr-relationship-card"]'
      );
      expect(cards.length).toBe(2);
    }));

    it('does not render the panel when there are no relationships', fakeAsync(() => {
      const eprResolverSpy = TestBed.inject(LAMAD_EPR_RESOLVER);
      (eprResolverSpy.resolveEprHead as Mock).mockReturnValue(of({
        version: 1,
        id: 'test-content-1',
        content: 'bafk-test',
        lamad: { title: 'Test', contentType: 'concept' },
        shefa: {},
        qahal: {},
        relationships: [],
      }));

      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      const panel = fixture.nativeElement.querySelector('[data-testid="viewer-relationships-panel"]');
      expect(panel).toBeFalsy();
      const innerPanel = fixture.nativeElement.querySelector(
        '[data-testid="epr-relationships-panel"]'
      );
      expect(innerPanel).toBeFalsy();
    }));
  });

  describe('Header — distribution + resilience side-by-side', () => {
    it('renders <elohim-distribution-badge> when node.distribution is hydrated', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      component.node = {
        ...(component.node as ContentNode),
        blobs: [
          {
            hash: 'sha256-content-viewer-test',
            mimeType: 'application/json',
            sizeBytes: 0,
            fallbackUrls: [],
          },
        ],
        distribution: {
          replicaCount: 3,
          replicaTarget: 4,
          replicaHealth: 'at_risk',
          projectorCount: 1,
          reachClass: 'public',
          diversityHint: { kind: 'region_metro', value: ['us-central'] },
          thisFetchSource: 'projected_via_doorway',
          lastVerifiedSeconds: 30,
        },
      };
      component.isLoading = false;
      fixture.detectChanges();

      expect(
        fixture.nativeElement.querySelector('[data-testid="viewer-distribution-info"]'),
      ).toBeTruthy();
    }));

    it('end-aligns the resilience fold-downs — the icon trails the title text', fakeAsync(() => {
      // The icon sits at the END of the title's last line; on phone widths a
      // start-pinned 240px panel projects off the right edge for any line
      // longer than ~135px (most titles). End-alignment grows it back into
      // the viewport (2026-06-12 regression class, omnibar spec §11.2).
      fixture.detectChanges();
      tick();
      fixture.detectChanges();
      component.isLoading = false;
      fixture.detectChanges();

      const wrap = fixture.nativeElement.querySelector(
        '[data-testid="viewer-resilience-info"] .resilience-icon-wrap',
      );
      expect(wrap).toBeTruthy();
      expect(wrap?.classList.contains('align-end')).toBe(true);
    }));

    it('hides the distribution badge when node.distribution is absent', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      component.node = {
        ...(component.node as ContentNode),
        distribution: undefined,
      };
      component.isLoading = false;
      fixture.detectChanges();

      expect(
        fixture.nativeElement.querySelector('[data-testid="viewer-distribution-info"]'),
      ).toBeFalsy();
    }));
  });

  // "Open in {pillar}" affordance — one lens among the legs. When the viewed
  // content's contentType is CLAIMED by the lamad pillar (today: only 'path'),
  // offer a cross-bundle deep-dive link to the rich pillar mount. Unclaimed
  // types get NO affordance: the universal viewer IS their home.
  describe('open-in-pillar affordance', () => {
    const PILLAR_AFFORDANCE = '[data-testid="epr-open-in-pillar"]';

    it('renders a cross-bundle "Open in Lamad" link for a lamad-claimed type (path)', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      component.node = {
        ...(component.node as ContentNode),
        id: 'foundations-christian-technology',
        contentType: 'path',
      };
      component.isLoading = false;
      fixture.detectChanges();

      const link = fixture.nativeElement.querySelector(PILLAR_AFFORDANCE) as HTMLAnchorElement;
      expect(link).toBeTruthy();
      // Cross-bundle: a plain href (full doorway load), never a routerLink.
      expect(link.getAttribute('href')).toBe('/lamad/path/foundations-christian-technology');
      // a11y: a real anchor with discernible text, not a bare icon.
      expect(link.tagName.toLowerCase()).toBe('a');
      expect(link.textContent?.trim()).toContain('Open in Lamad');
    }));

    it('omits the affordance for an unclaimed type (concept)', fakeAsync(() => {
      // mockContentNode.contentType is 'concept' — not in LAMAD_ROUTE_CLAIMS.
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      expect(component.node?.contentType).toBe('concept');
      expect(fixture.nativeElement.querySelector(PILLAR_AFFORDANCE)).toBeFalsy();
    }));

    it('omits the affordance for another unclaimed type (unit)', fakeAsync(() => {
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      component.node = {
        ...(component.node as ContentNode),
        contentType: 'unit',
      };
      component.isLoading = false;
      fixture.detectChanges();

      expect(fixture.nativeElement.querySelector(PILLAR_AFFORDANCE)).toBeFalsy();
    }));
  });
});
