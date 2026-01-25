/**
 * Steward Economy Model - Sustainable Income for Knowledge Stewards
 *
 * From the Manifesto (Part IV-C: The Economics of Stewardship):
 * "The Steward Economy enables sustainable income for those who care-take the
 * knowledge graph. Stewards may or may not be the original creators - they
 * earn from maintaining, curating, and making knowledge accessible."
 *
 * Key concepts:
 * - StewardCredential: Proof of qualification (mastery, peer attestations, track record)
 * - PremiumGate: Access control with pricing and revenue sharing
 * - AccessGrant: Record of learner gaining access
 * - StewardRevenue: Three-way split (steward, contributor, commons)
 *
 * Holochain mapping:
 * - Entry types: steward_credential, premium_gate, access_grant, steward_revenue
 * - Coordinator functions: create_steward_credential, create_premium_gate, grant_access, etc.
 */

// ============================================================================
// Steward Tiers
// ============================================================================

/**
 * StewardTier - Levels of stewardship qualification.
 *
 * Higher tiers require more mastery, attestations, and track record.
 */
export type StewardTier = 'caretaker' | 'curator' | 'expert' | 'pioneer';

/**
 * Steward tier descriptions.
 */
export const STEWARD_TIER_DESCRIPTIONS: Record<StewardTier, string> = {
  caretaker: 'Basic stewardship - maintains existing content',
  curator: 'Active curation - organizes and improves paths',
  expert: 'Domain expertise - creates premium content with attestations',
  pioneer: 'Original researcher/synthesizer - foundational contributions',
};

// ============================================================================
// Pricing Models
// ============================================================================

/**
 * PricingModel - How access to gated content is priced.
 */
export type PricingModel =
  | 'one_time' // Single payment for lifetime access
  | 'subscription' // Recurring payment for continued access
  | 'pay_what_you_can' // Learner chooses amount (with optional minimum)
  | 'free_with_attribution' // Free but requires attribution/citation
  | 'commons_sponsored'; // Free, sponsored by commons fund

/**
 * Pricing model descriptions.
 */
export const PRICING_MODEL_DESCRIPTIONS: Record<PricingModel, string> = {
  one_time: 'Single payment for lifetime access',
  subscription: 'Recurring payment for continued access',
  pay_what_you_can: 'Learner chooses amount',
  free_with_attribution: 'Free with attribution requirement',
  commons_sponsored: 'Free, sponsored by commons fund',
};

// ============================================================================
// Grant Types
// ============================================================================

/**
 * GrantType - How access was granted to a learner.
 */
export type GrantType =
  | 'lifetime' // Permanent access via payment
  | 'subscription' // Time-limited recurring access
  | 'scholarship' // Sponsored access for qualifying learners
  | 'creator_gift' // Gift from steward/contributor
  | 'trial'; // Temporary trial access

// ============================================================================
// Access Requirements
// ============================================================================

/**
 * RequiredAttestation - An attestation required for gate access.
 */
export interface RequiredAttestation {
  attestationType: string;
  attestationId?: string;
}

/**
 * RequiredMastery - Mastery level required for gate access.
 */
export interface RequiredMastery {
  contentId: string;
  minLevel: string;
}

/**
 * RequiredVouches - Peer vouches required for gate access.
 */
export interface RequiredVouches {
  minCount: number;
  fromTier?: StewardTier;
}

// ============================================================================
// StewardCredential
// ============================================================================

/**
 * StewardCredential - Proof of qualification to steward content.
 *
 * A credential tracks:
 * - What the steward has demonstrated mastery of
 * - Who has attested to their expertise
 * - Their stewardship track record (quality, learners served)
 * - What they currently steward (presences, content, paths)
 */
export interface StewardCredential {
  id: string;
  stewardPresenceId: string;
  agentId: string;
  tier: StewardTier;

  // Stewardship scope
  stewartedPresenceIds: string[];
  stewartedContentIds: string[];
  stewartedPathIds: string[];

  // Domain qualification
  masteryContentIds: string[];
  masteryLevelAchieved: string;
  qualificationVerifiedAt: string;

  // Peer attestation
  peerAttestationIds: string[];
  uniqueAttesterCount: number;
  attesterReputationSum: number;

  // Track record
  stewardshipQualityScore: number;
  totalLearnersServed: number;
  totalContentImprovements: number;

  // Domain scope
  domainTags: string[];

  // Status
  isActive: boolean;
  deactivationReason: string | null;

  // Metadata
  note: string | null;
  metadata: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

// ============================================================================
// PremiumGate
// ============================================================================

/**
 * PremiumGate - Access control for curated content/paths.
 *
 * A gate defines:
 * - What resources are behind the gate
 * - What requirements learners must meet (attestations, mastery, vouches)
 * - How it's priced and how revenue is shared
 * - Scholarship support for qualifying learners
 */
export interface PremiumGate {
  id: string;
  stewardCredentialId: string;
  stewardPresenceId: string;
  contributorPresenceId: string | null;

  // What's being gated
  gatedResourceType: string;
  gatedResourceIds: string[];
  gateTitle: string;
  gateDescription: string;
  gateImage: string | null;

  // Access requirements
  requiredAttestations: RequiredAttestation[];
  requiredMastery: RequiredMastery[];
  requiredVouches: RequiredVouches | null;

  // Pricing
  pricingModel: PricingModel;
  priceAmount: number | null;
  priceUnit: string | null;
  subscriptionPeriodDays: number | null;
  minAmount: number | null;

  // Revenue share (must sum to 100%)
  stewardSharePercent: number;
  commonsSharePercent: number;
  contributorSharePercent: number | null;

  // Scholarship support
  scholarshipEligible: boolean;
  maxScholarshipsPerPeriod: number | null;
  scholarshipCriteria: Record<string, unknown> | null;

  // Status
  isActive: boolean;
  deactivationReason: string | null;

  // Stats
  totalAccessGrants: number;
  totalRevenueGenerated: number;
  totalToSteward: number;
  totalToContributor: number;
  totalToCommons: number;
  totalScholarshipsGranted: number;

  // Metadata
  note: string | null;
  metadata: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

// ============================================================================
// AccessGrant
// ============================================================================

/**
 * AccessGrant - Record of learner gaining access to gated content.
 *
 * Tracks how access was granted (payment, scholarship, gift) and
 * manages the access window for subscriptions.
 */
export interface AccessGrant {
  id: string;
  gateId: string;
  learnerAgentId: string;

  // How access was granted
  grantType: GrantType;
  grantedVia: string;

  // Payment details (if applicable)
  paymentEventId: string | null;
  paymentAmount: number | null;
  paymentUnit: string | null;

  // Scholarship details (if applicable)
  scholarshipSponsorId: string | null;
  scholarshipReason: string | null;

  // Access window
  grantedAt: string;
  validUntil: string | null;
  renewalDueAt: string | null;

  // Status
  isActive: boolean;
  revokedAt: string | null;
  revokeReason: string | null;

  // Metadata
  metadata: Record<string, unknown>;
  createdAt: string;
}

// ============================================================================
// StewardRevenue
// ============================================================================

/**
 * RevenueStatus - Processing state of revenue distribution.
 */
export type RevenueStatus = 'pending' | 'completed' | 'failed';

/**
 * StewardRevenue - Value flowing from gate access to stewards and contributors.
 *
 * Implements the three-way split:
 * - Steward: Payment for curation and maintenance
 * - Contributor: Recognition for original creation (if different from steward)
 * - Commons: Infrastructure and governance support
 */
export interface StewardRevenue {
  id: string;
  accessGrantId: string;
  gateId: string;

  // Parties
  fromLearnerId: string;
  toStewardPresenceId: string;
  toContributorPresenceId: string | null;

  // Amounts
  grossAmount: number;
  paymentUnit: string;
  stewardAmount: number;
  contributorAmount: number;
  commonsAmount: number;

  // Shefa linkage (EconomicEvent references)
  stewardEconomicEventId: string;
  contributorEconomicEventId: string | null;
  commonsEconomicEventId: string;

  // Status
  status: RevenueStatus;
  completedAt: string | null;
  failureReason: string | null;

  // Metadata
  note: string | null;
  metadata: Record<string, unknown>;
  createdAt: string;
}

// ============================================================================
// Revenue Summary
// ============================================================================

/**
 * GateRevenueSummary - Revenue summary for a single gate.
 */
export interface GateRevenueSummary {
  gateId: string;
  gateTitle: string;
  totalRevenue: number;
  grantCount: number;
}

/**
 * StewardRevenueSummary - Aggregated revenue report for a steward.
 */
export interface StewardRevenueSummary {
  stewardPresenceId: string;
  totalRevenue: number;
  totalGrants: number;
  revenueByGate: GateRevenueSummary[];
}

// ============================================================================
// Input Types (for creating entities)
// ============================================================================

/**
 * CreateStewardCredentialInput - Input for creating a steward credential.
 */
export interface CreateStewardCredentialInput {
  stewardPresenceId: string;
  tier: StewardTier;
  domainTags: string[];
  masteryContentIds: string[];
  masteryLevelAchieved: string;
  peerAttestationIds: string[];
  stewartedPresenceIds: string[];
  stewartedContentIds: string[];
  stewartedPathIds: string[];
  note?: string;
}

/**
 * CreatePremiumGateInput - Input for creating a premium gate.
 */
export interface CreatePremiumGateInput {
  stewardCredentialId: string;
  stewardPresenceId: string;
  contributorPresenceId?: string;
  gatedResourceType: string;
  gatedResourceIds: string[];
  gateTitle: string;
  gateDescription: string;
  gateImage?: string;
  requiredAttestations: RequiredAttestation[];
  requiredMastery: RequiredMastery[];
  requiredVouches?: RequiredVouches;
  pricingModel: PricingModel;
  priceAmount?: number;
  priceUnit?: string;
  subscriptionPeriodDays?: number;
  minAmount?: number;
  stewardSharePercent: number;
  commonsSharePercent: number;
  contributorSharePercent?: number;
  scholarshipEligible: boolean;
  maxScholarshipsPerPeriod?: number;
  scholarshipCriteria?: Record<string, unknown>;
  note?: string;
}

/**
 * GrantAccessInput - Input for granting access to gated content.
 */
export interface GrantAccessInput {
  gateId: string;
  grantType: GrantType;
  grantedVia: string;
  paymentAmount?: number;
  paymentUnit?: string;
  scholarshipSponsorId?: string;
  scholarshipReason?: string;
}

// ============================================================================
// Contributor Dashboard Types
// ============================================================================

/**
 * LamadContentImpactSummary - Impact summary for a single content piece.
 */
export interface LamadContentImpactSummary {
  contentId: string;
  recognitionPoints: number;
  learnersReached: number;
  masteryCount: number;
}

/**
 * LamadRecognitionEventSummary - Recent recognition event for timeline display.
 */
export interface LamadRecognitionEventSummary {
  learnerId: string;
  contentId: string;
  flowType: string;
  recognitionPoints: number;
  occurredAt: string;
}

/**
 * LamadContributorImpact - Aggregate learning impact for a contributor.
 */
export interface LamadContributorImpact {
  id: string;
  contributorId: string;
  totalRecognitionPoints: number;
  totalLearnersReached: number;
  totalContentMastered: number;
  totalDiscoveriesSparked: number;
  uniqueContentEngaged: number;
  impactByContent: LamadContentImpactSummary[];
  firstRecognitionAt: string;
  lastRecognitionAt: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * LamadContributorDashboard - Aggregated view of contributor impact.
 *
 * Shows the impact of a contributor's work as recognition flows from learners.
 * This is a Lamad-specific aggregation over Shefa hREA flows.
 */
export interface LamadContributorDashboard {
  contributorId: string;
  totalRecognitionPoints: number;
  totalLearnersReached: number;
  totalContentMastered: number;
  totalDiscoveriesSparked: number;
  impactByContent: LamadContentImpactSummary[];
  recentEvents: LamadRecognitionEventSummary[];
  impact: LamadContributorImpact | null;
}

/**
 * LamadContributorRecognition - Recognition flowing to a contributor.
 *
 * Created when a learner engages with content, automatically flowing
 * recognition to the content's contributor via Shefa Appreciation.
 */
export interface LamadContributorRecognition {
  id: string;
  contributorId: string; // ContributorPresence ID
  contentId: string;
  learnerId: string;
  appreciationOfEventId: string; // References the triggering EconomicEvent
  flowType: string;
  recognitionPoints: number;
  pathId: string | null;
  challengeId: string | null;
  note: string | null;
  metadata: Record<string, unknown>;
  occurredAt: string;
}
