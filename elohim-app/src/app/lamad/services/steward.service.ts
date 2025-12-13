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
import { BehaviorSubject, Observable, of, from, defer } from 'rxjs';
import { catchError, shareReplay, tap } from 'rxjs/operators';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import {
  StewardCredential,
  StewardTier,
  PremiumGate,
  PricingModel,
  AccessGrant,
  GrantType,
  StewardRevenue,
  StewardRevenueSummary,
  GateRevenueSummary,
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
  steward_presence_id: string;
  agent_id: string;
  tier: string;
  stewarded_presence_ids_json: string;
  stewarded_content_ids_json: string;
  stewarded_path_ids_json: string;
  mastery_content_ids_json: string;
  mastery_level_achieved: string;
  qualification_verified_at: string;
  peer_attestation_ids_json: string;
  unique_attester_count: number;
  attester_reputation_sum: number;
  stewardship_quality_score: number;
  total_learners_served: number;
  total_content_improvements: number;
  domain_tags_json: string;
  is_active: boolean;
  deactivation_reason: string | null;
  note: string | null;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

interface HolochainStewardCredentialOutput {
  action_hash: Uint8Array;
  credential: HolochainStewardCredential;
}

interface HolochainPremiumGate {
  id: string;
  steward_credential_id: string;
  steward_presence_id: string;
  contributor_presence_id: string | null;
  gated_resource_type: string;
  gated_resource_ids_json: string;
  gate_title: string;
  gate_description: string;
  gate_image: string | null;
  required_attestations_json: string;
  required_mastery_json: string;
  required_vouches_json: string;
  pricing_model: string;
  price_amount: number | null;
  price_unit: string | null;
  subscription_period_days: number | null;
  min_amount: number | null;
  steward_share_percent: number;
  commons_share_percent: number;
  contributor_share_percent: number | null;
  scholarship_eligible: boolean;
  max_scholarships_per_period: number | null;
  scholarship_criteria_json: string | null;
  is_active: boolean;
  deactivation_reason: string | null;
  total_access_grants: number;
  total_revenue_generated: number;
  total_to_steward: number;
  total_to_contributor: number;
  total_to_commons: number;
  total_scholarships_granted: number;
  note: string | null;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

interface HolochainPremiumGateOutput {
  action_hash: Uint8Array;
  gate: HolochainPremiumGate;
}

interface HolochainAccessGrant {
  id: string;
  gate_id: string;
  learner_agent_id: string;
  grant_type: string;
  granted_via: string;
  payment_event_id: string | null;
  payment_amount: number | null;
  payment_unit: string | null;
  scholarship_sponsor_id: string | null;
  scholarship_reason: string | null;
  granted_at: string;
  valid_until: string | null;
  renewal_due_at: string | null;
  is_active: boolean;
  revoked_at: string | null;
  revoke_reason: string | null;
  metadata_json: string;
  created_at: string;
}

interface HolochainAccessGrantOutput {
  action_hash: Uint8Array;
  grant: HolochainAccessGrant;
}

interface HolochainStewardRevenueSummary {
  steward_presence_id: string;
  total_revenue: number;
  total_grants: number;
  revenue_by_gate: HolochainGateRevenueSummary[];
}

interface HolochainGateRevenueSummary {
  gate_id: string;
  gate_title: string;
  total_revenue: number;
  grant_count: number;
}

// =============================================================================
// Service Implementation
// =============================================================================

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
      throw new Error('Steward service not available');
    }

    return defer(() =>
      from(this.doCreateCredential(input))
    ).pipe(
      tap(() => this.refreshMyCredentials()),
      catchError((err) => {
        console.error('[StewardService] Failed to create credential:', err);
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
      const request = defer(() =>
        from(this.fetchCredential(credentialId))
      ).pipe(
        shareReplay(1),
        catchError((err) => {
          console.warn(`[StewardService] Failed to fetch credential "${credentialId}":`, err);
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

    return defer(() =>
      from(this.fetchMyCredentials())
    ).pipe(
      tap(credentials => this.myCredentialsSubject.next(credentials)),
      catchError((err) => {
        console.warn('[StewardService] Failed to fetch my credentials:', err);
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

    return defer(() =>
      from(this.fetchCredentialsForHuman(humanPresenceId))
    ).pipe(
      catchError((err) => {
        console.warn(`[StewardService] Failed to fetch credentials for "${humanPresenceId}":`, err);
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
      throw new Error('Steward service not available');
    }

    return defer(() =>
      from(this.doCreateGate(input))
    ).pipe(
      tap(() => this.refreshMyGates()),
      catchError((err) => {
        console.error('[StewardService] Failed to create gate:', err);
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

    return defer(() =>
      from(this.fetchGate(gateId))
    ).pipe(
      catchError((err) => {
        console.warn(`[StewardService] Failed to fetch gate "${gateId}":`, err);
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
      const request = defer(() =>
        from(this.fetchGatesForResource(resourceId))
      ).pipe(
        shareReplay(1),
        catchError((err) => {
          console.warn(`[StewardService] Failed to fetch gates for "${resourceId}":`, err);
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
      const request = defer(() =>
        from(this.fetchAccessCheck(gateId))
      ).pipe(
        shareReplay(1),
        catchError((err) => {
          console.warn(`[StewardService] Failed to check access for "${gateId}":`, err);
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
      throw new Error('Steward service not available');
    }

    return defer(() =>
      from(this.doGrantAccess(input))
    ).pipe(
      tap(() => {
        // Clear relevant caches
        this.accessCheckCache.delete(input.gateId);
        this.refreshMyAccessGrants();
      }),
      catchError((err) => {
        console.error('[StewardService] Failed to grant access:', err);
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

    return defer(() =>
      from(this.fetchMyAccessGrants())
    ).pipe(
      tap(grants => this.myAccessGrantsSubject.next(grants)),
      catchError((err) => {
        console.warn('[StewardService] Failed to fetch my access grants:', err);
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

    return defer(() =>
      from(this.fetchRevenueSummary(stewardPresenceId))
    ).pipe(
      catchError((err) => {
        console.warn(`[StewardService] Failed to fetch revenue for "${stewardPresenceId}":`, err);
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
    } catch (err) {
      console.warn('[StewardService] Availability test failed:', err);
      this.availableSignal.set(false);
      return false;
    }
  }

  // ===========================================================================
  // Private Methods - Zome Calls
  // ===========================================================================

  private async doCreateCredential(input: CreateStewardCredentialInput): Promise<StewardCredential> {
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
      scholarship_criteria_json: input.scholarshipCriteria ? JSON.stringify(input.scholarshipCriteria) : null,
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

  private async fetchRevenueSummary(stewardPresenceId: string): Promise<StewardRevenueSummary | null> {
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
      stewardPresenceId: hc.steward_presence_id,
      agentId: hc.agent_id,
      tier: hc.tier as StewardTier,
      stewartedPresenceIds: this.safeParseJson<string[]>(hc.stewarded_presence_ids_json, []),
      stewartedContentIds: this.safeParseJson<string[]>(hc.stewarded_content_ids_json, []),
      stewartedPathIds: this.safeParseJson<string[]>(hc.stewarded_path_ids_json, []),
      masteryContentIds: this.safeParseJson<string[]>(hc.mastery_content_ids_json, []),
      masteryLevelAchieved: hc.mastery_level_achieved,
      qualificationVerifiedAt: hc.qualification_verified_at,
      peerAttestationIds: this.safeParseJson<string[]>(hc.peer_attestation_ids_json, []),
      uniqueAttesterCount: hc.unique_attester_count,
      attesterReputationSum: hc.attester_reputation_sum,
      stewardshipQualityScore: hc.stewardship_quality_score,
      totalLearnersServed: hc.total_learners_served,
      totalContentImprovements: hc.total_content_improvements,
      domainTags: this.safeParseJson<string[]>(hc.domain_tags_json, []),
      isActive: hc.is_active,
      deactivationReason: hc.deactivation_reason,
      note: hc.note,
      metadata: this.safeParseJson<Record<string, unknown>>(hc.metadata_json, {}),
      createdAt: hc.created_at,
      updatedAt: hc.updated_at,
    };
  }

  private transformGate(output: HolochainPremiumGateOutput): PremiumGate {
    const hc = output.gate;

    return {
      id: hc.id,
      stewardCredentialId: hc.steward_credential_id,
      stewardPresenceId: hc.steward_presence_id,
      contributorPresenceId: hc.contributor_presence_id,
      gatedResourceType: hc.gated_resource_type,
      gatedResourceIds: this.safeParseJson<string[]>(hc.gated_resource_ids_json, []),
      gateTitle: hc.gate_title,
      gateDescription: hc.gate_description,
      gateImage: hc.gate_image,
      requiredAttestations: this.safeParseJson<RequiredAttestation[]>(hc.required_attestations_json, []),
      requiredMastery: this.safeParseJson<RequiredMastery[]>(hc.required_mastery_json, []),
      requiredVouches: this.safeParseJson<RequiredVouches | null>(hc.required_vouches_json, null),
      pricingModel: hc.pricing_model as PricingModel,
      priceAmount: hc.price_amount,
      priceUnit: hc.price_unit,
      subscriptionPeriodDays: hc.subscription_period_days,
      minAmount: hc.min_amount,
      stewardSharePercent: hc.steward_share_percent,
      commonsSharePercent: hc.commons_share_percent,
      contributorSharePercent: hc.contributor_share_percent,
      scholarshipEligible: hc.scholarship_eligible,
      maxScholarshipsPerPeriod: hc.max_scholarships_per_period,
      scholarshipCriteria: this.safeParseJson<Record<string, unknown> | null>(hc.scholarship_criteria_json, null),
      isActive: hc.is_active,
      deactivationReason: hc.deactivation_reason,
      totalAccessGrants: hc.total_access_grants,
      totalRevenueGenerated: hc.total_revenue_generated,
      totalToSteward: hc.total_to_steward,
      totalToContributor: hc.total_to_contributor,
      totalToCommons: hc.total_to_commons,
      totalScholarshipsGranted: hc.total_scholarships_granted,
      note: hc.note,
      metadata: this.safeParseJson<Record<string, unknown>>(hc.metadata_json, {}),
      createdAt: hc.created_at,
      updatedAt: hc.updated_at,
    };
  }

  private transformAccessGrant(output: HolochainAccessGrantOutput): AccessGrant {
    const hc = output.grant;

    return {
      id: hc.id,
      gateId: hc.gate_id,
      learnerAgentId: hc.learner_agent_id,
      grantType: hc.grant_type as GrantType,
      grantedVia: hc.granted_via,
      paymentEventId: hc.payment_event_id,
      paymentAmount: hc.payment_amount,
      paymentUnit: hc.payment_unit,
      scholarshipSponsorId: hc.scholarship_sponsor_id,
      scholarshipReason: hc.scholarship_reason,
      grantedAt: hc.granted_at,
      validUntil: hc.valid_until,
      renewalDueAt: hc.renewal_due_at,
      isActive: hc.is_active,
      revokedAt: hc.revoked_at,
      revokeReason: hc.revoke_reason,
      metadata: this.safeParseJson<Record<string, unknown>>(hc.metadata_json, {}),
      createdAt: hc.created_at,
    };
  }

  private transformRevenueSummary(hc: HolochainStewardRevenueSummary): StewardRevenueSummary {
    return {
      stewardPresenceId: hc.steward_presence_id,
      totalRevenue: hc.total_revenue,
      totalGrants: hc.total_grants,
      revenueByGate: hc.revenue_by_gate.map(g => ({
        gateId: g.gate_id,
        gateTitle: g.gate_title,
        totalRevenue: g.total_revenue,
        grantCount: g.grant_count,
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
