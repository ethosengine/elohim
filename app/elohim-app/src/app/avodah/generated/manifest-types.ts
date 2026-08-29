// AUTO-GENERATED from app manifest: elohim/sdk/domains/avodah/manifest.json
// DO NOT EDIT — regenerate with: pnpm run avodah:codegen

export const AVODAH_CONTENT_TYPES = ['work-story', 'work-project'] as const;
export type AvodahContentType = (typeof AVODAH_CONTENT_TYPES)[number];

export const AVODAH_RELATIONSHIPS = ['CONTAINS', 'BELONGS_TO', 'DEPENDS_ON', 'REQUIRES'] as const;
export type AvodahRelationship = (typeof AVODAH_RELATIONSHIPS)[number];

export const AVODAH_SIGNALS = ['task-completed', 'sprint-completed', 'cadence-reset'] as const;
export type AvodahSignal = (typeof AVODAH_SIGNALS)[number];

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

export const AVODAH_OBSERVATION_KINDS: ObservationKindDeclaration[] = [];

export const AVODAH_SIGNAL_KINDS: Record<string, SignalKindDeclaration> = {};
