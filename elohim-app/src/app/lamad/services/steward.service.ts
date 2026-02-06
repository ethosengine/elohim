/**
 * Steward Service - Lamad Steward Economy Operations
 *
 * From the Manifesto (Part IV-C: The Economics of Stewardship):
 * "The Steward Economy enables sustainable income for those who care-take the
 * knowledge graph. Stewards may or may not be the original creators - they
 * earn from maintaining, curating, and making knowledge accessible."
 *
 * This service provides access to:
 * - StewardCredential: Proof of qualification (mastery, attestations, track record)
 * - PremiumGate: Access control with pricing and revenue sharing
 * - AccessGrant: Learner access records
 * - StewardRevenue: Three-way split (steward, contributor, commons)
 *
 * Architecture:
 *   StewardService → HolochainClientService → Holochain Conductor → DHT
 *
 * @see ContributorService for contributor dashboards and recognition
 * @see EconomicService for underlying hREA primitives
 */

import { Injectable, signal, computed } from '@angular/core';

// @coverage: 23.4% (2026-02-05)

import { catchError, shareReplay, tap } from 'rxjs/operators';

import { BehaviorSubject, Observable, of, from, defer } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import {
  StewardCredential,
  StewardTier,
  PremiumGate,
  PricingModel,
  AccessGrant,
  GrantType,
  StewardRevenueSummary,
  CreateStewardCredentialInput,
  CreatePremiumGateInput,
  GrantAccessInput,
  RequiredAttestation,
  RequiredMastery,
  RequiredVouches,
} from '../models/steward-economy.model';

// =============================================================================
// Holochain Types (match Rust DNA structures)
// =============================================================================

interface HolochainStewardCredential {
  id: string;
  stewardPresenceId: string;
  agentId: string;
  tier: string;
  stewartedPresenceIdsJson: string;
  stewartedContentIdsJson: string;
  stewartedPathIdsJson: string;
  masteryContentIdsJson: string;
  masteryLevelAchieved: string;
  qualificationVerifiedAt: string;
  peerAttestationIdsJson: string;
  uniqueAttesterCount: number;
  attesterReputationSum: number;
  stewardshipQualityScore: number;
  totalLearnersServed: number;
  totalContentImprovements: number;
  domainTagsJson: string;
  isActive: boolean;
  deactivationReason: string | null;
  note: string | null;
  metadataJson: string;
  createdAt: string;
  updatedAt: string;
}

interface HolochainStewardCredentialOutput {
  actionHash: Uint8Array;
  credential: HolochainStewardCredential;
}

interface HolochainPremiumGate {
  id: string;
  stewardCredentialId: string;
  stewardPresenceId: string;
  contributorPresenceId: string | null;
  gatedResourceType: string;
  gatedResourceIdsJson: string;
  gateTitle: string;
  gateDescription: string;
  gateImage: string | null;
  requiredAttestationsJson: string;
  requiredMasteryJson: string;
  requiredVouchesJson: string;
  pricingModel: string;
  priceAmount: number | null;
  priceUnit: string | null;
  subscriptionPeriodDays: number | null;
  minAmount: number | null;
  stewardSharePercent: number;
  commonsSharePercent: number;
  contributorSharePercent: number | null;
  scholarshipEligible: boolean;
  maxScholarshipsPerPeriod: number | null;
  scholarshipCriteriaJson: string | null;
  isActive: boolean;
  deactivationReason: string | null;
  totalAccessGrants: number;
  totalRevenueGenerated: number;
  totalToSteward: number;
  totalToContributor: number;
  totalToCommons: number;
  totalScholarshipsGranted: number;
  note: string | null;
  metadataJson: string;
  createdAt: string;
  updatedAt: string;
}

interface HolochainPremiumGateOutput {
  actionHash: Uint8Array;
  gate: HolochainPremiumGate;
}

interface HolochainAccessGrant {
  id: string;
  gateId: string;
  learnerAgentId: string;
  grantType: string;
  grantedVia: string;
  paymentEventId: string | null;
  paymentAmount: number | null;
  paymentUnit: string | null;
  scholarshipSponsorId: string | null;
  scholarshipReason: string | null;
  grantedAt: string;
  validUntil: string | null;
  renewalDueAt: string | null;
  isActive: boolean;
  revokedAt: string | null;
  revokeReason: string | null;
  metadataJson: string;
  createdAt: string;
}

interface HolochainAccessGrantOutput {
  actionHash: Uint8Array;
  grant: HolochainAccessGrant;
}

interface HolochainStewardRevenueSummary {
  stewardPresenceId: string;
  totalRevenue: number;
  totalGrants: number;
  revenueByGate: HolochainGateRevenueSummary[];
}

interface HolochainGateRevenueSummary {
  gateId: string;
  gateTitle: string;
  totalRevenue: number;
  grantCount: number;
}

// =============================================================================
// Service Implementation
// =============================================================================

const STEWARD_NOT_AVAILABLE = 'Steward service not available';

@Injectable({
  providedIn: 'root',
})
export class StewardService {
  /**
   * Whether steward service is available.
   * Mirrors HolochainClientService connection state.
   */
  private readonly availableSignal = signal(false);
  readonly available = this.availableSignal.asReadonly();

  /** Computed: true when Holochain client is connected AND service available */
  readonly ready = computed(() => this.available() && this.holochainClient.isConnected());

  /**
   * Current agent's steward credentials (reactive state).
   */
  private readonly myCredentialsSubject = new BehaviorSubject<StewardCredential[]>([]);
  readonly myCredentials$ = this.myCredentialsSubject.asObservable();

  /**
   * Current agent's premium gates (reactive state).
   */
  private readonly myGatesSubject = new BehaviorSubject<PremiumGate[]>([]);
  readonly myGates$ = this.myGatesSubject.asObservable();

  /**
   * Current agent's access grants (reactive state).
   */
  private readonly myAccessGrantsSubject = new BehaviorSubject<AccessGrant[]>([]);
  readonly myAccessGrants$ = this.myAccessGrantsSubject.asObservable();

  /** Cache for credentials by ID */
  private readonly credentialCache = new Map<string, Observable<StewardCredential | null>>();

  /** Cache for gates by resource ID */
  private readonly gatesByResourceCache = new Map<string, Observable<PremiumGate[]>>();

  /** Cache for access checks */
  private readonly accessCheckCache = new Map<string, Observable<AccessGrant | null>>();

  constructor(private readonly holochainClient: HolochainClientService) {}

  /**
   * Check if steward service is available.
   */
  isAvailable(): boolean {
    return this.availableSignal();
  }

  // ===========================================================================
  // Credential Methods
  // ===========================================================================

  /**
   * Create a new steward credential.
   *
   * @param input - The credential creation input
   * @returns Observable of the created credential
   */
  createCredential(input: CreateStewardCredentialInput): Observable<StewardCredential> {
    if (!this.isAvailable()) {
      throw new Error(STEWARD_NOT_AVAILABLE);
    }

    return defer(() => from(this.doCreateCredential(input))).pipe(
      tap(() => this.refreshMyCredentials()),
      catchError(err => {
        throw err;
      })
    );
  }

  /**
   * Get a steward credential by ID.
   *
   * @param credentialId - The credential ID
   * @returns Observable of the credential (or null if not found)
   */
  getCredential(credentialId: string): Observable<StewardCredential | null> {
    if (!this.isAvailable()) {
      return of(null);
    }

    if (!this.credentialCache.has(credentialId)) {
      const request = defer(() => from(this.fetchCredential(credentialId))).pipe(
        shareReplay(1),
        catchError(_err => {
          return of(null);
        })
      );

      this.credentialCache.set(credentialId, request);
    }

    return this.credentialCache.get(credentialId)!;
  }

  /**
   * Get the current agent's steward credentials.
   *
   * Also updates the reactive myCredentials$ observable.
   */
  getMyCredentials(): Observable<StewardCredential[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.fetchMyCredentials())).pipe(
      tap(credentials => this.myCredentialsSubject.next(credentials)),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Get credentials for a specific human presence.
   *
   * @param humanPresenceId - The human presence ID
   * @returns Observable of credentials for that presence
   */
  getCredentialsForHuman(humanPresenceId: string): Observable<StewardCredential[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.fetchCredentialsForHuman(humanPresenceId))).pipe(
      catchError(_err => {
        return of([]);
      })
    );
  }

  // ===========================================================================
  // Gate Methods
  // ===========================================================================

  /**
   * Create a new premium gate.
   *
   * @param input - The gate creation input
   * @returns Observable of the created gate
   */
  createGate(input: CreatePremiumGateInput): Observable<PremiumGate> {
    if (!this.isAvailable()) {
      throw new Error(STEWARD_NOT_AVAILABLE);
    }

    return defer(() => from(this.doCreateGate(input))).pipe(
      tap(() => this.refreshMyGates()),
      catchError(err => {
        throw err;
      })
    );
  }

  /**
   * Get a premium gate by ID.
   *
   * @param gateId - The gate ID
   * @returns Observable of the gate (or null if not found)
   */
  getGate(gateId: string): Observable<PremiumGate | null> {
    if (!this.isAvailable()) {
      return of(null);
    }

    return defer(() => from(this.fetchGate(gateId))).pipe(
      catchError(_err => {
        return of(null);
      })
    );
  }

  /**
   * Get all gates for a resource.
   *
   * Used to check if content is gated and what access is required.
   *
   * @param resourceId - The resource (content/path) ID
   * @returns Observable of gates for that resource
   */
  getGatesForResource(resourceId: string): Observable<PremiumGate[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    if (!this.gatesByResourceCache.has(resourceId)) {
      const request = defer(() => from(this.fetchGatesForResource(resourceId))).pipe(
        shareReplay(1),
        catchError(_err => {
          return of([]);
        })
      );

      this.gatesByResourceCache.set(resourceId, request);
    }

    return this.gatesByResourceCache.get(resourceId)!;
  }

  // ===========================================================================
  // Access Control Methods
  // ===========================================================================

  /**
   * Check if the current agent has access to a gated resource.
   *
   * @param gateId - The gate ID to check access for
   * @returns Observable of the access grant (or null if no access)
   */
  checkAccess(gateId: string): Observable<AccessGrant | null> {
    if (!this.isAvailable()) {
      return of(null);
    }

    if (!this.accessCheckCache.has(gateId)) {
      const request = defer(() => from(this.fetchAccessCheck(gateId))).pipe(
        shareReplay(1),
        catchError(_err => {
          return of(null);
        })
      );

      this.accessCheckCache.set(gateId, request);
    }

    return this.accessCheckCache.get(gateId)!;
  }

  /**
   * Grant access to a learner.
   *
   * @param input - The access grant input
   * @returns Observable of the created access grant
   */
  grantAccess(input: GrantAccessInput): Observable<AccessGrant> {
    if (!this.isAvailable()) {
      throw new Error(STEWARD_NOT_AVAILABLE);
    }

    return defer(() => from(this.doGrantAccess(input))).pipe(
      tap(() => {
        // Clear relevant caches
        this.accessCheckCache.delete(input.gateId);
        this.refreshMyAccessGrants();
      }),
      catchError(err => {
        throw err;
      })
    );
  }

  /**
   * Get the current agent's access grants.
   *
   * Also updates the reactive myAccessGrants$ observable.
   */
  getMyAccessGrants(): Observable<AccessGrant[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.fetchMyAccessGrants())).pipe(
      tap(grants => this.myAccessGrantsSubject.next(grants)),
      catchError(_err => {
        return of([]);
      })
    );
  }

  // ===========================================================================
  // Revenue Methods
  // ===========================================================================

  /**
   * Get revenue summary for a steward.
   *
   * @param stewardPresenceId - The steward presence ID
   * @returns Observable of the revenue summary
   */
  getRevenueSummary(stewardPresenceId: string): Observable<StewardRevenueSummary | null> {
    if (!this.isAvailable()) {
      return of(null);
    }

    return defer(() => from(this.fetchRevenueSummary(stewardPresenceId))).pipe(
      catchError(_err => {
        return of(null);
      })
    );
  }

  // ===========================================================================
  // Cache Management
  // ===========================================================================

  /**
   * Clear all caches.
   */
  clearCache(): void {
    this.credentialCache.clear();
    this.gatesByResourceCache.clear();
    this.accessCheckCache.clear();
  }

  /**
   * Refresh current agent's credentials.
   */
  private refreshMyCredentials(): void {
    this.getMyCredentials().subscribe();
  }

  /**
   * Refresh current agent's gates.
   */
  private refreshMyGates(): void {
    // Would need a getMyGates() method - for now just clear cache
    this.gatesByResourceCache.clear();
  }

  /**
   * Refresh current agent's access grants.
   */
  private refreshMyAccessGrants(): void {
    this.getMyAccessGrants().subscribe();
  }

  /**
   * Test if steward API is reachable.
   * Updates the available signal based on result.
   */
  async testAvailability(): Promise<boolean> {
    try {
      const result = await this.holochainClient.callZome<HolochainAccessGrantOutput[]>({
        zomeName: 'content_store',
        fnName: 'get_my_access_grants',
        payload: null,
      });

      this.availableSignal.set(result.success);
      return result.success;
    } catch {
      // Service availability check failed - Steward service unavailable
      this.availableSignal.set(false);
      return false;
    }
  }

  // ===========================================================================
  // Private Methods - Zome Calls
  // ===========================================================================

  private async doCreateCredential(
    input: CreateStewardCredentialInput
  ): Promise<StewardCredential> {
    const payload = {
      steward_presence_id: input.stewardPresenceId,
      tier: input.tier,
      domain_tags: input.domainTags,
      mastery_content_ids: input.masteryContentIds,
      mastery_level_achieved: input.masteryLevelAchieved,
      peer_attestation_ids: input.peerAttestationIds,
      stewarded_presence_ids: input.stewartedPresenceIds,
      stewarded_content_ids: input.stewartedContentIds,
      stewarded_path_ids: input.stewartedPathIds,
      note: input.note ?? null,
    };

    const result = await this.holochainClient.callZome<HolochainStewardCredentialOutput>({
      zomeName: 'content_store',
      fnName: 'create_steward_credential',
      payload,
    });

    if (!result.success || !result.data) {
      throw new Error(result.error ?? 'Failed to create steward credential');
    }

    return this.transformCredential(result.data);
  }

  private async fetchCredential(credentialId: string): Promise<StewardCredential | null> {
    const result = await this.holochainClient.callZome<HolochainStewardCredentialOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_steward_credential',
      payload: credentialId,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return this.transformCredential(result.data);
  }

  private async fetchMyCredentials(): Promise<StewardCredential[]> {
    // Note: This assumes there's a zome function to get current agent's credentials
    // If not available, this would need to be implemented differently
    const result = await this.holochainClient.callZome<HolochainStewardCredentialOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_my_credentials',
      payload: null,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data.map(o => this.transformCredential(o));
  }

  private async fetchCredentialsForHuman(humanPresenceId: string): Promise<StewardCredential[]> {
    const result = await this.holochainClient.callZome<HolochainStewardCredentialOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_credentials_for_human',
      payload: humanPresenceId,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data.map(o => this.transformCredential(o));
  }

  private async doCreateGate(input: CreatePremiumGateInput): Promise<PremiumGate> {
    const payload = {
      steward_credential_id: input.stewardCredentialId,
      steward_presence_id: input.stewardPresenceId,
      contributor_presence_id: input.contributorPresenceId ?? null,
      gated_resource_type: input.gatedResourceType,
      gated_resource_ids: input.gatedResourceIds,
      gate_title: input.gateTitle,
      gate_description: input.gateDescription,
      gate_image: input.gateImage ?? null,
      required_attestations: input.requiredAttestations,
      required_mastery: input.requiredMastery,
      required_vouches: input.requiredVouches ?? null,
      pricing_model: input.pricingModel,
      price_amount: input.priceAmount ?? null,
      price_unit: input.priceUnit ?? null,
      subscription_period_days: input.subscriptionPeriodDays ?? null,
      min_amount: input.minAmount ?? null,
      steward_share_percent: input.stewardSharePercent,
      commons_share_percent: input.commonsSharePercent,
      contributor_share_percent: input.contributorSharePercent ?? null,
      scholarship_eligible: input.scholarshipEligible,
      max_scholarships_per_period: input.maxScholarshipsPerPeriod ?? null,
      scholarship_criteria_json: input.scholarshipCriteria
        ? JSON.stringify(input.scholarshipCriteria)
        : null,
      note: input.note ?? null,
    };

    const result = await this.holochainClient.callZome<HolochainPremiumGateOutput>({
      zomeName: 'content_store',
      fnName: 'create_premium_gate',
      payload,
    });

    if (!result.success || !result.data) {
      throw new Error(result.error ?? 'Failed to create premium gate');
    }

    return this.transformGate(result.data);
  }

  private async fetchGate(gateId: string): Promise<PremiumGate | null> {
    const result = await this.holochainClient.callZome<HolochainPremiumGateOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_premium_gate',
      payload: gateId,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return this.transformGate(result.data);
  }

  private async fetchGatesForResource(resourceId: string): Promise<PremiumGate[]> {
    const result = await this.holochainClient.callZome<HolochainPremiumGateOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_gates_for_resource',
      payload: resourceId,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data.map(o => this.transformGate(o));
  }

  private async doGrantAccess(input: GrantAccessInput): Promise<AccessGrant> {
    const payload = {
      gate_id: input.gateId,
      grant_type: input.grantType,
      granted_via: input.grantedVia,
      payment_amount: input.paymentAmount ?? null,
      payment_unit: input.paymentUnit ?? null,
      scholarship_sponsor_id: input.scholarshipSponsorId ?? null,
      scholarship_reason: input.scholarshipReason ?? null,
    };

    const result = await this.holochainClient.callZome<HolochainAccessGrantOutput>({
      zomeName: 'content_store',
      fnName: 'grant_access',
      payload,
    });

    if (!result.success || !result.data) {
      throw new Error(result.error ?? 'Failed to grant access');
    }

    return this.transformAccessGrant(result.data);
  }

  private async fetchAccessCheck(gateId: string): Promise<AccessGrant | null> {
    const result = await this.holochainClient.callZome<HolochainAccessGrantOutput | null>({
      zomeName: 'content_store',
      fnName: 'check_access',
      payload: gateId,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return this.transformAccessGrant(result.data);
  }

  private async fetchMyAccessGrants(): Promise<AccessGrant[]> {
    const result = await this.holochainClient.callZome<HolochainAccessGrantOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_my_access_grants',
      payload: null,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data.map(o => this.transformAccessGrant(o));
  }

  private async fetchRevenueSummary(
    stewardPresenceId: string
  ): Promise<StewardRevenueSummary | null> {
    const result = await this.holochainClient.callZome<HolochainStewardRevenueSummary | null>({
      zomeName: 'content_store',
      fnName: 'get_steward_revenue_summary',
      payload: stewardPresenceId,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return this.transformRevenueSummary(result.data);
  }

  // ===========================================================================
  // Transformation - Holochain → Angular
  // ===========================================================================

  private transformCredential(output: HolochainStewardCredentialOutput): StewardCredential {
    const hc = output.credential;

    return {
      id: hc.id,
      stewardPresenceId: hc.stewardPresenceId,
      agentId: hc.agentId,
      tier: hc.tier as StewardTier,
      stewartedPresenceIds: this.safeParseJson<string[]>(hc.stewartedPresenceIdsJson, []),
      stewartedContentIds: this.safeParseJson<string[]>(hc.stewartedContentIdsJson, []),
      stewartedPathIds: this.safeParseJson<string[]>(hc.stewartedPathIdsJson, []),
      masteryContentIds: this.safeParseJson<string[]>(hc.masteryContentIdsJson, []),
      masteryLevelAchieved: hc.masteryLevelAchieved,
      qualificationVerifiedAt: hc.qualificationVerifiedAt,
      peerAttestationIds: this.safeParseJson<string[]>(hc.peerAttestationIdsJson, []),
      uniqueAttesterCount: hc.uniqueAttesterCount,
      attesterReputationSum: hc.attesterReputationSum,
      stewardshipQualityScore: hc.stewardshipQualityScore,
      totalLearnersServed: hc.totalLearnersServed,
      totalContentImprovements: hc.totalContentImprovements,
      domainTags: this.safeParseJson<string[]>(hc.domainTagsJson, []),
      isActive: hc.isActive,
      deactivationReason: hc.deactivationReason,
      note: hc.note,
      metadata: this.safeParseJson<Record<string, unknown>>(hc.metadataJson, {}),
      createdAt: hc.createdAt,
      updatedAt: hc.updatedAt,
    };
  }

  private transformGate(output: HolochainPremiumGateOutput): PremiumGate {
    const hc = output.gate;

    return {
      id: hc.id,
      stewardCredentialId: hc.stewardCredentialId,
      stewardPresenceId: hc.stewardPresenceId,
      contributorPresenceId: hc.contributorPresenceId,
      gatedResourceType: hc.gatedResourceType,
      gatedResourceIds: this.safeParseJson<string[]>(hc.gatedResourceIdsJson, []),
      gateTitle: hc.gateTitle,
      gateDescription: hc.gateDescription,
      gateImage: hc.gateImage,
      requiredAttestations: this.safeParseJson<RequiredAttestation[]>(
        hc.requiredAttestationsJson,
        []
      ),
      requiredMastery: this.safeParseJson<RequiredMastery[]>(hc.requiredMasteryJson, []),
      requiredVouches: this.safeParseJson<RequiredVouches | null>(hc.requiredVouchesJson, null),
      pricingModel: hc.pricingModel as PricingModel,
      priceAmount: hc.priceAmount,
      priceUnit: hc.priceUnit,
      subscriptionPeriodDays: hc.subscriptionPeriodDays,
      minAmount: hc.minAmount,
      stewardSharePercent: hc.stewardSharePercent,
      commonsSharePercent: hc.commonsSharePercent,
      contributorSharePercent: hc.contributorSharePercent,
      scholarshipEligible: hc.scholarshipEligible,
      maxScholarshipsPerPeriod: hc.maxScholarshipsPerPeriod,
      scholarshipCriteria: this.safeParseJson<Record<string, unknown> | null>(
        hc.scholarshipCriteriaJson,
        null
      ),
      isActive: hc.isActive,
      deactivationReason: hc.deactivationReason,
      totalAccessGrants: hc.totalAccessGrants,
      totalRevenueGenerated: hc.totalRevenueGenerated,
      totalToSteward: hc.totalToSteward,
      totalToContributor: hc.totalToContributor,
      totalToCommons: hc.totalToCommons,
      totalScholarshipsGranted: hc.totalScholarshipsGranted,
      note: hc.note,
      metadata: this.safeParseJson<Record<string, unknown>>(hc.metadataJson, {}),
      createdAt: hc.createdAt,
      updatedAt: hc.updatedAt,
    };
  }

  private transformAccessGrant(output: HolochainAccessGrantOutput): AccessGrant {
    const hc = output.grant;

    return {
      id: hc.id,
      gateId: hc.gateId,
      learnerAgentId: hc.learnerAgentId,
      grantType: hc.grantType as GrantType,
      grantedVia: hc.grantedVia,
      paymentEventId: hc.paymentEventId,
      paymentAmount: hc.paymentAmount,
      paymentUnit: hc.paymentUnit,
      scholarshipSponsorId: hc.scholarshipSponsorId,
      scholarshipReason: hc.scholarshipReason,
      grantedAt: hc.grantedAt,
      validUntil: hc.validUntil,
      renewalDueAt: hc.renewalDueAt,
      isActive: hc.isActive,
      revokedAt: hc.revokedAt,
      revokeReason: hc.revokeReason,
      metadata: this.safeParseJson<Record<string, unknown>>(hc.metadataJson, {}),
      createdAt: hc.createdAt,
    };
  }

  private transformRevenueSummary(hc: HolochainStewardRevenueSummary): StewardRevenueSummary {
    return {
      stewardPresenceId: hc.stewardPresenceId,
      totalRevenue: hc.totalRevenue,
      totalGrants: hc.totalGrants,
      revenueByGate: hc.revenueByGate.map(g => ({
        gateId: g.gateId,
        gateTitle: g.gateTitle,
        totalRevenue: g.totalRevenue,
        grantCount: g.grantCount,
      })),
    };
  }

  private safeParseJson<T>(json: string | null | undefined, defaultValue: T): T {
    if (!json) return defaultValue;
    try {
      return JSON.parse(json) as T;
    } catch {
      return defaultValue;
    }
  }
}
