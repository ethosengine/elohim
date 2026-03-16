import { InjectionToken } from '@angular/core';

import type {
  GovernanceStateView,
  ChallengeView,
  ProposalView,
  PrecedentView,
  DiscussionView,
  ProposalOptionView,
  CreateProposalOptionInputView,
  CastRankedVoteInputView,
  RankedVoteView,
  TallyResult,
  RecordSignalInputView,
  GovernanceSignalView,
} from '@elohim/storage-client/generated';

export interface IGovernance {
  getGovernanceState(entityType: string, entityId: string): Promise<GovernanceStateView | null>;
  queryGovernanceStates(entityType: string): Promise<GovernanceStateView[]>;
  getChallengeById(id: string): Promise<ChallengeView | null>;
  queryChallenges(contentId: string): Promise<ChallengeView[]>;
  getProposalById(id: string): Promise<ProposalView | null>;
  queryProposals(contentId: string, status?: string): Promise<ProposalView[]>;
  getPrecedentById(id: string): Promise<PrecedentView | null>;
  queryPrecedents(contentId: string): Promise<PrecedentView[]>;
  getDiscussionById(id: string): Promise<DiscussionView | null>;
  queryDiscussions(contentId: string): Promise<DiscussionView[]>;

  // Proposal Options
  getProposalOptions(proposalId: string): Promise<ProposalOptionView[]>;
  createProposalOptions(
    proposalId: string,
    options: CreateProposalOptionInputView[]
  ): Promise<ProposalOptionView[]>;

  // Ranked Votes (multi-mechanism)
  castRankedVotes(
    proposalId: string,
    ballot: CastRankedVoteInputView
  ): Promise<RankedVoteView[]>;
  getRankedVotes(proposalId: string): Promise<RankedVoteView[]>;

  // Tally
  getTally(proposalId: string): Promise<TallyResult>;

  // Governance Signals
  recordSignal(signal: RecordSignalInputView): Promise<GovernanceSignalView>;
  getSignals(entityType: string, entityId: string): Promise<GovernanceSignalView[]>;
}

export const GOVERNANCE = new InjectionToken<IGovernance>('Governance');
