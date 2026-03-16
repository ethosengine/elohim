/**
 * GovernanceApiService Tests
 *
 * Tests that Sprint 4 governance API methods call the correct
 * HTTP endpoints with proper parameters.
 */

import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { describe, expect, it } from 'vitest';

import type {
  CastRankedVoteInputView,
  GovernanceSignalView,
  ProposalOptionView,
  RankedVoteView,
  RecordSignalInputView,
  TallyResult,
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
});
