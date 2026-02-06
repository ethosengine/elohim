/**
 * Insurance Mutual Service - Phase 1 Implementation
 *
 * Service layer for Elohim Mutual operations.
 * Handles:
 * - Member enrollment and risk assessment
 * - Coverage policy management
 * - Claims processing and adjudication
 * - Reserve and premium management
 *
 * All operations create immutable EconomicEvent entries
 * for full audit trail and transparency.
 *
 * Integration: Works with EconomicService for event creation,
 * relies on Observer protocol for risk assessment data.
 */

/* eslint-disable @typescript-eslint/require-await -- Phase 1: most methods are TODO stubs returning Promise<T> without async work */

import { Injectable } from '@angular/core';

// @coverage: 33.6% (2026-02-05)

import { firstValueFrom } from 'rxjs';

import { EconomicEvent } from '@app/elohim/models/economic-event.model';
import {
  CommonsPool,
  AttributionClaim,
  Measure,
  REAAgent,
} from '@app/elohim/models/rea-bridge.model';
import {
  MemberRiskProfile,
  CoveragePolicy,
  CoveredRisk,
  InsuranceClaim,
  InsuranceClaimStatus,
  AdjustmentReasoning,
} from '@app/shefa/models/insurance-mutual.model';

import { EconomicService } from './economic.service';

/**
 * Reasons for flagging a claim for governance review.
 * Used when adjuster decisions need community oversight.
 */
type GovernanceReviewReason =
  | 'large-claim'
  | 'unusual-interpretation'
  | 'pattern-concern'
  | 'other';

@Injectable({
  providedIn: 'root',
})
export class InsuranceMutualService {
  constructor(private readonly economicService: EconomicService) {}

  // ============================================================================
  // MEMBER ENROLLMENT & RISK ASSESSMENT
  // ============================================================================

  /**
   * Enroll a member in the mutual.
   *
   * Creates:
   * - MemberRiskProfile (initial assessment)
   * - CoveragePolicy (starting coverage)
   * - EconomicEvent (enrollment recorded)
   */
  async enrollMember(
    memberId: string,
    qahalId: string,
    initialRiskFactors: Partial<MemberRiskProfile>
  ): Promise<{
    riskProfile: MemberRiskProfile;
    policy: CoveragePolicy;
    enrollmentEvent: EconomicEvent;
  }> {
    // Step 1: Validate member exists and has profile
    const member = await this.getMember(memberId);
    if (!member) {
      throw new Error(`Member ${memberId} not found`);
    }

    // Step 2: Get Qahal's coverage template (governance-decided defaults)
    const qahalCoverage = await this.getQahalCoverageTemplate(qahalId);
    if (!qahalCoverage) {
      throw new Error(`Qahal ${qahalId} has no coverage template`);
    }

    // Step 3: Create initial risk profile
    const riskProfile: MemberRiskProfile = {
      id: generateId(),
      memberId,
      riskType: 'health', // Default; can be specialized per risk type
      careMaintenanceScore: initialRiskFactors.careMaintenanceScore ?? 50,
      communityConnectednessScore: initialRiskFactors.communityConnectednessScore ?? 50,
      historicalClaimsRate: initialRiskFactors.historicalClaimsRate ?? 0,
      riskScore: calculateRiskScore({
        careMaintenanceScore: initialRiskFactors.careMaintenanceScore ?? 50,
        communityConnectednessScore: initialRiskFactors.communityConnectednessScore ?? 50,
        historicalClaimsRate: initialRiskFactors.historicalClaimsRate ?? 0,
      }),
      riskTier: determineRiskTier(
        calculateRiskScore({
          careMaintenanceScore: initialRiskFactors.careMaintenanceScore ?? 50,
          communityConnectednessScore: initialRiskFactors.communityConnectednessScore ?? 50,
          historicalClaimsRate: initialRiskFactors.historicalClaimsRate ?? 0,
        })
      ),
      riskTierRationale: `Initial assessment based on provided factors. Care score: ${
        initialRiskFactors.careMaintenanceScore ?? 50
      }, Connectedness: ${
        initialRiskFactors.communityConnectednessScore ?? 50
      }, Claims rate: ${initialRiskFactors.historicalClaimsRate ?? 0}`,
      evidenceEventIds: initialRiskFactors.evidenceEventIds ?? [],
      evidenceBreakdown: {
        careMaintenanceEventsCount: 0,
        communityConnectednessEventsCount: 0,
        claimsHistoryEventsCount: 0,
      },
      riskTrendDirection: 'stable',
      lastRiskScore: calculateRiskScore({
        careMaintenanceScore: initialRiskFactors.careMaintenanceScore ?? 50,
        communityConnectednessScore: initialRiskFactors.communityConnectednessScore ?? 50,
        historicalClaimsRate: initialRiskFactors.historicalClaimsRate ?? 0,
      }),
      assessedAt: new Date().toISOString(),
      lastAssessmentAt: new Date().toISOString(),
      nextAssessmentDue: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString(),
      assessmentEventIds: [],
      metadata: { enrollmentInitiation: true },
    };

    // Step 4: Create coverage policy at Qahal governance level
    // Note: 'faith_community' is the closest GovernanceLayer match for Qahal concept
    const policy: CoveragePolicy = {
      id: generateId(),
      memberId,
      coverageLevel: 'community', // Qahal-decided pooling
      governedAt: 'faith_community',
      coveredRisks: qahalCoverage.defaultRisks ?? getDefaultCoveredRisks(),
      deductible: qahalCoverage.deductible,
      coinsurance: qahalCoverage.coinsurance,
      outOfPocketMaximum: qahalCoverage.outOfPocketMaximum,
      effectiveFrom: new Date().toISOString(),
      renewalTerms: 'annual',
      renewalDueAt: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString(),
      constitutionalBasis: qahalCoverage.constitutionalBasis ?? 'community-health-coverage.md',
      lastPremiumEventId: undefined, // No premium yet
      createdAt: new Date().toISOString(),
      lastModifiedAt: new Date().toISOString(),
      modificationEventIds: [],
      metadata: {
        enrollment: true,
        qahalId,
        initialRiskTier: riskProfile.riskTier,
      },
    };

    // Step 5: Create immutable enrollment event
    const enrollmentEvent = await firstValueFrom(
      this.economicService.createEvent({
        action: 'deliver-service',
        providerId: memberId,
        receiverId: 'elohim-mutual',
        resourceClassifiedAs: ['stewardship', 'membership'],
        note: `Member ${memberId} enrolled in ${qahalId} mutual. Risk tier: ${
          riskProfile.riskTier
        }. Coverage level: community.`,
        lamadEventType: 'coverage-decision',
      })
    );

    // Step 6: Link policy and risk profile to event
    riskProfile.assessmentEventIds.push(enrollmentEvent.id);
    policy.modificationEventIds.push(enrollmentEvent.id);

    // Step 7: Persist to storage (in production, Holochain DHT)
    await this.persistRiskProfile(riskProfile);
    await this.persistCoveragePolicy(policy);

    return {
      riskProfile,
      policy,
      enrollmentEvent,
    };
  }

  // ─────────────────────────────────────────────────────────────────
  // Helper methods for enrollment
  // ─────────────────────────────────────────────────────────────────

  private async getMember(memberId: string): Promise<REAAgent | null> {
    // TODO: Fetch from Holochain / service layer
    // For now, return mock
    return { id: memberId, name: 'Member', type: 'human' } as REAAgent;
  }

  private async getQahalCoverageTemplate(_qahalId: string): Promise<{
    defaultRisks: CoveredRisk[];
    deductible: Measure;
    coinsurance: number;
    outOfPocketMaximum: Measure;
    constitutionalBasis: string;
  }> {
    // TODO: Fetch Qahal's coverage constitution from governance layer
    // For now, return defaults
    return {
      defaultRisks: getDefaultCoveredRisks(),
      deductible: { hasNumericalValue: 500, hasUnit: 'unit-token' },
      coinsurance: 20,
      outOfPocketMaximum: { hasNumericalValue: 5000, hasUnit: 'unit-token' },
      constitutionalBasis: 'community-health-coverage-2024.md',
    };
  }

  private async persistRiskProfile(_profile: MemberRiskProfile): Promise<void> {
    // TODO: Persist to Holochain DHT
    // For now, just log
  }

  private async persistCoveragePolicy(_policy: CoveragePolicy): Promise<void> {
    // TODO: Persist to Holochain DHT
  }

  private async getMemberRiskProfile(_memberId: string): Promise<MemberRiskProfile | null> {
    // TODO: Fetch from Holochain / service layer
    // For now, return null (will be fetched in real implementation)
    return null;
  }

  private async persistClaim(_claim: InsuranceClaim): Promise<void> {
    // TODO: Persist to Holochain DHT
  }

  private async persistAdjustmentReasoning(_reasoning: AdjustmentReasoning): Promise<void> {
    // TODO: Persist to Holochain DHT
  }

  private async persistAttributionClaim(_attribution: AttributionClaim): Promise<void> {
    // TODO: Persist to Holochain DHT and update CommonsPool
  }

  // ============================================================================
  // RISK ASSESSMENT
  // ============================================================================

  /**
   * Assess member's risk based on Observer attestation data.
   *
   * The core of the information asymmetry flip:
   * Use actual behavioral observation instead of proxy data.
   *
   * Returns updated MemberRiskProfile with new risk score.
   */
  async assessMemberRisk(
    memberId: string,
    _riskType: 'health' | 'property' | 'casualty' | 'care'
  ): Promise<MemberRiskProfile> {
    // Step 1: Get current risk profile
    const currentProfile = await this.getMemberRiskProfile(memberId);
    if (!currentProfile) {
      throw new Error(`Risk profile not found for member ${memberId}`);
    }

    // Step 2: Query Observer attestations for this member
    // Get all events where member is the provider, then filter by lamadEventType
    const allMemberEvents = await firstValueFrom(
      this.economicService.getEventsForAgent(memberId, 'provider')
    );
    const relevantEventTypes = new Set([
      'preventive-care-completed',
      'community-support-provided',
      'claim-filed',
      'risk-reduction-verified',
    ]);
    const memberEvents = allMemberEvents.filter(e => {
      const eventType = e.metadata?.['lamadEventType'];
      return typeof eventType === 'string' && relevantEventTypes.has(eventType);
    });

    // Step 3: Extract care maintenance score from preventive events
    const careEvents = memberEvents.filter(e => {
      const eventType = e.metadata?.['lamadEventType'];
      return eventType === 'preventive-care-completed' || eventType === 'risk-reduction-verified';
    });
    const careMaintenanceScore = calculateCareMaintenanceScore(careEvents);
    const careEventCount = careEvents.length;

    // Step 4: Extract community connectedness score from support network events
    const supportEvents = memberEvents.filter(e => {
      const eventType = e.metadata?.['lamadEventType'];
      return eventType === 'community-support-provided';
    });
    const communityConnectednessScore = calculateCommunityConnectednessScore(supportEvents);
    const communityEventCount = supportEvents.length;

    // Step 5: Extract claims history
    const claimEvents = memberEvents.filter(e => {
      const eventType = e.metadata?.['lamadEventType'];
      return eventType === 'claim-filed';
    });
    const historicalClaimsRate = calculateHistoricalClaimsRate(
      claimEvents,
      currentProfile.historicalClaimsRate
    );
    const claimsEventCount = claimEvents.length;

    // Step 6: Calculate new risk score
    const newRiskScore = calculateRiskScore({
      careMaintenanceScore,
      communityConnectednessScore,
      historicalClaimsRate,
    });

    // Step 7: Determine new risk tier
    const newRiskTier = determineRiskTier(newRiskScore);

    // Step 8: Identify trend (improving | stable | declining)
    const riskTrend = determineRiskTrend(currentProfile.riskScore, newRiskScore);

    // Step 9: Create updated profile
    const updatedProfile: MemberRiskProfile = {
      ...currentProfile,
      careMaintenanceScore,
      communityConnectednessScore,
      historicalClaimsRate,
      riskScore: newRiskScore,
      riskTier: newRiskTier,
      riskTierRationale: `Updated assessment from behavioral observation. Care score: ${careMaintenanceScore} (${careEventCount} attestations), Connectedness: ${communityConnectednessScore} (${communityEventCount} events), Claims rate: ${historicalClaimsRate.toFixed(
        2
      )} (${claimsEventCount} claims). Trend: ${riskTrend}`,
      lastRiskScore: currentProfile.riskScore,
      riskTrendDirection: riskTrend,
      lastAssessmentAt: currentProfile.assessedAt,
      assessedAt: new Date().toISOString(),
      nextAssessmentDue: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString(),
      evidenceEventIds: [...currentProfile.evidenceEventIds, ...memberEvents.map(e => e.id)],
      evidenceBreakdown: {
        careMaintenanceEventsCount:
          currentProfile.evidenceBreakdown.careMaintenanceEventsCount + careEventCount,
        communityConnectednessEventsCount:
          currentProfile.evidenceBreakdown.communityConnectednessEventsCount + communityEventCount,
        claimsHistoryEventsCount:
          currentProfile.evidenceBreakdown.claimsHistoryEventsCount + claimsEventCount,
      },
      assessmentEventIds: currentProfile.assessmentEventIds,
    };

    // Step 10: Create assessment event
    const assessmentEvent = await firstValueFrom(
      this.economicService.createEvent({
        action: 'raise',
        providerId: 'elohim-mutual',
        receiverId: memberId,
        resourceClassifiedAs: ['risk-assessment', 'behavioral-observation'],
        note: `Risk assessment for ${_riskType}. New score: ${newRiskScore} (tier: ${newRiskTier}). Trend: ${riskTrend}. Evidence: ${careEventCount} care events, ${communityEventCount} community events, ${claimsEventCount} claims.`,
        lamadEventType: 'preventive-care-completed',
      })
    );

    // Step 11: Link event to profile
    updatedProfile.assessmentEventIds.push(assessmentEvent.id);

    // Step 12: Persist updated profile
    await this.persistRiskProfile(updatedProfile);

    return updatedProfile;
  }

  /**
   * Batch assessment for annual review.
   * Updates risk profiles for all members in a Qahal.
   */
  async assessQahalRisks(_qahalId: string): Promise<MemberRiskProfile[]> {
    // TODO: Implementation
    // Get all members of Qahal, reassess each
    // Identify improving/declining members for governance review
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // COVERAGE POLICY MANAGEMENT
  // ============================================================================

  /**
   * Create or update a member's coverage policy.
   *
   * Policies are created at the governance layer that decides them:
   * - Individual: member's own deductible/coinsurance choices
   * - Household: family-level pooling decisions
   * - Community (Qahal): local community pool
   * - Network: rare, for catastrophic/constitutional coverage
   * - Constitutional: cannot be opted out of
   */
  async updateCoveragePolicy(
    _memberId: string,
    _newPolicy: Partial<CoveragePolicy>,
    _governanceLevel: 'individual' | 'household' | 'community' | 'network' | 'constitutional'
  ): Promise<CoveragePolicy> {
    // TODO: Implementation
    // 1. Validate coverage level is appropriate for member
    // 2. Validate cost-sharing terms (deductible, coinsurance, OOP max)
    // 3. Check against constitutional constraints (dignity floor, etc)
    // 4. Record policy creation/modification as EconomicEvent
    // 5. Return updated policy with event reference
    throw new Error('Not yet implemented');
  }

  /**
   * Add a new covered risk to member's policy.
   * Example: member adds dental coverage, mental health coverage
   */
  async addCoveredRisk(_policyId: string, _risk: CoveredRisk): Promise<CoveragePolicy> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get member's current coverage policy.
   */
  async getCoveragePolicy(_memberId: string): Promise<CoveragePolicy | null> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // CLAIMS PROCESSING
  // ============================================================================

  /**
   * File a claim.
   *
   * Creates:
   * - InsuranceClaim (claim record)
   * - EconomicEvent (claim-filed, work event)
   * - Triggers adjuster assignment workflow
   */
  async fileClaim(
    memberId: string,
    policyId: string,
    lossDetails: {
      lossType: string;
      lossDate: string;
      description: string;
      estimatedLossAmount: Measure;
      observerAttestationIds: string[]; // Evidence from Observer
      memberDocumentIds?: string[]; // Supporting docs uploaded by member
    }
  ): Promise<{
    claim: InsuranceClaim;
    filedEvent: EconomicEvent;
  }> {
    // Step 1: Validate policy exists and is active
    const policy = await this.getCoveragePolicy(memberId);
    if (!policy) {
      throw new Error(`Coverage policy ${policyId} not found for member ${memberId}`);
    }

    const policyEffectiveDate = new Date(policy.effectiveFrom);
    const lossDate = new Date(lossDetails.lossDate);

    // Step 2: Check if member was covered on loss date
    if (lossDate < policyEffectiveDate) {
      throw new Error(
        `Loss date ${lossDetails.lossDate} is before policy effective date ${policy.effectiveFrom}`
      );
    }

    // Step 3: Validate covered risk exists on policy
    const coveredRisk = policy.coveredRisks?.find(
      r => r.riskType.toLowerCase() === lossDetails.lossType.toLowerCase()
    );
    if (!coveredRisk?.isCovered) {
      throw new Error(`Risk type "${lossDetails.lossType}" is not covered under this policy`);
    }

    // Step 4: Create InsuranceClaim record
    const now = new Date().toISOString();
    const claim: InsuranceClaim = {
      id: generateId(),
      claimNumber: `CLM-${Date.now().toString().slice(-10)}`,
      policyId,
      memberId,
      coveredRiskId: coveredRisk.id,
      lossType: lossDetails.lossType as InsuranceClaim['lossType'],
      lossDate: lossDetails.lossDate,
      reportedDate: now,
      description: lossDetails.description,
      memberEstimatedLossAmount: lossDetails.estimatedLossAmount,
      status: 'reported',
      statusReason: 'Claim filed by member',
      observerAttestationIds: lossDetails.observerAttestationIds,
      memberDocumentIds: lossDetails.memberDocumentIds ?? [],
      eventIds: {
        filedEventId: '', // Will be updated after event creation
      },
      createdAt: now,
      lastUpdatedAt: now,
      metadata: {
        coveredRiskId: coveredRisk.id,
      },
    };

    // Step 5: Create 'claim-filed' EconomicEvent
    const claimFiledEvent = await firstValueFrom(
      this.economicService.createEvent({
        action: 'work',
        providerId: memberId,
        receiverId: 'elohim-mutual',
        resourceConformsTo: lossDetails.lossType,
        resourceQuantityValue: lossDetails.estimatedLossAmount.hasNumericalValue,
        resourceQuantityUnit: lossDetails.estimatedLossAmount.hasUnit,
        note: `Member filed claim for ${lossDetails.lossType}. Loss date: ${
          lossDetails.lossDate
        }. Description: ${lossDetails.description}. Observer attestations: ${
          lossDetails.observerAttestationIds.length
        }`,
        lamadEventType: 'claim-filed',
      })
    );

    // Step 6: Link event to claim
    claim.eventIds.filedEventId = claimFiledEvent.id;

    // Step 7: Persist claim (in production, to Holochain DHT)
    await this.persistClaim(claim);

    // Step 8: Trigger adjuster assignment workflow
    // TODO: This would normally trigger a queue/assignment system
    // For now, just log that it needs assignment

    return {
      claim,
      filedEvent: claimFiledEvent,
    };
  }

  /**
   * Submit supporting documentation for a claim.
   *
   * Creates:
   * - Updated InsuranceClaim
   * - EconomicEvent (claim-evidence-submitted)
   */
  async submitClaimEvidence(
    _claimId: string,
    _evidenceDocumentIds: string[]
  ): Promise<InsuranceClaim> {
    // TODO: Implementation
    // 1. Add docs to claim
    // 2. Create event recording submission
    // 3. Notify assigned adjuster
    throw new Error('Not yet implemented');
  }

  /**
   * Get claim status and history.
   */
  async getClaim(_claimId: string): Promise<InsuranceClaim | null> {
    // TODO: In production, fetch from Holochain DHT by claim ID
    // For now, return null (will be populated by real implementation)

    return null;
  }

  /**
   * Get all claims for a member.
   */
  async getMemberClaims(
    _memberId: string,
    _filters?: {
      status?: InsuranceClaimStatus;
      fromDate?: string;
      toDate?: string;
    }
  ): Promise<InsuranceClaim[]> {
    // TODO: In production, query Holochain DHT for all claims where memberId matches
    // Apply filters if provided

    return [];
  }

  /**
   * Search members by criteria.
   *
   * Enables discovery for:
   * - Governance: finding members in specific risk tiers
   * - Adjusters: finding members by policy
   * - Analytics: finding cohorts for trends
   */
  async searchMembers(_filters: {
    qahalId?: string;
    riskTier?: 'low' | 'standard' | 'high' | 'uninsurable';
    riskTrendDirection?: 'improving' | 'stable' | 'declining';
    careMaintenanceScoreMin?: number;
    communityConnectednessScoreMin?: number;
    limit?: number;
    offset?: number;
  }): Promise<
    {
      riskProfile: MemberRiskProfile;
      policy: CoveragePolicy;
    }[]
  > {
    // TODO: In production, query Holochain DHT with multiple filters
    // Index on: qahalId, riskTier, riskTrendDirection
    // Support aggregation for analytics

    return [];
  }

  /**
   * Search claims by criteria.
   *
   * Enables discovery for:
   * - Adjusters: finding assigned claims
   * - Governance: finding claims to review
   * - Analytics: finding trends
   */
  async searchClaims(_filters: {
    qahalId?: string;
    status?: InsuranceClaimStatus;
    lossType?: string;
    amountMin?: number;
    amountMax?: number;
    fromDate?: string;
    toDate?: string;
    flaggedForGovernance?: boolean;
    limit?: number;
    offset?: number;
  }): Promise<
    {
      claim: InsuranceClaim;
      adjustment?: AdjustmentReasoning;
    }[]
  > {
    // TODO: In production, query Holochain DHT with multiple criteria
    // Index on: qahalId, status, lossType, flags
    // Support date range queries

    return [];
  }

  /**
   * Get all members in a Qahal.
   * For governance reports and community oversight.
   */
  async getQahalMembers(
    _qahalId: string,
    _options?: {
      limit?: number;
      offset?: number;
    }
  ): Promise<
    {
      member: REAAgent;
      riskProfile: MemberRiskProfile;
      policy: CoveragePolicy;
    }[]
  > {
    // TODO: In production, query DHT for all members in qahalId
    // With pagination support

    return [];
  }

  /**
   * Get all claims filed in a Qahal.
   * For governance oversight and reserves analysis.
   */
  async getQahalClaims(
    _qahalId: string,
    _filters?: {
      status?: InsuranceClaimStatus;
      fromDate?: string;
      toDate?: string;
    },
    _options?: {
      limit?: number;
      offset?: number;
    }
  ): Promise<InsuranceClaim[]> {
    // TODO: In production, query DHT for claims in qahalId
    // With filters and pagination

    return [];
  }

  /**
   * Get members by risk tier.
   * For targeted prevention programs and rate analysis.
   */
  async getMembersByRiskTier(
    _qahalId: string,
    _riskTier: 'low' | 'standard' | 'high' | 'uninsurable'
  ): Promise<MemberRiskProfile[]> {
    // TODO: In production, query DHT with index on (qahalId, riskTier)

    return [];
  }

  /**
   * Get members with improving risk trends.
   * For governance recognition and prevention incentive rewards.
   */
  async getMembersWithImprovingRisk(_qahalId: string): Promise<
    {
      member: MemberRiskProfile;
      previousScore: number;
      currentScore: number;
      improvement: number;
    }[]
  > {
    // TODO: In production, query DHT for members with trend='improving'

    return [];
  }

  /**
   * Get claims pending adjudication.
   * For adjuster queue management.
   */
  async getPendingClaims(_qahalId?: string): Promise<InsuranceClaim[]> {
    // TODO: In production, query DHT for claims with status='filed'

    return [];
  }

  /**
   * Get high-value claims (above threshold).
   * For governance review of large payouts.
   */
  async getHighValueClaims(_threshold: Measure, _qahalId?: string): Promise<InsuranceClaim[]> {
    // TODO: In production, query DHT for claims with amount > threshold
    return [];
  }

  /**
   * Get claims denied by adjuster.
   * For appeals processing and adjuster performance review.
   */
  async getDeniedClaims(
    _adjusterId?: string,
    _qahalId?: string
  ): Promise<
    {
      claim: InsuranceClaim;
      reasoning: AdjustmentReasoning;
    }[]
  > {
    // TODO: In production, query DHT for claims with adjustment status='denied'
    return [];
  }

  // ============================================================================
  // CLAIMS ADJUDICATION (Adjuster Operations)
  // ============================================================================

  /**
   * Assign a claim to an adjuster.
   *
   * Links Elohim agent (adjuster) to claim processing workflow.
   * Creates EconomicEvent recording assignment.
   */
  async assignClaimToAdjuster(_claimId: string, _adjusterId: string): Promise<void> {
    // TODO: Implementation
    // 1. Verify adjuster is qualified (check tier, certification)
    // 2. Update claim with adjuster ID
    // 3. Create 'claim-assigned' event (or use 'stewardship-begin'?)
    // 4. Notify adjuster of assignment
    throw new Error('Not yet implemented');
  }

  /**
   * Submit adjuster's determination on a claim.
   *
   * Core of the Bob Parr principle:
   * "Explain every decision in plain language"
   * "Can be audited by governance"
   * "Interpret generously within terms"
   *
   * Creates:
   * - AdjustmentReasoning (full constitutional reasoning)
   * - Updated InsuranceClaim
   * - EconomicEvent (claim-adjusted)
   */
  async adjustClaim(
    claimId: string,
    _adjusterId: string,
    reasoning: Omit<
      AdjustmentReasoning,
      'id' | 'claimId' | 'adjusterId' | 'adjustmentDate' | 'createdAt'
    >
  ): Promise<{
    claim: InsuranceClaim;
    reasoning: AdjustmentReasoning;
    adjustedEvent: EconomicEvent;
  }> {
    // Step 1: Get claim
    const claim = await this.getClaim(claimId);
    if (!claim) {
      throw new Error(`Claim ${claimId} not found`);
    }

    // Step 2: Validate reasoning has constitutional basis
    if (!reasoning.constitutionalCitation) {
      throw new Error(
        'Adjuster decision must cite constitutional basis (coverage policy, Qahal governance document, etc)'
      );
    }

    if (!reasoning.plainLanguageExplanation) {
      throw new Error('Adjuster decision must include plain language explanation');
    }

    // Step 3: Check if generosity principle applies
    // Generosity principle: if coverage is ambiguous or could be interpreted either way,
    // interpret generously in member's favor
    const generosityApplied =
      reasoning.determinations.coverageApplies && reasoning.generosityInterpretationApplied;

    // Step 4: Create AdjustmentReasoning record
    const adjustmentReasoning: AdjustmentReasoning = {
      id: generateId(),
      claimId,
      adjusterId: _adjusterId,
      adjustmentDate: new Date().toISOString(),
      createdAt: new Date().toISOString(),
      ...reasoning,
      generosityInterpretationApplied: generosityApplied,
    };

    // Step 5: Determine if claim should be flagged for governance review
    let flagForGovernance = false;
    let flagReason = '';

    // Flag if: large approval (>80% of coverage limit)
    const finalApprovedAmount = adjustmentReasoning.determinations?.finalApprovedAmount;
    if (
      reasoning.determinations.coverageApplies &&
      claim.metadata?.['coverageLimit'] &&
      finalApprovedAmount &&
      typeof claim.metadata['coverageLimit'] === 'object' &&
      'hasNumericalValue' in claim.metadata['coverageLimit'] &&
      typeof claim.metadata['coverageLimit'].hasNumericalValue === 'number' &&
      finalApprovedAmount.hasNumericalValue >
        claim.metadata['coverageLimit'].hasNumericalValue * 0.8
    ) {
      flagForGovernance = true;
      flagReason = 'large-claim';
    }

    // Flag if: generosity principle applied
    if (generosityApplied) {
      flagForGovernance = true;
      flagReason = 'unusual-interpretation';
    }

    // Flag if: denial (always auditable)
    if (!reasoning.determinations.coverageApplies) {
      flagForGovernance = true;
      flagReason = 'claim-denial';
    }

    // Step 6: Update claim status and history
    claim.status = 'adjustment-made';
    claim.adjustmentReasoning = adjustmentReasoning;
    claim.approvedAmount = finalApprovedAmount;
    claim.lastUpdatedAt = new Date().toISOString();

    // Step 7: Create 'claim-adjusted' EconomicEvent
    const adjustedEvent = await firstValueFrom(
      this.economicService.createEvent({
        action: 'modify',
        providerId: _adjusterId,
        receiverId: claim.memberId,
        note: `Claim adjusted. ${reasoning.plainLanguageExplanation}`,
        lamadEventType: 'claim-adjusted',
      })
    );

    // Update event reference in claim
    claim.eventIds.adjustedEventId = adjustedEvent.id;

    // Step 8: Persist updated claim and reasoning
    await this.persistClaim(claim);
    await this.persistAdjustmentReasoning(adjustmentReasoning);

    // Step 9: Flag for governance if needed
    if (flagForGovernance) {
      await this.flagClaimForGovernanceReview(
        claimId,
        flagReason as GovernanceReviewReason,
        `Adjuster decision. Requires governance review.`
      );
    }

    return {
      claim,
      reasoning: adjustmentReasoning,
      adjustedEvent,
    };
  }

  /**
   * Approve claim for settlement.
   *
   * Adjuster approved → member will be paid
   * This is a state transition event.
   */
  async approveClaim(
    _claimId: string,
    _adjusterId: string,
    _note?: string
  ): Promise<InsuranceClaim> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Deny a claim.
   *
   * With full reasoning (why coverage doesn't apply).
   * Creates AttributionClaim for appeal process if denied.
   */
  async denyClaim(
    _claimId: string,
    _adjusterId: string,
    _denialReasoning: string
  ): Promise<InsuranceClaim> {
    // TODO: Implementation
    // 1. Create AdjustmentReasoning with denial explanation
    // 2. Update claim status to 'denied'
    // 3. Create 'claim-denied' EconomicEvent
    // 4. Create appeal window (e.g., 30 days) for member
    throw new Error('Not yet implemented');
  }

  /**
   * Settle a claim (pay member).
   *
   * Final step of claims process.
   * Creates:
   * - AttributionClaim against CommonsPool (member receives settlement)
   * - EconomicEvent (claim-settled, transfer of currency)
   * - CommonsPool deduction
   */
  async settleClaim(
    claimId: string,
    settledAmount: Measure,
    _paymentMethod: 'mutual-credit' | 'fiat-transfer'
  ): Promise<{
    claim: InsuranceClaim;
    settlementEvent: EconomicEvent;
    attribution: AttributionClaim;
  }> {
    // Step 1: Get claim and policy details
    const claim = await this.getClaim(claimId);
    if (!claim) {
      throw new Error(`Claim ${claimId} not found`);
    }

    const policy = await this.getCoveragePolicy(claim.memberId);
    if (!policy) {
      throw new Error(`Coverage policy not found for member ${claim.memberId}`);
    }

    // Step 2: Calculate cost sharing (deductible + coinsurance)
    const deductibleAmount = policy.deductible?.hasNumericalValue ?? 0;
    const coinsurancePercent = policy.coinsurance ?? 0;

    const amountAfterDeductible = Math.max(0, settledAmount.hasNumericalValue - deductibleAmount);
    const coinsuranceAmount = Math.round(amountAfterDeductible * (coinsurancePercent / 100));
    const netPaymentToMember = amountAfterDeductible - coinsuranceAmount;

    // Step 3: Create 'claim-settled' EconomicEvent (transfer)
    const settlementEvent = await firstValueFrom(
      this.economicService.createEvent({
        action: 'transfer',
        providerId: 'elohim-mutual',
        receiverId: claim.memberId,
        resourceConformsTo: 'settlement-payment',
        resourceQuantityValue: netPaymentToMember,
        resourceQuantityUnit: settledAmount.hasUnit ?? 'unit-token',
        note: `Claim settlement. Gross: ${settledAmount.hasNumericalValue}. Deductible: -${deductibleAmount}. Coinsurance: -${coinsuranceAmount}. Net to member: ${netPaymentToMember}`,
        lamadEventType: 'claim-settled',
      })
    );

    // Step 4: Create AttributionClaim for member against CommonsPool
    // This records the member's claim on the pool's reserves
    // Note: We would need to create a ValueAttribution first, then claim against it
    // For now, create a placeholder attribution claim
    const attribution: AttributionClaim = {
      id: generateId(),
      attributionId: `attr-${claimId}`, // Reference to ValueAttribution (would be created separately)
      claimantId: claim.memberId,
      amount: {
        hasNumericalValue: netPaymentToMember,
        hasUnit: settledAmount.hasUnit ?? 'unit-token',
      },
      requiredAttestationLevel: 'basic',
      identityVerified: true, // Member already verified through policy enrollment
      responsibilityVerified: true,
      state: 'approved',
      submittedAt: new Date().toISOString(),
      processedAt: new Date().toISOString(),
    };

    // Step 5: Update claim status
    claim.status = 'settled';
    // Note: statusHistory and settlementEventIds not on InsuranceClaim model
    // claim.statusHistory.push({
    //   status: 'settled',
    //   changedAt: new Date().toISOString(),
    //   changedBy: 'elohim-mutual',
    //   note: `Claim settled. Paid: ${netPaymentToMember} via ${_paymentMethod}`,
    // });
    // claim.settlementEventIds.push(settlementEvent.id);

    // Step 6: Persist updated claim
    await this.persistClaim(claim);

    // Step 7: Persist attribution claim
    // TODO: In production, would update CommonsPool balance
    // For now, just create the attribution record
    await this.persistAttributionClaim(attribution);

    return {
      claim,
      settlementEvent,
      attribution,
    };
  }

  // ============================================================================
  // APPEALS & DISPUTE RESOLUTION
  // ============================================================================

  /**
   * Member appeals adjuster's decision.
   *
   * Initiates governance review of adjuster's reasoning.
   * Second adjuster or governance committee reviews.
   */
  async appealClaimDecision(
    _claimId: string,
    _memberId: string,
    _appealReason: string
  ): Promise<InsuranceClaim> {
    // TODO: Implementation
    // 1. Create appeal record on claim
    // 2. Assign to different adjuster or governance committee
    // 3. Create 'claim-appealed' EconomicEvent
    // 4. Set review deadline
    throw new Error('Not yet implemented');
  }

  /**
   * Review and decide on member's appeal.
   * Second-level adjudication.
   */
  async resolveAppeal(
    _claimId: string,
    _reviewingAdjusterId: string,
    _decision: 'upheld' | 'overturned' | 'modified',
    _reasoning: string
  ): Promise<InsuranceClaim> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // PREVENTION & RISK MITIGATION
  // ============================================================================

  /**
   * Record verified risk mitigation activity.
   *
   * Observer protocol attests member completed prevention activity.
   * Triggers premium discount for next period.
   *
   * Creates:
   * - Updated MemberRiskProfile (risk score improves)
   * - EconomicEvent (risk-reduction-verified or prevention-incentive-awarded)
   */
  async recordRiskMitigation(
    _memberId: string,
    _riskType: 'health' | 'property' | 'casualty' | 'care',
    _observerAttestationId: string, // Evidence from Observer
    _mitigationActivity: string // What was done (e.g., "completed-driving-course")
  ): Promise<{
    updatedProfile: MemberRiskProfile;
    incentiveEvent: EconomicEvent;
  }> {
    // TODO: Implementation
    // 1. Verify Observer attestation
    // 2. Check if activity triggers premium discount
    // 3. Update risk profile (score improves, trend updates)
    // 4. Create 'prevention-incentive-awarded' EconomicEvent
    // 5. Calculate premium adjustment for next period
    // 6. Return updated profile and event
    throw new Error('Not yet implemented');
  }

  /**
   * Get prevention incentive opportunities for member.
   *
   * Returns list of activities that would trigger discounts.
   * Personalized to member's risk profile and covered risks.
   */
  async getPreventionIncentives(_memberId: string): Promise<
    {
      activity: string;
      discountPercent: number;
      description: string;
    }[]
  > {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // GOVERNANCE & OVERSIGHT
  // ============================================================================

  /**
   * Flag claim for governance review.
   *
   * Adjuster decisions are auditable.
   * Large claims, unusual interpretations, or patterns
   * are flagged for community review.
   */
  async flagClaimForGovernanceReview(
    _claimId: string,
    _reason: GovernanceReviewReason,
    _note?: string
  ): Promise<void> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get claims flagged for governance review.
   * For governance committees.
   */
  async getFlaggedClaims(_qahalId?: string): Promise<
    {
      claim: InsuranceClaim;
      reasoning: AdjustmentReasoning;
      flagReason: string;
    }[]
  > {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get adjuster performance metrics.
   *
   * Transparency on adjuster decisions:
   * - Claims processed
   * - Denial rate
   * - Appeal rate
   * - Constitutional compliance
   * - "Generosity index" (% of decisions that interpreted coverage generously)
   */
  async getAdjusterMetrics(_adjusterId: string): Promise<{
    claimsProcessed: number;
    denialRate: number;
    appealRate: number;
    constitutionalComplianceScore: number;
    generosityIndex: number;
    qualityTrend: 'improving' | 'stable' | 'declining';
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // RESERVES & ACTUARIAL
  // ============================================================================

  /**
   * Get mutual's reserve status.
   *
   * Transparency on financial health.
   * Regulators require proof of adequate reserves.
   */
  async getReserveStatus(_poolId: string): Promise<{
    pool: CommonsPool;
    balance: Measure;
    expectedAnnualClaims: Measure;
    adequacyRatio: number;
    statutoryMinimum: Measure;
    isAdequate: boolean;
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Analyze claims trends for reserve projections.
   *
   * Loss ratio, frequency, severity trends.
   * Used for: rate setting, reserve adequacy, governance decisions.
   */
  async analyzeClaimsTrends(
    _poolId: string,
    _period: '30-days' | '90-days' | 'annual'
  ): Promise<{
    totalClaims: number;
    totalClaimed: Measure;
    totalPaid: Measure;
    averageClaimSize: Measure;
    denialRate: number;
    trend: 'stable' | 'increasing' | 'decreasing';
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // PREMIUM MANAGEMENT
  // ============================================================================

  /**
   * Calculate member's premium based on risk profile.
   *
   * Uses actual risk data (not proxies):
   * - MemberRiskProfile
   * - Risk mitigations (Observer attestations)
   * - Historical claims
   * - Prevention incentive achievements
   */
  async calculatePremium(
    memberId: string,
    _coverageLevel: 'individual' | 'household' | 'community' | 'network'
  ): Promise<{
    basePremium: Measure;
    riskAdjustment: Measure;
    preventionDiscount: Measure;
    finalPremium: Measure;
    breakdown: string; // Plain language breakdown for member
  }> {
    // Step 1: Get member's risk profile
    const riskProfile = await this.getMemberRiskProfile(memberId);
    if (!riskProfile) {
      throw new Error(`Risk profile not found for member ${memberId}`);
    }

    // Step 2: Get member's coverage policy
    const policy = await this.getCoveragePolicy(memberId);
    if (!policy) {
      throw new Error(`Coverage policy not found for member ${memberId}`);
    }

    // Step 3: Calculate base premium (pool-level expected claims + overhead)
    // Base community premium: 500 units/year (standard risk assumption)
    // This comes from: expected claims for standard-tier member + 15% overhead
    const basePremiumAmount = 500;

    const basePremium: Measure = {
      hasNumericalValue: basePremiumAmount,
      hasUnit: 'unit-token',
    };

    // Step 4: Risk adjustment based on risk tier
    // low tier: -20% (excellent preventive care)
    // standard tier: 0% (baseline)
    // high tier: +30% (poor preventive care or weak support)
    // uninsurable tier: error (shouldn't be enrolled)
    let riskAdjustmentPercent = 0;
    switch (riskProfile.riskTier) {
      case 'low':
        riskAdjustmentPercent = -20;
        break;
      case 'standard':
        riskAdjustmentPercent = 0;
        break;
      case 'high':
        riskAdjustmentPercent = 30;
        break;
      case 'uninsurable':
        throw new Error(
          `Member ${memberId} is uninsurable. Requires risk mitigation before enrollment.`
        );
    }

    const riskAdjustmentAmount = Math.round(basePremiumAmount * (riskAdjustmentPercent / 100));
    const riskAdjustment: Measure = {
      hasNumericalValue: riskAdjustmentAmount,
      hasUnit: 'unit-token',
    };

    // Step 5: Prevention discount
    // 5% discount for each point of care maintenance score above 60
    // 3% discount for each point of community connectedness above 60
    // Maximum discount: 35%
    let preventionDiscountPercent = 0;

    if (riskProfile.careMaintenanceScore > 60) {
      preventionDiscountPercent += (riskProfile.careMaintenanceScore - 60) * 0.05;
    }

    if (riskProfile.communityConnectednessScore > 60) {
      preventionDiscountPercent += (riskProfile.communityConnectednessScore - 60) * 0.03;
    }

    preventionDiscountPercent = Math.min(35, preventionDiscountPercent);

    const premiumBeforeDiscount = basePremiumAmount + riskAdjustmentAmount;
    const preventionDiscountAmount = Math.round(
      premiumBeforeDiscount * (preventionDiscountPercent / 100)
    );
    const preventionDiscount: Measure = {
      hasNumericalValue: -preventionDiscountAmount,
      hasUnit: 'unit-token',
    };

    // Step 6: Calculate final premium
    const finalPremiumAmount = premiumBeforeDiscount - preventionDiscountAmount;
    const finalPremium: Measure = {
      hasNumericalValue: Math.max(100, finalPremiumAmount), // Minimum 100
      hasUnit: 'unit-token',
    };

    // Step 7: Create human-readable breakdown
    const breakdown = `
PREMIUM CALCULATION FOR ${memberId}

Base Premium (Community Expected Claims): ${basePremium.hasNumericalValue} ${basePremium.hasUnit}

Risk Adjustment (Tier: ${riskProfile.riskTier}): ${riskAdjustmentPercent}% = ${riskAdjustment.hasNumericalValue} ${riskAdjustment.hasUnit}
  Your preventive care score: ${riskProfile.careMaintenanceScore}/100
  Your support network score: ${riskProfile.communityConnectednessScore}/100

Subtotal: ${premiumBeforeDiscount} ${basePremium.hasUnit}

Prevention Discount: ${preventionDiscountPercent}% = -${preventionDiscountAmount} ${basePremium.hasUnit}
  (${Math.max(0, riskProfile.careMaintenanceScore - 60)} points from preventive care × 0.05 each)
  (${Math.max(0, riskProfile.communityConnectednessScore - 60)} points from community × 0.03 each)

YOUR ANNUAL PREMIUM: ${finalPremium.hasNumericalValue} ${finalPremium.hasUnit}
Monthly: ${Math.round(finalPremium.hasNumericalValue / 12)} ${finalPremium.hasUnit}

HOW TO LOWER YOUR PREMIUM:
- Increase preventive care (checkups, screenings, vaccinations)
- Build your support network (community events, mutual aid activities)
- Each improvement will reduce your premium the next renewal period
    `.trim();

    return {
      basePremium,
      riskAdjustment,
      preventionDiscount,
      finalPremium,
      breakdown,
    };
  }

  /**
   * Record premium payment.
   *
   * Creates:
   * - EconomicEvent (credit-transfer or premium-payment)
   * - Updates CommonsPool (premium flows to reserves)
   * - Updates member's coverage status
   */
  async recordPremiumPayment(
    _memberId: string,
    _amount: Measure,
    _paymentMethod: 'mutual-credit' | 'fiat-transfer',
    _periodCovered: { from: string; to: string }
  ): Promise<{
    paymentEvent: EconomicEvent;
    updatedPolicy: CoveragePolicy;
  }> {
    // TODO: Implementation
    // 1. Validate payment amount matches policy premium
    // 2. Create 'premium-payment' EconomicEvent
    // 3. Update CommonsPool (add to reserves)
    // 4. Update policy (coverage effective dates, last premium event)
    // 5. Record in deductible tracker (reset if new period)
    throw new Error('Not yet implemented');
  }

  // ============================================================================
  // REPORTING & TRANSPARENCY
  // ============================================================================

  /**
   * Get member's mutual statement.
   *
   * What member sees about their coverage, claims, and pool.
   * Full transparency on how their premiums flow.
   */
  async getMemberStatement(_memberId: string): Promise<{
    member: REAAgent;
    riskProfile: MemberRiskProfile;
    policy: CoveragePolicy;
    claimsThisPeriod: InsuranceClaim[];
    premiumsPaid: Measure;
    estimatedPoolShare: Measure; // How much of reserves are "theirs"
    preventionOpportunities: string[];
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }

  /**
   * Get community (Qahal) mutual analytics.
   *
   * What Qahal governance sees about their risk pool.
   */
  async getQahalAnalytics(_qahalId: string): Promise<{
    memberCount: number;
    totalPremiumsCollected: Measure;
    totalClaimsPaid: Measure;
    lossRatio: number;
    averagePremium: Measure;
    riskTierDistribution: {
      low: number;
      standard: number;
      high: number;
      uninsurable: number;
    };
    topClaimReasons: { reason: string; count: number }[];
  }> {
    // TODO: Implementation
    throw new Error('Not yet implemented');
  }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Generate a unique ID for insurance entities.
 * Uses timestamp + random suffix for pseudo-uniqueness.
 * In production, would use UUID or Holochain hash.
 */
function generateId(): string {
  const timestamp = Date.now().toString(36);
  const random = (crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32)
    .toString(36)
    .substring(2, 9);
  return `id-${timestamp}-${random}`;
}

/**
 * Calculate risk score from behavioral factors.
 *
 * Score: weighted average of three factors, each 0-100
 * - careMaintenanceScore (40% weight): Does member maintain preventive care?
 * - communityConnectednessScore (35% weight): Does member have support network?
 * - historicalClaimsRate (25% weight): Claims frequency (lower is better)
 *
 * Result: 0-100 score where lower = lower risk
 */
function calculateRiskScore(factors: {
  careMaintenanceScore: number;
  communityConnectednessScore: number;
  historicalClaimsRate: number;
}): number {
  const care = Math.min(100, Math.max(0, factors.careMaintenanceScore));
  const connected = Math.min(100, Math.max(0, factors.communityConnectednessScore));
  // Invert claims rate: high claims rate = high risk contribution
  const claimsRisk = Math.min(100, factors.historicalClaimsRate * 100);

  // Weighted average: lower scores = lower risk
  const score = care * 0.4 + connected * 0.35 + (100 - claimsRisk) * 0.25;
  return Math.round(score);
}

/**
 * Determine risk tier based on numerical risk score.
 *
 * Tiers:
 * - 80-100: low (excellent preventive care, strong support network)
 * - 60-79: standard (average preventive maintenance, moderate support)
 * - 40-59: high (poor preventive care or weak support network)
 * - 0-39: uninsurable (too risky; typically requires intervention before coverage)
 */
function determineRiskTier(riskScore: number): 'low' | 'standard' | 'high' | 'uninsurable' {
  if (riskScore >= 80) return 'low';
  if (riskScore >= 60) return 'standard';
  if (riskScore >= 40) return 'high';
  return 'uninsurable';
}

/**
 * Get default covered risks for new member enrollment.
 *
 * These are the risks that a Qahal typically covers at baseline.
 * Can be customized per Qahal via governance.
 */
function getDefaultCoveredRisks(): CoveredRisk[] {
  return [
    {
      id: generateId(),
      riskType: 'emergency-health',
      isCovered: true,
      coverage: {
        limitPerIncident: { hasNumericalValue: 100000, hasUnit: 'unit-token' },
        coveragePercent: 80, // 20% coinsurance
      },
      exclusions: ['Cosmetic procedures', 'Experimental treatments'],
      preventionIncentive: {
        discountPercent: 15,
        requiredAttestations: ['preventive-care-completed'],
      },
      waitingPeriod: '0 days',
      addedAt: new Date().toISOString(),
      metadata: { description: 'Unexpected medical costs from illness, injury, or emergency care' },
    },
    {
      id: generateId(),
      riskType: 'preventive-health',
      isCovered: true,
      coverage: {
        annualLimit: { hasNumericalValue: 10000, hasUnit: 'unit-token' },
        coveragePercent: 90, // 10% coinsurance
      },
      exclusions: [],
      preventionIncentive: {
        discountPercent: 20,
        requiredAttestations: ['preventive-care-completed'],
      },
      waitingPeriod: '0 days',
      addedAt: new Date().toISOString(),
      metadata: {
        description:
          'Ongoing prescription medication costs, annual checkups, screenings, vaccinations',
      },
    },
    {
      id: generateId(),
      riskType: 'mental-health',
      isCovered: true,
      coverage: {
        annualLimit: { hasNumericalValue: 20000, hasUnit: 'unit-token' },
        coveragePercent: 80, // 20% coinsurance
      },
      exclusions: [],
      preventionIncentive: {
        discountPercent: 10,
        requiredAttestations: ['community-support-provided'],
      },
      waitingPeriod: '30 days',
      addedAt: new Date().toISOString(),
      metadata: { description: 'Therapy, counseling, psychiatric care' },
    },
    {
      id: generateId(),
      riskType: 'dental',
      isCovered: true,
      coverage: {
        annualLimit: { hasNumericalValue: 5000, hasUnit: 'unit-token' },
        coveragePercent: 70, // 30% coinsurance
      },
      exclusions: ['Cosmetic dental work', 'Orthodontics for adults'],
      waitingPeriod: '90 days',
      addedAt: new Date().toISOString(),
      metadata: { description: 'Preventive dental care, fillings, extractions' },
    },
    {
      id: generateId(),
      riskType: 'other',
      isCovered: true,
      coverage: {
        limitPerIncident: { hasNumericalValue: 500000, hasUnit: 'unit-token' },
        coveragePercent: 80, // 20% coinsurance
      },
      exclusions: ['Pre-existing conditions in first 12 months'],
      waitingPeriod: '0 days',
      addedAt: new Date().toISOString(),
      metadata: { description: 'Inpatient hospital stays and surgical procedures' },
    },
  ];
}

/**
 * Calculate care maintenance score from Observer attestations.
 *
 * Score: 0-100
 * - 0-20: No preventive care (unattested)
 * - 20-40: Minimal preventive care
 * - 40-60: Moderate preventive care
 * - 60-80: Good preventive care
 * - 80-100: Excellent preventive care with risk mitigation
 *
 * Based on: frequency of preventive activities and risk reduction verifications
 */
function calculateCareMaintenanceScore(careEvents: EconomicEvent[]): number {
  if (careEvents.length === 0) {
    return 20; // No evidence = low score
  }

  // Score based on event frequency
  // Assume ideal is 4+ events per year (quarterly checkups + preventive activities)
  const eventsPerYear = Math.min(100, (careEvents.length / 4) * 25);

  // Check recency (more recent = higher score)
  const now = Date.now();
  const sixMonthsAgo = now - 6 * 30 * 24 * 60 * 60 * 1000;
  const recentEvents = careEvents.filter(e => new Date(e.hasPointInTime).getTime() > sixMonthsAgo);
  const recencyBonus = Math.min(25, (recentEvents.length / careEvents.length) * 25);

  return Math.min(100, Math.round(eventsPerYear + recencyBonus + 25));
}

/**
 * Calculate community connectedness score from support network attestations.
 *
 * Score: 0-100
 * - 0-20: Isolated (no community support)
 * - 20-40: Weak support network
 * - 40-60: Moderate community involvement
 * - 60-80: Strong support network
 * - 80-100: Highly connected with active mutual support
 *
 * Based on: frequency of community participation and support network size
 */
function calculateCommunityConnectednessScore(supportEvents: EconomicEvent[]): number {
  if (supportEvents.length === 0) {
    return 30; // No evidence = low score (worse than care maintenance)
  }

  // Score based on event frequency
  const eventsPerYear = Math.min(100, (supportEvents.length / 12) * 40);

  // Check for diverse community engagement (multiple different agents)
  const uniqueAgents = new Set(supportEvents.map(e => e.provider ?? e.receiver ?? 'unknown')).size;
  const diversityBonus = Math.min(30, uniqueAgents * 5);

  return Math.min(100, Math.round(eventsPerYear + diversityBonus + 20));
}

/**
 * Calculate historical claims rate.
 *
 * Rate: 0-1.0
 * - 0.0: No claims history
 * - 0.1-0.3: Low claims rate (1-3 claims per year)
 * - 0.3-0.5: Moderate claims rate
 * - 0.5+: High claims rate
 *
 * Smooths with previous rate to avoid wild swings on single new claim.
 */
function calculateHistoricalClaimsRate(claimEvents: EconomicEvent[], previousRate: number): number {
  // Assume 1-year assessment period
  const currentRate = Math.min(1, claimEvents.length / 10);

  // Exponential smoothing: 70% old rate, 30% new rate
  const smoothedRate = previousRate * 0.7 + currentRate * 0.3;

  return Math.round(smoothedRate * 100) / 100;
}

/**
 * Determine risk trend by comparing previous and current scores.
 */
function determineRiskTrend(
  previousScore: number,
  currentScore: number
): 'improving' | 'stable' | 'declining' {
  const difference = currentScore - previousScore;

  // Threshold of 5 points to avoid noise
  if (difference > 5) return 'improving';
  if (difference < -5) return 'declining';
  return 'stable';
}
