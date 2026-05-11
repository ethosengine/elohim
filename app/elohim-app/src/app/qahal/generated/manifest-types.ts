// AUTO-GENERATED from app manifest: elohim/sdk/domains/qahal/manifest.json
// DO NOT EDIT — regenerate with: pnpm run qahal:codegen

export const QAHAL_CONTENT_TYPES = [
  'collective',
  'proposal',
  'challenge',
  'appeal',
  'statement',
  'post',
  'event',
  'group',
  'message',
  'thread',
] as const;
export type QahalContentType = (typeof QAHAL_CONTENT_TYPES)[number];

export const QAHAL_RELATIONSHIPS = [
  'CONTAINS',
  'BELONGS_TO',
  'CHALLENGES',
  'APPEALS',
  'RELATES_TO',
] as const;
export type QahalRelationship = (typeof QAHAL_RELATIONSHIPS)[number];

export const QAHAL_SIGNALS = [
  'governance-decision',
  'community-report',
  'challenge-filed',
  'appeal-filed',
  'consensus-reached',
  'social-engagement',
  'relationship-formed',
] as const;
export type QahalSignal = (typeof QAHAL_SIGNALS)[number];

export interface DiversityThreshold {
  distinct_households?: number;
  distinct_collectives?: number;
  distinct_regions?: number;
  distinct_archetypes?: number;
  min_count?: number;
}

export interface ObservationKindDeclaration {
  kind: string;
  namespace: string;
  schema: Record<string, string>;
  retention_class: 'operational' | 'contextual' | 'archival' | 'attestation-feeding' | 'wisdom';
  reach: 'agent-private' | 'household' | 'community' | 'commons' | 'commons-attested';
  diversity_threshold?: DiversityThreshold | null;
  graduates_to?: string | null;
  graduation_window_seconds?: number | null;
  graduation_policy?: 'self-threshold' | 'diversity-threshold' | 'summarize' | null;
}

export interface SignalKindDeclaration {
  description: string;
  target_kinds: string[];
  evidence_required?: boolean;
  standing_impact_allowed?: Array<'advisory' | 'consequential' | 'binding'>;
}

export const QAHAL_OBSERVATION_KINDS: ObservationKindDeclaration[] = [];

export const QAHAL_SIGNAL_KINDS: Record<string, SignalKindDeclaration> = {};
