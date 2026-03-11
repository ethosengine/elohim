/**
 * Content Access Model
 *
 * Defines access control for Lamad content based on:
 * - Human identity (visitor vs Holochain member)
 * - Attestations held by the human
 * - Path completion requirements
 *
 * Philosophy:
 * - Open by default: Most content is freely explorable
 * - Gated content: Requires joining the network (Holochain identity)
 * - Protected content: Requires attestations + path completion
 *
 * Examples:
 * - Open: General learning content, introductory paths
 * - Gated: Community discussions, advanced content
 * - Protected: CSAM handling training, governance procedures
 */

/**
 * AccessLevel - The human's current access level.
 */
export type AccessLevel = 'visitor' | 'member' | 'attested';

/**
 * ContentAccessRequirement - What's needed to access content.
 */
export interface ContentAccessRequirement {
  // Minimum access level required
  minLevel: AccessLevel;

  // Required attestations (for 'attested' level)
  requiredAttestations?: string[];

  // Required path completions (for gated paths leading to sensitive content)
  requiredPaths?: string[];

  // Optional: Specific governance approval
  requiresGovernanceApproval?: boolean;

  // Optional: Age verification
  requiresAgeVerification?: boolean;

  // Human-readable reason for the requirement
  reason?: string;
}

/**
 * ContentAccessLevel - Access level for a specific piece of content.
 */
export type ContentAccessLevel = 'open' | 'gated' | 'protected';

/**
 * ContentAccessMetadata - Access control metadata for content nodes.
 *
 * This extends the ContentNode.metadata to include access requirements.
 */
export interface ContentAccessMetadata {
  // The access level for this content
  accessLevel: ContentAccessLevel;

  // Detailed requirements (for gated/protected content)
  requirements?: ContentAccessRequirement;

  // Why this content is restricted (shown to humans)
  restrictionReason?: string;

  // Path that grants access (for protected content)
  unlockPath?: string;
}

/**
 * AccessCheckResult - Result of checking human access to content.
 */
export interface AccessCheckResult {
  // Can the human access this content?
  canAccess: boolean;

  // Why access was denied (if canAccess is false)
  reason?: AccessDeniedReason;

  // What the human needs to do to gain access
  actionRequired?: AccessAction;

  // The path to unlock access (if applicable)
  unlockPath?: string;

  // Attestations the human is missing
  missingAttestations?: string[];

  // Paths the human needs to complete
  missingPaths?: string[];
}

/**
 * AccessDeniedReason - Why access was denied.
 */
export type AccessDeniedReason =
  | 'not-authenticated' // User is a visitor, content requires member
  | 'missing-attestation' // User lacks required attestation
  | 'missing-path' // User hasn't completed required path
  | 'governance-pending' // Governance approval not yet granted
  | 'age-verification' // Age verification required
  | 'content-removed'; // Content has been removed

/**
 * AccessAction - What the human can do to gain access.
 */
export interface AccessAction {
  type: AccessActionType;
  label: string;
  description: string;

  // For 'install-holochain'
  installUrl?: string;

  // For 'complete-path'
  pathId?: string;
  pathTitle?: string;

  // For 'earn-attestation'
  attestationId?: string;
  attestationName?: string;
}

/**
 * AccessActionType - Types of actions to gain access.
 */
export type AccessActionType =
  | 'install-holochain' // Join the network
  | 'complete-path' // Complete a prerequisite path
  | 'earn-attestation' // Earn a required attestation
  | 'request-access' // Request governance approval
  | 'verify-age'; // Complete age verification

/**
 * Predefined access requirements for common scenarios.
 */
export const ACCESS_PRESETS = {
  /**
   * Open content - accessible to everyone.
   */
  OPEN: {
    accessLevel: 'open' as ContentAccessLevel,
  },

  /**
   * Member content - requires Holochain identity.
   */
  MEMBER_ONLY: {
    accessLevel: 'gated' as ContentAccessLevel,
    requirements: {
      minLevel: 'member' as AccessLevel,
      reason: 'This content is available to network members.',
    },
  },

  /**
   * Community content - requires membership + community attestation.
   */
  COMMUNITY: {
    accessLevel: 'gated' as ContentAccessLevel,
    requirements: {
      minLevel: 'member' as AccessLevel,
      requiredAttestations: ['community-member'],
      reason: 'This content is for community members.',
    },
  },

  /**
   * Protected content - requires specific training path completion.
   * Example: CSAM handling procedures.
   */
  PROTECTED_TRAINING: (pathId: string, attestationId: string, reason: string) => ({
    accessLevel: 'protected' as ContentAccessLevel,
    requirements: {
      minLevel: 'attested' as AccessLevel,
      requiredAttestations: [attestationId],
      requiredPaths: [pathId],
      reason,
    },
    restrictionReason: reason,
    unlockPath: pathId,
  }),

  /**
   * Governance-gated content - requires governance approval.
   */
  GOVERNANCE_GATED: {
    accessLevel: 'protected' as ContentAccessLevel,
    requirements: {
      minLevel: 'attested' as AccessLevel,
      requiresGovernanceApproval: true,
      reason: 'This content requires governance approval.',
    },
  },

  /**
   * Age-restricted content.
   */
  AGE_RESTRICTED: {
    accessLevel: 'gated' as ContentAccessLevel,
    requirements: {
      minLevel: 'member' as AccessLevel,
      requiresAgeVerification: true,
      reason: 'This content requires age verification.',
    },
  },
} as const;

/**
 * Helper to check if access metadata requires authentication.
 */
export function requiresAuthentication(access?: ContentAccessMetadata): boolean {
  if (!access) return false;
  return access.accessLevel !== 'open';
}

/**
 * Helper to check if access metadata requires attestations.
 */
export function requiresAttestations(access?: ContentAccessMetadata): boolean {
  if (!access?.requirements) return false;
  return (access.requirements.requiredAttestations?.length ?? 0) > 0;
}

/**
 * Helper to check if access metadata requires path completion.
 */
export function requiresPathCompletion(access?: ContentAccessMetadata): boolean {
  if (!access?.requirements) return false;
  return (access.requirements.requiredPaths?.length ?? 0) > 0;
}
