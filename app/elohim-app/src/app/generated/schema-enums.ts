// AUTO-GENERATED — aligned with protocol schema (elohim/sdk/schemas/current/enums/content-type.schema.json)
// TODO: Phase 2 will replace this with imports from protocol schema codegen
// Generated at: 2026-03-17T22:00:00.000Z

/**
 * Content types — DNA-notarized types + storage-only types (human, role)
 * Must match protocol schema enum: elohim/sdk/schemas/v1/enums/content-type.schema.json
 */
export const CONTENT_TYPES = [
  'epic',
  'concept',
  'lesson',
  'scenario',
  'assessment',
  'collective',
  'reflection',
  'discussion',
  'exercise',
  'example',
  'reference',
  'article',
  'human',
  'role',
] as const;

export type ContentType = typeof CONTENT_TYPES[number];

/**
 * Reach levels - must match REACH_LEVELS in lib.rs
 */
export const REACH_LEVELS = [
  'private',
  'self',
  'intimate',
  'trusted',
  'familiar',
  'community',
  'public',
  'commons',
] as const;

export type ReachLevel = typeof REACH_LEVELS[number];

/**
 * Content formats - all formats used in data/lamad content
 */
export const CONTENT_FORMATS = [
  'markdown',
  'html',
  'plaintext',
  'text',
  'plain',
  'video',
  'audio',
  'interactive',
  'external',
  'gherkin',
  'perseus',
  'perseus-json',
  'perseus-quiz-json',
  'video-embed',
  'audio-file',
  'html5-app',
  'human-json',
  'organization-json',
  'json',
  'sophia',
  'sophia-quiz-json',
] as const;

export type ContentFormat = typeof CONTENT_FORMATS[number];

export const PATH_VISIBILITIES = [
  'private',
  'intimate',
  'unlisted',
  'community',
  'public',
  'draft',
] as const;

export type PathVisibility = typeof PATH_VISIBILITIES[number];

export const STEP_TYPES = [
  'content',
  'read',
  'path',
  'external',
  'practice',
  'assess',
  'video',
  'interactive',
] as const;

export type StepType = typeof STEP_TYPES[number];

/**
 * Mastery levels - must match MASTERY_LEVELS in lib.rs (Bloom's Taxonomy)
 */
export const MASTERY_LEVELS = [
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

export type MasteryLevel = typeof MASTERY_LEVELS[number];

export const COMPLETION_CRITERIA = [
  'all-required',
  'pass-assessment',
  'view-content',
] as const;

export type CompletionCriteria = typeof COMPLETION_CRITERIA[number];

/**
 * Engagement types for mastery tracking
 */
export const ENGAGEMENT_TYPES = [
  'view',
  'quiz',
  'practice',
  'discuss',
  'create',
  'peer',
  'teach',
  'apply',
] as const;

export type EngagementType = typeof ENGAGEMENT_TYPES[number];
