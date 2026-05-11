// AUTO-GENERATED from app manifest: elohim/sdk/domains/lamad/manifest.json
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen

export const LAMAD_CONTENT_TYPES = [
  'concept',
  'lesson',
  'assessment',
  'exercise',
  'experience-story',
  'experience-moment',
  'reflection',
  'discussion',
  'article',
  'path',
  'epic',
  'scenario',
  'discovery-assessment',
  'instrument',
  'quiz',
  'course-module',
  'module',
  'community',
  'tool',
  'placeholder',
  'simulation',
  'feature',
  'practice',
  'gate-process-declaration',
  'universal-band-declaration',
  'gate-rules-declaration',
  'aggregation-spec',
  'escalation-target-spec',
] as const;
export type LamadContentType = (typeof LAMAD_CONTENT_TYPES)[number];

export const LAMAD_CONTENT_FORMATS = [
  'markdown',
  'gherkin',
  'sophia-quiz-json',
  'html5-app',
  'spa-bundle',
  'epr-composite',
  'html',
  'plaintext',
  'video-embed',
  'json',
] as const;
export type LamadContentFormat = (typeof LAMAD_CONTENT_FORMATS)[number];

export const LAMAD_RELATIONSHIPS = [
  'CONTAINS',
  'BELONGS_TO',
  'DESCRIBES',
  'IMPLEMENTS',
  'VALIDATES',
  'RELATES_TO',
  'REFERENCES',
  'DEPENDS_ON',
  'REQUIRES',
  'FOLLOWS',
  'ATTACHED_TO',
] as const;
export type LamadRelationship = (typeof LAMAD_RELATIONSHIPS)[number];

export const LAMAD_SIGNALS = [
  'learning-signal',
  'mastery-achieved',
  'assessment-completed',
  'path-completed',
  'practice-engagement',
  'contribution-created',
  'experience-attestation',
  'peer-review-completed',
] as const;
export type LamadSignal = (typeof LAMAD_SIGNALS)[number];

export const LAMAD_RENDERER_MAP: Record<string, string> = {
  'markdown': 'MarkdownRendererComponent',
  'html': 'MarkdownRendererComponent',
  'plaintext': 'MarkdownRendererComponent',
  'gherkin': 'GherkinRendererComponent',
  'sophia-quiz-json': 'SophiaRendererComponent',
  'sophia': 'SophiaRendererComponent',
  'perseus-quiz-json': 'SophiaRendererComponent',
  'html5-app': 'IframeRendererComponent',
  'video-embed': 'IframeRendererComponent',
  'epr-composite': 'PathViewerComponent',
};

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

export const LAMAD_OBSERVATION_KINDS: ObservationKindDeclaration[] = [
  {
    kind: 'lamad:content-viewed',
    namespace: 'elohim/observations/lamad',
    schema: {
    ref_cid: 'Cid',
    dwell_ms: 'u64',
    scroll_depth_pct: 'u8',
    session_id: 'Cid',
    },
    retention_class: 'contextual',
    reach: 'agent-private',
    diversity_threshold: null,
    graduates_to: null,
    graduation_window_seconds: null,
    graduation_policy: null,
  },
  {
    kind: 'lamad:mastery-check-result',
    namespace: 'elohim/observations/lamad',
    schema: {
    node_id: 'Cid',
    score: 'f32',
    hint_count: 'u16',
    },
    retention_class: 'archival',
    reach: 'agent-private',
    diversity_threshold: null,
    graduates_to: 'attestation:mastery',
    graduation_window_seconds: 86400,
    graduation_policy: 'self-threshold',
  },
];

export const LAMAD_SIGNAL_KINDS: Record<string, SignalKindDeclaration> = {};
