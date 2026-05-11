// AUTO-GENERATED from app manifest: elohim/sdk/domains/shefa/manifest.json
// DO NOT EDIT — regenerate with: pnpm run shefa:codegen

export const SHEFA_CONTENT_TYPES = ['stewardship-context'] as const;
export type ShefaContentType = (typeof SHEFA_CONTENT_TYPES)[number];

export const SHEFA_RELATIONSHIPS = ['STEWARDS', 'STEWARDED_BY', 'FUNDS', 'OBLIGATES'] as const;
export type ShefaRelationship = (typeof SHEFA_RELATIONSHIPS)[number];

export const SHEFA_SIGNALS = [
  'economic-event-recorded',
  'stewardship-allocated',
  'resource-transferred',
  'obligation-fulfilled',
  'custodian-attestation',
  'insurance-claim',
] as const;
export type ShefaSignal = (typeof SHEFA_SIGNALS)[number];

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

export const SHEFA_OBSERVATION_KINDS: ObservationKindDeclaration[] = [
  {
    kind: 'shefa:appreciation-emitted',
    namespace: 'elohim/observations/shefa',
    schema: {
    recipient_cid: 'Cid',
    magnitude: 'f32',
    context_cid: 'Cid',
    },
    retention_class: 'archival',
    reach: 'community',
    diversity_threshold: null,
    graduates_to: 'event:appreciation-summary',
    graduation_window_seconds: 3600,
    graduation_policy: 'summarize',
  },
];

export const SHEFA_SIGNAL_KINDS: Record<string, SignalKindDeclaration> = {};
