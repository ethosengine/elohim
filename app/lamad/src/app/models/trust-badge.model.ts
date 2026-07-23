/**
 * Trust Badge Model - UI-ready representation of content trust state.
 *
 * P-migrated from @app/elohim/models/trust-badge.model (Slice 2.1c).
 * LocalityLevel now imported directly from @elohim/storage-client (the SDK
 * canonical definition — see elohim/sdk/storage-client-ts/src/protocol-core.model.ts),
 * replacing the prior inlined ReachLevel copy (slice 2, 2026-07-23).
 *
 * @app/elohim/models/trust-badge.model re-exports from here for any remaining
 * elohim-app consumers (none at time of migration, but kept for safety).
 */

import type { LocalityLevel } from '@elohim/storage-client';

import { ATTESTATION_BADGE_CONFIG, REACH_BADGE_CONFIG } from './trust-badge-config';

// @coverage: 88.4% (2026-02-24)

// ContentAttestationType is lamad-specific - defined locally for cross-pillar trust display.
export type ContentAttestationType =
  | 'author-verified'
  | 'steward-approved'
  | 'community-endorsed'
  | 'peer-reviewed'
  | 'governance-ratified'
  | 'curriculum-canonical'
  | 'safety-reviewed'
  | 'accuracy-verified'
  | 'accessibility-checked'
  | 'license-cleared';

// Use LocalityLevel directly (ContentReach was just an alias)
export type ContentReach = LocalityLevel;

// ============================================================================
// Trust Badge Types
// ============================================================================

export interface TrustBadge {
  contentId: string;
  primary: BadgeDisplay;
  secondary: BadgeDisplay[];
  trustLevel: TrustLevel;
  trustPercentage: number;
  reach: ContentReach;
  reachLabel: string;
  hasWarnings: boolean;
  warnings: BadgeWarning[];
  summary: string;
  ariaLabel: string;
  actions?: BadgeAction[];
}

export interface BadgeDisplay {
  type: BadgeType;
  icon: string;
  label: string;
  description: string;
  color: BadgeColor;
  verified: boolean;
  attestationType?: ContentAttestationType;
  grantedBy?: string;
  grantedAt?: string;
}

export type BadgeType =
  | 'reach'
  | 'review'
  | 'safety'
  | 'canonical'
  | 'community'
  | 'author'
  | 'license'
  | 'accessibility';

export type BadgeColor =
  | 'gold'
  | 'blue'
  | 'green'
  | 'gray'
  | 'orange'
  | 'red';

export type TrustLevel =
  | 'verified'
  | 'trusted'
  | 'emerging'
  | 'unverified'
  | 'flagged';

export interface BadgeWarning {
  type: 'disputed' | 'outdated' | 'under-review' | 'appeal-pending' | 'partial-revocation';
  icon: string;
  label: string;
  description: string;
  color: 'orange' | 'red';
  flaggedAt: string;
}

// ============================================================================
// Unified Trust Indicator (Badges + Flags)
// ============================================================================

export interface TrustIndicator {
  id: string;
  polarity: 'positive' | 'negative';
  priority: number;
  icon: string;
  label: string;
  description: string;
  color: BadgeColor;
  verified: boolean;
  source: IndicatorSource;
  timestamp: string;
  sourceType: ContentAttestationType | BadgeWarning['type'];
}

export interface IndicatorSource {
  type: 'attestation' | 'flag' | 'system' | 'community';
  id: string;
  name?: string;
}

export interface TrustIndicatorSet {
  contentId: string;
  indicators: TrustIndicator[];
  badges: TrustIndicator[];
  flags: TrustIndicator[];
  primary: TrustIndicator | null;
  trustLevel: TrustLevel;
  trustPercentage: number;
  reach: ContentReach;
  summary: string;
  ariaLabel: string;
}

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
      name: badge.grantedBy,
    },
    timestamp: badge.grantedAt ?? new Date().toISOString(),
    sourceType: badge.attestationType ?? 'author-verified',
  };
}

export function warningToIndicator(warning: BadgeWarning): TrustIndicator {
  const priorityMap: Record<BadgeWarning['type'], number> = {
    'partial-revocation': 95,
    disputed: 90,
    'appeal-pending': 80,
    'under-review': 70,
    outdated: 60,
  };

  return {
    id: `flag-${warning.type}`,
    polarity: 'negative',
    priority: priorityMap[warning.type] ?? 50,
    icon: warning.icon,
    label: warning.label,
    description: warning.description,
    color: warning.color,
    verified: true,
    source: {
      type: 'flag',
      id: warning.type,
    },
    timestamp: warning.flaggedAt,
    sourceType: warning.type,
  };
}

const ATTESTATION_TYPES = {
  GOVERNANCE_RATIFIED: 'governance-ratified' as ContentAttestationType,
  CURRICULUM_CANONICAL: 'curriculum-canonical' as ContentAttestationType,
  PEER_REVIEWED: 'peer-reviewed' as ContentAttestationType,
  STEWARD_APPROVED: 'steward-approved' as ContentAttestationType,
  SAFETY_REVIEWED: 'safety-reviewed' as ContentAttestationType,
  ACCURACY_VERIFIED: 'accuracy-verified' as ContentAttestationType,
  COMMUNITY_ENDORSED: 'community-endorsed' as ContentAttestationType,
  ACCESSIBILITY_CHECKED: 'accessibility-checked' as ContentAttestationType,
  LICENSE_CLEARED: 'license-cleared' as ContentAttestationType,
  AUTHOR_VERIFIED: 'author-verified' as ContentAttestationType,
} as const;

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
  'author-verified': 30,
};

export interface BadgeAction {
  action: 'view-trust-profile' | 'request-attestation' | 'endorse' | 'report' | 'appeal';
  label: string;
  icon: string;
  available: boolean;
  unavailableReason?: string;
  route?: string;
}

export function calculateTrustLevel(
  _reach: ContentReach,
  attestationTypes: ContentAttestationType[],
  hasFlags: boolean
): TrustLevel {
  if (hasFlags) {
    return 'flagged';
  }

  if (
    attestationTypes.includes(ATTESTATION_TYPES.GOVERNANCE_RATIFIED) ||
    attestationTypes.includes(ATTESTATION_TYPES.CURRICULUM_CANONICAL)
  ) {
    return 'verified';
  }

  if (
    attestationTypes.includes(ATTESTATION_TYPES.PEER_REVIEWED) ||
    attestationTypes.includes(ATTESTATION_TYPES.STEWARD_APPROVED)
  ) {
    return 'trusted';
  }

  if (
    attestationTypes.includes(ATTESTATION_TYPES.AUTHOR_VERIFIED) ||
    attestationTypes.includes(ATTESTATION_TYPES.COMMUNITY_ENDORSED)
  ) {
    return 'emerging';
  }

  return 'unverified';
}

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

  let topAttestation = attestationTypes[0];
  if (attestationTypes.includes(ATTESTATION_TYPES.GOVERNANCE_RATIFIED)) {
    topAttestation = ATTESTATION_TYPES.GOVERNANCE_RATIFIED;
  } else if (attestationTypes.includes(ATTESTATION_TYPES.PEER_REVIEWED)) {
    topAttestation = ATTESTATION_TYPES.PEER_REVIEWED;
  } else if (attestationTypes.includes(ATTESTATION_TYPES.STEWARD_APPROVED)) {
    topAttestation = ATTESTATION_TYPES.STEWARD_APPROVED;
  }

  const topBadge = ATTESTATION_BADGE_CONFIG[topAttestation];

  return `${reachConfig.label} content. ${topBadge.label}. Trust score: ${Math.round(trustScore * 100)}%.`;
}

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

export interface CompactTrustBadge {
  contentId: string;
  icon: string;
  color: BadgeColor;
  tooltip: string;
  ariaLabel: string;
  trustLevel: TrustLevel;
  showWarning: boolean;
}

export function toCompactBadge(badge: TrustBadge): CompactTrustBadge {
  return {
    contentId: badge.contentId,
    icon: badge.primary.icon,
    color: badge.hasWarnings ? 'orange' : badge.primary.color,
    tooltip: badge.summary,
    ariaLabel: badge.ariaLabel,
    trustLevel: badge.trustLevel,
    showWarning: badge.hasWarnings,
  };
}

export { WARNING_CONFIG, ATTESTATION_BADGE_CONFIG, REACH_BADGE_CONFIG } from './trust-badge-config';
