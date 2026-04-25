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
] as const;
export const CONTENT_TYPES = ALL_CONTENT_TYPES;
export type ContentType = (typeof ALL_CONTENT_TYPES)[number];

export const CORE_DEVICE_ARCHETYPES = ['node', 'desktop', 'mobile', 'steward'] as const;
export const ALL_DEVICE_ARCHETYPES = ['node', 'desktop', 'mobile', 'steward'] as const;
export const DEVICE_ARCHETYPES = ALL_DEVICE_ARCHETYPES;
export type DeviceArchetype = (typeof ALL_DEVICE_ARCHETYPES)[number];

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

export const CORE_OBSERVATION_POLARITIES = ['positive', 'negative'] as const;
export const ALL_OBSERVATION_POLARITIES = ['positive', 'negative'] as const;
export const OBSERVATION_POLARITIES = ALL_OBSERVATION_POLARITIES;
export type ObservationPolarity = (typeof ALL_OBSERVATION_POLARITIES)[number];

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
