/**
 * Insurance Mutual Models - Phase 1 Implementation
 *
 * These models operationalize Elohim Mutual - the autonomous mutual insurance entity
 * described in the autonomous_entity/mutual epic. They extend the existing Shefa
 * REA/ValueFlows infrastructure with insurance-specific domain language.
 *
 * Core Principle: All state changes create immutable EconomicEvent entries.
 * These models are state containers; events are the source of truth.
 *
 * Integration Points:
 * - EconomicEvent: All changes flow through immutable event ledger
 * - CommonsPool: Reserves held in trust for members
 * - AttributionClaim: Members claim against the pool when losses occur
 * - Commitment/Agreement: Coverage terms as constitutional agreements
 * - REAAgent: Adjusters, members, Qahal communities as economic agents
 *
 * Files that depend on this:
 * - economic-event.model.ts (insurance event types)
 * - rea-bridge.model.ts (CommonsPool integration)
 * - steward-economy.model.ts (premium gate system)
 */

import {
  Measure,
  ResourceClassification,
  GovernanceLayer,
} from '@app/elohim/models/rea-bridge.model';

// ============================================================================
// MEMBER RISK PROFILE
// ============================================================================

/**
 * MemberRiskProfile - Actual behavioral risk assessment for a member.
 *
 * The core of Elohim Mutual's information asymmetry flip:
 * Rather than using proxy data (credit scores, zip codes, demographics),
 * Elohim uses actual behavioral observation via the Observer protocol.
 *
 * This solves the insurance paradox: traditional insurers profit from
 * information disadvantage; Elohim Mutual profits from helping members
 * reduce risk.
 */
export interface MemberRiskProfile {
  /** Unique identifier */
  id: string;

  /** Member this profile belongs to */
  memberId: string;

  /** Risk type this profile addresses */
  riskType: 'health' | 'property' | 'casualty' | 'care';

  // ─────────────────────────────────────────────────────────────────
  // Observed Risk Factors (via Observer Protocol)
  // ─────────────────────────────────────────────────────────────────

  /**
   * Care maintenance score (0-100).
   * Observed: Does member regularly maintain/prevent?
   * Examples: preventive health visits, property maintenance, vehicle upkeep
   */
  careMaintenanceScore: number;

  /**
   * Community connectedness score (0-100).
   * Observed: Does member have support network?
   * Why it matters: Social isolation increases health risks, financial vulnerability
   */
  communityConnectednessScore: number;

  /**
   * Historical claims behavior (0-100).
   * Observed: What's member's actual claims pattern?
   * Range: 0 = no claims, 100 = very frequent claims
   */
  historicalClaimsRate: number;

  // ─────────────────────────────────────────────────────────────────
  // Derived Risk Assessment
  // ─────────────────────────────────────────────────────────────────

  /**
   * Composite risk score (0-100).
   * Calculated from: careMaintenanceScore, communityConnectedness, claimsRate
   * 0 = very low risk, 100 = very high risk
   *
   * Note: This is actual risk, not insurance industry proxies.
   * A member with poor credit but perfect maintenance gets low risk score.
   * A member with perfect credit but no preventive care gets high risk score.
   */
  riskScore: number;

  /**
   * Risk tier based on score.
   * Used for premium calculation and coverage determination.
   */
  riskTier: 'low' | 'standard' | 'high' | 'uninsurable';

  /**
   * Risk tier rationale (why this tier).
   * Used for transparency and governance review.
   */
  riskTierRationale: string;

  // ─────────────────────────────────────────────────────────────────
  // Evidence & Attestation
  // ─────────────────────────────────────────────────────────────────

  /**
   * Observer attestation chain IDs that inform this profile.
   * These are cryptographic proofs of observed behavior.
   * Example: observer-attestation-12345 proves "member completed preventive health visit"
   */
  evidenceEventIds: string[];

  /**
   * Attestation sources by score component.
   * Transparency: show which events support each risk factor.
   */
  evidenceBreakdown: {
    careMaintenanceEventsCount: number;
    communityConnectednessEventsCount: number;
    claimsHistoryEventsCount: number;
  };

  // ─────────────────────────────────────────────────────────────────
  // Trend Data (for prevention incentives)
  // ─────────────────────────────────────────────────────────────────

  /**
   * Direction of risk trend.
   * Reflects: Is member improving or deteriorating?
   */
  riskTrendDirection: 'improving' | 'stable' | 'declining';

  /**
   * Previous risk score (for trend calculation).
   */
  lastRiskScore: number;

  /**
   * Projected risk score (if trend continues).
   * Used for member engagement: "You're on track to improve by X%"
   */
  projectedRiskScore?: number;

  // ─────────────────────────────────────────────────────────────────
  // Assessment Lifecycle
  // ─────────────────────────────────────────────────────────────────

  /** When this assessment was created */
  assessedAt: string;

  /** When this assessment was last updated */
  lastAssessmentAt: string;

  /** When next assessment is due (e.g., annual review) */
  nextAssessmentDue: string;

  /**
   * Assessment events.
   * Links to EconomicEvent IDs that created assessments.
   * Immutable trail of how risk profile evolved.
   */
  assessmentEventIds: string[];

  /**
   * Manual override reasons (if risk tier was overridden by governance).
   * For transparency and accountability.
   */
  governanceOverrides?: {
    date: string;
    previousTier: 'low' | 'standard' | 'high' | 'uninsurable';
    newTier: 'low' | 'standard' | 'high' | 'uninsurable';
    reason: string;
    authorizedBy: string; // Agent ID of governance authority
  }[];

  /** Arbitrary metadata (for extensibility) */
  metadata?: Record<string, unknown>;
}

// ============================================================================
// COVERAGE POLICY
// ============================================================================

/**
 * CoveragePolicy - Defines what is covered for a member.
 *
 * Key design: Coverage decisions flow to the lowest competent governance level.
 * - Individual layer: What am I willing to self-insure?
 * - Household layer: What does our family need?
 * - Community layer (Qahal): What do we pool locally?
 * - Network layer: What requires broad pooling?
 * - Constitutional layer: What can no community opt out of?
 *
 * This model captures that graduated structure.
 */
export interface CoveragePolicy {
  /** Unique identifier */
  id: string;

  /** Member this policy covers */
  memberId: string;

  /** Optional: Household this policy is part of (if multi-member) */
  householdId?: string;

  // ─────────────────────────────────────────────────────────────────
  // Coverage Structure (Graduated Governance)
  // ─────────────────────────────────────────────────────────────────

  /**
   * Coverage scope.
   * Reflects which governance layer decided this coverage.
   */
  coverageLevel: 'individual' | 'household' | 'community' | 'network' | 'constitutional';

  /**
   * Governance layer that decided coverage.
   * Links policy to the authority that governs it.
   */
  governedAt: GovernanceLayer;

  /**
   * Covered risks in this policy.
   */
  coveredRisks: CoveredRisk[];

  // ─────────────────────────────────────────────────────────────────
  // Cost-Sharing Terms
  // ─────────────────────────────────────────────────────────────────

  /**
   * Deductible - member pays this before mutual starts paying.
   * Prevents moral hazard; aligns member incentives with prevention.
   */
  deductible: Measure;

  /**
   * Coinsurance - member's percentage of covered costs.
   * Example: 20% coinsurance means mutual pays 80%, member pays 20%.
   * Ranges 0-100. Most policies: 0-30%.
   */
  coinsurance: number;

  /**
   * Out-of-pocket maximum - member stops paying coinsurance after this.
   * Once member hits OOP max, mutual covers 100% (after deductible).
   * Example: $5,000 OOP max means member never pays more than $5k + deductible
   */
  outOfPocketMaximum: Measure;

  /**
   * Annual limit - maximum mutual pays per year for this policy.
   * Optional (many policies have no annual limit).
   */
  annualLimit?: Measure;

  // ─────────────────────────────────────────────────────────────────
  // Effective Dates & Terms
  // ─────────────────────────────────────────────────────────────────

  /** When this policy became effective */
  effectiveFrom: string;

  /** When this policy expires (if not continuous) */
  effectiveUntil?: string;

  /**
   * Renewal terms.
   * Guides when policy auto-renews or requires renegotiation.
   */
  renewalTerms: 'annual' | 'monthly' | 'continuous' | 'one-time';

  /**
   * Renewal reminder date.
   * When member/adjuster should review coverage.
   */
  renewalDueAt?: string;

  // ─────────────────────────────────────────────────────────────────
  // Constitutional Basis
  // ─────────────────────────────────────────────────────────────────

  /**
   * Constitutional document citation.
   * Example: "community-health-coverage-2024.md" or "dignity-floor.md"
   * Proves coverage decision has governance backing, not arbitrary.
   */
  constitutionalBasis: string;

  /**
   * Specific section of constitutional document.
   * For precise reference: "section 3.2, subsection A"
   */
  constitutionalSection?: string;

  // ─────────────────────────────────────────────────────────────────
  // Premium Connection
  // ─────────────────────────────────────────────────────────────────

  /**
   * Most recent premium payment event ID.
   * Links to EconomicEvent that recorded premium.
   * Enables: member's premium history, premium audit trail
   */
  lastPremiumEventId?: string;

  /**
   * Premium rate basis.
   * What this policy's premiums are based on.
   * Example: "standard-risk-2024" or "high-risk-health-2024"
   */
  premiumRateBasis?: string;

  // ─────────────────────────────────────────────────────────────────
  // Policy Lifecycle Events
  // ─────────────────────────────────────────────────────────────────

  /** When policy was created */
  createdAt: string;

  /** When policy was last modified */
  lastModifiedAt: string;

  /**
   * Policy modification events.
   * EconomicEvent IDs where policy was reviewed/updated.
   * Supports: auditing coverage decisions, governance review
   */
  modificationEventIds: string[];

  /**
   * Coverage review events.
   * When policy was reviewed for continued appropriateness.
   * Supports: preventing stale/inappropriate coverage
   */
  reviewEventIds?: string[];

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

/**
 * CoveredRisk - What specific risks are covered under a policy.
 *
 * Insurance coverage is granular: you might cover emergency room visits
 * but not dental; you might cover fire but not flood.
 */
export interface CoveredRisk {
  /** Unique ID for this covered risk */
  id: string;

  /**
   * Risk classification.
   * Describes what type of loss is covered.
   */
  riskType:
    | 'emergency-health'
    | 'preventive-health'
    | 'dental'
    | 'mental-health'
    | 'property-damage'
    | 'liability'
    | 'disaster'
    | 'care-interruption'
    | 'income-protection'
    | 'other';

  /**
   * Is this risk covered?
   * Allows policies to explicitly exclude certain risks.
   */
  isCovered: boolean;

  /**
   * Coverage details (if covered).
   */
  coverage?: {
    /**
     * Limit per incident.
     * Maximum mutual pays for a single loss of this type.
     * Optional - unlimited if not specified.
     */
    limitPerIncident?: Measure;

    /**
     * Annual limit for this risk type.
     * Optional - many don't have per-risk limits.
     */
    annualLimit?: Measure;

    /**
     * Coverage percentage.
     * 100% = mutual covers full cost
     * 80% = mutual covers 80%, member pays 20% (coinsurance)
     * Optional - uses policy-level coinsurance if not specified
     */
    coveragePercent?: number;

    /**
     * Special deductible for this risk.
     * Some risks have their own deductible (e.g., disaster deductible)
     */
    specialDeductible?: Measure;
  };

  /**
   * Exclusions or conditions for this risk.
   * Plain language explanation of what's NOT covered.
   * Example: "Cosmetic dental work", "Pre-existing conditions", "War or terrorism"
   */
  exclusions?: string[];

  /**
   * Prevention incentives.
   * Can members reduce premium by mitigating this risk?
   */
  preventionIncentive?: {
    /**
     * Premium discount for risk mitigation.
     * Example: 15% discount for completed safety course
     */
    discountPercent: number;

    /**
     * What Observer events trigger the discount?
     * Example: Observer confirms "completed-certified-driving-course"
     */
    requiredAttestations: string[];

    /**
     * How long does discount last?
     */
    discountValidityPeriod?: string;
  };

  /**
   * Waiting period.
   * How long must member have coverage before this risk is covered?
   * Prevents "buy coverage, claim immediately" gaming.
   * Example: "30 days" for health conditions, "0 days" for acute incidents
   */
  waitingPeriod?: string;

  /** When this risk was added to policy */
  addedAt: string;

  /** When this risk expires (if time-limited) */
  expiresAt?: string;

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

// ============================================================================
// INSURANCE CLAIM
// ============================================================================

/**
 * InsuranceClaim - A member's claim for covered loss.
 *
 * The epic emphasizes transparent claims processing:
 * "The crisis doesn't sever you from the network. It activates it."
 *
 * This model captures the full claims lifecycle with immutable event tracking.
 */
export interface InsuranceClaim {
  /** Unique identifier (ActionHash in Holochain) */
  id: string;

  /**
   * Human-readable claim number.
   * Makes it easier for members to track: "Claim #INS-2024-001234"
   */
  claimNumber: string;

  // ─────────────────────────────────────────────────────────────────
  // Claim Identity
  // ─────────────────────────────────────────────────────────────────

  /** Member filing claim */
  memberId: string;

  /** Policy this claim is under */
  policyId: string;

  /** Covered risk category (from policy) */
  coveredRiskId: string;

  // ─────────────────────────────────────────────────────────────────
  // Loss Event Details
  // ─────────────────────────────────────────────────────────────────

  /**
   * Type of loss.
   * Classification of what happened.
   */
  lossType:
    | 'emergency-health'
    | 'preventive-health'
    | 'dental'
    | 'mental-health'
    | 'property-damage'
    | 'liability'
    | 'disaster'
    | 'care-interruption'
    | 'income-protection'
    | 'other';

  /**
   * When the loss occurred.
   * Critical for coverage determination ("was member covered on this date?")
   */
  lossDate: string;

  /**
   * When member reported the loss.
   * Shows member acted timely (most policies require prompt reporting)
   */
  reportedDate: string;

  /**
   * Description of what happened.
   * Plain language explanation of the loss event.
   */
  description: string;

  /**
   * Location of loss (if applicable).
   * For property/casualty: address; for health: facility name
   */
  lossLocation?: string;

  // ─────────────────────────────────────────────────────────────────
  // Loss Valuation
  // ─────────────────────────────────────────────────────────────────

  /**
   * Estimated loss amount.
   * What member believes the loss is worth.
   * May differ from adjuster's determination.
   */
  memberEstimatedLossAmount: Measure;

  /**
   * Actual loss amount (determined after investigation).
   * What adjuster verified.
   */
  determinedLossAmount?: Measure;

  // ─────────────────────────────────────────────────────────────────
  // Evidence & Attestation
  // ─────────────────────────────────────────────────────────────────

  /**
   * Observer attestation chain IDs.
   * Cryptographic proof the loss actually occurred.
   * Example: observer-attestation-54321 proves damage was photographed
   */
  observerAttestationIds: string[];

  /**
   * Member-provided supporting documentation.
   * Links to content nodes uploaded by member.
   * Example: photos, receipts, repair estimates
   */
  memberDocumentIds?: string[];

  /**
   * Third-party evidence.
   * Example: police report number, medical records, contractor estimate
   */
  thirdPartyEvidence?: {
    source: string; // e.g., "police-report", "medical-records"
    reference: string; // e.g., report number, case number
    verificationStatus: 'pending' | 'verified' | 'failed';
  }[];

  // ─────────────────────────────────────────────────────────────────
  // Claims Processing State
  // ─────────────────────────────────────────────────────────────────

  /**
   * Current claim status.
   * Reflects where claim is in processing pipeline.
   */
  status: InsuranceClaimStatus;

  /**
   * Status reason (for context).
   * Why claim is in this status.
   * Example: "Awaiting member to submit property inspection report"
   */
  statusReason?: string;

  // ─────────────────────────────────────────────────────────────────
  // Adjudication
  // ─────────────────────────────────────────────────────────────────

  /**
   * Assigned adjuster.
   * Elohim agent who's reviewing claim.
   * Null = unassigned
   */
  assignedAdjusterId?: string;

  /**
   * Date assigned to adjuster.
   */
  assignedAt?: string;

  /**
   * Adjuster's reasoning and determination.
   * Contains full constitutional basis for decision.
   */
  adjustmentReasoning?: AdjustmentReasoning;

  /**
   * Amount approved for settlement.
   * What mutual will actually pay.
   */
  approvedAmount?: Measure;

  /**
   * Appeal/dispute information (if applicable).
   */
  appeal?: {
    appealInitiatedAt: string;
    appealedBy: string; // Member or adjuster
    appealReason: string;
    reviewingAdjusterId?: string; // Different adjuster reviewing appeal
    appealDecision?: 'upheld' | 'overturned' | 'modified';
    decisionNote?: string;
  };

  // ─────────────────────────────────────────────────────────────────
  // Settlement
  // ─────────────────────────────────────────────────────────────────

  /**
   * When claim was fully settled.
   */
  settledAt?: string;

  /**
   * Paid amount (may differ from approved if member already paid coinsurance).
   */
  paidAmount?: Measure;

  /**
   * Payment method.
   * How member received settlement (mutual credit, fiat, etc.)
   */
  paymentMethod?: 'mutual-credit' | 'fiat-transfer' | 'other';

  /**
   * Payment reference.
   * Transaction ID or reference for payment.
   */
  paymentReference?: string;

  // ─────────────────────────────────────────────────────────────────
  // REA Event Integration
  // ─────────────────────────────────────────────────────────────────

  /**
   * Event IDs for full immutable trail.
   * Links to EconomicEvent entries that record all state changes.
   */
  eventIds: {
    /** When claim was filed */
    filedEventId: string;
    /** When claim was investigated */
    investigatedEventId?: string;
    /** When claim was adjusted/determined */
    adjustedEventId?: string;
    /** When claim was settled */
    settledEventId?: string;
    /** Any correction events (if claim was reopened/modified) */
    correctionEventIds?: string[];
  };

  // ─────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────

  /** When claim record was created */
  createdAt: string;

  /** When claim was last updated */
  lastUpdatedAt: string;

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

/**
 * InsuranceClaimStatus - Lifecycle of a claim.
 */
export type InsuranceClaimStatus =
  | 'reported' // Member reported loss
  | 'documented' // Evidence collected
  | 'under-investigation' // Adjuster gathering info
  | 'pending-adjustment' // Adjuster reviewing
  | 'adjustment-made' // Adjuster determination made
  | 'approved' // Approved for settlement
  | 'appealed' // Member/other party appealing
  | 'dispute-resolved' // Appeal resolved
  | 'ready-for-settlement' // Ready to pay
  | 'settled' // Paid
  | 'denied'; // Claim denied

// ============================================================================
// ADJUSTMENT REASONING
// ============================================================================

/**
 * AdjustmentReasoning - Constitutional basis for claim determination.
 *
 * The Bob Parr test: "Does this adjuster's decision honor the covenant?"
 *
 * This model ensures every claim determination is:
 * - Auditable (full reasoning recorded)
 * - Constitutional (cites coverage basis)
 * - Generous (applies "love as flourishing" principle)
 * - Transparent (plain language for members)
 */
export interface AdjustmentReasoning {
  /** Unique identifier */
  id: string;

  /** The claim being adjusted */
  claimId: string;

  /**
   * Adjuster making determination.
   * Links to Elohim agent ID.
   */
  adjusterId: string;

  /**
   * Date/time of adjustment determination.
   */
  adjustmentDate: string;

  // ─────────────────────────────────────────────────────────────────
  // Constitutional Basis
  // ─────────────────────────────────────────────────────────────────

  /**
   * Citation to coverage policy/constitution.
   * What rule governs this determination?
   * Example: "coverage-policy-martinez-2024.md, section 3.2"
   */
  constitutionalCitation: string;

  /**
   * Full text of cited section.
   * For transparency: show what rule was applied.
   */
  citedText?: string;

  // ─────────────────────────────────────────────────────────────────
  // Reasoning (For Human Understanding)
  // ─────────────────────────────────────────────────────────────────

  /**
   * Plain language explanation of determination.
   * This is what's communicated to member.
   * Example: "You have full coverage for emergency room visits under section 3.2.
   *           Your $500 deductible was met on January 15. We reviewed the emergency
   *           room bill and determined $8,500 in covered costs. Under your 80/20
   *           coinsurance, we pay $6,800."
   */
  plainLanguageExplanation: string;

  /**
   * Key facts considered.
   * What information was material to decision?
   * Example: ["member had active coverage on loss date", "evidence supports loss occurred",
   *           "covered risk type applies", "deductible already met"]
   */
  materialFacts: string[];

  /**
   * Alternative interpretations considered.
   * Shows adjuster thought through ambiguity.
   * Example: ["Could argue deductible applies differently"] + reasoning why not adopted
   */
  alternativesConsidered?: {
    interpretation: string;
    whyNotAdopted: string;
  }[];

  // ─────────────────────────────────────────────────────────────────
  // Generosity Principle
  // ─────────────────────────────────────────────────────────────────

  /**
   * Was "love as flourishing" principle applied?
   * The epic: "When ambiguity exists, resolve toward making member whole."
   * Tracks whether adjuster gave benefit of doubt.
   */
  generosityInterpretationApplied: boolean;

  /**
   * If generosity applied, explain how.
   * What ambiguity was resolved in member's favor?
   * Example: "Submitted receipt was partially illegible; we accepted estimate from
   *           network contractor as evidence of reasonable repair cost."
   */
  generosityExplanation?: string;

  // ─────────────────────────────────────────────────────────────────
  // The Actual Determination
  // ─────────────────────────────────────────────────────────────────

  /**
   * Adjuster's findings on coverage.
   */
  determinations: {
    /**
     * Is the loss covered under policy terms?
     */
    coverageApplies: boolean;

    /**
     * If coverage doesn't apply, why not?
     */
    coverageFailureReason?: string;

    /**
     * Coverage amount per policy terms.
     * What policy says about this type of loss.
     */
    policyCoverageAmount: Measure;

    /**
     * Verified loss amount.
     * What adjuster actually verified loss costs.
     */
    verifiedLossAmount: Measure;

    /**
     * Deductible application.
     * How much member already paid toward deductible.
     */
    deductibleApplied: Measure;

    /**
     * Coinsurance application.
     * Member's percentage of remaining cost.
     */
    coinsuranceApplied: Measure;

    /**
     * Out-of-pocket maximum applied.
     * Has member hit their OOP max?
     */
    outOfPocketMaximumMet: boolean;

    /**
     * Final approved settlement amount.
     * What mutual will actually pay.
     */
    finalApprovedAmount: Measure;

    /**
     * Explanation of any denied portion.
     * If member claimed $10k but only $8k approved, explain why.
     */
    deniedPortionExplanation?: string;
  };

  // ─────────────────────────────────────────────────────────────────
  // Integrity & Auditability
  // ─────────────────────────────────────────────────────────────────

  /**
   * Full audit log of reasoning.
   * Every step of analysis, documented for governance review.
   * This is what governance committee reviews to catch patterns.
   */
  auditableDecisionLog: string;

  /**
   * Integrity score assessment.
   * Did adjuster follow constitutional constraints?
   * Used to identify adjuster performance patterns.
   */
  integrityCheck?: {
    /** Does reasoning cite authority? */
    properlyAuthorized: boolean;
    /** Is logic consistent? */
    logicallyCoherent: boolean;
    /** Are facts well-supported? */
    factuallySupported: boolean;
    /** Does decision align with prior similar cases? */
    consistent: boolean;
  };

  /**
   * Governance review flag.
   * Should governance committee review this decision?
   * Example: unusual interpretation, large denial, pattern concern
   */
  flaggedForGovernanceReview: boolean;

  /**
   * Review reason (if flagged).
   */
  reviewFlagReason?: string;

  /**
   * Governance review status (when review happens).
   */
  governanceReviewStatus?: 'pending' | 'in-review' | 'upheld' | 'overturned' | 'modified';

  /**
   * Governance review notes.
   */
  governanceReviewNotes?: string;

  /** When reasoning was created */
  createdAt: string;

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

// Note: Types are exported inline with their interface declarations above.
