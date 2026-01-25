/**
 * Stewardship Models - Graduated Capability Management
 *
 * Types for managing stewardship relationships where one agent can manage
 * capabilities for another. This is about identity and self-knowledge.
 *
 * Core philosophy:
 * - Everyone has limits (even the most capable benefit from exploring their constraints)
 * - Power scales with responsibility, not role assignment
 * - Relational accountability - limits negotiated through relationships
 * - Self-reflection tool - helps people recognize where they need support
 */

// =============================================================================
// Stewardship Constants
// =============================================================================

/**
 * Graduated capability tiers - same surface, different depth.
 * Power scales with demonstrated responsibility, not assigned role.
 */
export type StewardCapabilityTier =
  | 'self' // Manage own settings only
  | 'guide' // Help others navigate their settings (advisory)
  | 'guardian' // Manage settings for verified dependents
  | 'coordinator' // Manage settings across organization/community
  | 'constitutional'; // Elohim-level governance capabilities

export const STEWARD_CAPABILITY_TIERS: StewardCapabilityTier[] = [
  'self',
  'guide',
  'guardian',
  'coordinator',
  'constitutional',
];

/**
 * Authority basis - how stewardship was established.
 * Must be verifiable and reviewable.
 */
export type AuthorityBasis =
  | 'minor_guardianship' // Legal guardian of minor
  | 'court_order' // Court-appointed custody
  | 'medical_necessity' // Disability requiring care
  | 'community_consensus' // Community-determined intervention
  | 'organizational_role' // Device managed by organization
  | 'mutual_consent'; // Subject explicitly consented

export const AUTHORITY_BASIS_TYPES: AuthorityBasis[] = [
  'minor_guardianship',
  'court_order',
  'medical_necessity',
  'community_consensus',
  'organizational_role',
  'mutual_consent',
];

/**
 * Grant status lifecycle
 */
export type GrantStatus = 'active' | 'suspended' | 'expired' | 'revoked';

/**
 * Appeal status lifecycle
 */
export type AppealStatus = 'filed' | 'reviewing' | 'decided' | 'closed';

/**
 * Appeal types
 */
export type AppealType =
  | 'scope' // Challenging scope of capabilities granted
  | 'excessive' // Claiming restrictions are disproportionate
  | 'invalid_evidence' // Questioning authority basis evidence
  | 'capability_request'; // Requesting additional capabilities

/**
 * Content categories for filtering
 */
export const CONTENT_CATEGORIES = [
  'violence',
  'adult',
  'gambling',
  'substances',
  'hate',
  'self_harm',
  'spam',
  'misinformation',
] as const;
export type ContentCategory = (typeof CONTENT_CATEGORIES)[number];

/**
 * Age ratings for content
 */
export const AGE_RATINGS = [
  { value: 'G', label: 'G - General Audience' },
  { value: 'PG', label: 'PG - Parental Guidance' },
  { value: 'PG-13', label: 'PG-13 - Parents Strongly Cautioned' },
  { value: 'R', label: 'R - Restricted' },
  { value: 'NC-17', label: 'NC-17 - Adults Only' },
] as const;
export type AgeRating = (typeof AGE_RATINGS)[number]['value'];

/**
 * Features that can be restricted
 */
export const RESTRICTABLE_FEATURES = [
  'post',
  'share',
  'vote',
  'comment',
  'transfer',
  'direct_message',
  'group_create',
  'profile_edit',
  'external_links',
  'download',
] as const;
export type RestrictableFeature = (typeof RESTRICTABLE_FEATURES)[number];

/**
 * Inalienable rights that cannot be disabled even by coordinators
 */
export const INALIENABLE_FEATURES = [
  'capabilities_dashboard',
  'file_appeal',
  'contact_steward',
  'elohim_chat',
  'emergency_call',
  'time_status',
] as const;
export type InalienableFeature = (typeof INALIENABLE_FEATURES)[number];

// =============================================================================
// Stewardship Grant Types
// =============================================================================

/**
 * Authority to steward another agent's device capabilities.
 * Earned through trust + verified need, not role assignment.
 */
export interface StewardshipGrant {
  id: string;
  stewardId: string;
  subjectId: string;

  // Tier level
  tier: StewardCapabilityTier;

  // Authority basis
  authorityBasis: AuthorityBasis;
  evidenceHash?: string;
  verifiedBy: string;

  // Capabilities
  contentFiltering: boolean;
  timeLimits: boolean;
  featureRestrictions: boolean;
  activityMonitoring: boolean;
  policyDelegation: boolean;

  // Delegation
  delegatable: boolean;
  delegatedFrom?: string;
  delegationDepth: number;

  // Lifecycle
  grantedAt: string;
  expiresAt: string;
  reviewAt: string;
  status: GrantStatus;

  // Appeal
  appealId?: string;

  // Metadata
  createdAt: string;
  updatedAt: string;
}

/**
 * Input for creating a stewardship grant
 */
export interface CreateGrantInput {
  subjectId: string;
  authorityBasis: AuthorityBasis;
  evidenceHash?: string;
  verifiedBy: string;
  // Capabilities
  contentFiltering: boolean;
  timeLimits: boolean;
  featureRestrictions: boolean;
  activityMonitoring: boolean;
  policyDelegation: boolean;
  // Options
  delegatable: boolean;
  expiresInDays: number;
  reviewInDays: number;
}

/**
 * Input for delegating a grant
 */
export interface DelegateGrantInput {
  parentGrantId: string;
  newStewardId: string;
  // Optional capability restrictions
  contentFiltering?: boolean;
  timeLimits?: boolean;
  featureRestrictions?: boolean;
  activityMonitoring?: boolean;
  policyDelegation?: boolean;
  expiresInDays: number;
}

// =============================================================================
// Device Policy Types
// =============================================================================

/**
 * Time window for allowed access
 */
export interface TimeWindow {
  daysOfWeek: number[]; // 0=Sun, 1=Mon, etc.
  startHour: number; // 0-23
  startMinute: number; // 0-59
  endHour: number;
  endMinute: number;
}

/**
 * Content filtering rules
 */
export interface ContentFilterRules {
  blockedCategories: string[];
  blockedHashes: string[];
  ageRatingMax?: string;
  reachLevelMax?: number;
}

/**
 * Time limit rules
 */
export interface TimeLimitRules {
  sessionMaxMinutes?: number;
  dailyMaxMinutes?: number;
  timeWindows: TimeWindow[];
  cooldownMinutes?: number;
}

/**
 * Feature restriction rules
 */
export interface FeatureRestrictionRules {
  disabledFeatures: string[];
  disabledRoutes: string[];
  requireApproval: string[];
}

/**
 * Policy event (violation/block)
 */
export interface PolicyEvent {
  timestamp: string;
  eventType: 'blocked_content' | 'time_limit' | 'feature_blocked' | 'approval_required';
  details: string;
  contentHash?: string;
  featureName?: string;
}

/**
 * Concrete policy rules applied to a device.
 * Policies compose: Organization -> Guardian -> Elohim -> Subject customization.
 */
export interface DevicePolicy {
  id: string;
  subjectId: string;
  deviceId?: string;

  // Authorship
  authorId: string;
  authorTier: StewardCapabilityTier;
  inheritsFrom?: string;

  // Nested rule objects (for convenience)
  contentRules: ContentFilterRules;
  timeRules: TimeLimitRules;
  featureRules: FeatureRestrictionRules;

  // Content rules (flat for backwards compatibility)
  blockedCategories: ContentCategory[];
  blockedHashes: string[];
  ageRatingMax?: AgeRating;
  reachLevelMax?: number;

  // Time rules (flat)
  sessionMaxMinutes?: number;
  dailyMaxMinutes?: number;
  timeWindows: TimeWindow[];
  cooldownMinutes?: number;

  // Feature rules (flat)
  disabledFeatures: RestrictableFeature[];
  disabledRoutes: string[];
  requireApproval: string[];

  // Monitoring rules
  logSessions: boolean;
  logCategories: boolean;
  logPolicyEvents: boolean;
  retentionDays: number;
  subjectCanView: boolean;

  // Lifecycle
  effectiveFrom: string;
  effectiveUntil?: string;
  version: number;

  // Metadata
  createdAt: string;
  updatedAt: string;
}

/**
 * Input for creating/updating a policy (nested rules format)
 */
export interface UpsertPolicyInput {
  subjectId?: string;
  deviceId?: string;
  contentRules: ContentFilterRules;
  timeRules: TimeLimitRules;
  featureRules: FeatureRestrictionRules;
  // Optional monitoring rules
  monitoringRules?: {
    logSessions: boolean;
    logCategories: boolean;
    logPolicyEvents: boolean;
    retentionDays: number;
    subjectCanView: boolean;
  };
}

/**
 * Input for creating/updating a policy (flat format, legacy)
 */
export interface UpsertPolicyInputFlat {
  subjectId: string;
  deviceId?: string;
  // Content rules
  blockedCategories: ContentCategory[];
  blockedHashes: string[];
  ageRatingMax?: AgeRating;
  reachLevelMax?: number;
  // Time rules
  sessionMaxMinutes?: number;
  dailyMaxMinutes?: number;
  timeWindows: TimeWindow[];
  cooldownMinutes?: number;
  // Feature rules
  disabledFeatures: RestrictableFeature[];
  disabledRoutes: string[];
  requireApproval: string[];
  // Monitoring rules
  logSessions: boolean;
  logCategories: boolean;
  logPolicyEvents: boolean;
  retentionDays: number;
  subjectCanView: boolean;
}

// =============================================================================
// Computed Policy Types
// =============================================================================

/**
 * Computed policy for a subject (merged from all layers)
 */
export interface ComputedPolicy {
  subjectId: string;
  computedAt: string;

  // Merged content rules
  blockedCategories: string[];
  blockedHashes: string[];
  ageRatingMax?: string;
  reachLevelMax?: number;

  // Merged time rules
  sessionMaxMinutes?: number;
  dailyMaxMinutes?: number;
  timeWindows?: TimeWindow[];
  timeWindowsJson?: string; // For serialization
  cooldownMinutes?: number;

  // Merged feature rules
  disabledFeatures: string[];
  disabledRoutes: string[];
  requireApproval: string[];

  // Merged monitoring rules
  logSessions: boolean;
  logCategories: boolean;
  logPolicyEvents: boolean;
  retentionDays: number;
  subjectCanView: boolean;
}

/**
 * Policy decision result
 */
export type PolicyDecision = { type: 'allow' } | { type: 'block'; reason: string };

// =============================================================================
// Time Access Types
// =============================================================================

/**
 * Time access decision
 */
export type TimeAccessDecision =
  | { status: 'allowed'; remainingSession?: number; remainingDaily?: number }
  | { status: 'outside_window' }
  | { status: 'session_limit' }
  | { status: 'daily_limit' };

// =============================================================================
// Appeal Types
// =============================================================================

/**
 * Appeal decision
 */
export interface AppealDecision {
  approved: boolean;
  notes: string;
  modifications?: {
    capabilitiesReduced?: string[];
    expirationExtended?: boolean;
    newExpiration?: string;
  };
  decidedBy: string;
  decidedAt: string;
}

/**
 * Appeal against stewardship grant or policy
 */
export interface StewardshipAppeal {
  id: string;
  appellantId: string;
  grantId: string;
  policyId?: string;

  // Appeal details
  appealType: AppealType;
  grounds: string[];
  evidenceJson: string;

  // Advocacy
  advocateId?: string;
  advocateNotes?: string;

  // Arbitration
  arbitrationLayer: string;
  assignedTo?: string;

  // Status
  status: AppealStatus;
  statusChangedAt?: string;

  // Decision
  decision?: AppealDecision;

  // Timestamps
  filedAt: string;
  expiresAt: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * Input for filing an appeal
 */
export interface FileAppealInput {
  grantId: string;
  policyId?: string;
  appealType: AppealType;
  grounds: string[];
  evidenceJson: string;
  advocateId?: string;
}

// =============================================================================
// Activity Log Types
// =============================================================================

/**
 * Activity log entry
 */
export interface ActivityLog {
  id: string;
  subjectId: string;
  deviceId?: string;

  // Session info
  sessionId: string;
  sessionStartedAt: string;
  sessionDurationMinutes: number;

  // Activity summary (aggregated)
  categoriesAccessed: string[];
  policyEvents: PolicyEvent[];

  // Metadata
  loggedAt: string;
  retentionExpiresAt: string;
}

// =============================================================================
// Policy Inheritance Types
// =============================================================================

/**
 * Chain link in policy inheritance
 */
export interface PolicyChainLink {
  policyId: string;
  authorTier: StewardCapabilityTier;
  layerOrder: number; // 0=org, 1=guardian, 2=elohim, 3=subject
}

/**
 * Policy inheritance chain
 */
export interface PolicyInheritance {
  id: string;
  subjectId: string;
  chain: PolicyChainLink[];
  computedPolicyId: string;
  computedAt: string;
  createdAt: string;
  updatedAt: string;
}

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Get display name for authority basis
 */
export function getAuthorityBasisLabel(basis: AuthorityBasis): string {
  const labels: Record<AuthorityBasis, string> = {
    minor_guardianship: 'Legal Guardian (Minor)',
    court_order: 'Court-Appointed',
    medical_necessity: 'Medical Necessity',
    community_consensus: 'Community Intervention',
    organizational_role: 'Organization Device',
    mutual_consent: 'Mutual Consent',
  };
  return labels[basis] || basis;
}

/**
 * Get display name for steward tier
 */
export function getStewardTierLabel(tier: StewardCapabilityTier): string {
  const labels: Record<StewardCapabilityTier, string> = {
    self: 'Self',
    guide: 'Guide',
    guardian: 'Guardian',
    coordinator: 'Coordinator',
    constitutional: 'Constitutional',
  };
  return labels[tier] || tier;
}

/**
 * Get description for steward tier
 */
export function getStewardTierDescription(tier: StewardCapabilityTier): string {
  const descriptions: Record<StewardCapabilityTier, string> = {
    self: 'Manage your own settings',
    guide: 'Help others navigate their settings (advisory)',
    guardian: 'Manage settings for verified dependents',
    coordinator: 'Manage settings across organization or community',
    constitutional: 'Elohim-level governance capabilities',
  };
  return descriptions[tier] || '';
}

/**
 * Check if a feature is inalienable (cannot be disabled)
 */
export function isInalienableFeature(feature: string): boolean {
  return INALIENABLE_FEATURES.includes(feature as InalienableFeature);
}

/**
 * Get review period for authority basis
 */
export function getReviewPeriodDays(basis: AuthorityBasis): number {
  const periods: Record<AuthorityBasis, number> = {
    minor_guardianship: 365,
    court_order: 180,
    medical_necessity: 90,
    community_consensus: 30,
    organizational_role: 365,
    mutual_consent: 0, // On-demand revocable
  };
  return periods[basis] || 365;
}

/**
 * Check if grant needs review
 */
export function grantNeedsReview(grant: StewardshipGrant): boolean {
  const reviewDate = new Date(grant.reviewAt);
  return reviewDate <= new Date();
}

/**
 * Check if grant is expired
 */
export function isGrantExpired(grant: StewardshipGrant): boolean {
  const expiresDate = new Date(grant.expiresAt);
  return expiresDate <= new Date();
}

// =============================================================================
// Community Intervention Types (The "Jerry Problem")
// =============================================================================

/**
 * Relationship intimacy levels for weighted voting.
 * People who know someone well have more weight in community decisions.
 */
export type RelationshipLevel = 'intimate' | 'trusted' | 'familiar' | 'acquainted' | 'public';

/**
 * Weight multipliers for relationship levels
 */
export const RELATIONSHIP_WEIGHTS: Record<RelationshipLevel, number> = {
  intimate: 3.0,
  trusted: 2.0,
  familiar: 1.0,
  acquainted: 0.5,
  public: 0.1,
};

/**
 * Threshold required to trigger community intervention
 */
export const COMMUNITY_INTERVENTION_THRESHOLD = 10.0;

/**
 * Intervention status lifecycle
 */
export type InterventionStatus =
  | 'gathering' // Still gathering community support
  | 'threshold_met' // Threshold reached, subject notified
  | 'response_window' // 7-day response period
  | 'arbitration' // Under Elohim review
  | 'decided' // Decision made
  | 'active' // Stewardship active
  | 'completed' // Stewardship ended, restored
  | 'cancelled'; // Intervention cancelled

/**
 * Community support for an intervention
 */
export interface InterventionSupport {
  supporterId: string;
  relationshipLevel: RelationshipLevel;
  weight: number;
  reason: string;
  evidenceHash?: string;
  supportedAt: string;
}

/**
 * Community intervention request (for moral deficit cases)
 *
 * Process:
 * 1. Community members trigger review (weighted by intimacy)
 * 2. Subject notified within 24h with full rights
 * 3. 7-day response window
 * 4. Elohim arbitration hearing
 * 5. Decision with mandatory appeal window
 * 6. If stewardship assigned: monthly review
 */
export interface CommunityIntervention {
  id: string;
  subjectId: string;

  // Support tracking
  supporters: InterventionSupport[];
  totalWeight: number;
  thresholdMet: boolean;
  thresholdMetAt?: string;

  // Pattern evidence
  patternDescription: string;
  evidenceHashes: string[];
  categories: string[]; // e.g., 'authoritarian_rhetoric', 'victim_blaming'

  // Status
  status: InterventionStatus;
  statusHistory: { status: InterventionStatus; at: string; by?: string }[];

  // Subject notification
  subjectNotifiedAt?: string;
  responseWindowEnds?: string;
  subjectResponse?: string;

  // Arbitration
  arbitrationStartedAt?: string;
  arbitrationAssignedTo?: string;

  // Decision
  decision?: {
    approved: boolean;
    stewardshipGrantId?: string;
    restrictionsApplied: string[];
    restorationPath: string;
    notes: string;
    decidedBy: string;
    decidedAt: string;
  };

  // Review (monthly for community consensus)
  nextReviewAt?: string;
  reviewHistory: { reviewedAt: string; continued: boolean; notes: string }[];

  // Timestamps
  createdAt: string;
  updatedAt: string;
}

/**
 * Input for initiating community intervention
 */
export interface InitiateInterventionInput {
  subjectId: string;
  relationshipLevel: RelationshipLevel;
  patternDescription: string;
  evidence?: string;
  categories: string[];
}

/**
 * Input for supporting an existing intervention
 */
export interface SupportInterventionInput {
  interventionId: string;
  relationshipLevel: RelationshipLevel;
  reason: string;
  evidenceHash?: string;
}

/**
 * Categories of concerning patterns (for intervention)
 */
export const INTERVENTION_CATEGORIES = [
  {
    value: 'authoritarian_rhetoric',
    label: 'Authoritarian Rhetoric',
    description: 'Justifying violence or oppression by authorities',
  },
  {
    value: 'victim_blaming',
    label: 'Victim Blaming',
    description: 'Blaming victims for harm done to them',
  },
  {
    value: 'dehumanization',
    label: 'Dehumanization',
    description: 'Dehumanizing language toward groups',
  },
  { value: 'harassment', label: 'Harassment', description: 'Persistent targeting of individuals' },
  {
    value: 'manipulation',
    label: 'Manipulation',
    description: 'Exploitative or manipulative behavior',
  },
  {
    value: 'misinformation',
    label: 'Misinformation',
    description: 'Spreading harmful false information',
  },
  {
    value: 'exploitation',
    label: 'Exploitation',
    description: 'Financial, emotional, or other exploitation',
  },
] as const;
export type InterventionCategory = (typeof INTERVENTION_CATEGORIES)[number]['value'];

/**
 * Calculate weighted support total
 */
export function calculateInterventionWeight(supporters: InterventionSupport[]): number {
  return supporters.reduce((sum, s) => sum + s.weight, 0);
}

/**
 * Check if intervention has reached threshold
 */
export function hasReachedThreshold(intervention: CommunityIntervention): boolean {
  return intervention.totalWeight >= COMMUNITY_INTERVENTION_THRESHOLD;
}

/**
 * Get relationship level label
 */
export function getRelationshipLevelLabel(level: RelationshipLevel): string {
  const labels: Record<RelationshipLevel, string> = {
    intimate: 'Intimate (family, close friends)',
    trusted: 'Trusted (good friends, colleagues)',
    familiar: 'Familiar (acquaintances, neighbors)',
    acquainted: 'Acquainted (met a few times)',
    public: 'Public (online only)',
  };
  return labels[level] || level;
}

/**
 * Get intervention status label
 */
export function getInterventionStatusLabel(status: InterventionStatus): string {
  const labels: Record<InterventionStatus, string> = {
    gathering: 'Gathering Support',
    threshold_met: 'Threshold Met',
    response_window: 'Awaiting Response',
    arbitration: 'Under Review',
    decided: 'Decision Made',
    active: 'Intervention Active',
    completed: 'Completed',
    cancelled: 'Cancelled',
  };
  return labels[status] || status;
}
