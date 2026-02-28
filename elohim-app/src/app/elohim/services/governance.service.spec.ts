import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { GovernanceService, ChallengeSubmission, ProposalSubmission, Vote, DiscussionMessage } from './governance.service';
import { DataLoaderService, ChallengeRecord, ProposalRecord, PrecedentRecord, DiscussionRecord, GovernanceStateRecord } from './data-loader.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { vi, Mock } from 'vitest';

describe('GovernanceService', () => {
  let service: GovernanceService;
  let dataLoaderMock: any;
  let sessionMock: any;

  const mockSession = {
    sessionId: 'test-session-123',
    displayName: 'Test User',
    createdAt: new Date().toISOString(),
    lastActiveAt: new Date().toISOString(),
    stats: {
      nodesViewed: 0,
      nodesWithAffinity: 0,
      pathsStarted: 0,
      pathsCompleted: 0,
      stepsCompleted: 0,
      totalSessionTime: 0,
      averageSessionLength: 0,
      sessionCount: 1,
    },
    isAnonymous: true,
    accessLevel: 'visitor' as const,
    sessionState: 'active' as const,
  } as any;

  const mockChallenge: ChallengeRecord = {
    id: 'challenge-1',
    entityType: 'content',
    entityId: 'content-123',
    challenger: {
      agentId: 'user-123',
      displayName: 'Challenger',
      standing: 'community-member',
    },
    grounds: 'factual-error',
    description: 'Content has factual error',
    status: 'acknowledged',
    filedAt: new Date().toISOString(),
    slaDeadline: new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString(),
  };

  const mockProposal: ProposalRecord = {
    id: 'proposal-1',
    title: 'Test Proposal',
    proposalType: 'sense-check',
    description: 'Test description',
    proposer: {
      agentId: 'user-123',
      displayName: 'Proposer',
    },
    status: 'voting',
    phase: 'voting',
    createdAt: new Date().toISOString(),
  };

  const mockPrecedent: PrecedentRecord = {
    id: 'precedent-1',
    title: 'Content Review Standard',
    summary: 'Standard for content review',
    fullReasoning: 'Reasoning for the precedent',
    binding: 'constitutional',
    scope: { entityTypes: ['content'] },
    citations: 5,
    status: 'active',
  };

  const mockDiscussion: DiscussionRecord = {
    id: 'discussion-1',
    title: 'Content Review Discussion',
    entityType: 'content',
    entityId: 'content-123',
    category: 'content-review',
    messages: [],
    status: 'active',
    messageCount: 0,
  };

  const mockGovernanceState: GovernanceStateRecord = {
    entityType: 'content',
    entityId: 'content-123',
    status: 'under-review',
    statusBasis: {
      method: 'automatic',
      reasoning: 'Under review',
      deciderId: 'system',
      deciderType: 'system',
      decidedAt: new Date().toISOString(),
    },
    labels: [],
    activeChallenges: [],
    lastUpdated: new Date().toISOString(),
  };

  beforeEach(() => {
    // Mock DataLoaderService with all governance methods
    dataLoaderMock = {
    getGovernanceIndex: vi.fn(),
    getChallenges: vi.fn(),
    getChallengesForEntity: vi.fn(),
    getGovernanceState: vi.fn(),
    getProposals: vi.fn(),
    getProposalsByStatus: vi.fn(),
    getPrecedents: vi.fn(),
    getPrecedentsByBinding: vi.fn(),
    getDiscussions: vi.fn(),
    getDiscussionsForEntity: vi.fn(),
  };

    // Mock SessionHumanService
    sessionMock = {
    getSessionId: vi.fn(),
    getSession: vi.fn(),
  };

    // Default mock return values
    dataLoaderMock.getGovernanceIndex.mockReturnValue(
      of({
        lastUpdated: new Date().toISOString(),
        challengeCount: 1,
        proposalCount: 1,
        precedentCount: 1,
        discussionCount: 1,
      })
    );
    dataLoaderMock.getChallenges.mockReturnValue(of([mockChallenge]));
    dataLoaderMock.getChallengesForEntity.mockReturnValue(of([mockChallenge]));
    dataLoaderMock.getGovernanceState.mockReturnValue(of(mockGovernanceState));
    dataLoaderMock.getProposals.mockReturnValue(of([mockProposal]));
    dataLoaderMock.getProposalsByStatus.mockReturnValue(of([mockProposal]));
    dataLoaderMock.getPrecedents.mockReturnValue(of([mockPrecedent]));
    dataLoaderMock.getPrecedentsByBinding.mockReturnValue(of([mockPrecedent]));
    dataLoaderMock.getDiscussions.mockReturnValue(of([mockDiscussion]));
    dataLoaderMock.getDiscussionsForEntity.mockReturnValue(of([mockDiscussion]));

    sessionMock.getSessionId.mockReturnValue('test-session-123');
    sessionMock.getSession.mockReturnValue(mockSession);

    TestBed.configureTestingModule({
      providers: [
        GovernanceService,
        { provide: DataLoaderService, useValue: dataLoaderMock },
        { provide: SessionHumanService, useValue: sessionMock },
      ],
    });

    service = TestBed.inject(GovernanceService);
  });

  // ===========================================================================
  // Service Creation
  // ===========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should have getGovernanceIndex method', () => {
      expect(service.getGovernanceIndex).toBeDefined();
      expect(typeof service.getGovernanceIndex).toBe('function');
    });

    it('should have getGovernanceSummary method', () => {
      expect(service.getGovernanceSummary).toBeDefined();
      expect(typeof service.getGovernanceSummary).toBe('function');
    });

    it('should have getChallenges method', () => {
      expect(service.getChallenges).toBeDefined();
      expect(typeof service.getChallenges).toBe('function');
    });

    it('should have getProposals method', () => {
      expect(service.getProposals).toBeDefined();
      expect(typeof service.getProposals).toBe('function');
    });

    it('should have getPrecedents method', () => {
      expect(service.getPrecedents).toBeDefined();
      expect(typeof service.getPrecedents).toBe('function');
    });

    it('should have getDiscussions method', () => {
      expect(service.getDiscussions).toBeDefined();
      expect(typeof service.getDiscussions).toBe('function');
    });

    it('should have submitChallenge method', () => {
      expect(service.submitChallenge).toBeDefined();
      expect(typeof service.submitChallenge).toBe('function');
    });

    it('should have submitProposal method', () => {
      expect(service.submitProposal).toBeDefined();
      expect(typeof service.submitProposal).toBe('function');
    });

    it('should have voteOnProposal method', () => {
      expect(service.voteOnProposal).toBeDefined();
      expect(typeof service.voteOnProposal).toBe('function');
    });

    it('should have postMessage method', () => {
      expect(service.postMessage).toBeDefined();
      expect(typeof service.postMessage).toBe('function');
    });

    it('should have clearCache method', () => {
      expect(service.clearCache).toBeDefined();
      expect(typeof service.clearCache).toBe('function');
    });
  });

  // ===========================================================================
  // getGovernanceIndex
  // ===========================================================================

  describe('getGovernanceIndex', () => {
    it('should return governance index', () => new Promise<void>(done => {
      service.getGovernanceIndex().subscribe((index) => {
        expect(index).toBeDefined();
        expect(index.lastUpdated).toBeDefined();
        done();
      });
    }));

    it('should have challenge count', () => new Promise<void>(done => {
      service.getGovernanceIndex().subscribe((index) => {
        expect(index.challengeCount).toBeGreaterThanOrEqual(0);
        done();
      });
    }));

    it('should call dataLoader.getGovernanceIndex', () => new Promise<void>(done => {
      service.getGovernanceIndex().subscribe(() => {
        expect(dataLoaderMock.getGovernanceIndex).toHaveBeenCalled();
        done();
      });
    }));
  });

  // ===========================================================================
  // getGovernanceSummary
  // ===========================================================================

  describe('getGovernanceSummary', () => {
    it('should return governance summary', () => new Promise<void>(done => {
      service.getGovernanceSummary().subscribe((summary) => {
        expect(summary).toBeDefined();
        expect(summary.activeChallenges).toBeDefined();
        expect(summary.votingProposals).toBeDefined();
        done();
      });
    }));

    it('should count active challenges', () => new Promise<void>(done => {
      dataLoaderMock.getChallenges.mockReturnValue(
        of([
          { ...mockChallenge, status: 'acknowledged' },
          { ...mockChallenge, id: 'challenge-2', status: 'under-review' },
          { ...mockChallenge, id: 'challenge-3', status: 'resolved' },
        ])
      );

      service.getGovernanceSummary().subscribe((summary) => {
        expect(summary.activeChallenges).toBe(2);
        done();
      });
    }));

    it('should count voting proposals', () => new Promise<void>(done => {
      dataLoaderMock.getProposals.mockReturnValue(
        of([
          { ...mockProposal, status: 'voting' },
          { ...mockProposal, id: 'proposal-2', status: 'discussion' },
        ])
      );

      service.getGovernanceSummary().subscribe((summary) => {
        expect(summary.votingProposals).toBe(1);
        done();
      });
    }));
  });

  // ===========================================================================
  // Challenge Methods
  // ===========================================================================

  describe('getChallenges', () => {
    it('should return all challenges', () => new Promise<void>(done => {
      service.getChallenges().subscribe((challenges) => {
        expect(Array.isArray(challenges)).toBe(true);
        expect(challenges.length).toBeGreaterThan(0);
        done();
      });
    }));

    it('should cache challenges after first call', () => new Promise<void>(done => {
      service.getChallenges().subscribe(() => {
        expect(dataLoaderMock.getChallenges).toHaveBeenCalledTimes(1);

        service.getChallenges().subscribe(() => {
          expect(dataLoaderMock.getChallenges).toHaveBeenCalledTimes(1);
          done();
        });
      });
    }));
  });

  describe('getChallengesForEntity', () => {
    it('should return challenges for entity', () => new Promise<void>(done => {
      service.getChallengesForEntity('content', 'content-123').subscribe((challenges) => {
        expect(Array.isArray(challenges)).toBe(true);
        done();
      });
    }));

    it('should call dataLoader with correct parameters', () => new Promise<void>(done => {
      service.getChallengesForEntity('content', 'content-123').subscribe(() => {
        expect(dataLoaderMock.getChallengesForEntity).toHaveBeenCalledWith('content', 'content-123');
        done();
      });
    }));
  });

  describe('getChallengesByStatus', () => {
    it('should filter challenges by status', () => new Promise<void>(done => {
      dataLoaderMock.getChallenges.mockReturnValue(
        of([
          { ...mockChallenge, status: 'acknowledged' },
          { ...mockChallenge, id: 'challenge-2', status: 'resolved' },
        ])
      );

      service.getChallengesByStatus('acknowledged').subscribe((challenges) => {
        expect(challenges.length).toBe(1);
        expect(challenges[0].status).toBe('acknowledged');
        done();
      });
    }));
  });

  describe('getMyChallenges', () => {
    it('should return current user challenges', () => new Promise<void>(done => {
      dataLoaderMock.getChallenges.mockReturnValue(
        of([
          { ...mockChallenge, challenger: { agentId: 'test-session-123', displayName: 'Me', standing: 'community-member' } },
          { ...mockChallenge, id: 'challenge-2', challenger: { agentId: 'other-user', displayName: 'Other', standing: 'community-member' } },
        ])
      );

      service.getMyChallenges().subscribe((challenges) => {
        expect(challenges.length).toBe(1);
        expect(challenges[0].challenger.agentId).toBe('test-session-123');
        done();
      });
    }));
  });

  describe('submitChallenge', () => {
    it('should create new challenge', () => new Promise<void>(done => {
      const submission: ChallengeSubmission = {
        entityType: 'content',
        entityId: 'content-123',
        grounds: 'factual-error',
        description: 'Test error',
      };

      service.submitChallenge(submission).subscribe((challenge) => {
        expect(challenge).toBeDefined();
        expect(challenge.entityType).toBe('content');
        expect(challenge.status).toBe('pending');
        done();
      });
    }));

    it('should set correct challenger', () => new Promise<void>(done => {
      const submission: ChallengeSubmission = {
        entityType: 'content',
        entityId: 'content-123',
        grounds: 'factual-error',
        description: 'Test error',
      };

      service.submitChallenge(submission).subscribe((challenge) => {
        expect(challenge.challenger.agentId).toBe('test-session-123');
        expect(challenge.challenger.displayName).toBe('Test User');
        done();
      });
    }));

    it('should set SLA deadline', () => new Promise<void>(done => {
      const submission: ChallengeSubmission = {
        entityType: 'content',
        entityId: 'content-123',
        grounds: 'factual-error',
        description: 'Test error',
      };

      service.submitChallenge(submission).subscribe((challenge) => {
        expect(challenge.slaDeadline).toBeDefined();
        if (challenge.slaDeadline) {
          const deadline = new Date(challenge.slaDeadline);
          const now = new Date();
          expect(deadline.getTime()).toBeGreaterThan(now.getTime());
        }
        done();
      });
    }));

    it('should clear cache after submission', () => new Promise<void>(done => {
      const submission: ChallengeSubmission = {
        entityType: 'content',
        entityId: 'content-123',
        grounds: 'factual-error',
        description: 'Test error',
      };

      service.submitChallenge(submission).subscribe(() => {
        service.getChallenges().subscribe(() => {
          expect(dataLoaderMock.getChallenges).toHaveBeenCalled();
          done();
        });
      });
    }));
  });

  // ===========================================================================
  // Proposal Methods
  // ===========================================================================

  describe('getProposals', () => {
    it('should return all proposals', () => new Promise<void>(done => {
      service.getProposals().subscribe((proposals) => {
        expect(Array.isArray(proposals)).toBe(true);
        done();
      });
    }));

    it('should cache proposals', () => new Promise<void>(done => {
      service.getProposals().subscribe(() => {
        expect(dataLoaderMock.getProposals).toHaveBeenCalledTimes(1);

        service.getProposals().subscribe(() => {
          expect(dataLoaderMock.getProposals).toHaveBeenCalledTimes(1);
          done();
        });
      });
    }));
  });

  describe('getProposalsByStatus', () => {
    it('should call dataLoader with status', () => new Promise<void>(done => {
      service.getProposalsByStatus('voting').subscribe(() => {
        expect(dataLoaderMock.getProposalsByStatus).toHaveBeenCalledWith('voting');
        done();
      });
    }));
  });

  describe('getActiveProposals', () => {
    it('should return voting proposals', () => new Promise<void>(done => {
      service.getActiveProposals().subscribe(() => {
        expect(dataLoaderMock.getProposalsByStatus).toHaveBeenCalledWith('voting');
        done();
      });
    }));
  });

  describe('getMyProposals', () => {
    it('should return current user proposals', () => new Promise<void>(done => {
      dataLoaderMock.getProposals.mockReturnValue(
        of([
          { ...mockProposal, proposer: { agentId: 'test-session-123', displayName: 'Me' } },
          { ...mockProposal, id: 'proposal-2', proposer: { agentId: 'other-user', displayName: 'Other' } },
        ])
      );

      service.getMyProposals().subscribe((proposals) => {
        expect(proposals.length).toBe(1);
        expect(proposals[0].proposer.agentId).toBe('test-session-123');
        done();
      });
    }));
  });

  describe('submitProposal', () => {
    it('should create new proposal', () => new Promise<void>(done => {
      const submission: ProposalSubmission = {
        title: 'New Proposal',
        proposalType: 'sense-check',
        description: 'Test proposal',
        rationale: 'To test',
      };

      service.submitProposal(submission).subscribe((proposal) => {
        expect(proposal).toBeDefined();
        expect(proposal.title).toBe('New Proposal');
        expect(proposal.status).toBe('discussion');
        done();
      });
    }));

    it('should set proposer', () => new Promise<void>(done => {
      const submission: ProposalSubmission = {
        title: 'New Proposal',
        proposalType: 'sense-check',
        description: 'Test proposal',
        rationale: 'To test',
      };

      service.submitProposal(submission).subscribe((proposal) => {
        expect(proposal.proposer.agentId).toBe('test-session-123');
        done();
      });
    }));
  });

  describe('voteOnProposal', () => {
    it('should record vote', () => new Promise<void>(done => {
      const vote: Vote = {
        proposalId: 'proposal-1',
        position: 'agree',
        reasoning: 'Good proposal',
      };

      service.voteOnProposal(vote).subscribe((result) => {
        expect(result).toBe(true);
        done();
      });
    }));

    it('should handle storage failure gracefully', () => new Promise<void>(done => {
      vi.spyOn(localStorage, 'setItem').mockImplementation(() => { throw 'Storage full'; });

      const vote: Vote = {
        proposalId: 'proposal-1',
        position: 'agree',
      };

      service.voteOnProposal(vote).subscribe((result) => {
        expect(result).toBe(false);
        done();
      });
    }));
  });

  describe('getMyVote', () => {
    beforeEach(() => {
      // Clear localStorage before each test to prevent pollution
      localStorage.clear();
    });

    it('should return null if no vote cast', () => new Promise<void>(done => {
      service.getMyVote('proposal-1').subscribe((vote) => {
        expect(vote).toBeNull();
        done();
      });
    }));

    it('should return stored vote', () => new Promise<void>(done => {
      const voteData = { proposalId: 'proposal-1', position: 'agree' };
      vi.spyOn(localStorage, 'getItem').mockReturnValue(JSON.stringify(voteData));

      service.getMyVote('proposal-1').subscribe((vote) => {
        expect(vote?.proposalId).toBe('proposal-1');
        done();
      });
    }));
  });

  // ===========================================================================
  // Precedent Methods
  // ===========================================================================

  describe('getPrecedents', () => {
    it('should return precedents', () => new Promise<void>(done => {
      service.getPrecedents().subscribe((precedents) => {
        expect(Array.isArray(precedents)).toBe(true);
        done();
      });
    }));

    it('should cache precedents', () => new Promise<void>(done => {
      service.getPrecedents().subscribe(() => {
        expect(dataLoaderMock.getPrecedents).toHaveBeenCalledTimes(1);

        service.getPrecedents().subscribe(() => {
          expect(dataLoaderMock.getPrecedents).toHaveBeenCalledTimes(1);
          done();
        });
      });
    }));
  });

  describe('getPrecedentsByBinding', () => {
    it('should call dataLoader with binding level', () => new Promise<void>(done => {
      service.getPrecedentsByBinding('constitutional').subscribe(() => {
        expect(dataLoaderMock.getPrecedentsByBinding).toHaveBeenCalledWith('constitutional');
        done();
      });
    }));
  });

  describe('getConstitutionalPrecedents', () => {
    it('should return constitutional precedents', () => new Promise<void>(done => {
      service.getConstitutionalPrecedents().subscribe(() => {
        expect(dataLoaderMock.getPrecedentsByBinding).toHaveBeenCalledWith('constitutional');
        done();
      });
    }));
  });

  describe('searchPrecedents', () => {
    it('should filter precedents by title', () => new Promise<void>(done => {
      dataLoaderMock.getPrecedents.mockReturnValue(
        of([
          { ...mockPrecedent, title: 'Content Review Standard', summary: 'Standard for content review' },
          { ...mockPrecedent, id: 'precedent-2', title: 'Safety Standard', summary: 'Safety procedures' },
        ])
      );

      service.searchPrecedents('Review').subscribe((results) => {
        expect(results.length).toBe(1);
        expect(results[0].title).toContain('Review');
        done();
      });
    }));

    it('should filter by summary', () => new Promise<void>(done => {
      dataLoaderMock.getPrecedents.mockReturnValue(
        of([
          { ...mockPrecedent, title: 'Standard 1', summary: 'About content' },
          { ...mockPrecedent, id: 'precedent-2', title: 'Standard 2', summary: 'About safety' },
        ])
      );

      service.searchPrecedents('safety').subscribe((results) => {
        expect(results.length).toBe(1);
        done();
      });
    }));

    it('should be case-insensitive', () => new Promise<void>(done => {
      dataLoaderMock.getPrecedents.mockReturnValue(
        of([{ ...mockPrecedent, title: 'Content Review Standard' }])
      );

      service.searchPrecedents('CONTENT').subscribe((results) => {
        expect(results.length).toBe(1);
        done();
      });
    }));
  });

  // ===========================================================================
  // Discussion Methods
  // ===========================================================================

  describe('getDiscussions', () => {
    it('should return discussions', () => new Promise<void>(done => {
      service.getDiscussions().subscribe((discussions) => {
        expect(Array.isArray(discussions)).toBe(true);
        done();
      });
    }));
  });

  describe('getDiscussionsForEntity', () => {
    it('should return discussions for entity', () => new Promise<void>(done => {
      service.getDiscussionsForEntity('content', 'content-123').subscribe((discussions) => {
        expect(Array.isArray(discussions)).toBe(true);
        done();
      });
    }));

    it('should call dataLoader with correct parameters', () => new Promise<void>(done => {
      service.getDiscussionsForEntity('content', 'content-123').subscribe(() => {
        expect(dataLoaderMock.getDiscussionsForEntity).toHaveBeenCalledWith('content', 'content-123');
        done();
      });
    }));
  });

  describe('postMessage', () => {
    it('should post message to discussion', () => new Promise<void>(done => {
      const message: DiscussionMessage = {
        discussionId: 'discussion-1',
        content: 'Great discussion',
      };

      service.postMessage(message).subscribe((result) => {
        expect(result).toBe(true);
        done();
      });
    }));

    it('should handle storage failure gracefully', () => new Promise<void>(done => {
      vi.spyOn(localStorage, 'setItem').mockImplementation(() => { throw 'Storage full'; });

      const message: DiscussionMessage = {
        discussionId: 'discussion-1',
        content: 'Great discussion',
      };

      service.postMessage(message).subscribe((result) => {
        expect(result).toBe(false);
        done();
      });
    }));
  });

  describe('getLocalMessages', () => {
    it('should return empty array if no messages', () => {
      const messages = service.getLocalMessages('discussion-1');
      expect(Array.isArray(messages)).toBe(true);
    });

    it('should return stored messages', () => {
      const mockMessages = [
        { id: 'msg-1', authorId: 'user-1', authorName: 'User', content: 'Hello', createdAt: new Date().toISOString() },
      ];
      vi.spyOn(localStorage, 'getItem').mockReturnValue(JSON.stringify(mockMessages));

      const messages = service.getLocalMessages('discussion-1');
      expect(messages.length).toBe(1);
      expect(messages[0].content).toBe('Hello');
    });
  });

  // ===========================================================================
  // Governance State & Entity Checks
  // ===========================================================================

  describe('getGovernanceState', () => {
    it('should return governance state for entity', () => new Promise<void>(done => {
      service.getGovernanceState('content', 'content-123').subscribe((state) => {
        expect(state).toBeDefined();
        done();
      });
    }));

    it('should call dataLoader with entity type and ID', () => new Promise<void>(done => {
      service.getGovernanceState('content', 'content-123').subscribe(() => {
        expect(dataLoaderMock.getGovernanceState).toHaveBeenCalledWith('content', 'content-123');
        done();
      });
    }));
  });

  describe('getEffectiveStatus', () => {
    it('should return state status if exists', () => new Promise<void>(done => {
      service.getEffectiveStatus('content', 'content-123').subscribe((status) => {
        expect(status).toBe('under-review');
        done();
      });
    }));

    it('should return unreviewed if no state', () => new Promise<void>(done => {
      dataLoaderMock.getGovernanceState.mockReturnValue(of(null));

      service.getEffectiveStatus('content', 'content-123').subscribe((status) => {
        expect(status).toBe('unreviewed');
        done();
      });
    }));
  });

  describe('isEntityChallenged', () => {
    it('should return true if active challenges exist', () => new Promise<void>(done => {
      dataLoaderMock.getChallengesForEntity.mockReturnValue(
        of([
          { ...mockChallenge, status: 'acknowledged' },
          { ...mockChallenge, id: 'challenge-2', status: 'resolved' },
        ])
      );

      service.isEntityChallenged('content', 'content-123').subscribe((result) => {
        expect(result).toBe(true);
        done();
      });
    }));

    it('should return false if no active challenges', () => new Promise<void>(done => {
      dataLoaderMock.getChallengesForEntity.mockReturnValue(of([{ ...mockChallenge, status: 'resolved' }]));

      service.isEntityChallenged('content', 'content-123').subscribe((result) => {
        expect(result).toBe(false);
        done();
      });
    }));
  });

  describe('getEntityLabels', () => {
    it('should return entity labels', () => new Promise<void>(done => {
      service.getEntityLabels('content', 'content-123').subscribe((labels) => {
        expect(Array.isArray(labels)).toBe(true);
        done();
      });
    }));

    it('should return empty array if no state', () => new Promise<void>(done => {
      dataLoaderMock.getGovernanceState.mockReturnValue(of(null));

      service.getEntityLabels('content', 'content-123').subscribe((labels) => {
        expect(labels).toEqual([]);
        done();
      });
    }));
  });

  // ===========================================================================
  // SLA & Deadline Tracking
  // ===========================================================================

  describe('getChallengesNearingDeadline', () => {
    it('should return challenges approaching deadline', () => new Promise<void>(done => {
      const soon = new Date(Date.now() + 2 * 24 * 60 * 60 * 1000).toISOString();
      dataLoaderMock.getChallenges.mockReturnValue(
        of([{ ...mockChallenge, status: 'acknowledged', slaDeadline: soon }])
      );

      service.getChallengesNearingDeadline(3).subscribe((challenges) => {
        expect(challenges.length).toBe(1);
        done();
      });
    }));

    it('should not return challenges beyond deadline window', () => new Promise<void>(done => {
      const far = new Date(Date.now() + 10 * 24 * 60 * 60 * 1000).toISOString();
      dataLoaderMock.getChallenges.mockReturnValue(
        of([{ ...mockChallenge, status: 'acknowledged', slaDeadline: far }])
      );

      service.getChallengesNearingDeadline(3).subscribe((challenges) => {
        expect(challenges.length).toBe(0);
        done();
      });
    }));
  });

  describe('isSlaBreached', () => {
    it('should return false if deadline not passed', () => {
      const future = new Date(Date.now() + 1 * 24 * 60 * 60 * 1000).toISOString();
      const challenge: ChallengeRecord = { ...mockChallenge, slaDeadline: future };

      expect(service.isSlaBreached(challenge)).toBe(false);
    });

    it('should return true if deadline passed', () => {
      const past = new Date(Date.now() - 1 * 24 * 60 * 60 * 1000).toISOString();
      const challenge: ChallengeRecord = { ...mockChallenge, slaDeadline: past };

      expect(service.isSlaBreached(challenge)).toBe(true);
    });

    it('should return false for resolved challenges', () => {
      const past = new Date(Date.now() - 1 * 24 * 60 * 60 * 1000).toISOString();
      const challenge: ChallengeRecord = {
        ...mockChallenge,
        status: 'resolved',
        slaDeadline: past,
      };

      expect(service.isSlaBreached(challenge)).toBe(false);
    });
  });

  // ===========================================================================
  // Cache Management
  // ===========================================================================

  describe('clearCache', () => {
    it('should clear all caches', () => new Promise<void>(done => {
      service.getChallenges().subscribe(() => {
        expect(dataLoaderMock.getChallenges).toHaveBeenCalledTimes(1);

        service.clearCache();

        service.getChallenges().subscribe(() => {
          expect(dataLoaderMock.getChallenges).toHaveBeenCalledTimes(2);
          done();
        });
      });
    }));

    it('should clear proposals cache', () => new Promise<void>(done => {
      service.getProposals().subscribe(() => {
        expect(dataLoaderMock.getProposals).toHaveBeenCalledTimes(1);

        service.clearCache();

        service.getProposals().subscribe(() => {
          expect(dataLoaderMock.getProposals).toHaveBeenCalledTimes(2);
          done();
        });
      });
    }));

    it('should clear precedents cache', () => new Promise<void>(done => {
      service.getPrecedents().subscribe(() => {
        expect(dataLoaderMock.getPrecedents).toHaveBeenCalledTimes(1);

        service.clearCache();

        service.getPrecedents().subscribe(() => {
          expect(dataLoaderMock.getPrecedents).toHaveBeenCalledTimes(2);
          done();
        });
      });
    }));
  });
});
