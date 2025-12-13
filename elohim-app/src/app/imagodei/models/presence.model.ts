/**
 * Contributor Presence Model - Stewardship lifecycle for absent contributors.
 *
 * Philosophy:
 * - Anyone can create a presence for an absent contributor (e.g., Lynn Foster)
 * - Recognition accumulates even while unclaimed
 * - Stewards care-take presences until the contributor joins
 * - Contributors can claim their presence and receive accumulated recognition
 *
 * Lifecycle:
 * 1. UNCLAIMED: Created, not yet stewarded
 * 2. STEWARDED: An Elohim agent or human has taken responsibility
 * 3. CLAIMED: The actual contributor has verified ownership
 */

// =============================================================================
// Presence States
// =============================================================================

/**
 * ContributorPresence lifecycle states.
 */
export const PresenceStates = {
  UNCLAIMED: 'unclaimed',
  STEWARDED: 'stewarded',
  CLAIMED: 'claimed',
} as const;

export type PresenceState = typeof PresenceStates[keyof typeof PresenceStates];

// =============================================================================
// External Identifiers
// =============================================================================

/**
 * Known external identity providers.
 */
export const ExternalIdentifierProviders = {
  ORCID: 'orcid',
  GITHUB: 'github',
  EMAIL: 'email',
  TWITTER: 'twitter',
  WEBSITE: 'website',
  LINKEDIN: 'linkedin',
  YOUTUBE: 'youtube',
  MASTODON: 'mastodon',
} as const;

export type ExternalIdentifierProvider = typeof ExternalIdentifierProviders[keyof typeof ExternalIdentifierProviders];

/**
 * External identifier for linking to other platforms.
 */
export interface ExternalIdentifier {
  provider: ExternalIdentifierProvider | string;
  value: string;
  verified?: boolean;
  verifiedAt?: string;
}

// =============================================================================
// Contributor Presence View
// =============================================================================

/**
 * ContributorPresence view model - simplified for Angular components.
 * Maps from Holochain ContributorPresence entry (snake_case) to camelCase.
 */
export interface ContributorPresenceView {
  id: string;
  displayName: string;
  presenceState: PresenceState;
  /** Parsed external identifiers */
  externalIdentifiers: ExternalIdentifier[];
  /** Content IDs that establish this contributor */
  establishingContentIds: string[];
  establishedAt: string;
  // Recognition metrics
  affinityTotal: number;
  uniqueEngagers: number;
  citationCount: number;
  recognitionScore: number;
  accumulatingSince: string;
  lastRecognitionAt: string;
  // Stewardship (if stewarded)
  stewardId: string | null;
  stewardshipStartedAt: string | null;
  stewardshipQualityScore: number | null;
  // Claim details (if claimed)
  claimInitiatedAt: string | null;
  claimVerifiedAt: string | null;
  claimVerificationMethod: string | null;
  claimedAgentId: string | null;
  // Metadata
  note: string | null;
  image: string | null;
  createdAt: string;
  updatedAt: string;
}

// =============================================================================
// Input Types
// =============================================================================

/**
 * Input for creating a contributor presence.
 */
export interface CreatePresenceRequest {
  displayName: string;
  externalIdentifiers?: ExternalIdentifier[];
  establishingContentIds?: string[];
  note?: string;
  image?: string;
}

/**
 * Input for beginning stewardship.
 */
export interface BeginStewardshipRequest {
  presenceId: string;
  commitmentNote?: string;
}

/**
 * Input for initiating a claim.
 */
export interface InitiateClaimRequest {
  presenceId: string;
  /** Evidence supporting the claim (e.g., signed message, email verification) */
  claimEvidence: Record<string, unknown>;
  /** Method used to verify (e.g., 'email', 'oauth', 'signed-message') */
  verificationMethod: string;
}

// =============================================================================
// Query Types
// =============================================================================

/**
 * Filters for querying presences.
 */
export interface PresenceQueryFilters {
  presenceState?: PresenceState;
  stewardId?: string;
  minRecognitionScore?: number;
  limit?: number;
}

// =============================================================================
// Display Helpers
// =============================================================================

/**
 * Get human-readable label for presence state.
 */
export function getPresenceStateLabel(state: PresenceState): string {
  const labels: Record<PresenceState, string> = {
    unclaimed: 'Unclaimed',
    stewarded: 'Stewarded',
    claimed: 'Claimed',
  };
  return labels[state] ?? state;
}

/**
 * Get description for presence state.
 */
export function getPresenceStateDescription(state: PresenceState): string {
  const descriptions: Record<PresenceState, string> = {
    unclaimed: 'This contributor has not yet been claimed and has no steward',
    stewarded: 'A steward is caring for this contributor\'s recognition',
    claimed: 'This contributor has verified ownership of their presence',
  };
  return descriptions[state] ?? '';
}

/**
 * Get color class for presence state (for UI badges).
 */
export function getPresenceStateColor(state: PresenceState): string {
  const colors: Record<PresenceState, string> = {
    unclaimed: 'gray',
    stewarded: 'blue',
    claimed: 'green',
  };
  return colors[state] ?? 'gray';
}

/**
 * Get icon for external identifier provider.
 */
export function getProviderIcon(provider: string): string {
  const icons: Record<string, string> = {
    orcid: 'badge',
    github: 'code',
    email: 'email',
    twitter: 'tag',
    website: 'language',
    linkedin: 'work',
    youtube: 'play_circle',
    mastodon: 'forum',
  };
  return icons[provider] ?? 'link';
}

/**
 * Get display label for external identifier provider.
 */
export function getProviderLabel(provider: string): string {
  const labels: Record<string, string> = {
    orcid: 'ORCID',
    github: 'GitHub',
    email: 'Email',
    twitter: 'Twitter/X',
    website: 'Website',
    linkedin: 'LinkedIn',
    youtube: 'YouTube',
    mastodon: 'Mastodon',
  };
  return labels[provider] ?? provider;
}

/**
 * Format recognition score for display.
 */
export function formatRecognitionScore(score: number): string {
  if (score >= 1000000) {
    return `${(score / 1000000).toFixed(1)}M`;
  }
  if (score >= 1000) {
    return `${(score / 1000).toFixed(1)}K`;
  }
  return score.toFixed(0);
}

/**
 * Parse external identifiers JSON from Holochain.
 */
export function parseExternalIdentifiers(json: string): ExternalIdentifier[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json);
    if (Array.isArray(parsed)) {
      return parsed;
    }
    // Handle object format { provider: value }
    if (typeof parsed === 'object') {
      return Object.entries(parsed).map(([provider, value]) => ({
        provider,
        value: String(value),
      }));
    }
    return [];
  } catch {
    return [];
  }
}

/**
 * Serialize external identifiers for Holochain.
 */
export function serializeExternalIdentifiers(identifiers: ExternalIdentifier[]): string {
  return JSON.stringify(identifiers);
}
