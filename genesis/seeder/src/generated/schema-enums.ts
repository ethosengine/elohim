// AUTO-GENERATED from healing.rs - DO NOT EDIT
// Generated at: 2026-02-25T15:22:03.020Z
// Source: holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs

/**
 * Content types - extended to support all imported content
 */
export const CONTENT_TYPES = [
  'epic',
  'concept',
  'lesson',
  'scenario',
  'assessment',
  'resource',
  'reflection',
  'discussion',
  'exercise',
  'example',
  'reference',
  'article',
  'feature',
  'practice',
  'human',
  'organization',
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
  'role',
  'simulation',
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
  'intimate',   // Mutual-attestation paths (e.g. love-map)
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
