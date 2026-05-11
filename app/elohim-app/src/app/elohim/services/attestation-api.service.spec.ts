/**
 * AttestationApiService Tests
 *
 * Verifies that the unified attestation API methods call the correct
 * HTTP endpoints with proper parameters.
 */

import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { describe, expect, it, beforeEach, afterEach } from 'vitest';

import type { AttestationView } from '@app/generated/attestation-view';

import { AttestationApiService, type IssueAttestationInput } from './attestation-api.service';

const mockAttestation: AttestationView = {
  id: 'bafkrei-test-attestation-cid',
  dhtAnchorHash: 'abc123deadbeef',
  attestationKind: 'attestation:humanness',
  subjectCid: 'bafkrei-subject-cid',
  subjectKind: 'agent',
  issuerCid: 'bafkrei-issuer-cid',
  parentGovernanceActionCid: null,
  voteValue: null,
  voteWeight: null,
  proofClass: 'witness',
  proofEvidenceJson: '{}',
  evidenceJson: '{}',
  expiresAt: null,
  supersedesCid: null,
  revocationReason: null,
  revokedAt: null,
  createdAt: '2026-05-11T00:00:00Z',
  manifestRef: 'imagodei',
  title: 'Humanness attestation',
  description: null,
};

describe('AttestationApiService', () => {
  let service: AttestationApiService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [AttestationApiService, provideHttpClient(), provideHttpClientTesting()],
    });

    service = TestBed.inject(AttestationApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('issue', () => {
    it('should POST to /api/v1/attestations/unified', async () => {
      const input: IssueAttestationInput = {
        attestationKind: 'attestation:humanness',
        subjectCid: 'bafkrei-subject-cid',
        subjectKind: 'agent',
        proofClass: 'witness',
      };

      const promise = service.issue(input);

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/attestations/unified' && r.method === 'POST',
      );
      expect(req.request.body).toEqual(input);
      req.flush(mockAttestation);

      const result = await promise;
      expect(result.id).toBe('bafkrei-test-attestation-cid');
      expect(result.attestationKind).toBe('attestation:humanness');
    });
  });

  describe('revoke', () => {
    it('should POST to /api/v1/attestations/unified/{id}/revoke', async () => {
      const promise = service.revoke('bafkrei-test-attestation-cid', 'superseded');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/attestations/unified/bafkrei-test-attestation-cid/revoke' &&
          r.method === 'POST',
      );
      expect(req.request.body).toEqual({ reason: 'superseded' });
      req.flush({ ...mockAttestation, revocationReason: 'superseded', revokedAt: '2026-05-11T01:00:00Z' });

      const result = await promise;
      expect(result.revocationReason).toBe('superseded');
    });
  });

  describe('getById', () => {
    it('should GET /api/v1/attestations/unified/{id}', async () => {
      const promise = service.getById('bafkrei-test-attestation-cid');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/attestations/unified/bafkrei-test-attestation-cid' &&
          r.method === 'GET',
      );
      req.flush(mockAttestation);

      const result = await promise;
      expect(result).not.toBeNull();
      expect(result!.id).toBe('bafkrei-test-attestation-cid');
    });

    it('should return null on 404', async () => {
      const promise = service.getById('bafkrei-missing');

      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/attestations/unified/bafkrei-missing',
      );
      req.flush('Not found', { status: 404, statusText: 'Not Found' });

      const result = await promise;
      expect(result).toBeNull();
    });
  });

  describe('listBySubject', () => {
    it('should GET /api/v1/attestations/unified with subjectCid param', async () => {
      const promise = service.listBySubject('bafkrei-subject-cid');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/attestations/unified' &&
          r.method === 'GET' &&
          r.params.get('subjectCid') === 'bafkrei-subject-cid',
      );
      req.flush([mockAttestation]);

      const result = await promise;
      expect(result).toHaveLength(1);
      expect(result[0].subjectCid).toBe('bafkrei-subject-cid');
    });

    it('should include kind param when provided', async () => {
      const promise = service.listBySubject('bafkrei-subject-cid', 'attestation:humanness');

      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/attestations/unified' &&
          r.params.get('kind') === 'attestation%3Ahumanness',
      );
      req.flush([mockAttestation]);

      const result = await promise;
      expect(result).toHaveLength(1);
    });

    it('should return empty array on error', async () => {
      const promise = service.listBySubject('bafkrei-bad-cid');

      const req = httpMock.expectOne((r) => r.url === '/api/v1/attestations/unified');
      req.flush('Server error', { status: 500, statusText: 'Internal Server Error' });

      const result = await promise;
      expect(result).toEqual([]);
    });
  });
});
