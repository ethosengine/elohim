// AUTO-GENERATED from protocol JSON schemas.
// DO NOT EDIT — regenerate with: pnpm run schema:codegen:ts
//
// Source: elohim/sdk/schemas/v1/enums/*.schema.json

export const CORE_COMPLETION_CRITERIA = [
  'all-required',
  'pass-assessment',
  'view-content',
] as const;
export const ALL_COMPLETION_CRITERIA = [
  'all-required',
  'pass-assessment',
  'view-content',
] as const;
export const COMPLETION_CRITERIA = ALL_COMPLETION_CRITERIA;
export type CompletionCriteria = (typeof ALL_COMPLETION_CRITERIA)[number];

export const CORE_CONTENT_FORMATS = [
  'markdown',
  'html',
  'video',
  'audio',
  'interactive',
  'external',
] as const;
export const ALL_CONTENT_FORMATS = [
  'markdown',
  'html',
  'video',
  'audio',
  'interactive',
  'external',
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
] as const;
export const CONTENT_TYPES = ALL_CONTENT_TYPES;
export type ContentType = (typeof ALL_CONTENT_TYPES)[number];

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

export const CORE_PATH_VISIBILITIES = [
  'private',
  'unlisted',
  'community',
  'public',
] as const;
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

export const CORE_STEP_TYPES = [
  'content',
  'path',
  'external',
  'checkpoint',
  'reflection',
] as const;
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

