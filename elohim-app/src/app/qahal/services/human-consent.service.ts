import { Injectable } from '@angular/core';
import { Observable, of, throwError, BehaviorSubject } from 'rxjs';
import { tap } from 'rxjs/operators';

// Services from other pillars
import { LocalSourceChainService } from '../../lamad/services/local-source-chain.service';
import { SessionHumanService } from '../../imagodei/services/session-human.service';
// Local models
import {
  HumanConsent,
  IntimacyLevel,
  ConsentState,
  ElevationRequest,
  hasMinimumIntimacy,
  isConsentActive,
  canElevate,
} from '../models/human-consent.model';
// Models from lamad pillar
import { HumanConsentContent } from '../../lamad/models/source-chain.model';
import type { LearningPath, PathVisibility } from '../../lamad/models/learning-path.model';

/**
 * HumanConsentService - Manages consent-based relationships between humans.
 *
 * This service handles:
 * - Creating and managing consent relationships
 * - Graduated intimacy level management
 * - Visibility checking for paths and content
 * - Attestation integration for intimate-level relationships
 *
 * Storage: Uses agent-centric source chain entries for Holochain compatibility.
 *
 * Key Concept: Consent relationships are NOT graph edges. Humans don't become
 * nodes in the knowledge graph. Instead, consent governs visibility and
 * negotiation permissions for shared resources.
 */
@Injectable({ providedIn: 'root' })
export class HumanConsentService {
  private readonly consentsSubject = new BehaviorSubject<HumanConsent[]>([]);
  public consents$ = this.consentsSubject.asObservable();

  constructor(
    private readonly sourceChain: LocalSourceChainService,
    private readonly sessionHuman: SessionHumanService
  ) {}

  // =========================================================================
  // INITIALIZATION
  // =========================================================================

  /**
   * Initialize consent service for current agent.
   * Loads existing consents from source chain.
   */
  initialize(): void {
    this.loadConsents();
  }

  /**
   * Load consents from source chain.
   */
  private loadConsents(): void {
    if (!this.sourceChain.isInitialized()) {
      return;
    }

    const entries = this.sourceChain.getEntriesByType<HumanConsentContent>('human-consent');
    const currentAgentId = this.sourceChain.getAgentId();

    // Build consent objects from entries
    // Each consent ID should have one entry (latest state)
    const consentMap = new Map<string, HumanConsent>();

    for (const entry of entries) {
      const content = entry.content;

      // Only load consents where current agent is initiator or participant
      if (content.initiatorId !== currentAgentId && content.participantId !== currentAgentId) {
        continue;
      }

      // Get or create consent object
      const existing = consentMap.get(content.consentId);
      if (!existing || entry.timestamp > existing.updatedAt) {
        consentMap.set(content.consentId, this.contentToConsent(content, entry.timestamp));
      }
    }

    this.consentsSubject.next(Array.from(consentMap.values()));
  }

  // =========================================================================
  // CONSENT MANAGEMENT
  // =========================================================================

  /**
   * Send recognition (one-way, no consent needed from target).
   *
   * Recognition is the lowest level of relationship - like a citation
   * or public acknowledgment. The target doesn't need to accept.
   */
  sendRecognition(targetHumanId: string, note?: string): Observable<HumanConsent> {
    const currentAgentId = this.getCurrentAgentId();

    if (targetHumanId === currentAgentId) {
      return throwError(() => new Error('Cannot send recognition to yourself'));
    }

    // Check if relationship already exists
    const existing = this.findConsentWith(targetHumanId);
    if (existing) {
      return throwError(() => new Error('Relationship already exists with this human'));
    }

    const consent = this.createConsentRecord(
      currentAgentId,
      targetHumanId,
      'recognition',
      'not_required',
      note
    );

    return of(consent).pipe(
      tap(c => this.saveConsent(c))
    );
  }

  /**
   * Request connection (requires consent from target).
   *
   * Connection is a mutual relationship - like friend requests.
   * The target must accept before the relationship is active.
   */
  requestConnection(targetHumanId: string, message?: string): Observable<HumanConsent> {
    const currentAgentId = this.getCurrentAgentId();

    if (targetHumanId === currentAgentId) {
      return throwError(() => new Error('Cannot request connection with yourself'));
    }

    // Check if relationship already exists
    const existing = this.findConsentWith(targetHumanId);
    if (existing) {
      // Allow upgrading from recognition to connection
      if (existing.intimacyLevel === 'recognition') {
        return this.proposeElevation({
          consentId: existing.id,
          newLevel: 'connection',
          message,
        });
      }
      return throwError(() => new Error('Connection already exists or pending'));
    }

    const consent = this.createConsentRecord(
      currentAgentId,
      targetHumanId,
      'connection',
      'pending',
      message
    );

    return of(consent).pipe(
      tap(c => this.saveConsent(c))
    );
  }

  /**
   * Accept a pending consent request.
   */
  acceptConsent(consentId: string): Observable<HumanConsent> {
    const consent = this.findConsentById(consentId);
    if (!consent) {
      return throwError(() => new Error('Consent not found'));
    }

    const currentAgentId = this.getCurrentAgentId();
    if (consent.participantId !== currentAgentId) {
      return throwError(() => new Error('Only the participant can accept a consent request'));
    }

    if (consent.consentState !== 'pending') {
      return throwError(() => new Error('Consent is not pending'));
    }

    const updatedConsent = this.updateConsentState(consent, 'accepted');
    return of(updatedConsent).pipe(
      tap(c => this.saveConsent(c))
    );
  }

  /**
   * Decline a pending consent request.
   */
  declineConsent(consentId: string, reason?: string): Observable<void> {
    const consent = this.findConsentById(consentId);
    if (!consent) {
      return throwError(() => new Error('Consent not found'));
    }

    const currentAgentId = this.getCurrentAgentId();
    if (consent.participantId !== currentAgentId) {
      return throwError(() => new Error('Only the participant can decline a consent request'));
    }

    if (consent.consentState !== 'pending') {
      return throwError(() => new Error('Consent is not pending'));
    }

    const updatedConsent = this.updateConsentState(consent, 'declined');
    updatedConsent.responseMessage = reason;

    return of(undefined).pipe(
      tap(() => this.saveConsent(updatedConsent))
    );
  }

  /**
   * Revoke previously given consent.
   */
  revokeConsent(consentId: string, reason?: string): Observable<void> {
    const consent = this.findConsentById(consentId);
    if (!consent) {
      return throwError(() => new Error('Consent not found'));
    }

    const currentAgentId = this.getCurrentAgentId();
    // Both parties can revoke
    if (consent.initiatorId !== currentAgentId && consent.participantId !== currentAgentId) {
      return throwError(() => new Error('Only relationship participants can revoke consent'));
    }

    if (consent.consentState !== 'accepted') {
      return throwError(() => new Error('Cannot revoke consent that is not accepted'));
    }

    const updatedConsent = this.updateConsentState(consent, 'revoked');
    updatedConsent.responseMessage = reason;

    return of(undefined).pipe(
      tap(() => this.saveConsent(updatedConsent))
    );
  }

  /**
   * Propose elevation to higher intimacy level.
   */
  proposeElevation(request: ElevationRequest): Observable<HumanConsent> {
    const consent = this.findConsentById(request.consentId);
    if (!consent) {
      return throwError(() => new Error('Consent not found'));
    }

    if (!canElevate(consent)) {
      return throwError(() => new Error('Cannot elevate this consent relationship'));
    }

    const currentAgentId = this.getCurrentAgentId();

    // Verify the new level is actually higher
    if (!hasMinimumIntimacy(request.newLevel, consent.intimacyLevel)) {
      return throwError(() => new Error('Cannot elevate to same or lower level'));
    }

    // For intimate level, require attestation type
    if (request.newLevel === 'intimate' && !request.requiredAttestationType) {
      return throwError(() => new Error('Intimate level requires attestation type'));
    }

    // Create elevation request (sets state to pending again)
    const updatedConsent: HumanConsent = {
      ...consent,
      intimacyLevel: request.newLevel,
      consentState: 'pending',
      updatedAt: new Date().toISOString(),
      requestMessage: request.message,
      requiredAttestationType: request.requiredAttestationType,
      elevationAttempts: (consent.elevationAttempts ?? 0) + 1,
      stateHistory: [
        ...consent.stateHistory,
        {
          fromState: consent.consentState,
          toState: 'pending',
          fromLevel: consent.intimacyLevel,
          toLevel: request.newLevel,
          timestamp: new Date().toISOString(),
          initiatedBy: consent.initiatorId === currentAgentId ? 'initiator' : 'participant',
          reason: 'Elevation requested',
        },
      ],
    };

    return of(updatedConsent).pipe(
      tap(c => this.saveConsent(c))
    );
  }

  // =========================================================================
  // QUERIES
  // =========================================================================

  /**
   * Get consent relationship with specific human.
   */
  getConsentWith(humanId: string): Observable<HumanConsent | null> {
    return of(this.findConsentWith(humanId));
  }

  /**
   * Get all consent relationships for current human.
   */
  getMyConsents(filter?: {
    level?: IntimacyLevel;
    state?: ConsentState;
  }): Observable<HumanConsent[]> {
    let consents = this.consentsSubject.value;

    if (filter?.level) {
      consents = consents.filter(c => c.intimacyLevel === filter.level);
    }

    if (filter?.state) {
      consents = consents.filter(c => c.consentState === filter.state);
    }

    return of(consents);
  }

  /**
   * Get pending consent requests (incoming).
   */
  getPendingRequests(): Observable<HumanConsent[]> {
    const currentAgentId = this.getCurrentAgentId();
    const pending = this.consentsSubject.value.filter(
      c => c.participantId === currentAgentId && c.consentState === 'pending'
    );
    return of(pending);
  }

  /**
   * Get humans at a given intimacy level (or higher).
   */
  getHumansAtLevel(level: IntimacyLevel): Observable<string[]> {
    const consents = this.consentsSubject.value.filter(
      c => isConsentActive(c.consentState) && hasMinimumIntimacy(c.intimacyLevel, level)
    );

    const currentAgentId = this.getCurrentAgentId();
    const humanIds = consents.map(c =>
      c.initiatorId === currentAgentId ? c.participantId : c.initiatorId
    );

    return of(humanIds);
  }

  // =========================================================================
  // VISIBILITY CHECKS
  // =========================================================================

  /**
   * Check if path is visible to current human.
   */
  canViewPath(path: LearningPath): Observable<boolean> {
    // Public paths are always visible
    if (path.visibility === 'public') {
      return of(true);
    }

    const currentAgentId = this.getCurrentAgentId();

    // Creator can always see their own paths
    if (path.createdBy === currentAgentId) {
      return of(true);
    }

    // Check if current user is in participantIds
    if (path.participantIds?.includes(currentAgentId)) {
      // For intimate paths, also check attestations if required
      // Note: Attestation service integration pending (Phase 6)
      // Currently trusts participantIds for intimate paths
      return of(true);
    }

    // Check consent level matches visibility requirement
    const requiredLevel = this.visibilityToIntimacyLevel(path.visibility);
    if (!requiredLevel) {
      return of(false);
    }

    // Check if we have consent with the creator at required level
    const consent = this.findConsentWith(path.createdBy);
    if (!consent || !isConsentActive(consent.consentState)) {
      return of(false);
    }

    return of(hasMinimumIntimacy(consent.intimacyLevel, requiredLevel));
  }

  /**
   * Map path visibility to required intimacy level.
   */
  private visibilityToIntimacyLevel(visibility: PathVisibility): IntimacyLevel | null {
    switch (visibility) {
      case 'public':
        return null; // No consent needed
      case 'connections':
      case 'organization': // Legacy
        return 'connection';
      case 'trusted':
        return 'trusted';
      case 'intimate':
      case 'private': // Legacy
        return 'intimate';
      default:
        return null;
    }
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

  private findConsentWith(humanId: string): HumanConsent | null {
    const currentAgentId = this.getCurrentAgentId();
    return this.consentsSubject.value.find(
      c =>
        (c.initiatorId === currentAgentId && c.participantId === humanId) ||
        (c.initiatorId === humanId && c.participantId === currentAgentId)
    ) ?? null;
  }

  private findConsentById(consentId: string): HumanConsent | null {
    return this.consentsSubject.value.find(c => c.id === consentId) ?? null;
  }

  private createConsentRecord(
    initiatorId: string,
    participantId: string,
    level: IntimacyLevel,
    state: ConsentState,
    message?: string
  ): HumanConsent {
    const now = new Date().toISOString();
    const consentId = this.generateConsentId();

    return {
      id: consentId,
      initiatorId,
      participantId,
      intimacyLevel: level,
      consentState: state,
      createdAt: now,
      updatedAt: now,
      consentedAt: state === 'accepted' || state === 'not_required' ? now : undefined,
      requestMessage: message,
      stateHistory: [
        {
          fromState: 'pending', // Initial state before creation
          toState: state,
          fromLevel: undefined,
          toLevel: level,
          timestamp: now,
          initiatedBy: 'initiator',
          reason: 'Initial creation',
        },
      ],
    };
  }

  private updateConsentState(consent: HumanConsent, newState: ConsentState): HumanConsent {
    const now = new Date().toISOString();
    const currentAgentId = this.getCurrentAgentId();

    return {
      ...consent,
      consentState: newState,
      updatedAt: now,
      consentedAt: newState === 'accepted' ? now : consent.consentedAt,
      stateHistory: [
        ...consent.stateHistory,
        {
          fromState: consent.consentState,
          toState: newState,
          timestamp: now,
          initiatedBy:
            consent.initiatorId === currentAgentId ? 'initiator' : 'participant',
        },
      ],
    };
  }

  private saveConsent(consent: HumanConsent): void {
    // Create source chain entry
    const content: HumanConsentContent = {
      consentId: consent.id,
      initiatorId: consent.initiatorId,
      participantId: consent.participantId,
      intimacyLevel: consent.intimacyLevel,
      consentState: consent.consentState,
      updatedAt: consent.updatedAt,
      consentedAt: consent.consentedAt,
      requestMessage: consent.requestMessage,
      responseMessage: consent.responseMessage,
      validatingAttestationIds: consent.validatingAttestationIds,
      requiredAttestationType: consent.requiredAttestationType,
    };

    this.sourceChain.createEntry('human-consent', content);

    // Update local state
    const consents = this.consentsSubject.value;
    const existingIndex = consents.findIndex(c => c.id === consent.id);

    if (existingIndex >= 0) {
      consents[existingIndex] = consent;
    } else {
      consents.push(consent);
    }

    this.consentsSubject.next([...consents]);
  }

  private contentToConsent(content: HumanConsentContent, timestamp: string): HumanConsent {
    return {
      id: content.consentId,
      initiatorId: content.initiatorId,
      participantId: content.participantId,
      intimacyLevel: content.intimacyLevel,
      consentState: content.consentState,
      createdAt: timestamp, // Use entry timestamp as approximation
      updatedAt: content.updatedAt,
      consentedAt: content.consentedAt,
      requestMessage: content.requestMessage,
      responseMessage: content.responseMessage,
      validatingAttestationIds: content.validatingAttestationIds,
      requiredAttestationType: content.requiredAttestationType,
      stateHistory: [], // Not stored in source chain entry
    };
  }

  private generateConsentId(): string {
    const timestamp = Date.now().toString(36);
    const random = Math.random().toString(36).substring(2, 10); // NOSONAR - Non-cryptographic ID generation
    return `consent-${timestamp}-${random}`;
  }
}
