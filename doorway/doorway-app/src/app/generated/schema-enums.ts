// AUTO-GENERATED from protocol JSON schemas.
// DO NOT EDIT — regenerate with: pnpm run schema:codegen:ts
//
// Source: elohim/sdk/schemas/v1/enums/*.schema.json

export const CORE_COMPLETION_CRITERIA = [
  'all-required',
  'pass-assessment',
  'view-content',
] as const;
export const ALL_COMPLETION_CRITERIA = ['all-required', 'pass-assessment', 'view-content'] as const;
export const COMPLETION_CRITERIA = ALL_COMPLETION_CRITERIA;
export type CompletionCriteria = (typeof ALL_COMPLETION_CRITERIA)[number];

export const CORE_CONTENT_FORMATS = [
  'markdown',
  'html',
  'video',
  'audio',
  'interactive',
  'external',
  'epr-composite',
] as const;
export const ALL_CONTENT_FORMATS = [
  'markdown',
  'html',
  'video',
  'audio',
  'interactive',
  'external',
  'epr-composite',
  'plaintext',
  'text',
  'plain',
  'gherkin',
  'perseus',
  'perseus-json',
  'perseus-quiz-json',
  'video-embed',
  'audio-file',
  'html5-app',
  'spa-bundle',
  'human-json',
  'organization-json',
  'json',
  'sophia',
  'sophia-quiz-json',
  'element-registry-manifest',
] as const;
export const CONTENT_FORMATS = ALL_CONTENT_FORMATS;
export type ContentFormat = (typeof ALL_CONTENT_FORMATS)[number];

export const CORE_CONTENT_TYPES = [
  'epic',
  'concept',
  'lesson',
  'scenario',
  'assessment',
  'reflection',
  'discussion',
  'exercise',
  'article',
  'path',
] as const;
export const ALL_CONTENT_TYPES = [
  'epic',
  'concept',
  'lesson',
  'scenario',
  'assessment',
  'reflection',
  'discussion',
  'exercise',
  'article',
  'path',
  'human',
  'role',
  'collective',
  'example',
  'reference',
  'feature',
  'practice',
  'contributor',
  'video',
  'audio',
  'book',
  'book-chapter',
  'documentary',
  'bible-verse',
  'activity',
  'narrative',
  'course-module',
  'module',
  'quiz',
  'podcast',
  'simulation',
  'node-context',
  'stewardship-context',
  'work-story',
  'work-project',
  'issue-report',
  'application',
  'gate-process-declaration',
  'universal-band-declaration',
  'gate-rules-declaration',
  'aggregation-spec',
  'escalation-target-spec',
  'element-registry',
] as const;
export const CONTENT_TYPES = ALL_CONTENT_TYPES;
export type ContentType = (typeof ALL_CONTENT_TYPES)[number];

export const CORE_DEPTHS = ['query', 'sparse', 'on-read', 'kept'] as const;
export const ALL_DEPTHS = ['query', 'sparse', 'on-read', 'kept'] as const;
export const DEPTHS = ALL_DEPTHS;
export type Depth = (typeof ALL_DEPTHS)[number];

export const CORE_DEVICE_ARCHETYPES = ['node', 'desktop', 'mobile', 'steward'] as const;
export const ALL_DEVICE_ARCHETYPES = ['node', 'desktop', 'mobile', 'steward'] as const;
export const DEVICE_ARCHETYPES = ALL_DEVICE_ARCHETYPES;
export type DeviceArchetype = (typeof ALL_DEVICE_ARCHETYPES)[number];

export const CORE_DISCLOSURES = ['holder', 'parties', 'at-reach'] as const;
export const ALL_DISCLOSURES = ['holder', 'parties', 'at-reach'] as const;
export const DISCLOSURES = ALL_DISCLOSURES;
export type Disclosure = (typeof ALL_DISCLOSURES)[number];

export const CORE_ELOHIM_SKILLS = [
  'content-safety-review',
  'discernment-evaluation',
  'reach-negotiation',
  'attestation-recommendation',
  'spiral-detection',
  'care-connection',
  'graduated-intervention',
  'constitutional-verification',
] as const;
export const ALL_ELOHIM_SKILLS = [
  'content-safety-review',
  'discernment-evaluation',
  'reach-negotiation',
  'attestation-recommendation',
  'spiral-detection',
  'care-connection',
  'graduated-intervention',
  'constitutional-verification',
  'accuracy-verification',
  'knowledge-map-synthesis',
  'affinity-analysis',
  'path-recommendation',
  'cross-layer-validation',
  'existential-boundary-enforcement',
  'governance-ratification',
  'path-analysis',
  'learning-objective-validation',
  'prerequisite-verification',
  'mastery-assessment-design',
  'family-value-alignment',
  'personal-agent-support',
  'feedback-profile-negotiation',
  'feedback-profile-enforcement',
  'feedback-profile-upgrade',
  'feedback-profile-downgrade',
  'place-attestation',
  'place-naming-governance',
  'geographic-reach-assignment',
  'bioregional-enforcement',
] as const;
export const ELOHIM_SKILLS = ALL_ELOHIM_SKILLS;
export type ElohimSkill = (typeof ALL_ELOHIM_SKILLS)[number];

export const CORE_ELOHIM_SPECIALTIES = [
  'child-safety',
  'family-dynamics',
  'content-safety',
  'discernment',
  'reach-evaluation',
  'medical',
  'legal',
  'crisis',
] as const;
export const ALL_ELOHIM_SPECIALTIES = [
  'child-safety',
  'family-dynamics',
  'content-safety',
  'discernment',
  'reach-evaluation',
  'medical',
  'legal',
  'crisis',
  'education',
  'code-review',
  'financial-advice',
  'creative-writing',
  'technical-documentation',
  'translation',
  'poetry',
  'philosophy',
  'governance',
  'conflict-resolution',
  'curriculum-design',
  'counseling',
  'theology',
  'science',
  'mathematics',
  'history',
  'language-learning',
  'mental-health',
  'disability-support',
  'elder-care',
  'research',
  'journalism',
] as const;
export const ELOHIM_SPECIALTIES = ALL_ELOHIM_SPECIALTIES;
export type ElohimSpecialty = (typeof ALL_ELOHIM_SPECIALTIES)[number];

export const CORE_ELOHIM_STRENGTHS = [
  'high-confidence-judgments',
  'appeals-sustained',
  'consensus-alignment',
  'novel-pattern-detection',
  'steady-baseline',
] as const;
export const ALL_ELOHIM_STRENGTHS = [
  'high-confidence-judgments',
  'appeals-sustained',
  'consensus-alignment',
  'novel-pattern-detection',
  'steady-baseline',
  'consistent-constitutional-reasoning',
  'low-false-positive-rate',
  'fast-resolution',
  'cross-context-consistency',
  'escalation-accuracy',
] as const;
export const ELOHIM_STRENGTHS = ALL_ELOHIM_STRENGTHS;
export type ElohimStrength = (typeof ALL_ELOHIM_STRENGTHS)[number];

export const CORE_ENGAGEMENT_TYPES = [
  'view',
  'quiz',
  'practice',
  'discuss',
  'create',
  'peer',
  'teach',
  'apply',
] as const;
export const ALL_ENGAGEMENT_TYPES = [
  'view',
  'quiz',
  'practice',
  'discuss',
  'create',
  'peer',
  'teach',
  'apply',
] as const;
export const ENGAGEMENT_TYPES = ALL_ENGAGEMENT_TYPES;
export type EngagementType = (typeof ALL_ENGAGEMENT_TYPES)[number];

export const CORE_EVIDENCE_STAGES = ['fixture', 'real'] as const;
export const ALL_EVIDENCE_STAGES = ['fixture', 'real'] as const;
export const EVIDENCE_STAGES = ALL_EVIDENCE_STAGES;
export type EvidenceStage = (typeof ALL_EVIDENCE_STAGES)[number];

export const ALL_FRAME_CAPABILITIES = [
  'forms',
  'popups',
  'downloads',
  'fullscreen',
  'clipboard-write',
  'camera',
  'microphone',
  'geolocation',
] as const;
export const FRAME_CAPABILITIES = ALL_FRAME_CAPABILITIES;
export type FrameCapability = (typeof ALL_FRAME_CAPABILITIES)[number];
export interface FrameCapabilityEntry {
  readonly value: FrameCapability;
  readonly label: string;
  readonly sandbox: string | null;
  readonly allow: string | null;
  readonly device: boolean;
}
export const FRAME_CAPABILITIES_TABLE: readonly FrameCapabilityEntry[] = [
  { value: 'forms', label: 'Submit forms', sandbox: 'allow-forms', allow: null, device: false },
  { value: 'popups', label: 'Open windows', sandbox: 'allow-popups', allow: null, device: false },
  {
    value: 'downloads',
    label: 'Download files',
    sandbox: 'allow-downloads',
    allow: null,
    device: false,
  },
  {
    value: 'fullscreen',
    label: 'Go fullscreen',
    sandbox: null,
    allow: 'fullscreen',
    device: false,
  },
  {
    value: 'clipboard-write',
    label: 'Copy to your clipboard',
    sandbox: null,
    allow: 'clipboard-write',
    device: false,
  },
  { value: 'camera', label: 'Camera', sandbox: null, allow: 'camera', device: true },
  { value: 'microphone', label: 'Microphone', sandbox: null, allow: 'microphone', device: true },
  { value: 'geolocation', label: 'Location', sandbox: null, allow: 'geolocation', device: true },
];

export const CORE_GRANT_SCOPES = ['sweettest-feedback', 'hosted-cell', 'measure-stage'] as const;
export const ALL_GRANT_SCOPES = ['sweettest-feedback', 'hosted-cell', 'measure-stage'] as const;
export const GRANT_SCOPES = ALL_GRANT_SCOPES;
export type GrantScope = (typeof ALL_GRANT_SCOPES)[number];

export const CORE_INFERENCE_SOURCES = ['explicit', 'path', 'tag', 'semantic'] as const;
export const ALL_INFERENCE_SOURCES = ['explicit', 'path', 'tag', 'semantic', 'system'] as const;
export const INFERENCE_SOURCES = ALL_INFERENCE_SOURCES;
export type InferenceSource = (typeof ALL_INFERENCE_SOURCES)[number];

export const CORE_INSTRUMENT_ARCHETYPES = [
  'retention-check',
  'outcome-correlation',
  'distribution-health',
  'cost-accumulation',
  'outcome-divergence',
  'community-report',
] as const;
export const ALL_INSTRUMENT_ARCHETYPES = [
  'retention-check',
  'outcome-correlation',
  'distribution-health',
  'cost-accumulation',
  'outcome-divergence',
  'community-report',
] as const;
export const INSTRUMENT_ARCHETYPES = ALL_INSTRUMENT_ARCHETYPES;
export type InstrumentArchetype = (typeof ALL_INSTRUMENT_ARCHETYPES)[number];

export const CORE_ISOLATION_LEVELS = [
  'none',
  'intrinsic',
  'delegated',
  'bounded',
  'enforced',
] as const;
export const ALL_ISOLATION_LEVELS = [
  'none',
  'intrinsic',
  'delegated',
  'bounded',
  'enforced',
] as const;
export const ISOLATION_LEVELS = ALL_ISOLATION_LEVELS;
export type Isolation = (typeof ALL_ISOLATION_LEVELS)[number];

export const CORE_LOSS_DOMAINS = ['device', 'household', 'region'] as const;
export const ALL_LOSS_DOMAINS = ['device', 'household', 'region'] as const;
export const LOSS_DOMAINS = ALL_LOSS_DOMAINS;
export type LossDomain = (typeof ALL_LOSS_DOMAINS)[number];

export const CORE_MASTERY_LEVELS = [
  'not_started',
  'seen',
  'remember',
  'understand',
  'apply',
  'analyze',
  'evaluate',
  'create',
] as const;
export const ALL_MASTERY_LEVELS = [
  'not_started',
  'seen',
  'remember',
  'understand',
  'apply',
  'analyze',
  'evaluate',
  'create',
  'recognize',
  'recall',
  'synthesize',
] as const;
export const MASTERY_LEVELS = ALL_MASTERY_LEVELS;
export type MasteryLevel = (typeof ALL_MASTERY_LEVELS)[number];

export const CORE_NEED_CLASSES = ['care', 'member', 'agent', 'commons'] as const;
export const ALL_NEED_CLASSES = ['care', 'member', 'agent', 'commons'] as const;
export const NEED_CLASSES = ALL_NEED_CLASSES;
export type NeedClass = (typeof ALL_NEED_CLASSES)[number];

export const CORE_OBSERVATION_POLARITIES = ['positive', 'negative'] as const;
export const ALL_OBSERVATION_POLARITIES = ['positive', 'negative'] as const;
export const OBSERVATION_POLARITIES = ALL_OBSERVATION_POLARITIES;
export type ObservationPolarity = (typeof ALL_OBSERVATION_POLARITIES)[number];

export const ALL_PAGE_INFO_SECTIONS = [
  'source',
  'delivery',
  'holders',
  'steward',
  'claims',
  'permissions',
  'yours',
] as const;
export const PAGE_INFO_SECTIONS = ALL_PAGE_INFO_SECTIONS;
export type PageInfoSection = (typeof ALL_PAGE_INFO_SECTIONS)[number];
export interface PageInfoSectionEntry {
  readonly value: PageInfoSection;
  readonly label: string;
}
export const PAGE_INFO_SECTIONS_TABLE: readonly PageInfoSectionEntry[] = [
  { value: 'source', label: 'Source' },
  { value: 'delivery', label: 'Delivery' },
  { value: 'holders', label: 'Holders' },
  { value: 'steward', label: 'Steward' },
  { value: 'claims', label: 'Claims' },
  { value: 'permissions', label: 'Permissions' },
  { value: 'yours', label: 'Yours' },
];

export const CORE_PATH_VISIBILITIES = ['private', 'unlisted', 'community', 'public'] as const;
export const ALL_PATH_VISIBILITIES = [
  'private',
  'intimate',
  'unlisted',
  'community',
  'public',
  'draft',
] as const;
export const PATH_VISIBILITIES = ALL_PATH_VISIBILITIES;
export type PathVisibility = (typeof ALL_PATH_VISIBILITIES)[number];

export const CORE_PLACEMENT_FACTORS = [
  'distance',
  'domain-spread',
  'spare-capacity',
  'near-bytes',
  'always-on',
] as const;
export const ALL_PLACEMENT_FACTORS = [
  'distance',
  'domain-spread',
  'spare-capacity',
  'near-bytes',
  'always-on',
] as const;
export const PLACEMENT_FACTORS = ALL_PLACEMENT_FACTORS;
export type PlacementFactor = (typeof ALL_PLACEMENT_FACTORS)[number];

export const CORE_PLACEMENT_KINDS = [
  'custody-blob',
  'custody-root',
  'live-session',
  'relay',
  'wake-ingress',
  'hub-runtime',
  'inference',
  'index-fold',
  'task',
] as const;
export const ALL_PLACEMENT_KINDS = [
  'custody-blob',
  'custody-root',
  'live-session',
  'relay',
  'wake-ingress',
  'hub-runtime',
  'inference',
  'index-fold',
  'task',
] as const;
export const PLACEMENT_KINDS = ALL_PLACEMENT_KINDS;
export type PlacementKind = (typeof ALL_PLACEMENT_KINDS)[number];

export const CORE_REA_ACTIONS = [
  'use',
  'consume',
  'cite',
  'produce',
  'raise',
  'lower',
  'transfer',
  'transfer-custody',
  'transfer-all-rights',
  'move',
  'modify',
  'combine',
  'separate',
  'work',
  'deliver-service',
  'pickup',
  'dropoff',
  'give',
  'take',
  'accept',
  'custody-blob',
  'serve-blob',
  'ack-projection',
  'operate-doorway',
  'project-epr',
] as const;
export const ALL_REA_ACTIONS = [
  'use',
  'consume',
  'cite',
  'produce',
  'raise',
  'lower',
  'transfer',
  'transfer-custody',
  'transfer-all-rights',
  'move',
  'modify',
  'combine',
  'separate',
  'work',
  'deliver-service',
  'pickup',
  'dropoff',
  'give',
  'take',
  'accept',
  'custody-blob',
  'serve-blob',
  'ack-projection',
  'operate-doorway',
  'project-epr',
  'affirm',
  'dismiss',
] as const;
export const REA_ACTIONS = ALL_REA_ACTIONS;
export type ReaAction = (typeof ALL_REA_ACTIONS)[number];

export const CORE_REACH_LEVELS = [
  'private',
  'self',
  'intimate',
  'trusted',
  'familiar',
  'community',
  'public',
  'commons',
] as const;
export const ALL_REACH_LEVELS = [
  'private',
  'self',
  'intimate',
  'trusted',
  'familiar',
  'community',
  'public',
  'commons',
] as const;
export const REACH_LEVELS = ALL_REACH_LEVELS;
export type Reach = (typeof ALL_REACH_LEVELS)[number];
export const REACH_OPENNESS: Record<Reach, number> = {
  private: 1,
  self: 2,
  intimate: 3,
  trusted: 4,
  familiar: 5,
  community: 6,
  public: 7,
  commons: 8,
} as const;

export function reachOpenness(r: Reach): number {
  return REACH_OPENNESS[r];
}

export function isReach(v: string): v is Reach {
  return Object.hasOwn(REACH_OPENNESS, v);
}

export const CORE_SESSION_LIFECYCLE_STATES = [
  'Anonymous',
  'OauthIdentified',
  'PeerNativeSampling',
  'PeerNativeMember',
] as const;
export const ALL_SESSION_LIFECYCLE_STATES = [
  'Anonymous',
  'OauthIdentified',
  'PeerNativeSampling',
  'PeerNativeMember',
] as const;
export const SESSION_LIFECYCLE_STATES = ALL_SESSION_LIFECYCLE_STATES;
export type SessionLifecycleState = (typeof ALL_SESSION_LIFECYCLE_STATES)[number];

export const CORE_STEP_TYPES = ['content', 'path', 'external', 'checkpoint', 'reflection'] as const;
export const ALL_STEP_TYPES = [
  'content',
  'read',
  'path',
  'external',
  'practice',
  'assess',
  'video',
  'interactive',
  'checkpoint',
  'reflection',
] as const;
export const STEP_TYPES = ALL_STEP_TYPES;
export type StepType = (typeof ALL_STEP_TYPES)[number];

export const CORE_SUBSTRATE_SIGNALS = [
  'attention',
  'compute',
  'storage',
  'bandwidth',
  'energy',
  'time',
  'resource',
] as const;
export const ALL_SUBSTRATE_SIGNALS = [
  'attention',
  'compute',
  'storage',
  'bandwidth',
  'energy',
  'time',
  'resource',
] as const;
export const SUBSTRATE_SIGNALS = ALL_SUBSTRATE_SIGNALS;
export type SubstrateSignal = (typeof ALL_SUBSTRATE_SIGNALS)[number];
