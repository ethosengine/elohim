import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { Router } from '@angular/router';

import { of, timer } from 'rxjs';
import { map } from 'rxjs/operators';

import { LearnerContextService } from '@app/lamad/services/learner-context.service';
import {
  PathRecommendationService,
  type PathRecommendation,
} from '@app/lamad/services/path-recommendation.service';

import { BannerService } from './banner.service';
import { ElohimAgentService } from './elohim-agent.service';
import { ElohimPresenceService } from './elohim-presence.service';

import type { ElohimResponse } from '../models/elohim-agent.model';

describe('ElohimPresenceService', () => {
  let service: ElohimPresenceService;
  let mockAgentService: jasmine.SpyObj<ElohimAgentService>;
  let mockBannerService: jasmine.SpyObj<BannerService>;
  let mockRouter: jasmine.SpyObj<Router>;
  let mockPathRecommendation: jasmine.SpyObj<PathRecommendationService>;
  let mockLearnerContext: jasmine.SpyObj<LearnerContextService>;

  const buildFulfilledResponse = (
    overrides: Partial<ElohimResponse> = {},
  ): ElohimResponse => ({
    requestId: 'req-test-1',
    elohimId: 'elohim-1',
    status: 'fulfilled',
    constitutionalReasoning: {
      primaryPrinciple: 'Love as committed action toward flourishing',
      interpretation: 'Guided by constitutional principles',
      valuesWeighed: [{ value: 'Human dignity', weight: 0.5, direction: 'for' }],
      confidence: 0.75,
    },
    respondedAt: new Date().toISOString(),
    cost: {
      tokensProcessed: 800,
      timeMs: 1200,
      constitutionalChecks: 3,
      precedentLookups: 1,
    },
    ...overrides,
  });

  const buildDeclinedResponse = (): ElohimResponse => ({
    requestId: 'req-test-2',
    elohimId: 'elohim-1',
    status: 'declined',
    constitutionalReasoning: {
      primaryPrinciple: 'Capability boundaries',
      interpretation: 'Request declined',
      valuesWeighed: [],
      confidence: 1,
    },
    declineReason: 'Not available',
    respondedAt: new Date().toISOString(),
  });

  beforeEach(() => {
    mockAgentService = jasmine.createSpyObj('ElohimAgentService', [
      'invoke',
      'requestAttestationRecommendation',
    ]);
    mockAgentService.invoke.and.returnValue(of(buildFulfilledResponse()));
    mockAgentService.requestAttestationRecommendation.and.returnValue(
      of(buildFulfilledResponse()),
    );

    mockBannerService = jasmine.createSpyObj('BannerService', [
      'registerProvider',
      'unregisterProvider',
    ]);

    mockRouter = jasmine.createSpyObj('Router', ['navigate']);
    mockRouter.navigate.and.returnValue(Promise.resolve(true));

    mockPathRecommendation = jasmine.createSpyObj('PathRecommendationService', [
      'getTopRecommendation',
      'getRecommendations',
    ]);
    mockPathRecommendation.getTopRecommendation.and.returnValue(of(null));

    mockLearnerContext = jasmine.createSpyObj('LearnerContextService', [
      'getDiscoveryProfile',
      'isMasteryUnlocked',
      'getAdaptationState',
    ]);
    mockLearnerContext.getDiscoveryProfile.and.returnValue(null);

    TestBed.configureTestingModule({
      providers: [
        ElohimPresenceService,
        { provide: ElohimAgentService, useValue: mockAgentService },
        { provide: BannerService, useValue: mockBannerService },
        { provide: Router, useValue: mockRouter },
        { provide: PathRecommendationService, useValue: mockPathRecommendation },
        { provide: LearnerContextService, useValue: mockLearnerContext },
      ],
    });

    service = TestBed.inject(ElohimPresenceService);
  });

  // =========================================================================
  // Construction
  // =========================================================================

  describe('construction', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should register with BannerService', () => {
      expect(mockBannerService.registerProvider).toHaveBeenCalledWith(service);
    });

    it('should have providerId', () => {
      expect(service.providerId).toBe('elohim-presence');
    });
  });

  // =========================================================================
  // onDiscoveryCompleted
  // =========================================================================

  describe('onDiscoveryCompleted', () => {
    it('should invoke agent with path-recommendation capability', () => {
      service.onDiscoveryCompleted('node-1', 'Attachment Style', 'user-1').subscribe();

      expect(mockAgentService.invoke).toHaveBeenCalledWith(
        jasmine.objectContaining({
          capability: 'path-recommendation',
          requesterId: 'user-1',
        }),
      );
    });

    it('should include assessment title in constitutional context', () => {
      service.onDiscoveryCompleted('node-1', 'Attachment Style', 'user-1').subscribe();

      const request = mockAgentService.invoke.calls.mostRecent().args[0];
      expect(request.constitutionalContext).toContain('Attachment Style');
    });

    it('should add insight moment on fulfilled response', fakeAsync(() => {
      let moments: unknown[] = [];
      service.presence$.subscribe(m => (moments = m));

      service.onDiscoveryCompleted('node-1', 'Attachment Style', 'user-1').subscribe();
      tick();

      expect(moments.length).toBe(1);
      expect((moments[0] as { type: string }).type).toBe('insight');
    }));

    it('should include assessment title in insight message', fakeAsync(() => {
      let moments: { message: string }[] = [];
      service.presence$.subscribe(m => (moments = m as { message: string }[]));

      service.onDiscoveryCompleted('node-1', 'Attachment Style', 'user-1').subscribe();
      tick();

      expect(moments[0].message).toContain('Attachment Style');
    }));

    it('should include view-path and dismiss actions', fakeAsync(() => {
      let moments: { actions?: { id: string }[] }[] = [];
      service.presence$.subscribe(m => (moments = m as { actions?: { id: string }[] }[]));

      service.onDiscoveryCompleted('node-1', 'Attachment Style', 'user-1').subscribe();
      tick();

      const actionIds = moments[0].actions!.map(a => a.id);
      expect(actionIds).toContain('view-path');
      expect(actionIds).toContain('dismiss');
    }));

    it('should not add moment on declined response', fakeAsync(() => {
      mockAgentService.invoke.and.returnValue(of(buildDeclinedResponse()));

      let moments: unknown[] = [];
      service.presence$.subscribe(m => (moments = m));

      service.onDiscoveryCompleted('node-1', 'Test', 'user-1').subscribe();
      tick();

      expect(moments.length).toBe(0);
    }));

    it('should use recommended path when discovery profile exists', fakeAsync(() => {
      const mockProfile = {
        byCategory: { values: [{ framework: 'values-hierarchy' }] },
        featured: [],
        totalAssessments: 1,
        topTraits: [],
      };
      const mockRec: PathRecommendation = {
        pathId: 'know-thyself-path',
        pathTitle: 'Know Thyself: Self-Discovery Journey',
        relevanceScore: 0.8,
        matchedCategories: ['values'],
        reason: 'Matches your values discovery results',
      };

      mockLearnerContext.getDiscoveryProfile.and.returnValue(mockProfile as never);
      mockPathRecommendation.getTopRecommendation.and.returnValue(of(mockRec));

      let moments: { metadata?: Record<string, unknown>; message: string }[] = [];
      service.presence$.subscribe(
        m => (moments = m as { metadata?: Record<string, unknown>; message: string }[]),
      );

      service.onDiscoveryCompleted('node-1', 'Values Hierarchy', 'user-1').subscribe();
      tick();

      expect(moments.length).toBe(1);
      expect(moments[0].metadata!['recommendedPathId']).toBe('know-thyself-path');
      expect(moments[0].message).toContain('Know Thyself');
    }));

    it('should fall back to contentNodeId when no recommendation found', fakeAsync(() => {
      const mockProfile = {
        byCategory: { values: [{ framework: 'values-hierarchy' }] },
        featured: [],
        totalAssessments: 1,
        topTraits: [],
      };

      mockLearnerContext.getDiscoveryProfile.and.returnValue(mockProfile as never);
      mockPathRecommendation.getTopRecommendation.and.returnValue(of(null));

      let moments: { metadata?: Record<string, unknown> }[] = [];
      service.presence$.subscribe(
        m => (moments = m as { metadata?: Record<string, unknown> }[]),
      );

      service.onDiscoveryCompleted('node-1', 'Test', 'user-1').subscribe();
      tick();

      expect(moments[0].metadata!['recommendedPathId']).toBe('node-1');
    }));
  });

  // =========================================================================
  // onContentCompleted
  // =========================================================================

  describe('onContentCompleted', () => {
    it('should invoke agent with knowledge-map-synthesis capability', () => {
      service.onContentCompleted('content-1', 'mastered', 'user-1').subscribe();

      expect(mockAgentService.invoke).toHaveBeenCalledWith(
        jasmine.objectContaining({
          capability: 'knowledge-map-synthesis',
        }),
      );
    });

    it('should add celebration moment for mastered level', fakeAsync(() => {
      let moments: { type: string }[] = [];
      service.presence$.subscribe(m => (moments = m as { type: string }[]));

      service.onContentCompleted('content-1', 'mastered', 'user-1').subscribe();
      tick();

      expect(moments[0].type).toBe('celebration');
    }));

    it('should add nudge moment for non-mastered level', fakeAsync(() => {
      let moments: { type: string; message: string }[] = [];
      service.presence$.subscribe(
        m => (moments = m as { type: string; message: string }[]),
      );

      service.onContentCompleted('content-1', 'practicing', 'user-1').subscribe();
      tick();

      expect(moments[0].type).toBe('nudge');
      expect(moments[0].message).toContain('Keep going');
    }));
  });

  // =========================================================================
  // checkLearnerWellbeing
  // =========================================================================

  describe('checkLearnerWellbeing', () => {
    it('should invoke agent with spiral-detection capability', () => {
      service.checkLearnerWellbeing('agent-1').subscribe();

      expect(mockAgentService.invoke).toHaveBeenCalledWith(
        jasmine.objectContaining({
          capability: 'spiral-detection',
        }),
      );
    });

    it('should add care moment when spiral detected', fakeAsync(() => {
      mockAgentService.invoke.and.returnValue(
        of(
          buildFulfilledResponse({
            payload: { spiralDetected: true } as unknown as ElohimResponse['payload'],
          }),
        ),
      );

      let moments: { type: string }[] = [];
      service.presence$.subscribe(m => (moments = m as { type: string }[]));

      service.checkLearnerWellbeing('agent-1').subscribe();
      tick();

      expect(moments.length).toBe(1);
      expect(moments[0].type).toBe('care');
    }));

    it('should not add moment when no spiral detected', fakeAsync(() => {
      mockAgentService.invoke.and.returnValue(
        of(
          buildFulfilledResponse({
            payload: { spiralDetected: false } as unknown as ElohimResponse['payload'],
          }),
        ),
      );

      let moments: unknown[] = [];
      service.presence$.subscribe(m => (moments = m));

      service.checkLearnerWellbeing('agent-1').subscribe();
      tick();

      expect(moments.length).toBe(0);
    }));

    it('should not add moment when no payload', fakeAsync(() => {
      mockAgentService.invoke.and.returnValue(of(buildFulfilledResponse({ payload: undefined })));

      let moments: unknown[] = [];
      service.presence$.subscribe(m => (moments = m));

      service.checkLearnerWellbeing('agent-1').subscribe();
      tick();

      expect(moments.length).toBe(0);
    }));
  });

  // =========================================================================
  // verifyAttestation
  // =========================================================================

  describe('verifyAttestation', () => {
    it('should delegate to agentService', () => {
      service.verifyAttestation('content-1', 'fact-check', 'user-1').subscribe();

      expect(mockAgentService.requestAttestationRecommendation).toHaveBeenCalledWith(
        'content-1',
        'fact-check',
        'user-1',
      );
    });
  });

  // =========================================================================
  // Cost accumulation
  // =========================================================================

  describe('cost accumulation', () => {
    it('should start at zero', fakeAsync(() => {
      let cost: { tokensProcessed: number } | undefined;
      service.cost$.subscribe(c => (cost = c as { tokensProcessed: number }));
      tick();

      expect(cost!.tokensProcessed).toBe(0);
    }));

    it('should accumulate across moments', fakeAsync(() => {
      let cost: { tokensProcessed: number; timeMs: number } | undefined;
      service.cost$.subscribe(c => (cost = c as { tokensProcessed: number; timeMs: number }));

      service.onDiscoveryCompleted('n1', 'Test1', 'u1').subscribe();
      tick();
      service.onContentCompleted('c1', 'mastered', 'u1').subscribe();
      tick();

      expect(cost!.tokensProcessed).toBe(1600); // 800 + 800
      expect(cost!.timeMs).toBe(2400); // 1200 + 1200
    }));
  });

  // =========================================================================
  // BannerNoticeProvider
  // =========================================================================

  describe('BannerNoticeProvider', () => {
    it('should map care moment to warning severity', fakeAsync(() => {
      mockAgentService.invoke.and.returnValue(
        of(
          buildFulfilledResponse({
            payload: { spiralDetected: true } as unknown as ElohimResponse['payload'],
          }),
        ),
      );

      let notices: { severity: string }[] = [];
      service.notices$.subscribe(n => (notices = n as { severity: string }[]));

      service.checkLearnerWellbeing('agent-1').subscribe();
      tick();

      expect(notices[0].severity).toBe('warning');
    }));

    it('should map insight moment to info severity', fakeAsync(() => {
      let notices: { severity: string }[] = [];
      service.notices$.subscribe(n => (notices = n as { severity: string }[]));

      service.onDiscoveryCompleted('n1', 'Test', 'u1').subscribe();
      tick();

      expect(notices[0].severity).toBe('info');
    }));

    it('should include global and lamad contexts', fakeAsync(() => {
      let notices: { contexts: string[] }[] = [];
      service.notices$.subscribe(n => (notices = n as { contexts: string[] }[]));

      service.onDiscoveryCompleted('n1', 'Test', 'u1').subscribe();
      tick();

      expect(notices[0].contexts).toContain('global');
      expect(notices[0].contexts).toContain('lamad');
    }));

    it('should dismiss notice by id', fakeAsync(() => {
      let notices: { id: string }[] = [];
      service.notices$.subscribe(n => (notices = n as { id: string }[]));

      service.onDiscoveryCompleted('n1', 'Test', 'u1').subscribe();
      tick();

      const noticeId = notices[0].id;
      expect(notices.length).toBe(1);

      service.dismissNotice(noticeId);
      tick();

      expect(notices.length).toBe(0);
    }));

    it('should handle dismiss action via handleAction', fakeAsync(() => {
      let notices: { id: string }[] = [];
      service.notices$.subscribe(n => (notices = n as { id: string }[]));

      service.onDiscoveryCompleted('n1', 'Test', 'u1').subscribe();
      tick();

      const noticeId = notices[0].id;
      service.handleAction(noticeId, 'dismiss');
      tick();

      expect(notices.length).toBe(0);
    }));
  });

  // =========================================================================
  // handleAction — view-path navigation
  // =========================================================================

  describe('handleAction view-path', () => {
    it('should navigate to recommended path from moment metadata', fakeAsync(() => {
      // Trigger a discovery completed to create a moment with metadata
      service.onDiscoveryCompleted('node-1', 'Attachment Style', 'user-1').subscribe();
      tick();

      // Get the moment id from notices
      let notices: { id: string }[] = [];
      service.notices$.subscribe(n => (notices = n as { id: string }[]));
      tick();

      const noticeId = notices[0].id;
      service.handleAction(noticeId, 'view-path');

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad/path', 'node-1']);
    }));

    it('should dismiss the moment after navigating', fakeAsync(() => {
      service.onDiscoveryCompleted('node-1', 'Test', 'user-1').subscribe();
      tick();

      let notices: { id: string }[] = [];
      service.notices$.subscribe(n => (notices = n as { id: string }[]));
      tick();

      const noticeId = notices[0].id;
      service.handleAction(noticeId, 'view-path');
      tick();

      expect(notices.length).toBe(0);
    }));

    it('should navigate to /lamad fallback when moment has no metadata', fakeAsync(() => {
      // Create a moment without metadata by completing content (not discovery)
      service.onContentCompleted('c1', 'mastered', 'user-1').subscribe();
      tick();

      let moments: { id: string }[] = [];
      service.presence$.subscribe(m => (moments = m as { id: string }[]));
      tick();

      const momentId = moments[0].id;
      service.handleAction(momentId, 'view-path');

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad']);
    }));

    it('should include recommendedPathId in discovery moment metadata', fakeAsync(() => {
      let moments: { metadata?: Record<string, unknown> }[] = [];
      service.presence$.subscribe(
        m => (moments = m as { metadata?: Record<string, unknown> }[]),
      );

      service.onDiscoveryCompleted('node-42', 'Big Five', 'user-1').subscribe();
      tick();

      expect(moments[0].metadata).toBeDefined();
      expect(moments[0].metadata!['recommendedPathId']).toBe('node-42');
    }));

    it('should navigate to recommended path when recommendation exists', fakeAsync(() => {
      const mockProfile = {
        byCategory: { values: [{ framework: 'values-hierarchy' }] },
        featured: [],
        totalAssessments: 1,
        topTraits: [],
      };
      const mockRec: PathRecommendation = {
        pathId: 'know-thyself-path',
        pathTitle: 'Know Thyself',
        relevanceScore: 0.8,
        matchedCategories: ['values'],
        reason: 'Matches your values results',
      };

      mockLearnerContext.getDiscoveryProfile.and.returnValue(mockProfile as never);
      mockPathRecommendation.getTopRecommendation.and.returnValue(of(mockRec));

      service.onDiscoveryCompleted('node-1', 'Values Hierarchy', 'user-1').subscribe();
      tick();

      let notices: { id: string }[] = [];
      service.notices$.subscribe(n => (notices = n as { id: string }[]));
      tick();

      const noticeId = notices[0].id;
      service.handleAction(noticeId, 'view-path');

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad/path', 'know-thyself-path']);
    }));
  });

  // =========================================================================
  // ngOnDestroy
  // =========================================================================

  describe('ngOnDestroy', () => {
    it('should unregister from BannerService', () => {
      service.ngOnDestroy();
      expect(mockBannerService.unregisterProvider).toHaveBeenCalledWith('elohim-presence');
    });
  });
});
