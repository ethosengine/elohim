/**
 * AttestationApiService — Thin HTTP client for the unified attestation endpoints.
 *
 * Calls doorway `/api/v1/attestations/unified/*` endpoints.
 * Replaces per-type attestation entry-type calls that were removed in the
 * attestation-consolidation sprint. Callers that previously issued
 * humanity-witness, renewal-attestation, key-stewardship, stewardship-grant,
 * recovery-request, recovery-vote, identity-challenge, challenge-support,
 * key-revocation, revocation-vote, identity-freeze, content-attestation, or
 * health-attestation entries should migrate to this service.
 *
 * Routes (from E.5 implementation):
 *   POST   /api/v1/attestations/unified         — issue
 *   GET    /api/v1/attestations/unified/{id}    — get by id
 *   GET    /api/v1/attestations/unified         — list (subjectCid + kind filters)
 *   POST   /api/v1/attestations/unified/{id}/revoke — revoke
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { catchError, firstValueFrom, of } from 'rxjs';

import type { AttestationView } from '@app/generated/attestation-view';

export interface IssueAttestationInput {
  /** Discriminator, e.g. 'attestation:humanness', 'attestation:content-quality'. */
  attestationKind: string;
  /** CID of the entity being attested. */
  subjectCid: string;
  /** Kind of the subject: 'agent' | 'content' | 'device' | 'hub' | 'computation' | 'governance-action'. */
  subjectKind: string;
  /** CID of the parent governance-action, required when this is a vote. */
  parentGovernanceActionCid?: string | null;
  /** Vote value — only set when this attestation is a vote. */
  voteValue?: 'approve' | 'reject' | 'abstain' | null;
  /** Optional vote weight as a decimal string. */
  voteWeight?: string | null;
  /** Proof class: 'witness' | 'self-attest' | 'audit-signature' | 'computational'. */
  proofClass: string;
  /** Serialised proof evidence JSON. */
  proofEvidenceJson?: string;
  /** Serialised full evidence JSON. */
  evidenceJson?: string;
  /** ISO 8601 expiry timestamp, if applicable. */
  expiresAt?: string | null;
  /** CID of an existing attestation this supersedes. */
  supersedesCid?: string | null;
  /** Manifest reference (e.g. 'mishpat', 'lamad'). */
  manifestRef?: string;
  /** Human-readable title. */
  title?: string;
  /** Optional description. */
  description?: string | null;
}

export interface RevokeAttestationInput {
  reason: string;
}

@Injectable({ providedIn: 'root' })
export class AttestationApiService {
  private readonly http = inject(HttpClient);

  /**
   * Issue a new attestation of any kind.
   */
  async issue(input: IssueAttestationInput): Promise<AttestationView> {
    return firstValueFrom(
      this.http.post<AttestationView>('/api/v1/attestations/unified', input)
    );
  }

  /**
   * Revoke an existing attestation.
   */
  async revoke(id: string, reason: string): Promise<AttestationView> {
    return firstValueFrom(
      this.http.post<AttestationView>(
        `/api/v1/attestations/unified/${encodeURIComponent(id)}/revoke`,
        { reason } satisfies RevokeAttestationInput
      )
    );
  }

  /**
   * Get a single attestation by CID.
   * Returns null if not found (404).
   */
  async getById(id: string): Promise<AttestationView | null> {
    return firstValueFrom(
      this.http
        .get<AttestationView>(`/api/v1/attestations/unified/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  /**
   * List attestations for a subject, optionally filtered by kind.
   *
   * @param subjectCid - CID of the subject entity.
   * @param kind       - Optional kind discriminator, e.g. 'attestation:humanness'.
   */
  async listBySubject(subjectCid: string, kind?: string): Promise<AttestationView[]> {
    const params: Record<string, string> = {
      subjectCid: encodeURIComponent(subjectCid),
    };
    if (kind) {
      params['kind'] = encodeURIComponent(kind);
    }
    return firstValueFrom(
      this.http
        .get<AttestationView[]>('/api/v1/attestations/unified', { params })
        .pipe(catchError(() => of([])))
    );
  }
}
