import { Injectable } from '@angular/core';

// @coverage: 74.4% (2026-01-31)

import { tap, switchMap } from 'rxjs/operators';

import { Observable, of, throwError, BehaviorSubject } from 'rxjs';

import { hasMinimumIntimacy } from '@app/elohim/models/human-consent.model';
import { PathNegotiationContent } from '@app/elohim/models/source-chain.model';
import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import { HumanConsentService } from '@app/elohim/services/human-consent.service';
import { LocalSourceChainService } from '@app/elohim/services/local-source-chain.service';

import {
  PathNegotiation,
  NegotiationStatus,
  BridgingStrategy,
  NegotiationMessage,
  NegotiationRequest,
  NegotiationResponse,
  PathAcceptance,
  AffinityAnalysis,
  ProposedPathStructure,
  isNegotiationActive,
  isNegotiationResolved,
} from '../models';

/**
 * PathNegotiationService - Placeholder for Elohim-to-Elohim path negotiation.
 *
 * MVP Implementation:
 * - Simple affinity comparison between two humans
 * - Manual bridging path generation based on shared concepts
 * - No actual agent-to-agent communication
 *
 * Future Holochain Implementation:
 * - Full agent-to-agent negotiation protocol
 * - AI-assisted path generation
 * - Real-time negotiation messaging
 *
 * This service requires intimate-level consent with valid attestations.
 */
@Injectable({ providedIn: 'root' })
export class PathNegotiationService {
  private readonly negotiationsSubject = new BehaviorSubject<PathNegotiation[]>([]);
  public negotiations$ = this.negotiationsSubject.asObservable();

  constructor(
    private readonly sourceChain: LocalSourceChainService,
    private readonly consentService: HumanConsentService,
    private readonly affinityService: AffinityTrackingService
  ) {}

  // =========================================================================
  // INITIALIZATION
  // =========================================================================

  /**
   * Initialize negotiation service.
   * Loads existing negotiations from source chain.
   */
  initialize(): void {
    this.loadNegotiations();
  }

  private loadNegotiations(): void {
    if (!this.sourceChain.isInitialized()) {
      return;
    }

    const entries = this.sourceChain.getEntriesByType<PathNegotiationContent>('path-negotiation');
    const currentAgentId = this.sourceChain.getAgentId();

    const negotiationMap = new Map<string, PathNegotiation>();

    for (const entry of entries) {
      const content = entry.content;

      // Only load negotiations where current agent is involved
      if (content.initiatorId !== currentAgentId && content.participantId !== currentAgentId) {
        continue;
      }

      const existing = negotiationMap.get(content.negotiationId);
      if (!existing) {
        negotiationMap.set(
          content.negotiationId,
          this.contentToNegotiation(content, entry.timestamp)
        );
      }
    }

    this.negotiationsSubject.next(Array.from(negotiationMap.values()));
  }

  // =========================================================================
  // NEGOTIATION LIFECYCLE
  // =========================================================================

  /**
   * Propose a love map negotiation.
   *
   * Requirements:
   * - Valid consent at intimate level
   * - Both parties have validating attestations
   */
  proposeNegotiation(request: NegotiationRequest): Observable<PathNegotiation> {
    const currentAgentId = this.getCurrentAgentId();

    if (request.participantId === currentAgentId) {
      return throwError(() => new Error('Cannot negotiate a path with yourself'));
    }

    // Verify consent exists at intimate level
    return this.consentService.getConsentWith(request.participantId).pipe(
      switchMap(consent => {
        if (!consent) {
          return throwError(() => new Error('No consent relationship exists with this human'));
        }

        if (!hasMinimumIntimacy(consent.intimacyLevel, 'intimate')) {
          return throwError(
            () => new Error('Intimate-level consent required for love map negotiation')
          );
        }

        if (consent.consentState !== 'accepted') {
          return throwError(() => new Error('Consent must be accepted to initiate negotiation'));
        }

        // Check for existing active negotiation
        const existingActive = this.findActiveNegotiationWith(request.participantId);
        if (existingActive) {
          return throwError(
            () => new Error('An active negotiation already exists with this human')
          );
        }

        // Create negotiation
        const negotiation = this.createNegotiationRecord(
          currentAgentId,
          request.participantId,
          consent.id,
          consent.validatingAttestationIds ?? [],
          request.preferredStrategy,
          request.message
        );

        return of(negotiation).pipe(tap(n => this.saveNegotiation(n)));
      })
    );
  }

  /**
   * Accept a negotiation proposal.
   * Triggers affinity analysis.
   */
  acceptNegotiation(
    negotiationId: string,
    response?: NegotiationResponse
  ): Observable<PathNegotiation> {
    const negotiation = this.findNegotiationById(negotiationId);
    if (!negotiation) {
      return throwError(() => new Error('Negotiation not found'));
    }

    const currentAgentId = this.getCurrentAgentId();
    if (negotiation.participantId !== currentAgentId) {
      return throwError(() => new Error('Only the participant can accept a negotiation'));
    }

    if (negotiation.status !== 'proposed') {
      return throwError(() => new Error('Negotiation is not in proposed state'));
    }

    // Update status to analyzing
    let updatedNegotiation = this.updateNegotiationStatus(negotiation, 'analyzing');

    // If participant specified a strategy preference, use it
    if (response?.preferredStrategy) {
      updatedNegotiation = {
        ...updatedNegotiation,
        bridgingStrategy: response.preferredStrategy,
      };
    }

    // Add accept message to log
    updatedNegotiation = this.addMessage(updatedNegotiation, {
      authorId: currentAgentId,
      timestamp: new Date().toISOString(),
      type: 'accept',
      content: response?.message ?? 'Accepted negotiation',
    });

    return of(updatedNegotiation).pipe(
      tap(n => this.saveNegotiation(n)),
      // Trigger affinity analysis
      switchMap(n => this.analyzeAffinities(n))
    );
  }

  /**
   * Decline a negotiation proposal.
   */
  declineNegotiation(negotiationId: string, reason?: string): Observable<void> {
    const negotiation = this.findNegotiationById(negotiationId);
    if (!negotiation) {
      return throwError(() => new Error('Negotiation not found'));
    }

    const currentAgentId = this.getCurrentAgentId();
    if (negotiation.participantId !== currentAgentId) {
      return throwError(() => new Error('Only the participant can decline a negotiation'));
    }

    if (!isNegotiationActive(negotiation.status)) {
      return throwError(() => new Error('Negotiation is not active'));
    }

    let updatedNegotiation = this.updateNegotiationStatus(negotiation, 'declined');
    updatedNegotiation = this.addMessage(updatedNegotiation, {
      authorId: currentAgentId,
      timestamp: new Date().toISOString(),
      type: 'decline',
      content: reason ?? 'Declined negotiation',
    });

    return of(undefined).pipe(tap(() => this.saveNegotiation(updatedNegotiation)));
  }

  // =========================================================================
  // AFFINITY ANALYSIS (MVP: Simple Implementation)
  // =========================================================================

  /**
   * Analyze shared affinities between two humans.
   * MVP: Simple comparison of affinity marks.
   * Future: Sophisticated graph analysis.
   */
  analyzeSharedAffinities(humanId1: string, humanId2: string): Observable<AffinityAnalysis> {
    // MVP: Get affinities for both humans from affinity service
    // For now, return a placeholder analysis
    const analysis: AffinityAnalysis = {
      human1Id: humanId1,
      human2Id: humanId2,
      analyzedAt: new Date().toISOString(),
      sharedHighAffinity: [],
      divergent: {
        human1Only: [],
        human2Only: [],
      },
      compatibilityScore: 0,
      recommendedStrategies: ['maximum_overlap', 'complementary'],
    };

    // In a real implementation, we would:
    // 1. Get all affinity marks for human1
    // 2. Get all affinity marks for human2
    // 3. Find overlaps where both have high affinity
    // 4. Find divergent concepts
    // 5. Calculate compatibility score

    return of(analysis);
  }

  /**
   * Analyze affinities for a negotiation and update it.
   */
  private analyzeAffinities(negotiation: PathNegotiation): Observable<PathNegotiation> {
    return this.analyzeSharedAffinities(negotiation.initiatorId, negotiation.participantId).pipe(
      switchMap(analysis => {
        const updatedNegotiation: PathNegotiation = {
          ...negotiation,
          status: 'negotiating',
          sharedAffinityNodes: analysis.sharedHighAffinity.map(n => n.nodeId),
          divergentNodes: {
            initiator: analysis.divergent.human1Only.map(n => n.nodeId),
            participant: analysis.divergent.human2Only.map(n => n.nodeId),
          },
          sharedAffinityScores: analysis.sharedHighAffinity.reduce(
            (acc, n) => ({ ...acc, [n.nodeId]: n.affinity }),
            {}
          ),
          updatedAt: new Date().toISOString(),
        };

        // Add system message about analysis completion
        const withMessage = this.addMessage(updatedNegotiation, {
          authorId: 'system',
          timestamp: new Date().toISOString(),
          type: 'system',
          content: `Analysis complete. Found ${analysis.sharedHighAffinity.length} shared concepts.`,
          metadata: {
            compatibilityScore: analysis.compatibilityScore,
            recommendedStrategies: analysis.recommendedStrategies,
          },
        });

        return of(withMessage).pipe(tap(n => this.saveNegotiation(n)));
      })
    );
  }

  // =========================================================================
  // PATH GENERATION (MVP: Simple, Future: Agent-Negotiated)
  // =========================================================================

  /**
   * Generate a bridging path from the negotiation.
   * MVP: Simple path structure based on strategy.
   * Future: AI-negotiated path with review cycles.
   */
  generateBridgingPath(
    negotiationId: string,
    strategy: BridgingStrategy
  ): Observable<ProposedPathStructure> {
    const negotiation = this.findNegotiationById(negotiationId);
    if (!negotiation) {
      return throwError(() => new Error('Negotiation not found'));
    }

    if (negotiation.status !== 'negotiating') {
      return throwError(() => new Error('Negotiation must be in negotiating state'));
    }

    // MVP: Generate a simple path structure
    const proposedPath = this.generatePathStructure(negotiation, strategy);

    // Update negotiation with proposed structure
    const updatedNegotiation: PathNegotiation = {
      ...negotiation,
      bridgingStrategy: strategy,
      proposedPathStructure: proposedPath,
      updatedAt: new Date().toISOString(),
    };

    return of(proposedPath).pipe(tap(() => this.saveNegotiation(updatedNegotiation)));
  }

  /**
   * Generate path structure based on strategy.
   * MVP implementation - simple structure.
   */
  private generatePathStructure(
    negotiation: PathNegotiation,
    strategy: BridgingStrategy
  ): ProposedPathStructure {
    const shared = negotiation.sharedAffinityNodes;
    const initiatorConcepts = negotiation.divergentNodes.initiator;
    const participantConcepts = negotiation.divergentNodes.participant;

    let conceptIds: string[] = [];
    let title = '';
    let description = '';

    switch (strategy) {
      case 'shortest_path':
        // Focus on shared concepts only
        conceptIds = shared.slice(0, 5);
        title = 'Quick Connection Path';
        description = 'A focused journey through your shared interests';
        break;

      case 'maximum_overlap':
        // All shared concepts
        conceptIds = shared;
        title = 'Common Ground Path';
        description = 'Explore the concepts you both resonate with';
        break;

      case 'complementary':
        // Mix of teaching opportunities
        conceptIds = [
          ...initiatorConcepts.slice(0, 3),
          ...shared.slice(0, 3),
          ...participantConcepts.slice(0, 3),
        ];
        title = 'Mutual Learning Path';
        description = 'Learn from each other while exploring shared interests';
        break;

      case 'exploration':
        // Include novel concepts (for MVP, just use divergent)
        conceptIds = [
          ...shared.slice(0, 2),
          ...initiatorConcepts.slice(0, 2),
          ...participantConcepts.slice(0, 2),
        ];
        title = 'Shared Adventure Path';
        description = 'Discover new territories together';
        break;

      default:
        conceptIds = shared;
        title = 'Love Map Path';
        description = 'A journey toward understanding';
    }

    return {
      title,
      description,
      stepCount: conceptIds.length,
      estimatedDuration: `${conceptIds.length * 15} minutes`,
      conceptIds,
      stats: {
        sharedConcepts: shared.filter(c => conceptIds.includes(c)).length,
        initiatorTeaching: initiatorConcepts.filter(c => conceptIds.includes(c)).length,
        participantTeaching: participantConcepts.filter(c => conceptIds.includes(c)).length,
        novelConcepts: 0, // MVP: No novel concept detection
      },
    };
  }

  /**
   * Accept the generated path and finalize negotiation.
   */
  acceptGeneratedPath(acceptance: PathAcceptance): Observable<PathNegotiation> {
    const negotiation = this.findNegotiationById(acceptance.negotiationId);
    if (!negotiation) {
      return throwError(() => new Error('Negotiation not found'));
    }

    if (!negotiation.proposedPathStructure) {
      return throwError(() => new Error('No proposed path to accept'));
    }

    const currentAgentId = this.getCurrentAgentId();

    if (acceptance.accept) {
      // Generate the actual path (MVP: just mark as accepted)
      // Future: Create actual LearningPath entry
      const pathId = this.generatePathId();

      const updatedNegotiation: PathNegotiation = {
        ...negotiation,
        status: 'accepted',
        generatedPathId: pathId,
        resolvedAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      return of(
        this.addMessage(updatedNegotiation, {
          authorId: currentAgentId,
          timestamp: new Date().toISOString(),
          type: 'accept',
          content: 'Path accepted and generated',
          metadata: { pathId },
        })
      ).pipe(tap(n => this.saveNegotiation(n)));
    } else {
      // Request changes
      return of(
        this.addMessage(negotiation, {
          authorId: currentAgentId,
          timestamp: new Date().toISOString(),
          type: 'counter',
          content: acceptance.feedback ?? 'Requested changes to path',
        })
      ).pipe(tap(n => this.saveNegotiation(n)));
    }
  }

  // =========================================================================
  // QUERIES
  // =========================================================================

  /**
   * Get active negotiations for current human.
   */
  getMyNegotiations(filter?: { status?: NegotiationStatus }): Observable<PathNegotiation[]> {
    let negotiations = this.negotiationsSubject.value;

    if (filter?.status) {
      negotiations = negotiations.filter(n => n.status === filter.status);
    }

    return of(negotiations);
  }

  /**
   * Get a specific negotiation.
   */
  getNegotiation(negotiationId: string): Observable<PathNegotiation | null> {
    return of(this.findNegotiationById(negotiationId));
  }

  /**
   * Get pending negotiations (where current user needs to respond).
   */
  getPendingNegotiations(): Observable<PathNegotiation[]> {
    const currentAgentId = this.getCurrentAgentId();
    const pending = this.negotiationsSubject.value.filter(
      n => n.participantId === currentAgentId && n.status === 'proposed'
    );
    return of(pending);
  }

  // =========================================================================
  // MESSAGING
  // =========================================================================

  /**
   * Send a message in an active negotiation.
   */
  sendMessage(
    negotiationId: string,
    content: string,
    type: NegotiationMessage['type'] = 'comment'
  ): Observable<PathNegotiation> {
    const negotiation = this.findNegotiationById(negotiationId);
    if (!negotiation) {
      return throwError(() => new Error('Negotiation not found'));
    }

    if (!isNegotiationActive(negotiation.status)) {
      return throwError(() => new Error('Cannot send messages to resolved negotiation'));
    }

    const currentAgentId = this.getCurrentAgentId();
    const updatedNegotiation = this.addMessage(negotiation, {
      authorId: currentAgentId,
      timestamp: new Date().toISOString(),
      type,
      content,
    });

    return of(updatedNegotiation).pipe(tap(n => this.saveNegotiation(n)));
  }

  // =========================================================================
  // PRIVATE HELPERS
  // =========================================================================

  private getCurrentAgentId(): string {
    if (!this.sourceChain.isInitialized()) {
      throw new Error('Source chain not initialized');
    }
    return this.sourceChain.getAgentId();
  }

  private findNegotiationById(negotiationId: string): PathNegotiation | null {
    return this.negotiationsSubject.value.find(n => n.id === negotiationId) ?? null;
  }

  private findActiveNegotiationWith(humanId: string): PathNegotiation | null {
    const currentAgentId = this.getCurrentAgentId();
    return (
      this.negotiationsSubject.value.find(
        n =>
          isNegotiationActive(n.status) &&
          ((n.initiatorId === currentAgentId && n.participantId === humanId) ||
            (n.initiatorId === humanId && n.participantId === currentAgentId))
      ) ?? null
    );
  }

  private createNegotiationRecord(
    initiatorId: string,
    participantId: string,
    consentId: string,
    attestationIds: string[],
    strategy?: BridgingStrategy,
    message?: string
  ): PathNegotiation {
    const now = new Date().toISOString();
    const negotiationId = this.generateNegotiationId();

    const negotiation: PathNegotiation = {
      id: negotiationId,
      initiatorId,
      participantId,
      status: 'proposed',
      consentId,
      requiredIntimacyLevel: 'intimate',
      validatingAttestationIds: attestationIds,
      sharedAffinityNodes: [],
      divergentNodes: { initiator: [], participant: [] },
      bridgingStrategy: strategy,
      createdAt: now,
      updatedAt: now,
      negotiationLog: [],
    };

    // Add initial proposal message
    if (message) {
      return this.addMessage(negotiation, {
        authorId: initiatorId,
        timestamp: now,
        type: 'proposal',
        content: message,
      });
    }

    return negotiation;
  }

  private updateNegotiationStatus(
    negotiation: PathNegotiation,
    newStatus: NegotiationStatus
  ): PathNegotiation {
    const now = new Date().toISOString();
    return {
      ...negotiation,
      status: newStatus,
      updatedAt: now,
      resolvedAt: isNegotiationResolved(newStatus) ? now : negotiation.resolvedAt,
    };
  }

  private addMessage(negotiation: PathNegotiation, message: NegotiationMessage): PathNegotiation {
    return {
      ...negotiation,
      negotiationLog: [...negotiation.negotiationLog, message],
      updatedAt: message.timestamp,
    };
  }

  private saveNegotiation(negotiation: PathNegotiation): void {
    // Create source chain entry
    const content: PathNegotiationContent = {
      negotiationId: negotiation.id,
      initiatorId: negotiation.initiatorId,
      participantId: negotiation.participantId,
      status: negotiation.status,
      consentId: negotiation.consentId,
      sharedAffinityNodes: negotiation.sharedAffinityNodes,
      bridgingStrategy: negotiation.bridgingStrategy,
      generatedPathId: negotiation.generatedPathId,
      resolvedAt: negotiation.resolvedAt,
    };

    this.sourceChain.createEntry('path-negotiation', content);

    // Update local state
    const negotiations = this.negotiationsSubject.value;
    const existingIndex = negotiations.findIndex(n => n.id === negotiation.id);

    if (existingIndex >= 0) {
      negotiations[existingIndex] = negotiation;
    } else {
      negotiations.push(negotiation);
    }

    this.negotiationsSubject.next([...negotiations]);
  }

  private contentToNegotiation(
    content: PathNegotiationContent,
    timestamp: string
  ): PathNegotiation {
    return {
      id: content.negotiationId,
      initiatorId: content.initiatorId,
      participantId: content.participantId,
      status: content.status,
      consentId: content.consentId,
      requiredIntimacyLevel: 'intimate',
      validatingAttestationIds: [],
      sharedAffinityNodes: content.sharedAffinityNodes,
      divergentNodes: { initiator: [], participant: [] },
      bridgingStrategy: content.bridgingStrategy,
      generatedPathId: content.generatedPathId,
      createdAt: timestamp,
      updatedAt: timestamp,
      resolvedAt: content.resolvedAt,
      negotiationLog: [],
    };
  }

  private generateNegotiationId(): string {
    const timestamp = Date.now().toString(36);
    const random = (crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 10); // Crypto-secure random ID
    return `negotiation-${timestamp}-${random}`;
  }

  private generatePathId(): string {
    const timestamp = Date.now().toString(36);
    const random = (crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 10); // Crypto-secure random ID
    return `love-map-${timestamp}-${random}`;
  }
}
