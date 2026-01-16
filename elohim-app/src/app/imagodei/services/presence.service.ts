/**
 * Presence Service - ContributorPresence management.
 *
 * Philosophy:
 * - Anyone can create a presence for an absent contributor
 * - Recognition accumulates even while unclaimed
 * - Stewards care-take presences until the contributor joins
 * - Contributors can claim their presence and receive accumulated recognition
 *
 * Lifecycle: UNCLAIMED → STEWARDED → CLAIMED
 */

import { Injectable, inject, signal, computed } from '@angular/core';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import { IdentityService } from './identity.service';
import {
  type ContributorPresenceView,
  type PresenceState,
  type CreatePresenceRequest,
  type BeginStewardshipRequest,
  type InitiateClaimRequest,
  type ExternalIdentifier,
  parseExternalIdentifiers,
  serializeExternalIdentifiers,
} from '../models/presence.model';

// =============================================================================
// Wire Format Types (internal - snake_case matches conductor response)
// =============================================================================

/** ContributorPresence entry as returned from conductor */
interface PresenceEntry {
  id: string;
  displayName: string;
  presenceState: string;
  externalIdentifiersJson: string;
  establishingContentIdsJson: string;
  establishedAt: string;
  affinityTotal: number;
  uniqueEngagers: number;
  citationCount: number;
  endorsementsJson: string;
  recognitionScore: number;
  recognitionByContentJson: string;
  accumulatingSince: string;
  lastRecognitionAt: string;
  stewardId: string | null;
  stewardshipStartedAt: string | null;
  stewardshipCommitmentId: string | null;
  stewardshipQualityScore: number | null;
  claimInitiatedAt: string | null;
  claimVerifiedAt: string | null;
  claimVerificationMethod: string | null;
  claimEvidenceJson: string | null;
  claimedAgentId: string | null;
  claimRecognitionTransferredValue: number | null;
  claimRecognitionTransferredUnit: string | null;
  claimFacilitatedBy: string | null;
  invitationsJson: string;
  note: string | null;
  image: string | null;
  metadataJson: string;
  createdAt: string;
  updatedAt: string;
}

/** Presence result from conductor */
interface PresenceResult {
  actionHash: Uint8Array;
  presence: PresenceEntry;
}

/** Payload for creating a presence */
interface CreatePresencePayload {
  displayName: string;
  externalIdentifiersJson?: string;
  establishingContentIdsJson?: string;
  note?: string;
  image?: string;
  metadataJson?: string;
}

/** Payload for beginning stewardship */
interface BeginStewardshipPayload {
  presenceId: string;
  stewardAgentId: string;
  commitmentNote?: string;
}

/** Payload for initiating a claim */
interface InitiateClaimPayload {
  presenceId: string;
  claimEvidenceJson: string;
  verificationMethod: string;
}

// =============================================================================
// Type Mappers
// =============================================================================

/**
 * Map wire format to domain ContributorPresenceView.
 */
function mapToPresenceView(entry: PresenceEntry): ContributorPresenceView {
  return {
    id: entry.id,
    displayName: entry.displayName,
    presenceState: entry.presenceState as PresenceState,
    externalIdentifiers: parseExternalIdentifiers(entry.externalIdentifiersJson),
    establishingContentIds: parseJsonArray(entry.establishingContentIdsJson),
    establishedAt: entry.establishedAt,
    affinityTotal: entry.affinityTotal,
    uniqueEngagers: entry.uniqueEngagers,
    citationCount: entry.citationCount,
    recognitionScore: entry.recognitionScore,
    accumulatingSince: entry.accumulatingSince,
    lastRecognitionAt: entry.lastRecognitionAt,
    stewardId: entry.stewardId,
    stewardshipStartedAt: entry.stewardshipStartedAt,
    stewardshipQualityScore: entry.stewardshipQualityScore,
    claimInitiatedAt: entry.claimInitiatedAt,
    claimVerifiedAt: entry.claimVerifiedAt,
    claimVerificationMethod: entry.claimVerificationMethod,
    claimedAgentId: entry.claimedAgentId,
    note: entry.note,
    image: entry.image,
    createdAt: entry.createdAt,
    updatedAt: entry.updatedAt,
  };
}

/**
 * Map domain CreatePresenceRequest to wire format.
 */
function toCreatePayload(request: CreatePresenceRequest): CreatePresencePayload {
  return {
    displayName: request.displayName,
    externalIdentifiersJson: request.externalIdentifiers
      ? serializeExternalIdentifiers(request.externalIdentifiers)
      : undefined,
    establishingContentIdsJson: request.establishingContentIds
      ? JSON.stringify(request.establishingContentIds)
      : undefined,
    note: request.note,
    image: request.image,
  };
}

/**
 * Parse JSON array string safely.
 */
function parseJsonArray(json: string): string[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

// =============================================================================
// Presence Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class PresenceService {
  private readonly holochainClient = inject(HolochainClientService);
  private readonly identityService = inject(IdentityService);

  // ==========================================================================
  // State
  // ==========================================================================

  /** Cache of presences by ID */
  private readonly presencesSignal = signal<Map<string, ContributorPresenceView>>(new Map());

  /** Presences I'm stewarding */
  private readonly myStewardedSignal = signal<ContributorPresenceView[]>([]);

  /** Loading state */
  private readonly loadingSignal = signal(false);

  /** Error state */
  private readonly errorSignal = signal<string | null>(null);

  // ==========================================================================
  // Public Signals
  // ==========================================================================

  /** All cached presences */
  readonly presences = this.presencesSignal.asReadonly();

  /** Presences I'm stewarding */
  readonly myStewardedPresences = this.myStewardedSignal.asReadonly();

  /** Loading state */
  readonly isLoading = this.loadingSignal.asReadonly();

  /** Error message */
  readonly error = this.errorSignal.asReadonly();

  // ==========================================================================
  // Create Presence
  // ==========================================================================

  /**
   * Create a new contributor presence for an absent contributor.
   */
  async createPresence(request: CreatePresenceRequest): Promise<ContributorPresenceView> {
    if (!this.holochainClient.isConnected()) {
      throw new Error('Not connected to network');
    }

    this.loadingSignal.set(true);
    this.errorSignal.set(null);

    try {
      const payload = toCreatePayload(request);
      // ContributorPresence is identity domain - routes to imagodei DNA
      // TODO: Implement create_contributor_presence in imagodei zome
      const result = await this.holochainClient.callZome<PresenceResult>({
        zomeName: 'imagodei',
        fnName: 'create_contributor_presence',
        payload,
        roleName: 'imagodei',
      });

      if (!result.success || !result.data) {
        throw new Error(result.error ?? 'Failed to create presence');
      }

      const presence = mapToPresenceView(result.data.presence);
      this.cachePresence(presence);
      this.loadingSignal.set(false);

      return presence;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to create presence';
      this.errorSignal.set(errorMessage);
      this.loadingSignal.set(false);
      throw err;
    }
  }

  // ==========================================================================
  // Stewardship
  // ==========================================================================

  /**
   * Begin stewardship of an unclaimed presence.
   */
  async beginStewardship(presenceId: string, commitmentNote?: string): Promise<ContributorPresenceView> {
    if (!this.holochainClient.isConnected()) {
      throw new Error('Not connected to network');
    }

    const agentPubKey = this.identityService.agentPubKey();
    if (!agentPubKey) {
      throw new Error('Must be authenticated to begin stewardship');
    }

    this.loadingSignal.set(true);
    this.errorSignal.set(null);

    try {
      const payload: BeginStewardshipPayload = {
        presenceId: presenceId,
        stewardAgentId: agentPubKey,
        commitmentNote: commitmentNote,
      };

      // ContributorPresence stewardship is identity domain - routes to imagodei DNA
      // TODO: Implement begin_stewardship in imagodei zome
      const result = await this.holochainClient.callZome<PresenceResult>({
        zomeName: 'imagodei',
        fnName: 'begin_stewardship',
        payload,
        roleName: 'imagodei',
      });

      if (!result.success || !result.data) {
        throw new Error(result.error ?? 'Failed to begin stewardship');
      }

      const presence = mapToPresenceView(result.data.presence);
      this.cachePresence(presence);

      // Update my stewarded list
      await this.refreshMyStewardedPresences();

      this.loadingSignal.set(false);
      return presence;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to begin stewardship';
      this.errorSignal.set(errorMessage);
      this.loadingSignal.set(false);
      throw err;
    }
  }

  /**
   * Get presences I'm currently stewarding.
   */
  async getMyStewardedPresences(): Promise<ContributorPresenceView[]> {
    if (!this.holochainClient.isConnected()) {
      return [];
    }

    const agentPubKey = this.identityService.agentPubKey();
    if (!agentPubKey) {
      return [];
    }

    try {
      // Query presences by steward - routes to imagodei DNA
      // TODO: Implement get_presences_by_steward in imagodei zome
      const result = await this.holochainClient.callZome<PresenceResult[]>({
        zomeName: 'imagodei',
        fnName: 'get_presences_by_steward',
        payload: agentPubKey,
        roleName: 'imagodei',
      });

      if (!result.success || !result.data) {
        return [];
      }

      const presences = result.data.map(r => mapToPresenceView(r.presence));
      presences.forEach(p => this.cachePresence(p));
      this.myStewardedSignal.set(presences);

      return presences;
    } catch (err) {
      console.error('[PresenceService] Failed to get stewarded presences:', err);
      return [];
    }
  }

  // ==========================================================================
  // Claiming
  // ==========================================================================

  /**
   * Initiate a claim on a presence.
   */
  async initiateClaim(request: InitiateClaimRequest): Promise<ContributorPresenceView> {
    if (!this.holochainClient.isConnected()) {
      throw new Error('Not connected to network');
    }

    this.loadingSignal.set(true);
    this.errorSignal.set(null);

    try {
      const payload: InitiateClaimPayload = {
        presenceId: request.presenceId,
        claimEvidenceJson: JSON.stringify(request.claimEvidence),
        verificationMethod: request.verificationMethod,
      };

      // Claim initiation is identity domain - routes to imagodei DNA
      // TODO: Implement initiate_claim in imagodei zome
      const result = await this.holochainClient.callZome<PresenceResult>({
        zomeName: 'imagodei',
        fnName: 'initiate_claim',
        payload,
        roleName: 'imagodei',
      });

      if (!result.success || !result.data) {
        throw new Error(result.error ?? 'Failed to initiate claim');
      }

      const presence = mapToPresenceView(result.data.presence);
      this.cachePresence(presence);
      this.loadingSignal.set(false);

      return presence;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to initiate claim';
      this.errorSignal.set(errorMessage);
      this.loadingSignal.set(false);
      throw err;
    }
  }

  /**
   * Verify and finalize a claim.
   */
  async verifyClaim(presenceId: string): Promise<ContributorPresenceView> {
    if (!this.holochainClient.isConnected()) {
      throw new Error('Not connected to network');
    }

    this.loadingSignal.set(true);
    this.errorSignal.set(null);

    try {
      // Claim verification is identity domain - routes to imagodei DNA
      // TODO: Implement verify_claim in imagodei zome
      const result = await this.holochainClient.callZome<PresenceResult>({
        zomeName: 'imagodei',
        fnName: 'verify_claim',
        payload: presenceId,
        roleName: 'imagodei',
      });

      if (!result.success || !result.data) {
        throw new Error(result.error ?? 'Failed to verify claim');
      }

      const presence = mapToPresenceView(result.data.presence);
      this.cachePresence(presence);
      this.loadingSignal.set(false);

      return presence;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to verify claim';
      this.errorSignal.set(errorMessage);
      this.loadingSignal.set(false);
      throw err;
    }
  }

  // ==========================================================================
  // Queries
  // ==========================================================================

  /**
   * Get a presence by ID.
   */
  async getPresenceById(id: string): Promise<ContributorPresenceView | null> {
    // Check cache first
    const cached = this.presencesSignal().get(id);
    if (cached) {
      return cached;
    }

    if (!this.holochainClient.isConnected()) {
      return null;
    }

    try {
      // Presence lookup by ID - routes to imagodei DNA
      // TODO: Implement get_contributor_presence_by_id in imagodei zome
      const result = await this.holochainClient.callZome<PresenceResult | null>({
        zomeName: 'imagodei',
        fnName: 'get_contributor_presence_by_id',
        payload: id,
        roleName: 'imagodei',
      });

      if (!result.success || !result.data) {
        return null;
      }

      const presence = mapToPresenceView(result.data.presence);
      this.cachePresence(presence);

      return presence;
    } catch (err) {
      console.error('[PresenceService] Failed to get presence:', err);
      return null;
    }
  }

  /**
   * Get presences by state.
   */
  async getPresencesByState(state: PresenceState): Promise<ContributorPresenceView[]> {
    if (!this.holochainClient.isConnected()) {
      return [];
    }

    try {
      // Query presences by state - routes to imagodei DNA
      // Note: Per LINK_ARCHITECTURE.md, *ByState queries should prefer projection
      // TODO: Implement get_presences_by_state in imagodei zome or use projection
      const result = await this.holochainClient.callZome<PresenceResult[]>({
        zomeName: 'imagodei',
        fnName: 'get_presences_by_state',
        payload: state,
        roleName: 'imagodei',
      });

      if (!result.success || !result.data) {
        return [];
      }

      const presences = result.data.map(r => mapToPresenceView(r.presence));
      presences.forEach(p => this.cachePresence(p));

      return presences;
    } catch (err) {
      console.error('[PresenceService] Failed to get presences by state:', err);
      return [];
    }
  }

  // ==========================================================================
  // Cache Management
  // ==========================================================================

  /**
   * Add or update a presence in the cache.
   */
  private cachePresence(presence: ContributorPresenceView): void {
    this.presencesSignal.update(cache => {
      const newCache = new Map(cache);
      newCache.set(presence.id, presence);
      return newCache;
    });
  }

  /**
   * Refresh the list of presences I'm stewarding.
   */
  private async refreshMyStewardedPresences(): Promise<void> {
    await this.getMyStewardedPresences();
  }

  /**
   * Clear error state.
   */
  clearError(): void {
    this.errorSignal.set(null);
  }

  /**
   * Clear all cached presences.
   */
  clearCache(): void {
    this.presencesSignal.set(new Map());
    this.myStewardedSignal.set([]);
  }
}
