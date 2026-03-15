import { Injectable, inject } from '@angular/core';

// @coverage: 100.0% (2026-02-24)

import { catchError, map, shareReplay, tap } from 'rxjs/operators';

import { Observable, of, combineLatest, from } from 'rxjs';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';
import type {
  CreateProposalInputView,
  CastVoteInputView,
  PostMessageInputView,
} from '@elohim/storage-client/generated';

// Services
import {
  DataLoaderService,
  GovernanceIndex,
  ChallengeRecord,
  ProposalRecord,
  PrecedentRecord,
  DiscussionRecord,
  GovernanceStateRecord,
} from '@app/elohim/services/data-loader.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';

/**
 * Challenge submission from a user.
 */
export interface ChallengeSubmission {
  entityType: string;
  entityId: string;
  grounds: ChallengeGrounds;
  description: string;
  evidence?: {
    type: 'document-reference' | 'external-reference' | 'testimony';
    reference: string;
    description?: string;
  }[];
}

export type ChallengeGrounds =
  | 'factual-error'
  | 'outdated'
  | 'superseded'
  | 'harmful'
  | 'misleading'
  | 'copyright'
  | 'new-evidence'
  | 'procedural-error'
  | 'other';

/**
 * Proposal submission for governance changes.
 */
export interface ProposalSubmission {
  title: string;
  proposalType: 'sense-check' | 'consent' | 'consensus';
  description: string;
  rationale: string;
  relatedEntityType?: string;
  relatedEntityId?: string;
}

/**
 * Vote on a proposal.
 */
export interface Vote {
  proposalId: string;
  position: 'agree' | 'abstain' | 'disagree' | 'block';
  reasoning?: string;
}

/**
 * Discussion message to post.
 */
export interface DiscussionMessage {
  discussionId: string;
  content: string;
  replyToId?: string;
}

/**
 * GovernanceService - Manages the governance dimension of entities.
 *
 * Responsibilities:
 * - Load governance state for any entity
 * - Display challenges, proposals, precedents, discussions
 * - Submit challenges and proposals (proposals via API, challenges MVP: Sprint 3)
 * - Vote on proposals (via API)
 * - Check SLA deadlines and status
 *
 * Constitutional principles:
 * - Every entity has a governance state
 * - Every decision can be challenged
 * - Every challenge gets a response (with SLA)
 * - Escalation paths are constitutional
 * - Feedback loops are visible
 */
const ACTIVE_CHALLENGE_STATUSES = new Set(['acknowledged', 'under-review']);

@Injectable({ providedIn: 'root' })
export class GovernanceService {
  // Cached governance data
  private challengesCache$: Observable<ChallengeRecord[]> | null = null;
  private proposalsCache$: Observable<ProposalRecord[]> | null = null;
  private precedentsCache$: Observable<PrecedentRecord[]> | null = null;

  private readonly dataLoader = inject(DataLoaderService);
  private readonly sessionUser = inject(SessionHumanService);
  private readonly governanceApi = inject(GovernanceApiService);

  // =========================================================================
  // Governance Index & Overview
  // =========================================================================

  /**
   * Get governance index with counts.
   */
  getGovernanceIndex(): Observable<GovernanceIndex> {
    return this.dataLoader.getGovernanceIndex();
  }

  /**
   * Get governance summary for dashboard display.
   */
  getGovernanceSummary(): Observable<{
    activeChallenges: number;
    votingProposals: number;
    recentPrecedents: number;
    activeDiscussions: number;
  }> {
    return combineLatest([
      this.getChallenges(),
      this.getProposals(),
      this.getPrecedents(),
      this.getDiscussions(),
    ]).pipe(
      map(([challenges, proposals, precedents, discussions]) => ({
        activeChallenges: challenges.filter(c => ACTIVE_CHALLENGE_STATUSES.has(c.status)).length,
        votingProposals: proposals.filter(p => p.status === 'voting').length,
        recentPrecedents: precedents.filter(p => p.status === 'active').length,
        activeDiscussions: discussions.filter(d => d.status === 'active').length,
      }))
    );
  }

  // =========================================================================
  // Entity Governance State
  // =========================================================================

  /**
   * Get governance state for a specific entity.
   * Returns null if no explicit state exists (defaults to 'unreviewed').
   */
  getGovernanceState(
    entityType: string,
    entityId: string
  ): Observable<GovernanceStateRecord | null> {
    return this.dataLoader.getGovernanceState(entityType, entityId);
  }

  /**
   * Get effective governance status for an entity.
   * Returns 'unreviewed' if no state exists.
   */
  getEffectiveStatus(entityType: string, entityId: string): Observable<string> {
    return this.getGovernanceState(entityType, entityId).pipe(
      map(state => state?.status ?? 'unreviewed')
    );
  }

  /**
   * Check if an entity is currently challenged.
   */
  isEntityChallenged(entityType: string, entityId: string): Observable<boolean> {
    return this.getChallengesForEntity(entityType, entityId).pipe(
      map(challenges => challenges.some(c => ACTIVE_CHALLENGE_STATUSES.has(c.status)))
    );
  }

  /**
   * Get active labels/flags on an entity.
   */
  getEntityLabels(
    entityType: string,
    entityId: string
  ): Observable<
    {
      labelType: string;
      severity: string;
    }[]
  > {
    return this.getGovernanceState(entityType, entityId).pipe(map(state => state?.labels ?? []));
  }

  // =========================================================================
  // Challenges
  // =========================================================================

  /**
   * Get all challenges.
   */
  getChallenges(): Observable<ChallengeRecord[]> {
    this.challengesCache$ ??= this.dataLoader.getChallenges().pipe(shareReplay(1));
    return this.challengesCache$;
  }

  /**
   * Get challenges for a specific entity.
   */
  getChallengesForEntity(entityType: string, entityId: string): Observable<ChallengeRecord[]> {
    return this.dataLoader.getChallengesForEntity(entityType, entityId);
  }

  /**
   * Get challenges by status.
   */
  getChallengesByStatus(status: string): Observable<ChallengeRecord[]> {
    return this.getChallenges().pipe(
      map(challenges => challenges.filter(c => c.status === status))
    );
  }

  /**
   * Get challenges filed by current user.
   */
  getMyChallenges(): Observable<ChallengeRecord[]> {
    const agentId = this.sessionUser.getSessionId() ?? 'anonymous';
    return this.getChallenges().pipe(
      map(challenges => challenges.filter(c => c.challenger.agentId === agentId))
    );
  }

  /**
   * Submit a new challenge (MVP: still localStorage, wired in Sprint 3).
   */
  submitChallenge(submission: ChallengeSubmission): Observable<ChallengeRecord> {
    const agentId = this.sessionUser.getSessionId() ?? 'anonymous';
    const session = this.sessionUser.getSession();
    const userName = session?.displayName ?? 'Anonymous';

    const challenge: ChallengeRecord = {
      id: `challenge-local-${Date.now()}`,
      entityType: submission.entityType,
      entityId: submission.entityId,
      challenger: {
        agentId,
        displayName: userName,
        standing: 'community-member',
      },
      grounds: submission.grounds,
      description: submission.description,
      status: 'pending',
      filedAt: new Date().toISOString(),
      slaDeadline: new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString(),
    };

    // Clear cache to pick up new challenge
    this.challengesCache$ = null;

    return of(challenge);
  }

  // =========================================================================
  // Proposals
  // =========================================================================

  /**
   * Get all proposals.
   */
  getProposals(): Observable<ProposalRecord[]> {
    this.proposalsCache$ ??= this.dataLoader.getProposals().pipe(shareReplay(1));
    return this.proposalsCache$;
  }

  /**
   * Get proposals by status.
   */
  getProposalsByStatus(status: string): Observable<ProposalRecord[]> {
    return this.dataLoader.getProposalsByStatus(status);
  }

  /**
   * Get proposals in voting phase.
   */
  getActiveProposals(): Observable<ProposalRecord[]> {
    return this.getProposalsByStatus('voting');
  }

  /**
   * Get proposals I've created.
   */
  getMyProposals(): Observable<ProposalRecord[]> {
    const agentId = this.sessionUser.getSessionId() ?? 'anonymous';
    return this.getProposals().pipe(
      map(proposals => proposals.filter(p => p.proposer.agentId === agentId))
    );
  }

  /**
   * Submit a new proposal via governance API.
   */
  submitProposal(submission: ProposalSubmission): Observable<ProposalRecord> {
    const presenceId = this.sessionUser.getSessionId() ?? 'anonymous';

    const input: CreateProposalInputView = {
      id: `proposal-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      contentId: submission.relatedEntityId ?? '',
      proposerPresenceId: presenceId,
      proposalType: submission.proposalType,
      title: submission.title,
      body: `${submission.description}\n\n**Rationale:** ${submission.rationale}`,
      votingAnonymous: false,
    };

    return from(this.governanceApi.createProposal(input)).pipe(
      map((view): ProposalRecord => ({
        id: view.id,
        title: view.title,
        proposalType: view.proposalType as ProposalRecord['proposalType'],
        description: view.body,
        proposer: {
          agentId: view.proposerPresenceId,
          displayName: view.proposerPresenceId,
        },
        status: view.status as ProposalRecord['status'],
        phase: view.status as ProposalRecord['phase'],
        createdAt: view.createdAt,
      })),
      tap(() => this.clearCache()),
    );
  }

  /**
   * Vote on a proposal via governance API.
   */
  voteOnProposal(vote: Vote): Observable<boolean> {
    const input: CastVoteInputView = {
      humanId: this.sessionUser.getSessionId() ?? 'anonymous',
      position: vote.position,
      reason: vote.reasoning ?? null,
    };

    return from(this.governanceApi.castVote(vote.proposalId, input)).pipe(
      map(() => true),
      catchError(() => of(false)),
    );
  }

  /**
   * Get my vote on a proposal.
   */
  getMyVote(proposalId: string): Observable<Vote | null> {
    const humanId = this.sessionUser.getSessionId();
    if (!humanId) return of(null);

    return from(this.governanceApi.getVotes(proposalId)).pipe(
      map(votes => {
        const mine = votes.find(v => v.humanId === humanId);
        if (!mine) return null;
        return {
          proposalId,
          position: mine.position as Vote['position'],
          reasoning: mine.reason ?? undefined,
        };
      }),
      catchError(() => of(null)),
    );
  }

  // =========================================================================
  // Precedents
  // =========================================================================

  /**
   * Get all precedents.
   */
  getPrecedents(): Observable<PrecedentRecord[]> {
    this.precedentsCache$ ??= this.dataLoader.getPrecedents().pipe(shareReplay(1));
    return this.precedentsCache$;
  }

  /**
   * Get precedents by binding level.
   */
  getPrecedentsByBinding(binding: string): Observable<PrecedentRecord[]> {
    return this.dataLoader.getPrecedentsByBinding(binding);
  }

  /**
   * Get constitutional precedents (highest authority).
   */
  getConstitutionalPrecedents(): Observable<PrecedentRecord[]> {
    return this.getPrecedentsByBinding('constitutional');
  }

  /**
   * Search precedents by keyword.
   */
  searchPrecedents(query: string): Observable<PrecedentRecord[]> {
    const lowerQuery = query.toLowerCase();
    return this.getPrecedents().pipe(
      map(precedents =>
        precedents.filter(
          p =>
            p.title.toLowerCase().includes(lowerQuery) ||
            p.summary.toLowerCase().includes(lowerQuery)
        )
      )
    );
  }

  // =========================================================================
  // Discussions
  // =========================================================================

  /**
   * Get all discussions.
   */
  getDiscussions(): Observable<DiscussionRecord[]> {
    return this.dataLoader.getDiscussions();
  }

  /**
   * Get discussions for an entity.
   */
  getDiscussionsForEntity(entityType: string, entityId: string): Observable<DiscussionRecord[]> {
    return this.dataLoader.getDiscussionsForEntity(entityType, entityId);
  }

  /**
   * Post a message to a discussion via governance API.
   */
  postMessage(message: DiscussionMessage): Observable<boolean> {
    const input: PostMessageInputView = {
      id: `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      authorPresenceId: this.sessionUser.getSessionId() ?? 'anonymous',
      body: message.content,
    };

    return from(this.governanceApi.postMessage(message.discussionId, input)).pipe(
      map(() => true),
      tap(() => this.clearCache()),
      catchError(() => of(false)),
    );
  }

  // =========================================================================
  // SLA & Deadline Tracking
  // =========================================================================

  /**
   * Get challenges approaching SLA deadline.
   */
  getChallengesNearingDeadline(withinDays = 3): Observable<ChallengeRecord[]> {
    const cutoff = new Date(Date.now() + withinDays * 24 * 60 * 60 * 1000);

    return this.getChallenges().pipe(
      map(challenges =>
        challenges.filter(c => {
          if (!c.slaDeadline) return false;
          if (!ACTIVE_CHALLENGE_STATUSES.has(c.status)) return false;

          const deadline = new Date(c.slaDeadline);
          return deadline <= cutoff;
        })
      )
    );
  }

  /**
   * Check if a challenge SLA is breached.
   */
  isSlaBreached(challenge: ChallengeRecord): boolean {
    if (!challenge.slaDeadline) return false;
    if (challenge.status === 'resolved') return false;

    return new Date(challenge.slaDeadline) < new Date();
  }

  // =========================================================================
  // Cache Management
  // =========================================================================

  /**
   * Clear all governance caches.
   */
  clearCache(): void {
    this.challengesCache$ = null;
    this.proposalsCache$ = null;
    this.precedentsCache$ = null;
  }

}
