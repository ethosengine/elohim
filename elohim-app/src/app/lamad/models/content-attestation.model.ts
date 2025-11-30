/**
 * Content Attestation Model - Bidirectional trust for content in the commons.
 *
 * Core insight: In a system with "boundaries around freedom of reach",
 * BOTH agents AND content must earn trust to participate in the commons.
 * Content doesn't get public visibility by default - it earns reach
 * through attestation from stewards, communities, or governance.
 *
 * This creates symmetric accountability:
 * - Agents need attestations to ACCESS certain content
 * - Content needs attestations to REACH certain audiences
 *
 * Holochain mapping:
 * - Entry type: "content_attestation"
 * - Links: content_node → content_attestation (one-to-many)
 * - Validation: only authorized grantors can create attestations
 * - Revocation: creates new entry with revoked status, doesn't delete
 */

// ============================================================================
// Content Reach Levels
// ============================================================================

/**
 * ContentReach - The audience a piece of content can reach.
 *
 * Content starts at 'private' and earns broader reach through attestation.
 * This is the content equivalent of Agent.visibility.
 *
 * Inspired by:
 * - Mastodon's post visibility (public, unlisted, followers-only, direct)
 * - Academic peer review (preprint → reviewed → published)
 * - Content moderation trust levels
 */
export type ContentReach =
  | 'private'      // Only the author can see/access
  | 'invited'      // Specific agents explicitly granted access
  | 'local'        // Author's direct connections/network
  | 'community'    // Members of specific communities/organizations
  | 'federated'    // Multiple communities that have agreed to share
  | 'commons';     // Public - available to all agents in the system

/**
 * ContentReachLevel - Numeric ordering for comparison
 */
export const CONTENT_REACH_LEVELS: Record<ContentReach, number> = {
  'private': 0,
  'invited': 1,
  'local': 2,
  'community': 3,
  'federated': 4,
  'commons': 5
};

// ============================================================================
// Content Attestation Types
// ============================================================================

/**
 * ContentAttestationType - How content earns trust/reach.
 *
 * Different attestation types grant different levels of reach.
 * Multiple attestations can stack to increase reach.
 */
export type ContentAttestationType =
  | 'author-verified'       // Author identity confirmed (baseline)
  | 'steward-approved'      // Domain steward reviewed and approved
  | 'community-endorsed'    // Received N endorsements from community
  | 'peer-reviewed'         // Formal review by qualified peers
  | 'governance-ratified'   // Approved through governance process
  | 'curriculum-canonical'  // Official learning content for a path
  | 'safety-reviewed'       // Checked for harmful content
  | 'accuracy-verified'     // Factual accuracy validated
  | 'accessibility-checked' // Meets accessibility standards
  | 'license-cleared';      // IP/licensing verified

/**
 * AttestationReachGrant - What reach level each attestation type can grant.
 *
 * This is configurable per-community, but these are sensible defaults.
 */
export const DEFAULT_ATTESTATION_REACH_GRANTS: Record<ContentAttestationType, ContentReach> = {
  'author-verified': 'local',
  'steward-approved': 'community',
  'community-endorsed': 'community',
  'peer-reviewed': 'federated',
  'governance-ratified': 'commons',
  'curriculum-canonical': 'commons',
  'safety-reviewed': 'federated',
  'accuracy-verified': 'federated',
  'accessibility-checked': 'community',
  'license-cleared': 'commons'
};

// ============================================================================
// Content Attestation Entity
// ============================================================================

/**
 * ContentAttestation - A trust credential granted to content.
 *
 * Attestations are:
 * - Granted by authorized entities (stewards, governance, community vote)
 * - Revocable (with reason and audit trail)
 * - Stackable (multiple attestations increase reach)
 * - Auditable (full history preserved)
 */
export interface ContentAttestation {
  /** Unique identifier for this attestation */
  id: string;

  /** The content receiving this attestation */
  contentId: string;

  /** Type of attestation being granted */
  attestationType: ContentAttestationType;

  /** Maximum reach this attestation grants */
  reachGranted: ContentReach;

  /** Who granted this attestation */
  grantedBy: AttestationGrantor;

  /** When granted */
  grantedAt: string;

  /** Optional expiration (some attestations need renewal) */
  expiresAt?: string;

  /** Is this attestation currently active? */
  status: 'active' | 'expired' | 'revoked' | 'superseded';

  /** Revocation details (if revoked) */
  revocation?: AttestationRevocation;

  /** Evidence/justification for granting */
  evidence?: AttestationEvidence;

  /** Scope limitations (if any) */
  scope?: AttestationScope;

  /** Metadata for specific attestation types */
  metadata?: Record<string, unknown>;
}

/**
 * AttestationGrantor - Who can grant attestations to content.
 */
export interface AttestationGrantor {
  /** Type of grantor */
  type: 'author' | 'steward' | 'community' | 'governance' | 'system';

  /** Identifier of the grantor */
  grantorId: string;

  /** Display name */
  grantorName?: string;

  /** What attestations does the grantor hold that qualify them? */
  qualifyingAttestations?: string[];
}

/**
 * AttestationRevocation - Details when an attestation is revoked.
 */
export interface AttestationRevocation {
  /** When revoked */
  revokedAt: string;

  /** Who revoked it */
  revokedBy: string;

  /** Why it was revoked */
  reason: RevocationReason;

  /** Detailed explanation */
  explanation?: string;

  /** Can this revocation be appealed? */
  appealable: boolean;

  /** Appeal deadline (if appealable) */
  appealDeadline?: string;
}

export type RevocationReason =
  | 'harmful-content'       // Content causes harm
  | 'misinformation'        // Factually incorrect
  | 'copyright-violation'   // IP issues
  | 'author-request'        // Author requested removal
  | 'policy-violation'      // Violates community policies
  | 'superseded'            // Replaced by newer attestation
  | 'expired-not-renewed'   // Time-based expiration
  | 'grantor-revoked'       // Grantor lost their authority
  | 'governance-decision';  // Formal governance process

/**
 * AttestationEvidence - Proof/justification for attestation.
 */
export interface AttestationEvidence {
  /** Type of evidence */
  type: 'review' | 'vote' | 'automated' | 'appeal' | 'governance';

  /** Description of the evidence */
  description: string;

  /** Links to supporting materials */
  references?: string[];

  /** For community endorsements: who endorsed */
  endorsers?: string[];

  /** For peer review: reviewer identities (may be anonymous) */
  reviewers?: string[];

  /** For automated checks: which checks passed */
  automatedChecks?: string[];

  /** Cryptographic proof (in production) */
  proof?: string;
}

/**
 * AttestationScope - Limitations on the attestation.
 */
export interface AttestationScope {
  /** Only valid in these communities */
  communities?: string[];

  /** Only valid for these content types */
  contentTypes?: string[];

  /** Geographic limitations */
  regions?: string[];

  /** Time-based limitations */
  validFrom?: string;
  validUntil?: string;

  /** Usage limitations */
  maxViews?: number;
  maxDerivatives?: number;
}

// ============================================================================
// Content Trust Profile
// ============================================================================

/**
 * ContentTrustProfile - Aggregated trust state for a piece of content.
 *
 * This is computed from all attestations and determines effective reach.
 */
export interface ContentTrustProfile {
  /** The content this profile describes */
  contentId: string;

  /** Current effective reach (highest granted by active attestations) */
  effectiveReach: ContentReach;

  /** All active attestations */
  activeAttestations: ContentAttestation[];

  /** Historical attestations (expired, revoked, superseded) */
  historicalAttestations: ContentAttestation[];

  /** Trust score (0.0 - 1.0) based on attestation quality/quantity */
  trustScore: number;

  /** Flags/warnings about this content */
  flags: ContentFlag[];

  /** When was this profile last computed */
  computedAt: string;
}

/**
 * ContentFlag - Warnings or issues with content.
 */
export interface ContentFlag {
  type: 'disputed' | 'outdated' | 'partial-revocation' | 'under-review' | 'appeal-pending';
  reason: string;
  flaggedAt: string;
  flaggedBy: string;
}

// ============================================================================
// Reach Requirements (Content → Agent direction)
// ============================================================================

/**
 * ContentReachRequirement - What agents need to access content at each reach level.
 *
 * This complements ContentAccessRequirement (which gates content behind
 * agent attestations). This gates REACH behind content attestations.
 */
export interface ContentReachRequirement {
  /** Target reach level */
  reach: ContentReach;

  /** Required attestation types (OR logic - any of these) */
  requiredAttestations?: ContentAttestationType[];

  /** Required attestation types (AND logic - all of these) */
  requiredAllAttestations?: ContentAttestationType[];

  /** Minimum trust score */
  minimumTrustScore?: number;

  /** Minimum number of endorsements */
  minimumEndorsements?: number;

  /** Must have no active flags of these types */
  noFlags?: ContentFlag['type'][];

  /** Custom requirements (community-specific) */
  customRequirements?: Record<string, unknown>;
}

/**
 * Default reach requirements - what attestations are needed for each reach level.
 */
export const DEFAULT_REACH_REQUIREMENTS: Record<ContentReach, ContentReachRequirement> = {
  'private': {
    reach: 'private',
    // No requirements - author always has private access
  },
  'invited': {
    reach: 'invited',
    requiredAttestations: ['author-verified'],
  },
  'local': {
    reach: 'local',
    requiredAttestations: ['author-verified'],
    minimumTrustScore: 0.2,
  },
  'community': {
    reach: 'community',
    requiredAttestations: ['steward-approved', 'community-endorsed', 'safety-reviewed'],
    minimumTrustScore: 0.4,
  },
  'federated': {
    reach: 'federated',
    requiredAttestations: ['peer-reviewed', 'governance-ratified'],
    minimumTrustScore: 0.6,
    noFlags: ['disputed', 'under-review'],
  },
  'commons': {
    reach: 'commons',
    requiredAllAttestations: ['safety-reviewed', 'license-cleared'],
    requiredAttestations: ['governance-ratified', 'curriculum-canonical'],
    minimumTrustScore: 0.8,
    noFlags: ['disputed', 'under-review', 'appeal-pending'],
  }
};

// ============================================================================
// Attestation Operations
// ============================================================================

/**
 * AttestationRequest - Request to grant an attestation to content.
 */
export interface AttestationRequest {
  contentId: string;
  attestationType: ContentAttestationType;
  requestedBy: string;
  evidence?: AttestationEvidence;
  scope?: AttestationScope;
  justification: string;
  requestedAt: string;
}

/**
 * AttestationDecision - Response to an attestation request.
 */
export interface AttestationDecision {
  requestId: string;
  decision: 'approved' | 'denied' | 'deferred';
  decidedBy: string;
  decidedAt: string;
  reason?: string;
  attestationId?: string; // If approved, the created attestation
  conditions?: string[];  // Conditions for approval
  deferredUntil?: string; // If deferred
}

/**
 * RevocationRequest - Request to revoke a content attestation.
 */
export interface RevocationRequest {
  attestationId: string;
  requestedBy: string;
  reason: RevocationReason;
  explanation: string;
  evidence?: string[];
  requestedAt: string;
}

// ============================================================================
// Index Types for Discovery
// ============================================================================

/**
 * ContentByReach - Index entry for discovering content by reach level.
 */
export interface ContentByReachIndex {
  reach: ContentReach;
  contentIds: string[];
  lastUpdated: string;
}

/**
 * ContentAttestationIndex - Lightweight index of content attestations.
 */
export interface ContentAttestationIndexEntry {
  contentId: string;
  contentTitle: string;
  effectiveReach: ContentReach;
  attestationTypes: ContentAttestationType[];
  trustScore: number;
  hasFlags: boolean;
  lastAttestationAt: string;
}
