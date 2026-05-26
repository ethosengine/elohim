/**
 * Shared Trust Badge Configuration
 *
 * P-migrated from @app/elohim/models/trust-badge-config (Slice 2.1c).
 * Moved here alongside trust-badge.model.ts.
 */

import type {
  ContentAttestationType,
  ContentReach,
  BadgeDisplay,
  BadgeWarning,
  BadgeColor,
} from './trust-badge.model';

// @coverage: 100.0% (2026-02-24)

export const ATTESTATION_BADGE_CONFIG: Record<
  ContentAttestationType,
  Omit<BadgeDisplay, 'grantedBy' | 'grantedAt' | 'attestationType'>
> = {
  'author-verified': {
    type: 'author',
    icon: '✓',
    label: 'Verified Author',
    description: 'Author identity has been cryptographically verified',
    color: 'gray',
    verified: true,
  },
  'steward-approved': {
    type: 'review',
    icon: '🛡️',
    label: 'Steward Approved',
    description: 'Reviewed and approved by a domain steward',
    color: 'green',
    verified: true,
  },
  'community-endorsed': {
    type: 'community',
    icon: '👥',
    label: 'Community Endorsed',
    description: 'Endorsed by community members',
    color: 'green',
    verified: false,
  },
  'peer-reviewed': {
    type: 'review',
    icon: '📋',
    label: 'Peer Reviewed',
    description: 'Formally reviewed by qualified peers',
    color: 'blue',
    verified: true,
  },
  'governance-ratified': {
    type: 'canonical',
    icon: '⚖️',
    label: 'Governance Ratified',
    description: 'Approved through formal governance process',
    color: 'gold',
    verified: true,
  },
  'curriculum-canonical': {
    type: 'canonical',
    icon: '📚',
    label: 'Official Curriculum',
    description: 'Designated as official learning content',
    color: 'gold',
    verified: true,
  },
  'safety-reviewed': {
    type: 'safety',
    icon: '🔒',
    label: 'Safety Reviewed',
    description: 'Checked for harmful content and constitutional alignment',
    color: 'blue',
    verified: true,
  },
  'accuracy-verified': {
    type: 'review',
    icon: '✔️',
    label: 'Accuracy Verified',
    description: 'Factual accuracy has been validated',
    color: 'blue',
    verified: true,
  },
  'accessibility-checked': {
    type: 'accessibility',
    icon: '♿',
    label: 'Accessible',
    description: 'Meets accessibility standards',
    color: 'green',
    verified: true,
  },
  'license-cleared': {
    type: 'license',
    icon: '©',
    label: 'License Cleared',
    description: 'Intellectual property and licensing verified',
    color: 'gray',
    verified: true,
  },
};

export const REACH_BADGE_CONFIG: Record<
  ContentReach,
  { icon: string; label: string; color: BadgeColor; description: string }
> = {
  private: {
    icon: '🔐',
    label: 'Private',
    color: 'gray',
    description: 'Only visible to the author',
  },
  invited: {
    icon: '✉️',
    label: 'Invited',
    color: 'gray',
    description: 'Shared with specific people',
  },
  local: {
    icon: '🏠',
    label: 'Local',
    color: 'gray',
    description: 'Visible to household',
  },
  neighborhood: {
    icon: '🏘️',
    label: 'Neighborhood',
    color: 'gray',
    description: 'Visible to immediate area',
  },
  municipal: {
    icon: '🏛️',
    label: 'Municipal',
    color: 'green',
    description: 'Available to city/town',
  },
  bioregional: {
    icon: '🌿',
    label: 'Bioregional',
    color: 'green',
    description: 'Available to watershed/ecosystem',
  },
  regional: {
    icon: '🗺️',
    label: 'Regional',
    color: 'blue',
    description: 'Available to state/province',
  },
  commons: {
    icon: '🌍',
    label: 'Commons',
    color: 'gold',
    description: 'Public - available to everyone',
  },
};

export const WARNING_CONFIG: Record<BadgeWarning['type'], Omit<BadgeWarning, 'flaggedAt'>> = {
  disputed: {
    type: 'disputed',
    icon: '⚠️',
    label: 'Disputed',
    description: 'The accuracy or appropriateness of this content is being disputed',
    color: 'orange',
  },
  outdated: {
    type: 'outdated',
    icon: '📅',
    label: 'Outdated',
    description: 'This content may be out of date',
    color: 'orange',
  },
  'under-review': {
    type: 'under-review',
    icon: '🔍',
    label: 'Under Review',
    description: 'This content is currently being reviewed',
    color: 'orange',
  },
  'appeal-pending': {
    type: 'appeal-pending',
    icon: '⏳',
    label: 'Appeal Pending',
    description: 'A decision about this content is being appealed',
    color: 'orange',
  },
  'partial-revocation': {
    type: 'partial-revocation',
    icon: '⚡',
    label: 'Partially Revoked',
    description: 'Some attestations for this content have been revoked',
    color: 'red',
  },
};
