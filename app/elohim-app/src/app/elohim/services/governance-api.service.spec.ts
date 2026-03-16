/**
 * GovernanceApiService Tests
 *
 * Tests that Sprint 4 & 5 governance API methods call the correct
 * HTTP endpoints with proper parameters.
 */

import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { describe, expect, it } from 'vitest';

import type {
  AppealView,
  CastRankedVoteInputView,
  ChallengeView,
  CreateStatementInputView,
  FileAppealInputView,
  FileChallengeInputView,
  GovernanceSignalView,
  ProposalOptionView,
  RankedVoteView,
  RecordSignalInputView,
  RespondToChallengeInputView,
  SensemakingResultView,
  StatementView,
  StatementVoteView,
  TallyResult,
  VoteOnStatementInputView,
} from '@elohim/storage-client/generated';

import { GovernanceApiService } from './governance-api.service';

describe('GovernanceApiService', () => {
  let service: GovernanceApiService;
  let httpMock: HttpTestingController;

  const mockOption: ProposalOptionView = {
    id: 'opt-1',
    proposalId: 'prop-1',
    label: 'Option A',
    description: 'First option',
    position: 0,
    source: null,
    sourceJustification: null,
    createdAt: '2026-03-15T00:00:00Z',
  };

  const mockRankedVote: RankedVoteView = {
    id: 'rv-1',
    proposalId: 'prop-1',
    humanId: 'human-1',
    optionId: 'opt-1',
    rank: 1,
    score: null,
    dots: null,
    approved: null,
    reasoning: null,
    proxyElohimId: null,
    createdAt: '2026-03-15T00:00:00Z',
  };

  const mockTally: TallyResult = {
    mechanism: 'ranked-choice',
    totalVoters: 5,
    quorumMet: true,
    optionResults: [],
    recommendation: 'Option A wins',
    rounds: null,
  };

  const mockSignal: GovernanceSignalView = {
    id: 'sig-1',
    entityType: 'content',
    entityId: 'entity-1',
    humanId: 'human-1',
    signalType: 'reaction',
    signalValue: 'agree',
    mechanismLevel: 1,
    proxyElohimId: null,
    createdAt: '2026-03-15T00:00:00Z',
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [GovernanceApiService, provideHttpClient(), provideHttpClientTesting()],
    });

    service = TestBed.inject(GovernanceApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getProposalOptions', () => {
    it('should GET /api/v1/governance/proposals/{id}/options', async () => {
      const promise = service.getProposalOptions('prop-1');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/proposals/prop-1/options' && r.method === 'GET',
      );
      req.flush([mockOption]);

      const result = await promise;
      expect(result).toHaveLength(1);
      expect(result[0].id).toBe('opt-1');
      expect(result[0].label).toBe('Option A');
    });

    it('should return empty array on error', async () => {
      const promise = service.getProposalOptions('prop-missing');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/proposals/prop-missing/options',
      );
      req.flush('Not found', { status: 404, statusText: 'Not Found' });

      const result = await promise;
      expect(result).toEqual([]);
    });
  });

  describe('castRankedVotes', () => {
    it('should POST to /api/v1/governance/proposals/{id}/ranked-votes', async () => {
      const ballot: CastRankedVoteInputView = {
        humanId: 'human-1',
        ballots: [
          { optionId: 'opt-1', rank: 1, score: null, dots: null, approved: null },
          { optionId: 'opt-2', rank: 2, score: null, dots: null, approved: null },
        ],
        reasoning: 'Option A is better',
        proxyElohimId: null,
        proxyJustification: null,
      };

      const promise = service.castRankedVotes('prop-1', ballot);

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/proposals/prop-1/ranked-votes' && r.method === 'POST',
      );
      expect(req.request.body).toEqual(ballot);
      req.flush([mockRankedVote]);

      const result = await promise;
      expect(result).toHaveLength(1);
      expect(result[0].rank).toBe(1);
    });
  });

  describe('getTally', () => {
    it('should GET /api/v1/governance/proposals/{id}/tally', async () => {
      const promise = service.getTally('prop-1');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/proposals/prop-1/tally' && r.method === 'GET',
      );
      req.flush(mockTally);

      const result = await promise;
      expect(result.mechanism).toBe('ranked-choice');
      expect(result.totalVoters).toBe(5);
      expect(result.quorumMet).toBe(true);
      expect(result.recommendation).toBe('Option A wins');
    });
  });

  describe('recordSignal', () => {
    it('should POST to /api/v1/governance/signals', async () => {
      const signal: RecordSignalInputView = {
        entityType: 'content',
        entityId: 'entity-1',
        humanId: 'human-1',
        signalType: 'reaction',
        signalValue: 'agree',
        mechanismLevel: 1,
        proxyElohimId: null,
      };

      const promise = service.recordSignal(signal);

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/signals' && r.method === 'POST',
      );
      expect(req.request.body).toEqual(signal);
      req.flush(mockSignal);

      const result = await promise;
      expect(result.id).toBe('sig-1');
      expect(result.signalType).toBe('reaction');
    });
  });

  describe('getSignals', () => {
    it('should GET /api/v1/governance/signals with entityType and entityId params', async () => {
      const promise = service.getSignals('content', 'entity-1');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/signals' &&
          r.method === 'GET' &&
          r.params.get('entityType') === 'content' &&
          r.params.get('entityId') === 'entity-1',
      );
      req.flush([mockSignal]);

      const result = await promise;
      expect(result).toHaveLength(1);
      expect(result[0].entityType).toBe('content');
      expect(result[0].entityId).toBe('entity-1');
    });

    it('should return empty array on error', async () => {
      const promise = service.getSignals('content', 'entity-missing');

      const req = httpMock.expectOne((r) => r.url === '/api/v1/governance/signals');
      req.flush('Server error', { status: 500, statusText: 'Internal Server Error' });

      const result = await promise;
      expect(result).toEqual([]);
    });
  });

  describe('getRankedVotes', () => {
    it('should GET /api/v1/governance/proposals/{id}/ranked-votes', async () => {
      const promise = service.getRankedVotes('prop-1');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/proposals/prop-1/ranked-votes' && r.method === 'GET',
      );
      req.flush([mockRankedVote]);

      const result = await promise;
      expect(result).toHaveLength(1);
      expect(result[0].optionId).toBe('opt-1');
    });
  });

  // --- Sprint 5: Challenges & Appeals ---

  const mockChallenge: ChallengeView = {
    id: 'ch-1',
    entityType: 'content',
    entityId: 'entity-1',
    challengerId: 'human-1',
    standingBasis: 'steward',
    groundsPrimary: 'accuracy',
    groundsSecondary: null,
    evidence: 'The claim is unsupported.',
    requestedOutcome: 'correction',
    slaDeadline: '2026-03-20T00:00:00Z',
    state: 'pending',
    outcome: null,
    reasoning: null,
    actions: null,
    setsPrecedent: false,
    respondedAt: null,
    respondedBy: null,
    createdAt: '2026-03-15T00:00:00Z',
  };

  const mockAppeal: AppealView = {
    id: 'appeal-1',
    challengeId: 'ch-1',
    appellantId: 'human-1',
    grounds: 'New evidence available',
    additionalEvidence: 'See document X',
    state: 'pending',
    escalationLevel: null,
    decision: null,
    decisionReasoning: null,
    decidedBy: null,
    filedAt: '2026-03-16T00:00:00Z',
    decidedAt: null,
    createdAt: '2026-03-16T00:00:00Z',
  };

  describe('fileChallenge', () => {
    it('should POST to /api/v1/governance/challenges', async () => {
      const input: FileChallengeInputView = {
        entityType: 'content',
        entityId: 'entity-1',
        challengerId: 'human-1',
        standingBasis: 'steward',
        groundsPrimary: 'accuracy',
        groundsSecondary: null,
        evidence: 'The claim is unsupported.',
        requestedOutcome: 'correction',
      };

      const promise = service.fileChallenge(input);

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/challenges' && r.method === 'POST',
      );
      expect(req.request.body).toEqual(input);
      req.flush(mockChallenge);

      const result = await promise;
      expect(result.id).toBe('ch-1');
      expect(result.state).toBe('pending');
      expect(result.groundsPrimary).toBe('accuracy');
    });
  });

  describe('respondToChallenge', () => {
    it('should POST to /api/v1/governance/challenges/{id}/respond', async () => {
      const input: RespondToChallengeInputView = {
        outcome: 'upheld',
        reasoning: 'The evidence supports the challenge.',
        actions: 'Content will be corrected.',
        setsPrecedent: true,
      };

      const promise = service.respondToChallenge('ch-1', input);

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/challenges/ch-1/respond' && r.method === 'POST',
      );
      expect(req.request.body).toEqual(input);
      req.flush({ ...mockChallenge, state: 'resolved', outcome: 'upheld' });

      const result = await promise;
      expect(result.state).toBe('resolved');
      expect(result.outcome).toBe('upheld');
    });
  });

  describe('fileAppeal', () => {
    it('should POST to /api/v1/governance/challenges/{id}/appeal', async () => {
      const input: FileAppealInputView = {
        grounds: 'New evidence available',
        additionalEvidence: 'See document X',
      };

      const promise = service.fileAppeal('ch-1', input);

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/challenges/ch-1/appeal' && r.method === 'POST',
      );
      expect(req.request.body).toEqual(input);
      req.flush(mockAppeal);

      const result = await promise;
      expect(result.id).toBe('appeal-1');
      expect(result.challengeId).toBe('ch-1');
      expect(result.grounds).toBe('New evidence available');
    });
  });

  describe('getChallengesForEntity', () => {
    it('should GET /api/v1/governance/challenges with entityType and entityId params', async () => {
      const promise = service.getChallengesForEntity('content', 'entity-1');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/challenges' &&
          r.method === 'GET' &&
          r.params.get('entityType') === 'content' &&
          r.params.get('entityId') === 'entity-1',
      );
      req.flush([mockChallenge]);

      const result = await promise;
      expect(result).toHaveLength(1);
      expect(result[0].entityType).toBe('content');
      expect(result[0].entityId).toBe('entity-1');
    });

    it('should return empty array on error', async () => {
      const promise = service.getChallengesForEntity('content', 'entity-missing');

      const req = httpMock.expectOne((r) => r.url === '/api/v1/governance/challenges');
      req.flush('Server error', { status: 500, statusText: 'Internal Server Error' });

      const result = await promise;
      expect(result).toEqual([]);
    });
  });

  describe('getChallenge', () => {
    it('should GET /api/v1/governance/challenges/{id}', async () => {
      const promise = service.getChallenge('ch-1');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/challenges/ch-1' && r.method === 'GET',
      );
      req.flush(mockChallenge);

      const result = await promise;
      expect(result).not.toBeNull();
      expect(result!.id).toBe('ch-1');
      expect(result!.challengerId).toBe('human-1');
    });

    it('should return null on error', async () => {
      const promise = service.getChallenge('ch-missing');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/challenges/ch-missing',
      );
      req.flush('Not found', { status: 404, statusText: 'Not Found' });

      const result = await promise;
      expect(result).toBeNull();
    });
  });

  describe('getAppeals', () => {
    it('should GET /api/v1/governance/challenges/{id}/appeals', async () => {
      const promise = service.getAppeals('ch-1');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/challenges/ch-1/appeals' && r.method === 'GET',
      );
      req.flush([mockAppeal]);

      const result = await promise;
      expect(result).toHaveLength(1);
      expect(result[0].id).toBe('appeal-1');
      expect(result[0].challengeId).toBe('ch-1');
    });

    it('should return empty array on error', async () => {
      const promise = service.getAppeals('ch-missing');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/challenges/ch-missing/appeals',
      );
      req.flush('Server error', { status: 500, statusText: 'Internal Server Error' });

      const result = await promise;
      expect(result).toEqual([]);
    });
  });

  // --- Sprint 7: Sensemaking ---

  const mockStatement: StatementView = {
    id: 'stmt-1',
    entityType: 'content',
    entityId: 'entity-1',
    humanId: 'human-1',
    text: 'We should focus on quality',
    agreeCount: 5,
    disagreeCount: 2,
    passCount: 1,
    groupId: null,
    isBridging: false,
    createdAt: '2026-03-16T00:00:00Z',
  };

  const mockStatementVote: StatementVoteView = {
    id: 'sv-1',
    statementId: 'stmt-1',
    humanId: 'human-2',
    vote: 'agree',
    createdAt: '2026-03-16T00:00:00Z',
  };

  describe('submitStatement', () => {
    it('should POST to /api/v1/governance/sensemaking/statements', async () => {
      const input: CreateStatementInputView = {
        entityType: 'content',
        entityId: 'entity-1',
        humanId: 'human-1',
        text: 'We should focus on quality',
      };

      const promise = service.submitStatement(input);

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/sensemaking/statements' && r.method === 'POST',
      );
      expect(req.request.body).toEqual(input);
      req.flush(mockStatement);

      const result = await promise;
      expect(result.id).toBe('stmt-1');
      expect(result.text).toBe('We should focus on quality');
      expect(result.entityType).toBe('content');
    });
  });

  describe('voteOnStatement', () => {
    it('should POST to /api/v1/governance/sensemaking/statements/{id}/vote', async () => {
      const input: VoteOnStatementInputView = {
        humanId: 'human-2',
        vote: 'agree',
      };

      const promise = service.voteOnStatement('stmt-1', input);

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/sensemaking/statements/stmt-1/vote' &&
          r.method === 'POST',
      );
      expect(req.request.body).toEqual(input);
      req.flush(mockStatementVote);

      const result = await promise;
      expect(result.id).toBe('sv-1');
      expect(result.statementId).toBe('stmt-1');
      expect(result.vote).toBe('agree');
    });
  });

  describe('getStatements', () => {
    it('should GET /api/v1/governance/sensemaking/statements with query params', async () => {
      const promise = service.getStatements('content', 'entity-1');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/sensemaking/statements' &&
          r.method === 'GET' &&
          r.params.get('entityType') === 'content' &&
          r.params.get('entityId') === 'entity-1',
      );
      req.flush([mockStatement]);

      const result = await promise;
      expect(result).toHaveLength(1);
      expect(result[0].id).toBe('stmt-1');
      expect(result[0].entityType).toBe('content');
    });

    it('should return empty array on error', async () => {
      const promise = service.getStatements('content', 'entity-missing');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/sensemaking/statements',
      );
      req.flush('Server error', { status: 500, statusText: 'Internal Server Error' });

      const result = await promise;
      expect(result).toEqual([]);
    });
  });

  describe('getClusters', () => {
    it('should GET /api/v1/governance/sensemaking/clusters with query params', async () => {
      const mockResult: SensemakingResultView = {
        entityType: 'content',
        entityId: 'entity-1',
        clusters: [
          {
            id: 'cluster-1',
            memberCount: 3,
            characteristicStatements: [mockStatement],
            internalAgreement: 0.85,
          },
        ],
        bridgingStatements: [{ ...mockStatement, isBridging: true }],
        totalParticipants: 5,
        totalStatements: 10,
      };

      const promise = service.getClusters('content', 'entity-1');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance/sensemaking/clusters' &&
          r.method === 'GET' &&
          r.params.get('entityType') === 'content' &&
          r.params.get('entityId') === 'entity-1',
      );
      req.flush(mockResult);

      const result = await promise;
      expect(result.clusters).toHaveLength(1);
      expect(result.clusters[0].id).toBe('cluster-1');
      expect(result.clusters[0].internalAgreement).toBe(0.85);
      expect(result.bridgingStatements).toHaveLength(1);
      expect(result.bridgingStatements[0].isBridging).toBe(true);
      expect(result.totalParticipants).toBe(5);
      expect(result.totalStatements).toBe(10);
    });

    it('should return empty result on error', async () => {
      const promise = service.getClusters('content', 'entity-missing');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance/sensemaking/clusters',
      );
      req.flush('Server error', { status: 500, statusText: 'Internal Server Error' });

      const result = await promise;
      expect(result.entityType).toBe('content');
      expect(result.entityId).toBe('entity-missing');
      expect(result.clusters).toEqual([]);
      expect(result.bridgingStatements).toEqual([]);
      expect(result.totalParticipants).toBe(0);
      expect(result.totalStatements).toBe(0);
    });
  });
});
