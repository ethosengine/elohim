/**
 * GovernanceActionApiService Tests
 *
 * Verifies that unified governance-action API methods call the correct
 * HTTP endpoints with proper parameters.
 */

import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { describe, expect, it, beforeEach, afterEach } from 'vitest';

import type { GovernanceActionView } from '@app/generated/governance-action-view';
import type { GovernanceActionTallyView } from '@app/generated/governance-action-tally-view';
import type { AttestationView } from '@app/generated/attestation-view';

import {
  GovernanceActionApiService,
  type ProposeGovernanceActionInput,
  type VoteInput,
} from './governance-action-api.service';

const mockAction: GovernanceActionView = {
  id: 'bafkrei-test-action-cid',
  dhtAnchorHash: 'deadbeef123',
  governanceKind: 'governance-action:key-revocation',
  subjectCid: 'bafkrei-subject-cid',
  proposerCid: 'bafkrei-proposer-cid',
  thresholdJson: '{"m":3}',
  eligibilityPredicateJson: null,
  ballotFormat: 'approve-reject',
  closesAt: '2026-06-01T00:00:00Z',
  parametersJson: null,
  title: 'Revoke key for compromised device',
  description: null,
  createdAt: '2026-05-11T00:00:00Z',
};

const mockTally: GovernanceActionTallyView = {
  parentCid: 'bafkrei-test-action-cid',
  governanceKind: 'governance-action:key-revocation',
  subjectCid: 'bafkrei-subject-cid',
  thresholdM: 3,
  thresholdN: null,
  thresholdPercentage: null,
  closesAt: '2026-06-01T00:00:00Z',
  currentApproveCount: 2,
  currentRejectCount: 0,
  currentAbstainCount: 0,
  computedStatus: 'pending',
  lastChildAt: '2026-05-11T01:00:00Z',
  rebuiltAt: '2026-05-11T01:00:00Z',
};

const mockVoteAttestation: AttestationView = {
  id: 'bafkrei-vote-cid',
  dhtAnchorHash: 'cafe1234',
  attestationKind: 'attestation:vote',
  subjectCid: 'bafkrei-test-action-cid',
  subjectKind: 'governance-action',
  issuerCid: 'bafkrei-voter-cid',
  parentGovernanceActionCid: 'bafkrei-test-action-cid',
  voteValue: 'approve',
  voteWeight: null,
  proofClass: 'self-attest',
  proofEvidenceJson: '{}',
  evidenceJson: '{}',
  expiresAt: null,
  supersedesCid: null,
  revocationReason: null,
  revokedAt: null,
  createdAt: '2026-05-11T01:30:00Z',
  manifestRef: 'mishpat',
  title: 'Vote: approve',
  description: null,
};

describe('GovernanceActionApiService', () => {
  let service: GovernanceActionApiService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [GovernanceActionApiService, provideHttpClient(), provideHttpClientTesting()],
    });

    service = TestBed.inject(GovernanceActionApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('propose', () => {
    it('should POST to /api/v1/governance-actions', async () => {
      const input: ProposeGovernanceActionInput = {
        governanceKind: 'governance-action:key-revocation',
        subjectCid: 'bafkrei-subject-cid',
        thresholdJson: '{"m":3}',
        ballotFormat: 'approve-reject',
        closesAt: '2026-06-01T00:00:00Z',
        title: 'Revoke key for compromised device',
      };

      const promise = service.propose(input);

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance-actions' && r.method === 'POST',
      );
      expect(req.request.body).toEqual(input);
      req.flush(mockAction);

      const result = await promise;
      expect(result.id).toBe('bafkrei-test-action-cid');
      expect(result.governanceKind).toBe('governance-action:key-revocation');
    });
  });

  describe('getById', () => {
    it('should GET /api/v1/governance-actions/{id}', async () => {
      const withChildren = { action: mockAction, votes: [mockVoteAttestation] };
      const promise = service.getById('bafkrei-test-action-cid');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance-actions/bafkrei-test-action-cid' && r.method === 'GET',
      );
      req.flush(withChildren);

      const result = await promise;
      expect(result).not.toBeNull();
      expect(result!.action.id).toBe('bafkrei-test-action-cid');
      expect(result!.votes).toHaveLength(1);
    });

    it('should return null on 404', async () => {
      const promise = service.getById('bafkrei-missing');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance-actions/bafkrei-missing',
      );
      req.flush('Not found', { status: 404, statusText: 'Not Found' });

      const result = await promise;
      expect(result).toBeNull();
    });
  });

  describe('getTally', () => {
    it('should GET /api/v1/governance-actions/{id}/tally', async () => {
      const promise = service.getTally('bafkrei-test-action-cid');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance-actions/bafkrei-test-action-cid/tally' &&
          r.method === 'GET',
      );
      req.flush(mockTally);

      const result = await promise;
      expect(result).not.toBeNull();
      expect(result!.parentCid).toBe('bafkrei-test-action-cid');
      expect(result!.currentApproveCount).toBe(2);
      expect(result!.computedStatus).toBe('pending');
    });

    it('should return null on error', async () => {
      const promise = service.getTally('bafkrei-missing');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/governance-actions/bafkrei-missing/tally',
      );
      req.flush('Not found', { status: 404, statusText: 'Not Found' });

      const result = await promise;
      expect(result).toBeNull();
    });
  });

  describe('vote', () => {
    it('should POST to /api/v1/governance-actions/{id}/vote', async () => {
      const input: VoteInput = { voteValue: 'approve' };

      const promise = service.vote('bafkrei-test-action-cid', input);

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/governance-actions/bafkrei-test-action-cid/vote' &&
          r.method === 'POST',
      );
      expect(req.request.body).toEqual(input);
      req.flush(mockVoteAttestation);

      const result = await promise;
      expect(result.attestationKind).toBe('attestation:vote');
      expect(result.voteValue).toBe('approve');
      expect(result.parentGovernanceActionCid).toBe('bafkrei-test-action-cid');
    });
  });
});
