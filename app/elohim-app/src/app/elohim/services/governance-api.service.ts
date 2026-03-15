/**
 * GovernanceApiService — Thin HTTP client for governance endpoints.
 *
 * Calls doorway `/api/v1/governance/*` endpoints, implementing IGovernance.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';
import { firstValueFrom, of } from 'rxjs';

import type {
  GovernanceStateView,
  ChallengeView,
  ProposalView,
  PrecedentView,
  DiscussionView,
  VoteView,
  CreateProposalInputView,
  CastVoteInputView,
  CreateDiscussionInputView,
  PostMessageInputView,
} from '@elohim/storage-client/generated';

import type { IGovernance } from '../interfaces/governance.interface';

@Injectable({ providedIn: 'root' })
export class GovernanceApiService implements IGovernance {
  private readonly http = inject(HttpClient);

  async getGovernanceState(
    entityType: string,
    entityId: string
  ): Promise<GovernanceStateView | null> {
    return firstValueFrom(
      this.http
        .get<GovernanceStateView>('/api/v1/governance/state', {
          params: {
            entityType: encodeURIComponent(entityType),
            entityId: encodeURIComponent(entityId),
          },
        })
        .pipe(catchError(() => of(null)))
    );
  }

  async queryGovernanceStates(entityType: string): Promise<GovernanceStateView[]> {
    return firstValueFrom(
      this.http
        .get<GovernanceStateView[]>('/api/v1/governance/states', {
          params: { entityType: encodeURIComponent(entityType) },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async getChallengeById(id: string): Promise<ChallengeView | null> {
    return firstValueFrom(
      this.http
        .get<ChallengeView>(`/api/v1/governance/challenges/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async queryChallenges(contentId: string): Promise<ChallengeView[]> {
    return firstValueFrom(
      this.http
        .get<ChallengeView[]>('/api/v1/governance/challenges', {
          params: { contentId: encodeURIComponent(contentId) },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async getProposalById(id: string): Promise<ProposalView | null> {
    return firstValueFrom(
      this.http
        .get<ProposalView>(`/api/v1/governance/proposals/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async queryProposals(contentId: string, status?: string): Promise<ProposalView[]> {
    const params: Record<string, string> = { contentId: encodeURIComponent(contentId) };
    if (status) {
      params['status'] = encodeURIComponent(status);
    }
    return firstValueFrom(
      this.http
        .get<ProposalView[]>('/api/v1/governance/proposals', { params })
        .pipe(catchError(() => of([])))
    );
  }

  async getPrecedentById(id: string): Promise<PrecedentView | null> {
    return firstValueFrom(
      this.http
        .get<PrecedentView>(`/api/v1/governance/precedents/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async queryPrecedents(contentId: string): Promise<PrecedentView[]> {
    return firstValueFrom(
      this.http
        .get<PrecedentView[]>('/api/v1/governance/precedents', {
          params: { contentId: encodeURIComponent(contentId) },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async getDiscussionById(id: string): Promise<DiscussionView | null> {
    return firstValueFrom(
      this.http
        .get<DiscussionView>(`/api/v1/governance/discussions/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async queryDiscussions(contentId: string): Promise<DiscussionView[]> {
    return firstValueFrom(
      this.http
        .get<DiscussionView[]>('/api/v1/governance/discussions', {
          params: { contentId: encodeURIComponent(contentId) },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async createProposal(input: CreateProposalInputView): Promise<ProposalView> {
    return firstValueFrom(
      this.http.post<ProposalView>('/api/v1/governance/proposals', input)
    );
  }

  async castVote(proposalId: string, input: CastVoteInputView): Promise<VoteView> {
    return firstValueFrom(
      this.http.post<VoteView>(
        `/api/v1/governance/proposals/${encodeURIComponent(proposalId)}/votes`,
        input
      )
    );
  }

  async getVotes(proposalId: string): Promise<VoteView[]> {
    return firstValueFrom(
      this.http
        .get<VoteView[]>(
          `/api/v1/governance/proposals/${encodeURIComponent(proposalId)}/votes`
        )
        .pipe(catchError(() => of([])))
    );
  }

  async createDiscussion(input: CreateDiscussionInputView): Promise<DiscussionView> {
    return firstValueFrom(
      this.http.post<DiscussionView>('/api/v1/governance/discussions', input)
    );
  }

  async postMessage(
    discussionId: string,
    input: PostMessageInputView
  ): Promise<DiscussionView> {
    return firstValueFrom(
      this.http.post<DiscussionView>(
        `/api/v1/governance/discussions/${encodeURIComponent(discussionId)}/messages`,
        input
      )
    );
  }
}
