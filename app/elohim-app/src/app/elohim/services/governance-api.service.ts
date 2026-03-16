/**
 * GovernanceApiService — Thin HTTP client for governance endpoints.
 *
 * Calls doorway `/api/v1/governance/*` endpoints, implementing IGovernance.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';

import { firstValueFrom, of } from 'rxjs';

import type { IGovernance } from '../interfaces/governance.interface';
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
  ProposalOptionView,
  CreateProposalOptionInputView,
  CastRankedVoteInputView,
  RankedVoteView,
  TallyResult,
  RecordSignalInputView,
  GovernanceSignalView,
  SignalAggregateView,
  FileChallengeInputView,
  RespondToChallengeInputView,
  FileAppealInputView,
  AppealView,
} from '@elohim/storage-client/generated';

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
    return firstValueFrom(this.http.post<ProposalView>('/api/v1/governance/proposals', input));
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
        .get<VoteView[]>(`/api/v1/governance/proposals/${encodeURIComponent(proposalId)}/votes`)
        .pipe(catchError(() => of([])))
    );
  }

  async createDiscussion(input: CreateDiscussionInputView): Promise<DiscussionView> {
    return firstValueFrom(this.http.post<DiscussionView>('/api/v1/governance/discussions', input));
  }

  async postMessage(discussionId: string, input: PostMessageInputView): Promise<DiscussionView> {
    return firstValueFrom(
      this.http.post<DiscussionView>(
        `/api/v1/governance/discussions/${encodeURIComponent(discussionId)}/messages`,
        input
      )
    );
  }

  // --- Proposal Options ---

  async getProposalOptions(proposalId: string): Promise<ProposalOptionView[]> {
    return firstValueFrom(
      this.http
        .get<ProposalOptionView[]>(
          `/api/v1/governance/proposals/${encodeURIComponent(proposalId)}/options`
        )
        .pipe(catchError(() => of([])))
    );
  }

  async createProposalOptions(
    proposalId: string,
    options: CreateProposalOptionInputView[]
  ): Promise<ProposalOptionView[]> {
    return firstValueFrom(
      this.http.post<ProposalOptionView[]>(
        `/api/v1/governance/proposals/${encodeURIComponent(proposalId)}/options`,
        options
      )
    );
  }

  // --- Ranked Votes (multi-mechanism) ---

  async castRankedVotes(
    proposalId: string,
    ballot: CastRankedVoteInputView
  ): Promise<RankedVoteView[]> {
    return firstValueFrom(
      this.http.post<RankedVoteView[]>(
        `/api/v1/governance/proposals/${encodeURIComponent(proposalId)}/ranked-votes`,
        ballot
      )
    );
  }

  async getRankedVotes(proposalId: string): Promise<RankedVoteView[]> {
    return firstValueFrom(
      this.http
        .get<RankedVoteView[]>(
          `/api/v1/governance/proposals/${encodeURIComponent(proposalId)}/ranked-votes`
        )
        .pipe(catchError(() => of([])))
    );
  }

  // --- Tally ---

  async getTally(proposalId: string): Promise<TallyResult> {
    return firstValueFrom(
      this.http.get<TallyResult>(
        `/api/v1/governance/proposals/${encodeURIComponent(proposalId)}/tally`
      )
    );
  }

  // --- Governance Signals ---

  async recordSignal(signal: RecordSignalInputView): Promise<GovernanceSignalView> {
    return firstValueFrom(
      this.http.post<GovernanceSignalView>('/api/v1/governance/signals', signal)
    );
  }

  async getSignalAggregate(
    entityType: string,
    entityId: string
  ): Promise<SignalAggregateView> {
    const emptyAggregate: SignalAggregateView = {
      entityType,
      entityId,
      totalSignals: BigInt(0),
      byType: {},
      byValue: {},
      uniqueParticipants: BigInt(0),
      consensusStrength: 0,
    };
    return firstValueFrom(
      this.http
        .get<SignalAggregateView>('/api/v1/governance/signals/aggregate', {
          params: {
            entityType: encodeURIComponent(entityType),
            entityId: encodeURIComponent(entityId),
          },
        })
        .pipe(catchError(() => of(emptyAggregate)))
    );
  }

  async getSignals(entityType: string, entityId: string): Promise<GovernanceSignalView[]> {
    return firstValueFrom(
      this.http
        .get<GovernanceSignalView[]>('/api/v1/governance/signals', {
          params: {
            entityType: encodeURIComponent(entityType),
            entityId: encodeURIComponent(entityId),
          },
        })
        .pipe(catchError(() => of([])))
    );
  }

  // --- Challenges & Appeals ---

  async fileChallenge(input: FileChallengeInputView): Promise<ChallengeView> {
    return firstValueFrom(
      this.http.post<ChallengeView>('/api/v1/governance/challenges', input)
    );
  }

  async respondToChallenge(
    id: string,
    input: RespondToChallengeInputView
  ): Promise<ChallengeView> {
    return firstValueFrom(
      this.http.post<ChallengeView>(
        `/api/v1/governance/challenges/${encodeURIComponent(id)}/respond`,
        input
      )
    );
  }

  async fileAppeal(challengeId: string, input: FileAppealInputView): Promise<AppealView> {
    return firstValueFrom(
      this.http.post<AppealView>(
        `/api/v1/governance/challenges/${encodeURIComponent(challengeId)}/appeal`,
        input
      )
    );
  }

  async getChallengesForEntity(
    entityType: string,
    entityId: string
  ): Promise<ChallengeView[]> {
    return firstValueFrom(
      this.http
        .get<ChallengeView[]>('/api/v1/governance/challenges', {
          params: {
            entityType: encodeURIComponent(entityType),
            entityId: encodeURIComponent(entityId),
          },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async getChallenge(id: string): Promise<ChallengeView | null> {
    return firstValueFrom(
      this.http
        .get<ChallengeView>(`/api/v1/governance/challenges/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async getAppeals(challengeId: string): Promise<AppealView[]> {
    return firstValueFrom(
      this.http
        .get<AppealView[]>(
          `/api/v1/governance/challenges/${encodeURIComponent(challengeId)}/appeals`
        )
        .pipe(catchError(() => of([])))
    );
  }
}
