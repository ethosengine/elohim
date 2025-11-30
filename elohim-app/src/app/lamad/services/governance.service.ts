import { Injectable } from '@angular/core';
import { Observable, of, BehaviorSubject, combineLatest } from 'rxjs';
import { map, tap, catchError, shareReplay } from 'rxjs/operators';
import {
  DataLoaderService,
  GovernanceIndex,
  ChallengeRecord,
  ProposalRecord,
  PrecedentRecord,
  DiscussionRecord,
  GovernanceStateRecord
} from './data-loader.service';
import { SessionHumanService } from './session-human.service';

/**
 * Challenge submission from a user.
 */
export interface ChallengeSubmission {
  entityType: string;
  entityId: string;
  grounds: ChallengeGrounds;
  description: string;
  evidence?: Array<{
    type: 'document-reference' | 'external-reference' | 'testimony';
    reference: string;
    description?: string;
  }>;
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
 * - Submit challenges and proposals (MVP: localStorage simulation)
 * - Vote on proposals (MVP: localStorage simulation)
 * - Check SLA deadlines and status
 *
 * Constitutional principles:
 * - Every entity has a governance state
 * - Every decision can be challenged
 * - Every challenge gets a response (with SLA)
 * - Escalation paths are constitutional
 * - Feedback loops are visible
 */
@Injectable({ providedIn: 'root' })
export class GovernanceService {
  private readonly STORAGE_PREFIX = 'lamad-governance-';

  // Cached governance data
  private challengesCache$: Observable<ChallengeRecord[]> | null = null;
  private proposalsCache$: Observable<ProposalRecord[]> | null = null;
  private precedentsCache$: Observable<PrecedentRecord[]> | null = null;

  constructor(
    private dataLoader: DataLoaderService,
    private sessionUser: SessionHumanService
  ) {}

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
      this.getDiscussions()
    ]).pipe(
      map(([challenges, proposals, precedents, discussions]) => ({
        activeChallenges: challenges.filter(c =>
          ['acknowledged', 'under-review'].includes(c.status)
        ).length,
        votingProposals: proposals.filter(p => p.status === 'voting').length,
        recentPrecedents: precedents.filter(p => p.status === 'active').length,
        activeDiscussions: discussions.filter(d => d.status === 'active').length
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
  getGovernanceState(entityType: string, entityId: string): Observable<GovernanceStateRecord | null> {
    return this.dataLoader.getGovernanceState(entityType, entityId);
  }

  /**
   * Get effective governance status for an entity.
   * Returns 'unreviewed' if no state exists.
   */
  getEffectiveStatus(entityType: string, entityId: string): Observable<string> {
    return this.getGovernanceState(entityType, entityId).pipe(
      map(state => state?.status || 'unreviewed')
    );
  }

  /**
   * Check if an entity is currently challenged.
   */
  isEntityChallenged(entityType: string, entityId: string): Observable<boolean> {
    return this.getChallengesForEntity(entityType, entityId).pipe(
      map(challenges => challenges.some(c =>
        ['acknowledged', 'under-review'].includes(c.status)
      ))
    );
  }

  /**
   * Get active labels/flags on an entity.
   */
  getEntityLabels(entityType: string, entityId: string): Observable<Array<{
    labelType: string;
    severity: string;
  }>> {
    return this.getGovernanceState(entityType, entityId).pipe(
      map(state => state?.labels || [])
    );
  }

  // =========================================================================
  // Challenges
  // =========================================================================

  /**
   * Get all challenges.
   */
  getChallenges(): Observable<ChallengeRecord[]> {
    if (!this.challengesCache$) {
      this.challengesCache$ = this.dataLoader.getChallenges().pipe(
        shareReplay(1)
      );
    }
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
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    return this.getChallenges().pipe(
      map(challenges => challenges.filter(c => c.challenger.agentId === agentId))
    );
  }

  /**
   * Submit a new challenge (MVP: saves to localStorage).
   */
  submitChallenge(submission: ChallengeSubmission): Observable<ChallengeRecord> {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    const session = this.sessionUser.getSession();
    const userName = session?.displayName || 'Anonymous';

    const challenge: ChallengeRecord = {
      id: `challenge-local-${Date.now()}`,
      entityType: submission.entityType,
      entityId: submission.entityId,
      challenger: {
        agentId,
        displayName: userName,
        standing: 'community-member'
      },
      grounds: submission.grounds,
      description: submission.description,
      status: 'pending',
      filedAt: new Date().toISOString(),
      slaDeadline: new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString()
    };

    // Save to localStorage
    this.saveLocalChallenge(challenge);

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
    if (!this.proposalsCache$) {
      this.proposalsCache$ = this.dataLoader.getProposals().pipe(
        shareReplay(1)
      );
    }
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
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    return this.getProposals().pipe(
      map(proposals => proposals.filter(p => p.proposer.agentId === agentId))
    );
  }

  /**
   * Submit a new proposal (MVP: saves to localStorage).
   */
  submitProposal(submission: ProposalSubmission): Observable<ProposalRecord> {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    const session = this.sessionUser.getSession();
    const userName = session?.displayName || 'Anonymous';

    const proposal: ProposalRecord = {
      id: `proposal-local-${Date.now()}`,
      title: submission.title,
      proposalType: submission.proposalType,
      description: submission.description,
      proposer: {
        agentId,
        displayName: userName
      },
      status: 'discussion',
      phase: 'discussion',
      createdAt: new Date().toISOString()
    };

    // Save to localStorage
    this.saveLocalProposal(proposal);

    // Clear cache
    this.proposalsCache$ = null;

    return of(proposal);
  }

  /**
   * Vote on a proposal (MVP: saves to localStorage).
   */
  voteOnProposal(vote: Vote): Observable<boolean> {
    // In MVP, we just record the vote locally
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    const key = `${this.STORAGE_PREFIX}vote-${agentId}-${vote.proposalId}`;

    try {
      localStorage.setItem(key, JSON.stringify({
        ...vote,
        votedAt: new Date().toISOString(),
        agentId
      }));
      return of(true);
    } catch {
      return of(false);
    }
  }

  /**
   * Get my vote on a proposal.
   */
  getMyVote(proposalId: string): Observable<Vote | null> {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    const key = `${this.STORAGE_PREFIX}vote-${agentId}-${proposalId}`;
    const data = localStorage.getItem(key);
    return of(data ? JSON.parse(data) : null);
  }

  // =========================================================================
  // Precedents
  // =========================================================================

  /**
   * Get all precedents.
   */
  getPrecedents(): Observable<PrecedentRecord[]> {
    if (!this.precedentsCache$) {
      this.precedentsCache$ = this.dataLoader.getPrecedents().pipe(
        shareReplay(1)
      );
    }
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
      map(precedents => precedents.filter(p =>
        p.title.toLowerCase().includes(lowerQuery) ||
        p.summary.toLowerCase().includes(lowerQuery)
      ))
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
   * Post a message to a discussion (MVP: saves to localStorage).
   */
  postMessage(message: DiscussionMessage): Observable<boolean> {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    const session = this.sessionUser.getSession();
    const userName = session?.displayName || 'Anonymous';

    const newMessage = {
      id: `msg-local-${Date.now()}`,
      authorId: agentId,
      authorName: userName,
      content: message.content,
      createdAt: new Date().toISOString(),
      replyToId: message.replyToId
    };

    // Save to localStorage
    const key = `${this.STORAGE_PREFIX}discussion-messages-${message.discussionId}`;
    const existing = localStorage.getItem(key);
    const messages = existing ? JSON.parse(existing) : [];
    messages.push(newMessage);

    try {
      localStorage.setItem(key, JSON.stringify(messages));
      return of(true);
    } catch {
      return of(false);
    }
  }

  /**
   * Get local messages for a discussion (MVP supplement to server data).
   */
  getLocalMessages(discussionId: string): Array<{
    id: string;
    authorId: string;
    authorName: string;
    content: string;
    createdAt: string;
  }> {
    const key = `${this.STORAGE_PREFIX}discussion-messages-${discussionId}`;
    const data = localStorage.getItem(key);
    return data ? JSON.parse(data) : [];
  }

  // =========================================================================
  // SLA & Deadline Tracking
  // =========================================================================

  /**
   * Get challenges approaching SLA deadline.
   */
  getChallengesNearingDeadline(withinDays: number = 3): Observable<ChallengeRecord[]> {
    const cutoff = new Date(Date.now() + withinDays * 24 * 60 * 60 * 1000);

    return this.getChallenges().pipe(
      map(challenges => challenges.filter(c => {
        if (!c.slaDeadline) return false;
        if (!['acknowledged', 'under-review'].includes(c.status)) return false;

        const deadline = new Date(c.slaDeadline);
        return deadline <= cutoff;
      }))
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

  // =========================================================================
  // Local Storage (MVP)
  // =========================================================================

  private saveLocalChallenge(challenge: ChallengeRecord): void {
    const key = `${this.STORAGE_PREFIX}local-challenges`;
    const existing = localStorage.getItem(key);
    const challenges = existing ? JSON.parse(existing) : [];
    challenges.push(challenge);

    try {
      localStorage.setItem(key, JSON.stringify(challenges));
    } catch (err) {
      console.error('[GovernanceService] Failed to save challenge', err);
    }
  }

  private saveLocalProposal(proposal: ProposalRecord): void {
    const key = `${this.STORAGE_PREFIX}local-proposals`;
    const existing = localStorage.getItem(key);
    const proposals = existing ? JSON.parse(existing) : [];
    proposals.push(proposal);

    try {
      localStorage.setItem(key, JSON.stringify(proposals));
    } catch (err) {
      console.error('[GovernanceService] Failed to save proposal', err);
    }
  }
}
