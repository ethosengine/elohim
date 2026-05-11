// AUTO-GENERATED from app manifest: elohim/sdk/domains/imagodei/manifest.json
// DO NOT EDIT — regenerate with: pnpm run imagodei:codegen

export const IMAGODEI_CONTENT_TYPES = ['human', 'role', 'contributor'] as const;
export type ImagodeiContentType = (typeof IMAGODEI_CONTENT_TYPES)[number];

export const IMAGODEI_RELATIONSHIPS = [
  'IDENTIFIES',
  'IDENTIFIED_BY',
  'ASSIGNED_TO',
  'GRANTS',
  'SCOPED_TO',
  'STEWARDS',
  'ATTESTS',
  'PARTICIPATES_IN',
] as const;
export type ImagodeiRelationship = (typeof IMAGODEI_RELATIONSHIPS)[number];

export const IMAGODEI_SIGNALS = [
  'identity-created',
  'presence-established',
  'attestation-granted',
  'attestation-revoked',
  'agency-progressed',
  'relationship-formed',
] as const;
export type ImagodeiSignal = (typeof IMAGODEI_SIGNALS)[number];

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

export const IMAGODEI_OBSERVATION_KINDS: ObservationKindDeclaration[] = [
  {
    kind: 'imagodei:presence-tick',
    namespace: 'elohim/observations/imagodei',
    schema: {
    device_cid: 'Cid',
    },
    retention_class: 'operational',
    reach: 'community',
    diversity_threshold: null,
    graduates_to: null,
    graduation_window_seconds: null,
    graduation_policy: null,
  },
];

export const IMAGODEI_SIGNAL_KINDS: Record<string, SignalKindDeclaration> = {};
