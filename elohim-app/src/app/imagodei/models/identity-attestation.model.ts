/**
 * Network-Attested Identity Models
 *
 * Types for the Phase 2 red-team enhancement to prevent account hijacking.
 * These enable:
 * - Continuous humanity attestation (not just one-time auth)
 * - Distributed key custody via Shamir Secret Sharing
 * - Behavioral anomaly detection
 * - Community override mechanism (binding reports)
 * - Key revocation via DHT consensus
 * - Emergency identity freeze
 *
 * Key insight: Identity is not just "who has the key" but "who is
 * consistently acting like this person" - proven by ongoing network attestation.
 *
 * @module imagodei/models/identity-attestation
 */

// =============================================================================
// Constants
// =============================================================================

/** Humanity attestation types for continuous identity verification */
export const ATTESTATION_TYPES = [
  'behavioral', // Passive monitoring shows consistent behavior
  'interaction', // Direct interaction with known human
  'video_call', // Video verification with trusted party
  'in_person', // Physical presence verification
  'elohim_check', // AI knowledge verification
] as const;
export type AttestationType = (typeof ATTESTATION_TYPES)[number];

/** Anomaly types detected by behavioral monitoring */
export const ANOMALY_TYPES = [
  'posting_pattern', // Sudden change in posting frequency/style
  'content_style', // AI-detected writing style deviation
  'relationship_change', // Rapid/mass relationship modifications
  'geo_shift', // Geographic location anomaly
  'session_anomaly', // Unusual login patterns
  'capability_abuse', // Excessive use of privileged operations
] as const;
export type AnomalyType = (typeof ANOMALY_TYPES)[number];

/** Anomaly severity levels */
export const ANOMALY_SEVERITIES = [
  'low', // Notable but not concerning
  'medium', // Warrants monitoring
  'high', // Should trigger alerts
  'critical', // Should trigger auto-freeze
] as const;
export type AnomalySeverity = (typeof ANOMALY_SEVERITIES)[number];

/** Identity challenge types (community override) */
export const CHALLENGE_TYPES = [
  'hijack_report', // Account appears compromised
  'impersonation', // Someone claiming to be this person
  'spam', // Posting spam/malicious content
  'anomaly_confirm', // Confirming detected anomaly
] as const;
export type ChallengeType = (typeof CHALLENGE_TYPES)[number];

/** Challenge status values */
export const CHALLENGE_STATUSES = [
  'pending', // Challenge open, accumulating support
  'upheld', // Challenge succeeded, action taken
  'dismissed', // Challenge rejected
  'expired', // Challenge timed out
] as const;
export type ChallengeStatus = (typeof CHALLENGE_STATUSES)[number];

/** Key revocation reasons */
export const REVOCATION_REASONS = [
  'compromised', // Key known to be compromised
  'stolen', // Device/key stolen
  'challenge_upheld', // Community challenge succeeded
  'voluntary', // User-initiated revocation
] as const;
export type RevocationReason = (typeof REVOCATION_REASONS)[number];

/** Identity freeze types */
export const FREEZE_TYPES = [
  'auto_anomaly', // Triggered by anomaly detection
  'community_challenge', // Triggered by community reports
  'steward_emergency', // Triggered by M-of-N steward consensus
] as const;
export type FreezeType = (typeof FREEZE_TYPES)[number];

/** Capabilities that can be frozen */
export const FREEZABLE_CAPABILITIES = [
  'post', // Create new content
  'transfer', // Transfer assets/points
  'modify_relationships', // Change relationship graph
  'vote', // Participate in governance votes
  'attest', // Issue attestations to others
] as const;
export type FreezableCapability = (typeof FREEZABLE_CAPABILITIES)[number];

/** Verification requirements to unfreeze */
export const UNFREEZE_REQUIREMENTS = [
  'elohim_check', // AI knowledge verification
  'social_recovery', // M-of-N steward approval
  'steward_interview', // Direct interview by steward
  'time_decay', // Auto-unfreeze after timeout (low severity only)
] as const;
export type UnfreezeRequirement = (typeof UNFREEZE_REQUIREMENTS)[number];

/** Signing policy levels for distributed key custody */
export const SIGNING_POLICIES = [
  'normal', // Standard M-of-N threshold
  'elevated', // Higher threshold for sensitive ops
  'emergency', // Recovery mode (relaxed threshold)
] as const;
export type SigningPolicy = (typeof SIGNING_POLICIES)[number];

// =============================================================================
// Entry Types
// =============================================================================

/**
 * HumanityWitness - Continuous attestation of identity.
 *
 * Unlike one-time authentication, HumanityWitness provides ongoing
 * proof that an agent is acting as a consistent human identity.
 * Attestations decay over time (expires_at) requiring ongoing relationship.
 */
export interface HumanityWitness {
  id: string;
  humanId: string; // Human being attested
  witnessAgentId: string; // Agent providing attestation

  // Attestation details
  attestationType: AttestationType;
  confidence: number; // 0.0 - 1.0 confidence in identity
  behavioralHash?: string; // Hash of behavioral baseline for comparison

  // Evidence
  evidenceJson?: string; // Supporting evidence
  verificationMethod?: string; // How identity was verified

  // Lifecycle
  createdAt: string;
  expiresAt: string; // Attestations decay - must be renewed
  revokedAt?: string; // Explicitly revoked if witness changes mind
}

/**
 * KeyStewardship - Distributed key custody via Shamir Secret Sharing.
 *
 * Instead of a single doorway holding the full custodial key,
 * the key is split into N shards held by M trusted stewards.
 * Any M-of-N can cooperate to sign, but no single party can act alone.
 */
export interface KeyStewardship {
  id: string;
  humanId: string;

  // Shard holders
  keyShardHolders: string[]; // Agent IDs holding key shards
  thresholdM: number; // M required to sign
  totalShardsN: number; // Total shards distributed

  // Policy
  signingPolicy: SigningPolicy;
  elevatedThreshold?: number; // Higher M for sensitive ops

  // Key metadata
  keyGenerationId: string; // Which key generation this stewardship covers
  shardCommitmentHash: string; // Commitment to verify shards

  // Lifecycle
  createdAt: string;
  updatedAt: string;
  rotatedAt?: string; // Last key rotation
}

/**
 * IdentityAnomaly - Detected behavioral deviation.
 *
 * AI monitors posting patterns, content style, relationship changes,
 * and login patterns. When deviation exceeds threshold, an anomaly is recorded.
 *
 * Critical anomalies can trigger auto-freeze (like the Sheila scenario).
 */
export interface IdentityAnomaly {
  id: string;
  humanId: string;

  // Anomaly details
  anomalyType: AnomalyType;
  severity: AnomalySeverity;
  deviationScore: number; // 0.0 - 1.0 (1.0 = extreme deviation)

  // Evidence
  evidenceJson: string; // { baseline: {...}, current: {...}, diff: {...} }
  detectionMethod: string; // AI model, rule-based, etc.

  // Actions
  autoFreezeTriggered: boolean;
  freezeId?: string; // Link to IdentityFreeze if triggered

  // Resolution
  acknowledgedAt?: string;
  resolvedAt?: string;
  resolutionJson?: string;

  // Timestamps
  detectedAt: string;
  expiresAt?: string;
}

/**
 * IdentityChallenge - Community override mechanism.
 *
 * When humans report "this isn't the real person," challenges accumulate.
 * Each challenger's report is weighted by their relationship trust level.
 * When weighted_support exceeds threshold, automatic action is triggered.
 *
 * Unlike Meta's ignored reports, challenges have BINDING EFFECT.
 */
export interface IdentityChallenge {
  id: string;
  humanId: string; // Human being challenged

  // Challenge details
  challengeType: ChallengeType;
  initiatorId: string; // Who started the challenge
  initiatorWeight: number; // Weight of initial challenger

  // Evidence
  evidenceJson: string; // { description: "", screenshots: [], etc. }
  supportingAnomalyId?: string; // Link to IdentityAnomaly if related

  // Support accumulation
  weightedSupport: number; // Sum of all supporter weights
  supporterCount: number; // Number of unique supporters
  supportersJson: string; // [{ agentId, weight, votedAt }]

  // Thresholds
  freezeThreshold: number; // Weight needed to trigger freeze (default 10.0)
  revokeThreshold: number; // Weight needed to trigger revocation (default 25.0)

  // Status
  status: ChallengeStatus;
  statusChangedAt?: string;
  resolutionJson?: string;

  // Timestamps
  createdAt: string;
  expiresAt: string; // Challenges expire after timeout
}

/**
 * ChallengeSupport - Support for an IdentityChallenge.
 *
 * Separate type to prevent single-point manipulation of weighted_support.
 */
export interface ChallengeSupport {
  id: string;
  challengeId: string;
  supporterId: string;
  weight: number;
  intimacyLevel: string;
  evidenceJson?: string;
  createdAt: string;
}

/**
 * KeyRevocation - DHT consensus to invalidate a compromised key.
 *
 * Once a key is revoked:
 * - All new actions signed by that key are REJECTED
 * - Content signed by revoked key can be flagged/removed
 * - User must go through recovery to get new key
 */
export interface KeyRevocation {
  id: string;
  humanId: string;

  // Revoked key
  revokedKey: string; // The agent_pub_key being revoked
  reason: RevocationReason;

  // Trigger
  initiatedBy: string; // challenge_id, steward consensus, or voluntary
  triggerType: string; // challenge, steward_vote, voluntary

  // Steward votes
  requiredVotes: number; // M required for revocation
  currentVotes: number;
  votesJson: string; // RevocationVote[]

  // Status
  thresholdReached: boolean;
  effectiveAt?: string; // When revocation became active

  // Timestamps
  createdAt: string;
  updatedAt: string;
}

/**
 * RevocationVote - Individual steward vote on key revocation.
 */
export interface RevocationVote {
  id: string;
  revocationId: string;
  stewardId: string;
  approved: boolean;
  attestation: string; // Why they're voting this way
  votedAt: string;
}

/**
 * IdentityFreeze - Emergency suspension of capabilities.
 *
 * When anomaly detection or community challenge threshold is reached,
 * an IdentityFreeze immediately suspends specified capabilities.
 * The hijacker cannot continue posting while verification is pending.
 */
export interface IdentityFreeze {
  id: string;
  humanId: string;

  // Freeze details
  freezeType: FreezeType;
  frozenCapabilities: FreezableCapability[];
  severity: AnomalySeverity;

  // Trigger
  triggeredBy: string; // ID of anomaly/challenge/steward action
  triggerType: string; // anomaly, challenge, steward

  // Verification required
  requiresVerification: UnfreezeRequirement;
  verificationAttempts: number;
  lastVerificationAt?: string;

  // Status
  isActive: boolean;
  liftedAt?: string;
  liftedBy?: string;
  liftReason?: string;

  // Timestamps
  frozenAt: string;
  expiresAt?: string; // Auto-lift for low severity
}

// =============================================================================
// Trust Weight Calculation
// =============================================================================

/**
 * Calculate trust weight based on intimacy level.
 *
 * Key innovation: Reports from intimate relationships count 3x,
 * from trusted 2x, familiar 1x, acquaintances 0.5x, public 0.1x.
 */
export function calculateTrustWeight(intimacyLevel: string): number {
  switch (intimacyLevel) {
    case 'intimate':
      return 3.0; // Family/very close
    case 'trusted':
      return 2.0; // Close friends
    case 'familiar':
      return 1.0; // Acquaintances
    case 'acquainted':
      return 0.5; // Casual connections
    case 'public':
      return 0.1; // Public followers
    default:
      return 0.0;
  }
}

/**
 * Default thresholds for challenges.
 *
 * Threshold: weight >= 10.0 triggers auto-freeze
 * Example: 3 intimate (9) + 2 trusted (4) = 13 -> freeze
 */
export const DEFAULT_FREEZE_THRESHOLD = 10.0;
export const DEFAULT_REVOKE_THRESHOLD = 25.0;

// =============================================================================
// Display Helpers
// =============================================================================

/** Anomaly severity display info */
export const ANOMALY_SEVERITY_DISPLAY = {
  low: { label: 'Low', color: '#6b7280', icon: 'info' },
  medium: { label: 'Medium', color: '#f59e0b', icon: 'warning' },
  high: { label: 'High', color: '#ef4444', icon: 'error' },
  critical: { label: 'Critical', color: '#dc2626', icon: 'dangerous' },
} as const;

/** Challenge status display info */
export const CHALLENGE_STATUS_DISPLAY = {
  pending: { label: 'Pending', color: '#f59e0b', icon: 'pending' },
  upheld: { label: 'Upheld', color: '#22c55e', icon: 'verified' },
  dismissed: { label: 'Dismissed', color: '#6b7280', icon: 'cancel' },
  expired: { label: 'Expired', color: '#9ca3af', icon: 'schedule' },
} as const;

/** Freeze type display info */
export const FREEZE_TYPE_DISPLAY = {
  auto_anomaly: { label: 'Anomaly Detected', icon: 'psychology' },
  community_challenge: { label: 'Community Challenge', icon: 'groups' },
  steward_emergency: { label: 'Steward Emergency', icon: 'admin_panel_settings' },
} as const;

/** Capability display labels */
export const CAPABILITY_DISPLAY = {
  post: { label: 'Post Content', icon: 'edit' },
  transfer: { label: 'Transfer Assets', icon: 'payments' },
  modify_relationships: { label: 'Modify Relationships', icon: 'group' },
  vote: { label: 'Vote', icon: 'how_to_vote' },
  attest: { label: 'Issue Attestations', icon: 'verified_user' },
} as const;

/**
 * Get human-readable anomaly type label
 */
export function getAnomalyTypeLabel(type: AnomalyType): string {
  switch (type) {
    case 'posting_pattern':
      return 'Posting Pattern Change';
    case 'content_style':
      return 'Content Style Deviation';
    case 'relationship_change':
      return 'Relationship Graph Change';
    case 'geo_shift':
      return 'Geographic Anomaly';
    case 'session_anomaly':
      return 'Session Pattern Anomaly';
    case 'capability_abuse':
      return 'Capability Overuse';
    default:
      return type;
  }
}

/**
 * Get human-readable challenge type label
 */
export function getChallengeTypeLabel(type: ChallengeType): string {
  switch (type) {
    case 'hijack_report':
      return 'Account Hijacking';
    case 'impersonation':
      return 'Impersonation';
    case 'spam':
      return 'Spam/Malicious Content';
    case 'anomaly_confirm':
      return 'Confirmed Anomaly';
    default:
      return type;
  }
}
