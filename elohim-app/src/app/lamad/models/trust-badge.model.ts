/**
 * Trust Badge Model - UI-ready representation of content trust state.
 *
 * This model provides everything the UI needs to render trust badges
 * on content cards, without requiring the UI to understand the full
 * attestation model.
 *
 * Design principles:
 * - Pre-computed: All display values ready for direct binding
 * - Accessible: Includes aria labels and semantic descriptions
 * - Flexible: Supports multiple badge sizes and contexts
 * - Actionable: Includes hints for user actions when applicable
 *
 * Unified Indicator Model:
 * Badges (positive trust) and flags (warnings) are unified as "TrustIndicators"
 * with a polarity (positive/negative). This allows consistent rendering
 * and sorting of all trust-related visual elements.
 */

import { ContentReach, ContentAttestationType } from './content-attestation.model';

// ============================================================================
// Trust Badge Types
// ============================================================================

/**
 * TrustBadge - Complete badge data for a piece of content.
 *
 * UI components can bind directly to these properties.
 */
export interface TrustBadge {
  /** Content ID this badge describes */
  contentId: string;

  /** Primary badge to display (most important trust indicator) */
  primary: BadgeDisplay;

  /** Secondary badges (additional trust indicators, show on hover/expand) */
  secondary: BadgeDisplay[];

  /** Overall trust level for color theming */
  trustLevel: TrustLevel;

  /** Numeric trust score (0-100) for progress indicators */
  trustPercentage: number;

  /** Current reach level */
  reach: ContentReach;

  /** Human-readable reach description */
  reachLabel: string;

  /** Whether content has any warnings/flags */
  hasWarnings: boolean;

  /** Warning details (if any) */
  warnings: BadgeWarning[];

  /** Tooltip/hover content summarizing trust state */
  summary: string;

  /** Accessibility label for screen readers */
  ariaLabel: string;

  /** Actions available to current user (if any) */
  actions?: BadgeAction[];
}

/**
 * BadgeDisplay - Visual representation of a single badge.
 */
export interface BadgeDisplay {
  /** Badge type for styling */
  type: BadgeType;

  /** Icon identifier (emoji, icon class, or SVG reference) */
  icon: string;

  /** Short label (1-2 words) */
  label: string;

  /** Longer description for tooltips */
  description: string;

  /** Color theme */
  color: BadgeColor;

  /** Whether this badge is "verified" (checkmark overlay) */
  verified: boolean;

  /** Attestation type that granted this badge (for detail views) */
  attestationType?: ContentAttestationType;

  /** Who granted this badge */
  grantedBy?: string;

  /** When granted (ISO 8601) */
  grantedAt?: string;
}

/**
 * BadgeType - Categories of trust badges.
 */
export type BadgeType =
  | 'reach'           // Reach level badge (private → commons)
  | 'review'          // Review status (peer-reviewed, steward-approved)
  | 'safety'          // Safety/moderation status
  | 'canonical'       // Official/curriculum content
  | 'community'       // Community endorsement
  | 'author'          // Author verification
  | 'license'         // Licensing status
  | 'accessibility';  // Accessibility certification

/**
 * BadgeColor - Color themes for badges.
 */
export type BadgeColor =
  | 'gold'      // Highest trust (governance-ratified, commons)
  | 'blue'      // Strong trust (peer-reviewed, federated)
  | 'green'     // Good trust (steward-approved, community)
  | 'gray'      // Neutral (author-verified, local)
  | 'orange'    // Warning/attention needed
  | 'red';      // Critical warning

/**
 * TrustLevel - Overall trust categorization for theming.
 */
export type TrustLevel =
  | 'verified'    // Highest trust - governance ratified, commons reach
  | 'trusted'     // Strong trust - peer reviewed, steward approved
  | 'emerging'    // Building trust - community endorsed, author verified
  | 'unverified'  // No attestations yet
  | 'flagged';    // Has active warnings

/**
 * BadgeWarning - Warning/flag display data.
 */
export interface BadgeWarning {
  /** Warning type */
  type: 'disputed' | 'outdated' | 'under-review' | 'appeal-pending' | 'partial-revocation';

  /** Warning icon */
  icon: string;

  /** Short warning text */
  label: string;

  /** Full explanation */
  description: string;

  /** Color */
  color: 'orange' | 'red';

  /** When flagged */
  flaggedAt: string;
}

// ============================================================================
// Unified Trust Indicator (Badges + Flags)
// ============================================================================

/**
 * TrustIndicator - Unified model for both positive badges and negative flags.
 *
 * This allows consistent rendering and sorting of all trust-related elements.
 * UI can render all indicators in a single list, sorted by priority/polarity.
 */
export interface TrustIndicator {
  /** Unique identifier for this indicator */
  id: string;

  /** Positive (badge/attestation) or negative (flag/warning) */
  polarity: 'positive' | 'negative';

  /** Display priority (higher = more prominent, 1-100) */
  priority: number;

  /** Icon to display */
  icon: string;

  /** Short label */
  label: string;

  /** Full description */
  description: string;

  /** Color theme */
  color: BadgeColor;

  /** Whether this is verified/active */
  verified: boolean;

  /** Source of this indicator */
  source: IndicatorSource;

  /** When this indicator was created/granted */
  timestamp: string;

  /** Original attestation or flag type */
  sourceType: ContentAttestationType | BadgeWarning['type'];
}

/**
 * IndicatorSource - Who/what created this indicator.
 */
export interface IndicatorSource {
  /** Source type */
  type: 'attestation' | 'flag' | 'system' | 'community';

  /** Source identifier */
  id: string;

  /** Display name */
  name?: string;
}

/**
 * TrustIndicatorSet - Complete set of indicators for a content node.
 *
 * This is the unified view that replaces separate badge/warning arrays.
 */
export interface TrustIndicatorSet {
  /** Content ID */
  contentId: string;

  /** All indicators, sorted by priority (highest first) */
  indicators: TrustIndicator[];

  /** Just positive indicators */
  badges: TrustIndicator[];

  /** Just negative indicators */
  flags: TrustIndicator[];

  /** Primary indicator to show in compact views */
  primary: TrustIndicator | null;

  /** Overall trust level */
  trustLevel: TrustLevel;

  /** Numeric trust score (0-100) */
  trustPercentage: number;

  /** Current reach level */
  reach: ContentReach;

  /** Summary for tooltips */
  summary: string;

  /** Aria label */
  ariaLabel: string;
}

/**
 * Convert BadgeDisplay to TrustIndicator.
 */
export function badgeToIndicator(badge: BadgeDisplay, priority: number): TrustIndicator {
  return {
    id: `badge-${badge.attestationType ?? badge.type}`,
    polarity: 'positive',
    priority,
    icon: badge.icon,
    label: badge.label,
    description: badge.description,
    color: badge.color,
    verified: badge.verified,
    source: {
      type: 'attestation',
      id: badge.attestationType ?? badge.type,
      name: badge.grantedBy
    },
    timestamp: badge.grantedAt ?? new Date().toISOString(),
    sourceType: badge.attestationType ?? 'author-verified'
  };
}

/**
 * Convert BadgeWarning to TrustIndicator.
 */
export function warningToIndicator(warning: BadgeWarning): TrustIndicator {
  // Warnings have high priority (show first in negative situations)
  const priorityMap: Record<BadgeWarning['type'], number> = {
    'partial-revocation': 95,
    'disputed': 90,
    'appeal-pending': 80,
    'under-review': 70,
    'outdated': 60
  };

  return {
    id: `flag-${warning.type}`,
    polarity: 'negative',
    priority: priorityMap[warning.type] ?? 50,
    icon: warning.icon,
    label: warning.label,
    description: warning.description,
    color: warning.color,
    verified: true, // Flags are always "verified" as active
    source: {
      type: 'flag',
      id: warning.type
    },
    timestamp: warning.flaggedAt,
    sourceType: warning.type
  };
}

/**
 * Priority values for attestation types (higher = more prominent).
 */
export const ATTESTATION_PRIORITY: Record<ContentAttestationType, number> = {
  'governance-ratified': 100,
  'curriculum-canonical': 95,
  'peer-reviewed': 85,
  'steward-approved': 75,
  'safety-reviewed': 70,
  'accuracy-verified': 65,
  'community-endorsed': 55,
  'accessibility-checked': 45,
  'license-cleared': 40,
  'author-verified': 30
};

/**
 * BadgeAction - Action the user can take related to trust.
 */
export interface BadgeAction {
  /** Action identifier */
  action: 'view-trust-profile' | 'request-attestation' | 'endorse' | 'report' | 'appeal';

  /** Button/link label */
  label: string;

  /** Icon */
  icon: string;

  /** Whether action is available to current user */
  available: boolean;

  /** Why unavailable (if applicable) */
  unavailableReason?: string;

  /** Route or handler for this action */
  route?: string;
}

// ============================================================================
// Badge Configuration
// ============================================================================

/**
 * Badge display configuration for each attestation type.
 */
export const ATTESTATION_BADGE_CONFIG: Record<ContentAttestationType, Omit<BadgeDisplay, 'grantedBy' | 'grantedAt' | 'attestationType'>> = {
  'author-verified': {
    type: 'author',
    icon: '✓',
    label: 'Verified Author',
    description: 'Author identity has been cryptographically verified',
    color: 'gray',
    verified: true
  },
  'steward-approved': {
    type: 'review',
    icon: '🛡️',
    label: 'Steward Approved',
    description: 'Reviewed and approved by a domain steward',
    color: 'green',
    verified: true
  },
  'community-endorsed': {
    type: 'community',
    icon: '👥',
    label: 'Community Endorsed',
    description: 'Endorsed by community members',
    color: 'green',
    verified: false
  },
  'peer-reviewed': {
    type: 'review',
    icon: '📋',
    label: 'Peer Reviewed',
    description: 'Formally reviewed by qualified peers',
    color: 'blue',
    verified: true
  },
  'governance-ratified': {
    type: 'canonical',
    icon: '⚖️',
    label: 'Governance Ratified',
    description: 'Approved through formal governance process',
    color: 'gold',
    verified: true
  },
  'curriculum-canonical': {
    type: 'canonical',
    icon: '📚',
    label: 'Official Curriculum',
    description: 'Designated as official learning content',
    color: 'gold',
    verified: true
  },
  'safety-reviewed': {
    type: 'safety',
    icon: '🔒',
    label: 'Safety Reviewed',
    description: 'Checked for harmful content and constitutional alignment',
    color: 'blue',
    verified: true
  },
  'accuracy-verified': {
    type: 'review',
    icon: '✔️',
    label: 'Accuracy Verified',
    description: 'Factual accuracy has been validated',
    color: 'blue',
    verified: true
  },
  'accessibility-checked': {
    type: 'accessibility',
    icon: '♿',
    label: 'Accessible',
    description: 'Meets accessibility standards',
    color: 'green',
    verified: true
  },
  'license-cleared': {
    type: 'license',
    icon: '©',
    label: 'License Cleared',
    description: 'Intellectual property and licensing verified',
    color: 'gray',
    verified: true
  }
};

/**
 * Reach level display configuration.
 */
export const REACH_BADGE_CONFIG: Record<ContentReach, { icon: string; label: string; color: BadgeColor; description: string }> = {
  'private': {
    icon: '🔐',
    label: 'Private',
    color: 'gray',
    description: 'Only visible to the author'
  },
  'invited': {
    icon: '✉️',
    label: 'Invited',
    color: 'gray',
    description: 'Shared with specific people'
  },
  'local': {
    icon: '🏠',
    label: 'Local',
    color: 'gray',
    description: 'Visible to author\'s network'
  },
  'community': {
    icon: '🏘️',
    label: 'Community',
    color: 'green',
    description: 'Available to community members'
  },
  'federated': {
    icon: '🌐',
    label: 'Federated',
    color: 'blue',
    description: 'Shared across communities'
  },
  'commons': {
    icon: '🌍',
    label: 'Commons',
    color: 'gold',
    description: 'Public - available to everyone'
  }
};

/**
 * Warning display configuration.
 */
export const WARNING_CONFIG: Record<BadgeWarning['type'], Omit<BadgeWarning, 'flaggedAt'>> = {
  'disputed': {
    type: 'disputed',
    icon: '⚠️',
    label: 'Disputed',
    description: 'The accuracy or appropriateness of this content is being disputed',
    color: 'orange'
  },
  'outdated': {
    type: 'outdated',
    icon: '📅',
    label: 'Outdated',
    description: 'This content may be out of date',
    color: 'orange'
  },
  'under-review': {
    type: 'under-review',
    icon: '🔍',
    label: 'Under Review',
    description: 'This content is currently being reviewed',
    color: 'orange'
  },
  'appeal-pending': {
    type: 'appeal-pending',
    icon: '⏳',
    label: 'Appeal Pending',
    description: 'A decision about this content is being appealed',
    color: 'orange'
  },
  'partial-revocation': {
    type: 'partial-revocation',
    icon: '⚡',
    label: 'Partially Revoked',
    description: 'Some attestations for this content have been revoked',
    color: 'red'
  }
};

// ============================================================================
// Trust Level Calculation
// ============================================================================

/**
 * Determine trust level from reach and attestations.
 */
export function calculateTrustLevel(
  reach: ContentReach,
  attestationTypes: ContentAttestationType[],
  hasFlags: boolean
): TrustLevel {
  if (hasFlags) {
    return 'flagged';
  }

  if (attestationTypes.includes('governance-ratified') ||
      attestationTypes.includes('curriculum-canonical')) {
    return 'verified';
  }

  if (attestationTypes.includes('peer-reviewed') ||
      attestationTypes.includes('steward-approved')) {
    return 'trusted';
  }

  if (attestationTypes.includes('author-verified') ||
      attestationTypes.includes('community-endorsed')) {
    return 'emerging';
  }

  return 'unverified';
}

/**
 * Generate summary text for trust badge tooltip.
 */
export function generateTrustSummary(
  reach: ContentReach,
  attestationTypes: ContentAttestationType[],
  trustScore: number
): string {
  const reachConfig = REACH_BADGE_CONFIG[reach];
  const attestationCount = attestationTypes.length;

  if (attestationCount === 0) {
    return `${reachConfig.label} content with no attestations yet.`;
  }

  const topAttestation = attestationTypes.includes('governance-ratified')
    ? 'governance-ratified'
    : attestationTypes.includes('peer-reviewed')
    ? 'peer-reviewed'
    : attestationTypes.includes('steward-approved')
    ? 'steward-approved'
    : attestationTypes[0];

  const topBadge = ATTESTATION_BADGE_CONFIG[topAttestation];

  return `${reachConfig.label} content. ${topBadge.label}. Trust score: ${Math.round(trustScore * 100)}%.`;
}

/**
 * Generate aria-label for accessibility.
 */
export function generateAriaLabel(
  title: string,
  reach: ContentReach,
  trustLevel: TrustLevel,
  hasWarnings: boolean
): string {
  const reachConfig = REACH_BADGE_CONFIG[reach];
  let label = `${title}. ${reachConfig.description}.`;

  switch (trustLevel) {
    case 'verified':
      label += ' Governance verified content.';
      break;
    case 'trusted':
      label += ' Trusted content with peer or steward review.';
      break;
    case 'emerging':
      label += ' Content building trust through community engagement.';
      break;
    case 'flagged':
      label += ' Content has active warnings, review with caution.';
      break;
    default:
      label += ' Content not yet verified.';
  }

  if (hasWarnings) {
    label += ' Warning: Content has flags that require attention.';
  }

  return label;
}

// ============================================================================
// Compact Badge (for lists and cards)
// ============================================================================

/**
 * CompactTrustBadge - Minimal badge for list views and cards.
 *
 * Use when space is limited. Shows only the most important indicator.
 */
export interface CompactTrustBadge {
  /** Content ID */
  contentId: string;

  /** Single icon to display */
  icon: string;

  /** Color for icon/border */
  color: BadgeColor;

  /** Tooltip text */
  tooltip: string;

  /** Aria label */
  ariaLabel: string;

  /** Trust level for additional styling */
  trustLevel: TrustLevel;

  /** Whether to show warning indicator */
  showWarning: boolean;
}

/**
 * Create a compact badge from full trust badge.
 */
export function toCompactBadge(badge: TrustBadge): CompactTrustBadge {
  return {
    contentId: badge.contentId,
    icon: badge.primary.icon,
    color: badge.hasWarnings ? 'orange' : badge.primary.color,
    tooltip: badge.summary,
    ariaLabel: badge.ariaLabel,
    trustLevel: badge.trustLevel,
    showWarning: badge.hasWarnings
  };
}
