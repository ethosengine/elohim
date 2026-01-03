/**
 * Validation Constants for Seeder Pre-flight Validation
 *
 * These constants MUST match the Rust validation in:
 * holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs
 *
 * Keep these in sync to ensure pre-flight validation catches the same
 * issues that zome validation would catch.
 */

/**
 * Content types - extended to support all imported content
 * Source: healing.rs CONTENT_TYPES
 */
export const CONTENT_TYPES = [
  // Core types (from lib.rs)
  'epic',           // High-level narrative/vision document
  'concept',        // Atomic knowledge unit
  'lesson',         // Digestible learning session (AI-derived from concepts)
  'scenario',       // Gherkin feature/scenario
  'assessment',     // Quiz or test
  'resource',       // Supporting material
  'reflection',     // Journaling/reflection prompt
  'discussion',     // Discussion topic
  'exercise',       // Practice activity
  'example',        // Illustrative example
  'reference',      // Reference material
  'article',        // Long-form article content
  'feature',        // Gherkin feature (imported from .feature files)
  'practice',       // Practice activity (legacy alias)
  'human',          // Human persona files
  // Extended types (from FCT and other imports)
  'organization',   // Organization reference
  'contributor',    // Contributor profile
  'video',          // Video content reference
  'audio',          // Audio content reference
  'book',           // Book reference
  'book-chapter',   // Book chapter reference
  'documentary',    // Documentary reference
  'bible-verse',    // Biblical scripture reference
  'activity',       // Learning activity
  'narrative',      // Narrative/story content
  'course-module',  // Course module structure
  'module',         // Generic module
  'quiz',           // Quiz content (distinct from assessment)
] as const;

/**
 * Reach levels - graduated visibility in the network
 * Source: healing.rs REACH_LEVELS
 */
export const REACH_LEVELS = [
  'private',    // Only self
  'self',       // Only self (alias)
  'intimate',   // Closest relationships
  'trusted',    // Trusted circle
  'familiar',   // Extended network
  'community',  // Community members
  'public',     // Anyone authenticated
  'commons',    // Anyone, including anonymous
] as const;

/**
 * Content formats - all formats used in data/lamad content
 * Source: healing.rs CONTENT_FORMATS
 */
export const CONTENT_FORMATS = [
  'markdown',          // Markdown format
  'html',              // HTML format
  'plaintext',         // Plain text
  'text',              // Plain text (alias)
  'plain',             // Plain text (alias)
  'video',             // Video media reference
  'audio',             // Audio media reference
  'interactive',       // Interactive content
  'external',          // External URL reference
  'gherkin',           // Gherkin/Cucumber scenario format
  'perseus',           // Perseus quiz/assessment format (canonical)
  'perseus-json',      // Perseus format (alias)
  'perseus-quiz-json', // Perseus quiz format (self-documenting)
  'video-embed',       // Embedded video (YouTube, Vimeo, etc.)
  'audio-file',        // Audio file reference
  'html5-app',         // HTML5 interactive application
  'human-json',        // Human persona JSON format
  'organization-json', // Organization JSON format
  'json',              // Generic JSON format
] as const;

/**
 * Step types for PathStep validation
 * Source: healing.rs STEP_TYPES
 */
export const STEP_TYPES = [
  'content',     // Regular content step
  'read',        // Reading content
  'path',        // Nested path reference
  'external',    // External resource
  'practice',    // Practice exercise
  'assess',      // Assessment step
  'video',       // Video content
  'interactive', // Interactive content
] as const;

/**
 * Mastery levels - Bloom's Taxonomy based progression
 * Source: healing.rs MASTERY_LEVELS
 */
export const MASTERY_LEVELS = [
  'not_started', // 0 - No engagement
  'seen',        // 1 - Content viewed
  'remember',    // 2 - Basic recall demonstrated
  'understand',  // 3 - Comprehension demonstrated
  'apply',       // 4 - Application in novel contexts (ATTESTATION GATE)
  'analyze',     // 5 - Can break down, connect, contribute analysis
  'evaluate',    // 6 - Can assess, critique, peer review
  'create',      // 7 - Can author, derive, synthesize
  // Additional levels from healing.rs
  'recognize',   // Alternative level
  'recall',      // Alternative level
  'synthesize',  // Alternative level
] as const;

/**
 * Path visibility types
 * Source: healing.rs PATH_VISIBILITIES
 */
export const PATH_VISIBILITIES = [
  'private',   // Only creator
  'unlisted',  // Accessible by link
  'community', // Community members
  'public',    // Anyone
] as const;

/**
 * Engagement types for mastery tracking
 * Source: healing.rs ENGAGEMENT_TYPES
 */
export const ENGAGEMENT_TYPES = [
  'view',       // Passive viewing
  'quiz',       // Took assessment
  'practice',   // Practice exercise
  'discuss',    // Participated in discussion
  'create',     // Created content
  'peer',       // Peer interaction
  'teach',      // Teaching/mentoring
  'apply',      // Real-world application
] as const;

/**
 * Completion criteria for path steps
 * Source: healing.rs COMPLETION_CRITERIA
 */
export const COMPLETION_CRITERIA = [
  'view',           // Simply viewed
  'time_spent',     // Spent minimum time
  'quiz_passed',    // Passed quiz
  'practice_done',  // Completed practice
  'attestation',    // Got attestation
  'self_report',    // Self-reported completion
] as const;

// Type exports for TypeScript type safety
export type ContentType = typeof CONTENT_TYPES[number];
export type ReachLevel = typeof REACH_LEVELS[number];
export type ContentFormat = typeof CONTENT_FORMATS[number];
export type StepType = typeof STEP_TYPES[number];
export type MasteryLevel = typeof MASTERY_LEVELS[number];
export type PathVisibility = typeof PATH_VISIBILITIES[number];
export type EngagementType = typeof ENGAGEMENT_TYPES[number];
export type CompletionCriteria = typeof COMPLETION_CRITERIA[number];
