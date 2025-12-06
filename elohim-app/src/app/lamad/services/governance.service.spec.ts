import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { GovernanceService, ChallengeSubmission, ProposalSubmission, Vote, DiscussionMessage } from './governance.service';
import { DataLoaderService, GovernanceIndex, ChallengeRecord, ProposalRecord, PrecedentRecord, DiscussionRecord, GovernanceStateRecord } from '@app/elohim/services/data-loader.service';
import { SessionHumanService } from './session-human.service';

describe('GovernanceService', () => {
  let service: GovernanceService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let sessionUserSpy: jasmine.SpyObj<SessionHumanService>;
  let localStorageMock: { [key: string]: string };
  let mockStorage: Storage;

  const mockGovernanceIndex: GovernanceIndex = {
    lastUpdated: '2025-01-01T00:00:00.000Z',
    challengeCount: 3,
    proposalCount: 2,
    precedentCount: 5,
    discussionCount: 4
  };

  const mockChallenges: ChallengeRecord[] = [
    {
      id: 'challenge-1',
      entityType: 'content',
      entityId: 'content-1',
      challenger: { agentId: 'user-1', displayName: 'User One', standing: 'community-member' },
      grounds: 'factual-error',
      description: 'Contains incorrect information',
      status: 'acknowledged',
      filedAt: '2025-01-01T00:00:00.000Z',
      slaDeadline: new Date(Date.now() + 2 * 24 * 60 * 60 * 1000).toISOString()
    },
    {
      id: 'challenge-2',
      entityType: 'content',
      entityId: 'content-2',
      challenger: { agentId: 'user-2', displayName: 'User Two', standing: 'community-member' },
      grounds: 'outdated',
      description: 'Information is outdated',
      status: 'under-review',
      filedAt: '2025-01-02T00:00:00.000Z'
    },
    {
      id: 'challenge-3',
      entityType: 'path',
      entityId: 'path-1',
      challenger: { agentId: 'session-123', displayName: 'Current User', standing: 'community-member' },
      grounds: 'harmful',
      description: 'Potentially harmful content',
      status: 'resolved',
      filedAt: '2024-12-15T00:00:00.000Z',
      resolution: {
        outcome: 'dismissed',
        reasoning: 'Content reviewed and found appropriate',
        decidedBy: 'governance-steward',
        decidedAt: '2024-12-20T00:00:00.000Z'
      }
    }
  ];

  const mockProposals: ProposalRecord[] = [
    {
      id: 'proposal-1',
      title: 'Add new content category',
      proposalType: 'consent',
      description: 'Proposal to add a new content category for tutorials',
      proposer: { agentId: 'user-1', displayName: 'User One' },
      status: 'voting',
      phase: 'voting',
      createdAt: '2025-01-01T00:00:00.000Z',
      votingConfig: {
        mechanism: 'consent',
        quorum: 0.5,
        passageThreshold: 0.75
      },
      currentVotes: { agree: 5, abstain: 1, disagree: 1, block: 0 }
    },
    {
      id: 'proposal-2',
      title: 'Update governance rules',
      proposalType: 'consensus',
      description: 'Proposal to update governance rules',
      proposer: { agentId: 'session-123', displayName: 'Current User' },
      status: 'discussion',
      phase: 'discussion',
      createdAt: '2025-01-02T00:00:00.000Z'
    }
  ];

  const mockPrecedents: PrecedentRecord[] = [
    {
      id: 'precedent-1',
      title: 'Content Accuracy Standard',
      summary: 'Establishes standards for factual accuracy',
      fullReasoning: 'Full reasoning here...',
      binding: 'constitutional',
      scope: { entityTypes: ['content'], categories: ['educational'] },
      citations: 15,
      status: 'active'
    },
    {
      id: 'precedent-2',
      title: 'Challenge Response Time',
      summary: 'Sets SLA for challenge responses',
      fullReasoning: 'Full reasoning here...',
      binding: 'binding-network',
      scope: { entityTypes: ['challenge'] },
      citations: 8,
      status: 'active'
    }
  ];

  const mockDiscussions: DiscussionRecord[] = [
    {
      id: 'discussion-1',
      entityType: 'proposal',
      entityId: 'proposal-1',
      category: 'general',
      title: 'Discussion on new category proposal',
      messages: [
        { id: 'msg-1', authorId: 'user-1', authorName: 'User One', content: 'Great idea!', createdAt: '2025-01-01T00:00:00.000Z' }
      ],
      status: 'active',
      messageCount: 1
    }
  ];

  const mockGovernanceState: GovernanceStateRecord = {
    entityType: 'content',
    entityId: 'content-1',
    status: 'under-review',
    statusBasis: {
      method: 'challenge-filed',
      reasoning: 'Challenge filed against content',
      deciderId: 'system',
      deciderType: 'automated',
      decidedAt: '2025-01-01T00:00:00.000Z'
    },
    labels: [{ labelType: 'disputed', severity: 'warning', appliedBy: 'system' }],
    activeChallenges: ['challenge-1'],
    lastUpdated: '2025-01-01T00:00:00.000Z'
  };

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', [
      'getGovernanceIndex',
      'getGovernanceState',
      'getChallenges',
      'getChallengesForEntity',
      'getProposals',
      'getProposalsByStatus',
      'getPrecedents',
      'getPrecedentsByBinding',
      'getDiscussions',
      'getDiscussionsForEntity'
    ]);
    const sessionUserSpyObj = jasmine.createSpyObj('SessionHumanService', [
      'getSessionId',
      'getSession'
    ]);

    // Mock localStorage
    localStorageMock = {};
    mockStorage = {
      getItem: (key: string) => localStorageMock[key] || null,
      setItem: (key: string, value: string) => { localStorageMock[key] = value; },
      removeItem: (key: string) => { delete localStorageMock[key]; },
      key: (index: number) => Object.keys(localStorageMock)[index] || null,
      get length() { return Object.keys(localStorageMock).length; },
      clear: () => { localStorageMock = {}; }
    };
    spyOnProperty(window, 'localStorage', 'get').and.returnValue(mockStorage);

    TestBed.configureTestingModule({
      providers: [
        GovernanceService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: SessionHumanService, useValue: sessionUserSpyObj }
      ]
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    sessionUserSpy = TestBed.inject(SessionHumanService) as jasmine.SpyObj<SessionHumanService>;

    // Default spy returns
    dataLoaderSpy.getGovernanceIndex.and.returnValue(of(mockGovernanceIndex));
    dataLoaderSpy.getGovernanceState.and.returnValue(of(mockGovernanceState));
    dataLoaderSpy.getChallenges.and.returnValue(of(mockChallenges));
    dataLoaderSpy.getChallengesForEntity.and.returnValue(of([mockChallenges[0]]));
    dataLoaderSpy.getProposals.and.returnValue(of(mockProposals));
    dataLoaderSpy.getProposalsByStatus.and.returnValue(of([mockProposals[0]]));
    dataLoaderSpy.getPrecedents.and.returnValue(of(mockPrecedents));
    dataLoaderSpy.getPrecedentsByBinding.and.returnValue(of([mockPrecedents[0]]));
    dataLoaderSpy.getDiscussions.and.returnValue(of(mockDiscussions));
    dataLoaderSpy.getDiscussionsForEntity.and.returnValue(of(mockDiscussions));

    sessionUserSpy.getSessionId.and.returnValue('session-123');
    sessionUserSpy.getSession.and.returnValue({ displayName: 'Current User' } as any);

    service = TestBed.inject(GovernanceService);
  });

  afterEach(() => {
    localStorageMock = {};
    service.clearCache();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // Governance Index & Overview
  // =========================================================================

  describe('getGovernanceIndex', () => {
    it('should return governance index from data loader', (done) => {
      service.getGovernanceIndex().subscribe(index => {
        expect(index).toEqual(mockGovernanceIndex);
        expect(dataLoaderSpy.getGovernanceIndex).toHaveBeenCalled();
        done();
      });
    });
  });

  describe('getGovernanceSummary', () => {
    it('should return summarized governance counts', (done) => {
      service.getGovernanceSummary().subscribe(summary => {
        expect(summary.activeChallenges).toBe(2); // acknowledged + under-review
        expect(summary.votingProposals).toBe(1);
        expect(summary.recentPrecedents).toBe(2);
        expect(summary.activeDiscussions).toBe(1);
        done();
      });
    });
  });

  // =========================================================================
  // Entity Governance State
  // =========================================================================

  describe('getGovernanceState', () => {
    it('should return governance state for entity', (done) => {
      service.getGovernanceState('content', 'content-1').subscribe(state => {
        expect(state).toEqual(mockGovernanceState);
        expect(dataLoaderSpy.getGovernanceState).toHaveBeenCalledWith('content', 'content-1');
        done();
      });
    });

    it('should return null for entity without state', (done) => {
      dataLoaderSpy.getGovernanceState.and.returnValue(of(null));

      service.getGovernanceState('content', 'unknown').subscribe(state => {
        expect(state).toBeNull();
        done();
      });
    });
  });

  describe('getEffectiveStatus', () => {
    it('should return status from governance state', (done) => {
      service.getEffectiveStatus('content', 'content-1').subscribe(status => {
        expect(status).toBe('under-review');
        done();
      });
    });

    it('should return "unreviewed" when no state exists', (done) => {
      dataLoaderSpy.getGovernanceState.and.returnValue(of(null));

      service.getEffectiveStatus('content', 'unknown').subscribe(status => {
        expect(status).toBe('unreviewed');
        done();
      });
    });
  });

  describe('isEntityChallenged', () => {
    it('should return true when entity has active challenges', (done) => {
      service.isEntityChallenged('content', 'content-1').subscribe(isChallenged => {
        expect(isChallenged).toBe(true);
        done();
      });
    });

    it('should return false when entity has no active challenges', (done) => {
      dataLoaderSpy.getChallengesForEntity.and.returnValue(of([mockChallenges[2]])); // resolved challenge

      service.isEntityChallenged('path', 'path-1').subscribe(isChallenged => {
        expect(isChallenged).toBe(false);
        done();
      });
    });
  });

  describe('getEntityLabels', () => {
    it('should return labels from governance state', (done) => {
      service.getEntityLabels('content', 'content-1').subscribe(labels => {
        expect(labels.length).toBe(1);
        expect(labels[0].labelType).toBe('disputed');
        done();
      });
    });

    it('should return empty array when no state exists', (done) => {
      dataLoaderSpy.getGovernanceState.and.returnValue(of(null));

      service.getEntityLabels('content', 'unknown').subscribe(labels => {
        expect(labels).toEqual([]);
        done();
      });
    });
  });

  // =========================================================================
  // Challenges
  // =========================================================================

  describe('getChallenges', () => {
    it('should return all challenges', (done) => {
      service.getChallenges().subscribe(challenges => {
        expect(challenges.length).toBe(3);
        done();
      });
    });

    it('should cache challenges', (done) => {
      service.getChallenges().subscribe();
      service.getChallenges().subscribe(challenges => {
        expect(dataLoaderSpy.getChallenges).toHaveBeenCalledTimes(1);
        done();
      });
    });
  });

  describe('getChallengesForEntity', () => {
    it('should return challenges for specific entity', (done) => {
      service.getChallengesForEntity('content', 'content-1').subscribe(challenges => {
        expect(dataLoaderSpy.getChallengesForEntity).toHaveBeenCalledWith('content', 'content-1');
        done();
      });
    });
  });

  describe('getChallengesByStatus', () => {
    it('should filter challenges by status', (done) => {
      service.getChallengesByStatus('acknowledged').subscribe(challenges => {
        expect(challenges.length).toBe(1);
        expect(challenges[0].status).toBe('acknowledged');
        done();
      });
    });
  });

  describe('getMyChallenges', () => {
    it('should return challenges filed by current user', (done) => {
      service.getMyChallenges().subscribe(challenges => {
        expect(challenges.length).toBe(1);
        expect(challenges[0].challenger.agentId).toBe('session-123');
        done();
      });
    });
  });

  describe('submitChallenge', () => {
    it('should create and save a new challenge', (done) => {
      const submission: ChallengeSubmission = {
        entityType: 'content',
        entityId: 'content-new',
        grounds: 'factual-error',
        description: 'This content has errors'
      };

      service.submitChallenge(submission).subscribe(challenge => {
        expect(challenge.id).toContain('challenge-local-');
        expect(challenge.entityType).toBe('content');
        expect(challenge.entityId).toBe('content-new');
        expect(challenge.status).toBe('pending');
        expect(challenge.challenger.agentId).toBe('session-123');
        expect(challenge.slaDeadline).toBeDefined();

        // Check localStorage
        const stored = localStorageMock['lamad-governance-local-challenges'];
        expect(stored).toBeDefined();
        done();
      });
    });

    it('should use anonymous when no session', (done) => {
      sessionUserSpy.getSessionId.and.returnValue(undefined as unknown as string);
      sessionUserSpy.getSession.and.returnValue(null);

      const submission: ChallengeSubmission = {
        entityType: 'content',
        entityId: 'content-new',
        grounds: 'outdated',
        description: 'Outdated content'
      };

      service.submitChallenge(submission).subscribe(challenge => {
        expect(challenge.challenger.agentId).toBe('anonymous');
        expect(challenge.challenger.displayName).toBe('Anonymous');
        done();
      });
    });
  });

  // =========================================================================
  // Proposals
  // =========================================================================

  describe('getProposals', () => {
    it('should return all proposals', (done) => {
      service.getProposals().subscribe(proposals => {
        expect(proposals.length).toBe(2);
        done();
      });
    });

    it('should cache proposals', (done) => {
      service.getProposals().subscribe();
      service.getProposals().subscribe(() => {
        expect(dataLoaderSpy.getProposals).toHaveBeenCalledTimes(1);
        done();
      });
    });
  });

  describe('getProposalsByStatus', () => {
    it('should filter proposals by status', (done) => {
      service.getProposalsByStatus('voting').subscribe(proposals => {
        expect(dataLoaderSpy.getProposalsByStatus).toHaveBeenCalledWith('voting');
        done();
      });
    });
  });

  describe('getActiveProposals', () => {
    it('should return proposals in voting phase', (done) => {
      service.getActiveProposals().subscribe(proposals => {
        expect(dataLoaderSpy.getProposalsByStatus).toHaveBeenCalledWith('voting');
        done();
      });
    });
  });

  describe('getMyProposals', () => {
    it('should return proposals created by current user', (done) => {
      service.getMyProposals().subscribe(proposals => {
        expect(proposals.length).toBe(1);
        expect(proposals[0].proposer.agentId).toBe('session-123');
        done();
      });
    });
  });

  describe('submitProposal', () => {
    it('should create and save a new proposal', (done) => {
      const submission: ProposalSubmission = {
        title: 'New Proposal',
        proposalType: 'sense-check',
        description: 'Testing a new idea',
        rationale: 'Because we need it'
      };

      service.submitProposal(submission).subscribe(proposal => {
        expect(proposal.id).toContain('proposal-local-');
        expect(proposal.title).toBe('New Proposal');
        expect(proposal.status).toBe('discussion');
        expect(proposal.proposer.agentId).toBe('session-123');

        // Check localStorage
        const stored = localStorageMock['lamad-governance-local-proposals'];
        expect(stored).toBeDefined();
        done();
      });
    });
  });

  describe('voteOnProposal', () => {
    it('should save vote to localStorage', (done) => {
      const vote: Vote = {
        proposalId: 'proposal-1',
        position: 'agree',
        reasoning: 'I support this proposal'
      };

      service.voteOnProposal(vote).subscribe(success => {
        expect(success).toBe(true);

        const key = 'lamad-governance-vote-session-123-proposal-1';
        const stored = JSON.parse(localStorageMock[key]);
        expect(stored.position).toBe('agree');
        expect(stored.reasoning).toBe('I support this proposal');
        done();
      });
    });
  });

  describe('getMyVote', () => {
    it('should return vote from localStorage', (done) => {
      const vote = {
        proposalId: 'proposal-1',
        position: 'disagree',
        votedAt: '2025-01-01T00:00:00.000Z',
        agentId: 'session-123'
      };
      localStorageMock['lamad-governance-vote-session-123-proposal-1'] = JSON.stringify(vote);

      service.getMyVote('proposal-1').subscribe(myVote => {
        expect(myVote).not.toBeNull();
        expect(myVote?.position).toBe('disagree');
        done();
      });
    });

    it('should return null when no vote exists', (done) => {
      service.getMyVote('proposal-unknown').subscribe(myVote => {
        expect(myVote).toBeNull();
        done();
      });
    });
  });

  // =========================================================================
  // Precedents
  // =========================================================================

  describe('getPrecedents', () => {
    it('should return all precedents', (done) => {
      service.getPrecedents().subscribe(precedents => {
        expect(precedents.length).toBe(2);
        done();
      });
    });

    it('should cache precedents', (done) => {
      service.getPrecedents().subscribe();
      service.getPrecedents().subscribe(() => {
        expect(dataLoaderSpy.getPrecedents).toHaveBeenCalledTimes(1);
        done();
      });
    });
  });

  describe('getPrecedentsByBinding', () => {
    it('should filter precedents by binding level', (done) => {
      service.getPrecedentsByBinding('constitutional').subscribe(precedents => {
        expect(dataLoaderSpy.getPrecedentsByBinding).toHaveBeenCalledWith('constitutional');
        done();
      });
    });
  });

  describe('getConstitutionalPrecedents', () => {
    it('should return constitutional precedents', (done) => {
      service.getConstitutionalPrecedents().subscribe(precedents => {
        expect(dataLoaderSpy.getPrecedentsByBinding).toHaveBeenCalledWith('constitutional');
        done();
      });
    });
  });

  describe('searchPrecedents', () => {
    it('should search precedents by title', (done) => {
      service.searchPrecedents('accuracy').subscribe(precedents => {
        expect(precedents.length).toBe(1);
        expect(precedents[0].title).toContain('Accuracy');
        done();
      });
    });

    it('should search precedents by summary', (done) => {
      service.searchPrecedents('SLA').subscribe(precedents => {
        expect(precedents.length).toBe(1);
        expect(precedents[0].summary).toContain('SLA');
        done();
      });
    });

    it('should return empty for no matches', (done) => {
      service.searchPrecedents('nonexistent').subscribe(precedents => {
        expect(precedents.length).toBe(0);
        done();
      });
    });
  });

  // =========================================================================
  // Discussions
  // =========================================================================

  describe('getDiscussions', () => {
    it('should return all discussions', (done) => {
      service.getDiscussions().subscribe(discussions => {
        expect(discussions.length).toBe(1);
        expect(dataLoaderSpy.getDiscussions).toHaveBeenCalled();
        done();
      });
    });
  });

  describe('getDiscussionsForEntity', () => {
    it('should return discussions for specific entity', (done) => {
      service.getDiscussionsForEntity('proposal', 'proposal-1').subscribe(discussions => {
        expect(dataLoaderSpy.getDiscussionsForEntity).toHaveBeenCalledWith('proposal', 'proposal-1');
        done();
      });
    });
  });

  describe('postMessage', () => {
    it('should save message to localStorage', (done) => {
      const message: DiscussionMessage = {
        discussionId: 'discussion-1',
        content: 'This is my contribution'
      };

      service.postMessage(message).subscribe(success => {
        expect(success).toBe(true);

        const messages = service.getLocalMessages('discussion-1');
        expect(messages.length).toBe(1);
        expect(messages[0].content).toBe('This is my contribution');
        expect(messages[0].authorId).toBe('session-123');
        done();
      });
    });

    it('should append to existing messages', (done) => {
      // Add first message
      service.postMessage({ discussionId: 'discussion-1', content: 'First' }).subscribe(() => {
        // Add second message
        service.postMessage({ discussionId: 'discussion-1', content: 'Second' }).subscribe(success => {
          expect(success).toBe(true);

          const messages = service.getLocalMessages('discussion-1');
          expect(messages.length).toBe(2);
          done();
        });
      });
    });
  });

  describe('getLocalMessages', () => {
    it('should return empty array when no messages', () => {
      const messages = service.getLocalMessages('unknown-discussion');
      expect(messages).toEqual([]);
    });
  });

  // =========================================================================
  // SLA & Deadline Tracking
  // =========================================================================

  describe('getChallengesNearingDeadline', () => {
    it('should return challenges nearing SLA deadline', (done) => {
      service.getChallengesNearingDeadline(5).subscribe(challenges => {
        // mockChallenges[0] has deadline in 2 days, should be included
        expect(challenges.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should exclude resolved challenges', (done) => {
      service.getChallengesNearingDeadline(30).subscribe(challenges => {
        const resolved = challenges.filter(c => c.status === 'resolved');
        expect(resolved.length).toBe(0);
        done();
      });
    });
  });

  describe('isSlaBreached', () => {
    it('should return true when deadline has passed', () => {
      const breachedChallenge: ChallengeRecord = {
        ...mockChallenges[0],
        slaDeadline: '2020-01-01T00:00:00.000Z' // Past date
      };

      expect(service.isSlaBreached(breachedChallenge)).toBe(true);
    });

    it('should return false when deadline is in future', () => {
      const futureChallenge: ChallengeRecord = {
        ...mockChallenges[0],
        slaDeadline: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString()
      };

      expect(service.isSlaBreached(futureChallenge)).toBe(false);
    });

    it('should return false for resolved challenges', () => {
      const resolved: ChallengeRecord = {
        ...mockChallenges[0],
        status: 'resolved',
        slaDeadline: '2020-01-01T00:00:00.000Z'
      };

      expect(service.isSlaBreached(resolved)).toBe(false);
    });

    it('should return false when no deadline set', () => {
      const noDeadline: ChallengeRecord = {
        ...mockChallenges[1] // No slaDeadline
      };

      expect(service.isSlaBreached(noDeadline)).toBe(false);
    });
  });

  // =========================================================================
  // Cache Management
  // =========================================================================

  describe('clearCache', () => {
    it('should clear all caches', (done) => {
      // Populate caches
      service.getChallenges().subscribe();
      service.getProposals().subscribe();
      service.getPrecedents().subscribe();

      // Clear caches
      service.clearCache();

      // Next call should hit data loader again
      service.getChallenges().subscribe(() => {
        expect(dataLoaderSpy.getChallenges).toHaveBeenCalledTimes(2);
        done();
      });
    });
  });
});
