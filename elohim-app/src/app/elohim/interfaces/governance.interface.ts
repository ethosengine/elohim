import { InjectionToken } from '@angular/core';

import type {
  GovernanceStateView,
  ChallengeView,
  ProposalView,
  PrecedentView,
  DiscussionView,
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
}

export const GOVERNANCE = new InjectionToken<IGovernance>('Governance');
