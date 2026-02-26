import { TestBed, fakeAsync, tick } from '@angular/core/testing';

import { of, throwError, timer } from 'rxjs';
import { map } from 'rxjs/operators';

import { IdentityService } from '@app/imagodei/services/identity.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { PathContextService } from '@app/lamad/services/path-context.service';
import { RelatedConceptsService } from '@app/lamad/services/related-concepts.service';

import { AgentService } from './agent.service';
import { ContextAssemblyService } from './context-assembly.service';
import { DataLoaderService } from './data-loader.service';
import { ElohimAgentService } from './elohim-agent.service';
import { GovernanceService } from './governance.service';

import type { CreatePayload } from '../models/create-context.model';
import type { ElohimResponse, ElohimResponsePayload } from '../models/elohim-agent.model';

// ============================================================================
// Test Fixtures
// ============================================================================

const buildPayload = (overrides: Partial<CreatePayload> = {}): CreatePayload =>
  ({
    actionType: 'post',
    content: 'Test content body',
    contentType: 'concept',
    contentFormat: 'markdown',
    requestedReach: 'community',
    tags: ['test'],
    ...overrides,
  }) as CreatePayload;

const buildFulfilledResponse = (overrides: Partial<ElohimResponse> = {}): ElohimResponse =>
  ({
    requestId: 'req-test-1',
    elohimId: 'elohim-1',
    status: 'fulfilled',
    constitutionalReasoning: {
      primaryPrinciple: 'Free speech does not mean free reach',
      interpretation: 'Reach negotiated based on trust and context',
      valuesWeighed: [{ value: 'Author sovereignty', weight: 0.3, direction: 'for' }],
      confidence: 0.85,
    },
    respondedAt: new Date().toISOString(),
    cost: {
      tokensProcessed: 800,
      timeMs: 1200,
      constitutionalChecks: 3,
      precedentLookups: 1,
    },
    payload: {
      type: 'reach-negotiation',
      recommendedReach: 'community',
      safetyCleared: true,
      conditions: [],
      transparencyReport: {
        layersConsidered: ['constitutional', 'author-profile', 'payload'],
        decisionFactors: [],
        authorStanding: 'Good standing',
        communityHealth: 'Normal',
        neighborhoodInfluence: 'Healthy',
      },
    } as unknown as ElohimResponsePayload,
    ...overrides,
  }) as ElohimResponse;

const buildAnalytics = (overrides: Record<string, unknown> = {}) => ({
  totalPathsStarted: 3,
  totalPathsCompleted: 1,
  totalContentNodesCompleted: 15,
  totalStepsCompleted: 42,
  totalLearningTime: 3600,
  lastActivityDate: new Date().toISOString(),
  firstActivityDate: '2026-01-01T00:00:00.000Z',
  currentStreak: 5,
  longestStreak: 10,
  mostActivePathId: 'path-1',
  mostActivePathTitle: 'Know Thyself',
  mostRecentPathId: 'path-1',
  mostRecentPathTitle: 'Know Thyself',
  averageAffinity: 0.7,
  highAffinityPaths: ['path-1'],
  totalAttestationsEarned: 2,
  attestationIds: ['att-1', 'att-2'],
  ...overrides,
});

const buildPrecedent = (id: string, title: string) => ({
  id,
  title,
  summary: 'Test precedent summary',
  fullReasoning: 'Full reasoning text',
  binding: 'advisory',
  scope: { entityTypes: ['content'] },
  citations: 3,
  status: 'active',
});

const buildProposal = (id: string) => ({
  id,
  title: 'Test Proposal',
  proposalType: 'policy',
  description: 'Test description',
  proposer: { agentId: 'agent-1', displayName: 'Test Author' },
  status: 'active',
  phase: 'voting',
  createdAt: new Date().toISOString(),
});

const buildElohimAgent = () => ({
  id: 'elohim-1',
  name: 'Test Elohim',
  layer: 'community' as const,
  constitutionalBinding: {
    constitutionHash: 'sha256-abc123',
    lastVerified: '2026-01-01T00:00:00.000Z',
  },
});

const buildContentNode = (overrides: Record<string, unknown> = {}) => ({
  id: 'node-1',
  contentType: 'concept',
  title: 'Test Content',
  description: 'Test description',
  content: 'Test body',
  contentFormat: 'markdown',
  tags: ['test'],
  relatedNodeIds: [],
  metadata: {},
  ...overrides,
});

const buildRelatedConceptsResult = () => ({
  prerequisites: [
    buildContentNode({ id: 'prereq-1', title: 'Prereq', reach: 'community', trustScore: 0.7 }),
  ],
  extensions: [],
  related: [
    buildContentNode({ id: 'related-1', title: 'Related', reach: 'local', trustScore: 0.5 }),
  ],
  children: [],
  parents: [],
  allRelationships: [],
});

const buildParentContent = () =>
  buildContentNode({
    id: 'parent-1',
    title: 'Parent Content',
    reach: 'community',
    trustScore: 0.8,
    authorId: 'author-parent',
    feedbackProfileId: 'fp-1',
    flags: [],
  });

const buildActivity = (overrides: Record<string, unknown> = {}) => ({
  type: 'view' as const,
  resourceId: 'r-1',
  resourceType: 'content' as const,
  metadata: { contentType: 'concept' },
  timestamp: new Date().toISOString(),
  ...overrides,
});

// ============================================================================
// Tests
// ============================================================================

describe('ContextAssemblyService', () => {
  let service: ContextAssemblyService;
  let mockAgentService: jasmine.SpyObj<AgentService>;
  let mockElohimAgentService: jasmine.SpyObj<ElohimAgentService>;
  let mockGovernanceService: jasmine.SpyObj<GovernanceService>;
  let mockDataLoader: jasmine.SpyObj<DataLoaderService>;
  let mockRelatedConcepts: jasmine.SpyObj<RelatedConceptsService>;
  let mockPathContextService: { currentContext: unknown };
  let mockIdentityService: { profile: jasmine.Spy };
  let mockSessionHumanService: jasmine.SpyObj<SessionHumanService>;

  beforeEach(() => {
    mockAgentService = jasmine.createSpyObj('AgentService', [
      'getCurrentAgentId',
      'getAttestations',
      'getLearningAnalytics',
    ]);
    mockAgentService.getCurrentAgentId.and.returnValue('agent-test-1');
    mockAgentService.getAttestations.and.returnValue(['author-verified']);
    mockAgentService.getLearningAnalytics.and.returnValue(of(buildAnalytics()));

    mockElohimAgentService = jasmine.createSpyObj('ElohimAgentService', ['invoke', 'selectElohim']);
    mockElohimAgentService.invoke.and.returnValue(of(buildFulfilledResponse()));
    // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
    mockElohimAgentService.selectElohim.and.returnValue(of(buildElohimAgent()) as any);

    mockGovernanceService = jasmine.createSpyObj('GovernanceService', [
      'getConstitutionalPrecedents',
      'getActiveProposals',
      'getGovernanceSummary',
    ]);
    mockGovernanceService.getConstitutionalPrecedents.and.returnValue(
      of([buildPrecedent('p-1', 'Human dignity'), buildPrecedent('p-2', 'Knowledge access')])
    );
    mockGovernanceService.getActiveProposals.and.returnValue(
      of([buildProposal('proposal-1'), buildProposal('proposal-2')])
    );
    mockGovernanceService.getGovernanceSummary.and.returnValue(
      of({ activeChallenges: 1, votingProposals: 5, recentPrecedents: 2, activeDiscussions: 0 })
    );

    mockDataLoader = jasmine.createSpyObj('DataLoaderService', ['getContent']);
    // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
    mockDataLoader.getContent.and.returnValue(of(buildParentContent()) as any);

    mockRelatedConcepts = jasmine.createSpyObj('RelatedConceptsService', ['getRelatedConcepts']);
    // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
    mockRelatedConcepts.getRelatedConcepts.and.returnValue(of(buildRelatedConceptsResult()) as any);

    mockPathContextService = { currentContext: null };

    mockIdentityService = {
      profile: jasmine.createSpy('profile').and.returnValue({ affinities: ['community-alpha'] }),
    };

    mockSessionHumanService = jasmine.createSpyObj('SessionHumanService', ['getActivityHistory']);
    mockSessionHumanService.getActivityHistory.and.returnValue([buildActivity()]);

    TestBed.configureTestingModule({
      providers: [
        ContextAssemblyService,
        { provide: AgentService, useValue: mockAgentService },
        { provide: ElohimAgentService, useValue: mockElohimAgentService },
        { provide: GovernanceService, useValue: mockGovernanceService },
        { provide: DataLoaderService, useValue: mockDataLoader },
        { provide: RelatedConceptsService, useValue: mockRelatedConcepts },
        { provide: PathContextService, useValue: mockPathContextService },
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: SessionHumanService, useValue: mockSessionHumanService },
      ],
    });

    service = TestBed.inject(ContextAssemblyService);
  });

  // =========================================================================
  // Construction
  // =========================================================================

  describe('construction', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });
  });

  // =========================================================================
  // Thread Context Layer
  // =========================================================================

  describe('thread context layer', () => {
    it('should include session actions from activity history', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.threadContext.sessionActions.length).toBeGreaterThan(0);
        expect(result.createContext.threadContext.sessionActions[0].actionType).toBe('view');
      });
      tick(5000);
    }));

    it('should include path context when available', fakeAsync(() => {
      mockPathContextService.currentContext = {
        pathId: 'path-know-thyself',
        pathTitle: 'Know Thyself',
        stepIndex: 3,
        totalSteps: 10,
      };

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.threadContext.pathContext).toBeDefined();
        expect(result.createContext.threadContext.pathContext!.pathId).toBe('path-know-thyself');
        expect(result.createContext.threadContext.pathContext!.stepIndex).toBe(3);
      });
      tick(5000);
    }));

    it('should omit path context when not available', fakeAsync(() => {
      mockPathContextService.currentContext = null;

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.threadContext.pathContext).toBeUndefined();
      });
      tick(5000);
    }));

    it('should include threadId when provided', fakeAsync(() => {
      service
        .assembleAndNegotiate(buildPayload(), { threadId: 'thread-42', timeoutMs: 5000 })
        .subscribe(result => {
          expect(result.createContext.threadContext.threadId).toBe('thread-42');
        });
      tick(5000);
    }));

    it('should limit session actions to last 20', fakeAsync(() => {
      const activities = Array.from({ length: 30 }, (_, i) =>
        buildActivity({ resourceId: `r-${i}`, timestamp: new Date(Date.now() + i).toISOString() })
      );
      mockSessionHumanService.getActivityHistory.and.returnValue(activities);

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.threadContext.sessionActions.length).toBe(20);
      });
      tick(5000);
    }));
  });

  // =========================================================================
  // Author Profile Layer
  // =========================================================================

  describe('author profile layer', () => {
    it('should compute trust score from attestation types', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue(['author-verified', 'peer-reviewed']);

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        const profile = result.createContext.authorProfile;
        expect(profile.trustIndicators.trustScore).toBeGreaterThan(0);
        expect(profile.trustIndicators.trustScore).toBeLessThanOrEqual(1);
      });
      tick(5000);
    }));

    it('should return zero trust score for no attestations', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue([]);

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.authorProfile.trustIndicators.trustScore).toBe(0);
      });
      tick(5000);
    }));

    it('should derive reach ceiling from attestation types', fakeAsync(() => {
      // 'author-verified' meets requirements for 'invited' and 'local'
      mockAgentService.getAttestations.and.returnValue(['author-verified']);

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.authorProfile.reachCeiling).toBe('local');
      });
      tick(5000);
    }));

    it('should return private reach ceiling for no attestations', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue([]);

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.authorProfile.reachCeiling).toBe('private');
      });
      tick(5000);
    }));

    it('should cache author profile for 15 minutes', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe();
      tick(5000);

      // Reset spy call count
      mockAgentService.getLearningAnalytics.calls.reset();

      // Second call should use cache
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe();
      tick(5000);

      expect(mockAgentService.getLearningAnalytics).not.toHaveBeenCalled();
    }));

    it('should include author ID from agent service', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.authorProfile.authorId).toBe('agent-test-1');
      });
      tick(5000);
    }));

    it('should use default profile when analytics fails', fakeAsync(() => {
      mockAgentService.getLearningAnalytics.and.returnValue(throwError(() => new Error('fail')));

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.authorProfile.trustIndicators.trustScore).toBe(0);
        expect(result.createContext.authorProfile.reachCeiling).toBe('private');
      });
      tick(5000);
    }));
  });

  // =========================================================================
  // Constitutional Layer
  // =========================================================================

  describe('constitutional layer', () => {
    it('should use constitutionalBinding hash from selected elohim', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.constitutional.stackHash).toBe('sha256-abc123');
      });
      tick(5000);
    }));

    it('should include relevant principles from governance precedents', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.constitutional.relevantPrinciples.length).toBe(2);
        expect(result.createContext.constitutional.relevantPrinciples[0].name).toBe(
          'Human dignity'
        );
      });
      tick(5000);
    }));

    it('should cache constitutional layer for session', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe();
      tick(5000);

      mockElohimAgentService.selectElohim.calls.reset();
      mockGovernanceService.getConstitutionalPrecedents.calls.reset();

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe();
      tick(5000);

      expect(mockElohimAgentService.selectElohim).not.toHaveBeenCalled();
      expect(mockGovernanceService.getConstitutionalPrecedents).not.toHaveBeenCalled();
    }));

    it('should use default when elohim selection fails', fakeAsync(() => {
      mockElohimAgentService.selectElohim.and.returnValue(throwError(() => new Error('fail')));

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.constitutional.stackHash).toBe('unverified');
      });
      tick(5000);
    }));
  });

  // =========================================================================
  // Community Norms Layer
  // =========================================================================

  describe('community norms layer', () => {
    it('should include community IDs from identity profile affinities', fakeAsync(() => {
      // Low trust forces deep pipeline which includes community-norms
      mockAgentService.getAttestations.and.returnValue([]);
      mockAgentService.getLearningAnalytics.and.returnValue(
        of({ ...buildAnalytics(), totalStepsCompleted: 0 })
      );

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.communityNorms.communityIds).toContain('community-alpha');
      });
      tick(5000);
    }));

    it('should include active governance proposals', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue([]);
      mockAgentService.getLearningAnalytics.and.returnValue(
        of({ ...buildAnalytics(), totalStepsCompleted: 0 })
      );

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.communityNorms.activeGovernanceContext).toContain('proposal-1');
      });
      tick(5000);
    }));

    it('should be skipped for high-trust authors', fakeAsync(() => {
      // High trust: > 0.8 trust score, > 10 contributions in last 30 days
      mockAgentService.getAttestations.and.returnValue([
        'governance-ratified',
        'peer-reviewed',
        'steward-approved',
        'safety-reviewed',
        'community-endorsed',
      ]);
      mockAgentService.getLearningAnalytics.and.returnValue(
        of({ ...buildAnalytics(), totalStepsCompleted: 50 })
      );

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        const skipped = result.createContext.layersSkipped.map(s => s.layer);
        expect(skipped).toContain('community-norms');
      });
      tick(5000);
    }));
  });

  // =========================================================================
  // Graph Neighborhood Layer
  // =========================================================================

  describe('graph neighborhood layer', () => {
    it('should include related nodes when parent exists', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue([]);
      mockAgentService.getLearningAnalytics.and.returnValue(
        of({ ...buildAnalytics(), totalStepsCompleted: 0 })
      );

      const payload = buildPayload({ parentContentId: 'parent-1' });
      service.assembleAndNegotiate(payload, { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.graphNeighborhood.relatedNodes.length).toBeGreaterThan(0);
      });
      tick(5000);
    }));

    it('should return empty neighborhood when no parent', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue([]);
      mockAgentService.getLearningAnalytics.and.returnValue(
        of({ ...buildAnalytics(), totalStepsCompleted: 0 })
      );

      const payload = buildPayload({ parentContentId: undefined });
      service.assembleAndNegotiate(payload, { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.graphNeighborhood.relatedNodes.length).toBe(0);
      });
      tick(5000);
    }));

    it('should include parent context with reach and trust', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue([]);
      mockAgentService.getLearningAnalytics.and.returnValue(
        of({ ...buildAnalytics(), totalStepsCompleted: 0 })
      );

      const payload = buildPayload({ parentContentId: 'parent-1' });
      service.assembleAndNegotiate(payload, { timeoutMs: 5000 }).subscribe(result => {
        const parent = result.createContext.graphNeighborhood.parentContext;
        expect(parent).toBeDefined();
        expect(parent!.nodeId).toBe('parent-1');
        expect(parent!.reach).toBe('community');
      });
      tick(5000);
    }));

    it('should be skipped for high-trust authors', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue([
        'governance-ratified',
        'peer-reviewed',
        'steward-approved',
        'safety-reviewed',
        'community-endorsed',
      ]);
      mockAgentService.getLearningAnalytics.and.returnValue(
        of({ ...buildAnalytics(), totalStepsCompleted: 50 })
      );

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        const skipped = result.createContext.layersSkipped.map(s => s.layer);
        expect(skipped).toContain('graph-neighborhood');
      });
      tick(5000);
    }));
  });

  // =========================================================================
  // Pipeline Depth
  // =========================================================================

  describe('pipeline depth', () => {
    it('should use light pipeline for high-trust authors', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue([
        'governance-ratified',
        'peer-reviewed',
        'steward-approved',
        'safety-reviewed',
        'community-endorsed',
      ]);
      mockAgentService.getLearningAnalytics.and.returnValue(
        of({ ...buildAnalytics(), totalStepsCompleted: 50 })
      );

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        // High trust skips community-norms and graph-neighborhood
        const populated = result.createContext.layersPopulated;
        expect(populated).toContain('constitutional');
        expect(populated).toContain('author-profile');
        expect(populated).toContain('payload');

        const skippedLayers = result.createContext.layersSkipped.map(s => s.layer);
        expect(skippedLayers).toContain('community-norms');
        expect(skippedLayers).toContain('graph-neighborhood');
      });
      tick(5000);
    }));

    it('should use deep pipeline for new authors', fakeAsync(() => {
      mockAgentService.getAttestations.and.returnValue([]);
      mockAgentService.getLearningAnalytics.and.returnValue(
        of({ ...buildAnalytics(), totalStepsCompleted: 0 })
      );

      const payload = buildPayload({ parentContentId: 'parent-1' });
      service.assembleAndNegotiate(payload, { timeoutMs: 5000 }).subscribe(result => {
        // Deep pipeline includes all layers
        const populated = result.createContext.layersPopulated;
        expect(populated).toContain('constitutional');
        expect(populated).toContain('community-norms');
        expect(populated).toContain('author-profile');
        expect(populated).toContain('graph-neighborhood');
        expect(populated).toContain('thread-context');
        expect(populated).toContain('payload');
      });
      tick(5000);
    }));
  });

  // =========================================================================
  // Integration — Happy Path
  // =========================================================================

  describe('happy path integration', () => {
    it('should produce complete ContextAssemblyResult', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext).toBeDefined();
        expect(result.negotiationResult).toBeDefined();
        expect(result.birthContext).toBeDefined();
        expect(result.timedOut).toBe(false);
      });
      tick(5000);
    }));

    it('should have a unique contextId', fakeAsync(() => {
      let contextId1 = '';
      let contextId2 = '';

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(r => {
        contextId1 = r.createContext.contextId;
      });
      tick(5000);

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(r => {
        contextId2 = r.createContext.contextId;
      });
      tick(5000);

      expect(contextId1).toBeTruthy();
      expect(contextId2).toBeTruthy();
      expect(contextId1).not.toBe(contextId2);
    }));

    it('should invoke reach negotiation with assembled context', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe();
      tick(5000);

      expect(mockElohimAgentService.invoke).toHaveBeenCalled();
      const request = mockElohimAgentService.invoke.calls.mostRecent().args[0];
      expect(request.capability).toBe('reach-negotiation');
    }));

    it('should derive birth context from negotiation result', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.birthContext.constitutionalStackHash).toBe('sha256-abc123');
        expect(result.birthContext.createdAt).toBeTruthy();
        expect(result.birthContext.sourceContextHash).toBeTruthy();
      });
      tick(5000);
    }));

    it('should include author intent in birth context when provided', fakeAsync(() => {
      service
        .assembleAndNegotiate(buildPayload(), {
          authorIntent: 'Share knowledge about attachment theory',
          timeoutMs: 5000,
        })
        .subscribe(result => {
          expect(result.birthContext.authorIntent).toBe('Share knowledge about attachment theory');
        });
      tick(5000);
    }));

    it('should track assembly cost', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        const cost = result.createContext.assemblyCost;
        expect(cost.totalSizeBytes).toBeGreaterThan(0);
        expect(cost.layerTimings).toBeDefined();
      });
      tick(5000);
    }));
  });

  // =========================================================================
  // Timeout
  // =========================================================================

  describe('timeout', () => {
    it('should return fallback result when assembly times out', fakeAsync(() => {
      // Make a service very slow
      mockAgentService.getLearningAnalytics.and.returnValue(
        timer(2000).pipe(map(() => buildAnalytics()))
      );

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 100 }).subscribe(result => {
        expect(result.timedOut).toBe(true);
        expect(result.createContext.layersSkipped.length).toBeGreaterThan(0);
      });
      tick(2000);
    }));

    it('should use default 500ms timeout when not specified', fakeAsync(() => {
      mockAgentService.getLearningAnalytics.and.returnValue(
        timer(1000).pipe(map(() => buildAnalytics()))
      );

      service.assembleAndNegotiate(buildPayload()).subscribe(result => {
        expect(result.timedOut).toBe(true);
      });
      tick(1500);
    }));

    it('should still return valid structure on timeout', fakeAsync(() => {
      mockAgentService.getLearningAnalytics.and.returnValue(
        timer(2000).pipe(map(() => buildAnalytics()))
      );

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 50 }).subscribe(result => {
        expect(result.createContext).toBeDefined();
        expect(result.negotiationResult).toBeDefined();
        expect(result.birthContext).toBeDefined();
        expect(result.createContext.payload).toEqual(
          jasmine.objectContaining({ actionType: 'post' })
        );
      });
      tick(2000);
    }));
  });

  // =========================================================================
  // Graceful Degradation
  // =========================================================================

  describe('graceful degradation', () => {
    it('should use default negotiation result when invoke fails', fakeAsync(() => {
      mockElohimAgentService.invoke.and.returnValue(throwError(() => new Error('Network error')));

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.negotiationResult.recommendedReach).toBe('community');
        expect(result.negotiationResult.safetyCleared).toBe(true);
      });
      tick(5000);
    }));

    it('should use default when governance service fails', fakeAsync(() => {
      mockGovernanceService.getConstitutionalPrecedents.and.returnValue(
        throwError(() => new Error('fail'))
      );
      mockElohimAgentService.selectElohim.and.returnValue(throwError(() => new Error('fail')));

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.createContext.constitutional.stackHash).toBe('unverified');
        expect(result.timedOut).toBe(false);
      });
      tick(5000);
    }));

    it('should never throw — always returns a result', fakeAsync(() => {
      // Make everything fail
      mockAgentService.getLearningAnalytics.and.returnValue(throwError(() => new Error('fail')));
      mockElohimAgentService.invoke.and.returnValue(throwError(() => new Error('fail')));
      mockElohimAgentService.selectElohim.and.returnValue(throwError(() => new Error('fail')));
      mockGovernanceService.getConstitutionalPrecedents.and.returnValue(
        throwError(() => new Error('fail'))
      );

      let received = false;
      let errored = false;

      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe({
        next: () => (received = true),
        error: () => (errored = true),
      });
      tick(5000);

      expect(received).toBe(true);
      expect(errored).toBe(false);
    }));
  });

  // =========================================================================
  // Birth Context Privacy Gradient
  // =========================================================================

  describe('birth context privacy gradient', () => {
    it('should include author trust snapshot', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.birthContext.authorTrustSnapshot).toBeDefined();
        expect(result.birthContext.authorTrustSnapshot.trustScore).toBeDefined();
        expect(result.birthContext.authorTrustSnapshot.reachCeiling).toBeDefined();
      });
      tick(5000);
    }));

    it('should include constitutional stack hash', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.birthContext.constitutionalStackHash).toBeTruthy();
      });
      tick(5000);
    }));

    it('should include neighborhood summary', fakeAsync(() => {
      service.assembleAndNegotiate(buildPayload(), { timeoutMs: 5000 }).subscribe(result => {
        expect(result.birthContext.neighborhoodSummary).toBeDefined();
      });
      tick(5000);
    }));
  });
});
