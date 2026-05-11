/**
 * GovernanceActionApiService — Thin HTTP client for the unified governance-action endpoints.
 *
 * Calls doorway `/api/v1/governance-actions/*` endpoints.
 * Replaces per-type governance entry-type calls (Proposal, ProposalVote,
 * StatementVote, Challenge, GateDecisionAttestation) that were removed in
 * the attestation-consolidation sprint.
 *
 * Routes (from E.5 implementation):
 *   POST  /api/v1/governance-actions              — propose
 *   GET   /api/v1/governance-actions/{id}         — get (parent + children)
 *   GET   /api/v1/governance-actions/{id}/tally   — get tally
 *   POST  /api/v1/governance-actions/{id}/vote    — vote
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { catchError, firstValueFrom, of } from 'rxjs';

import type { GovernanceActionView } from '@app/generated/governance-action-view';
import type { GovernanceActionTallyView } from '@app/generated/governance-action-tally-view';
import type { AttestationView } from '@app/generated/attestation-view';

export interface ProposeGovernanceActionInput {
  /** Discriminator, e.g. 'governance-action:key-revocation', 'governance-action:content-succession'. */
  governanceKind: string;
  /** CID of the entity being acted upon. */
  subjectCid: string;
  /** Serialised threshold JSON, e.g. '{"m":3}' or '{"percentage":0.51}'. */
  thresholdJson: string;
  /** Ballot format: 'approve-reject' | 'ranked-choice' | 'weighted'. */
  ballotFormat: string;
  /** ISO 8601 close time for the voting window. */
  closesAt: string;
  /** Serialised eligibility predicate JSON, if constrained. */
  eligibilityPredicateJson?: string | null;
  /** Serialised additional parameters JSON, if any. */
  parametersJson?: string | null;
  /** Human-readable title. */
  title?: string;
  /** Optional description. */
  description?: string | null;
}

export interface VoteInput {
  /** Vote value. */
  voteValue: 'approve' | 'reject' | 'abstain';
  /** Optional vote weight as a decimal string. */
  voteWeight?: string | null;
  /** Serialised proof evidence JSON. */
  proofEvidenceJson?: string;
}

export interface GovernanceActionWithChildrenView {
  action: GovernanceActionView;
  votes: AttestationView[];
}

@Injectable({ providedIn: 'root' })
export class GovernanceActionApiService {
  private readonly http = inject(HttpClient);

  /**
   * Propose a new governance action.
   */
  async propose(input: ProposeGovernanceActionInput): Promise<GovernanceActionView> {
    return firstValueFrom(
      this.http.post<GovernanceActionView>('/api/v1/governance-actions', input)
    );
  }

  /**
   * Get a governance action by CID, including its child vote attestations.
   * Returns null if not found (404).
   */
  async getById(id: string): Promise<GovernanceActionWithChildrenView | null> {
    return firstValueFrom(
      this.http
        .get<GovernanceActionWithChildrenView>(
          `/api/v1/governance-actions/${encodeURIComponent(id)}`
        )
        .pipe(catchError(() => of(null)))
    );
  }

  /**
   * Get the live tally for a governance action.
   * Returns null if the action is not found.
   */
  async getTally(id: string): Promise<GovernanceActionTallyView | null> {
    return firstValueFrom(
      this.http
        .get<GovernanceActionTallyView>(
          `/api/v1/governance-actions/${encodeURIComponent(id)}/tally`
        )
        .pipe(catchError(() => of(null)))
    );
  }

  /**
   * Cast a vote on an open governance action.
   * The resulting vote is an AttestationView with attestationKind = 'attestation:vote'.
   */
  async vote(id: string, input: VoteInput): Promise<AttestationView> {
    return firstValueFrom(
      this.http.post<AttestationView>(
        `/api/v1/governance-actions/${encodeURIComponent(id)}/vote`,
        input
      )
    );
  }
}
