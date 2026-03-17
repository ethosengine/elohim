/**
 * Collective Models - Governance Contexts with Graduated Participation
 *
 * Collectives unify communities and organizations under a single model.
 * They are governance contexts, not containers with rigid membership.
 */

export type {
  CollectiveView,
  CollectiveParticipationView,
  CollectiveSeedView,
  CreateCollectiveInputView,
  OrganizationContextView,
} from '@elohim/storage-client';

/** Governance layer types for constitutional context taxonomy */
export const GOVERNANCE_LAYERS = [
  'family',
  'neighborhood',
  'faith',
  'education',
  'interest',
  'geographic',
  'workplace',
  'economic',
  'community',
] as const;

export type GovernanceLayer = (typeof GOVERNANCE_LAYERS)[number];

/** Intimacy levels for participation (reused from qahal consent model) */
export const PARTICIPATION_INTIMACY_LEVELS = [
  'recognition',
  'connection',
  'trusted',
  'intimate',
] as const;

export type ParticipationIntimacyLevel = (typeof PARTICIPATION_INTIMACY_LEVELS)[number];

// ConsentState is re-exported from @app/elohim/models/human-consent.model (via protocol-core)
// to avoid duplicate export through qahal/models/index.ts
