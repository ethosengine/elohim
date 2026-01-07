/**
 * Validation Constants for Seeder Pre-flight Validation
 *
 * These constants are AUTO-GENERATED from the Rust DNA schema.
 * Source: holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs
 *
 * To regenerate:
 *   1. Run hc-rna-schema with --export-enums --constants-file healing.rs
 *   2. Run generate-schema-types.ts
 *
 * DO NOT EDIT MANUALLY - changes will be overwritten.
 */

// Re-export all generated constants as the single source of truth
export {
  COMPLETION_CRITERIA,
  CONTENT_FORMATS,
  CONTENT_TYPES,
  ENGAGEMENT_TYPES,
  MASTERY_LEVELS,
  PATH_VISIBILITIES,
  REACH_LEVELS,
  STEP_TYPES,
} from './generated/schema-enums.js';

// Re-export types from generated file
export type {
  CompletionCriteria,
  ContentFormat,
  ContentType,
  EngagementType,
  MasteryLevel,
  PathVisibility,
  ReachLevel,
  StepType,
} from './generated/schema-enums.js';

// Type aliases for backward compatibility (plural form)
import type {
  CompletionCriteria as CompletionCriteriaType,
  ContentFormat as ContentFormatType,
  ContentType as ContentTypeAlias,
  EngagementType as EngagementTypeAlias,
  MasteryLevel as MasteryLevelAlias,
  PathVisibility as PathVisibilityAlias,
  ReachLevel as ReachLevelAlias,
  StepType as StepTypeAlias,
} from './generated/schema-enums.js';

export type ContentFormats = ContentFormatType;
export type ContentTypes = ContentTypeAlias;
export type EngagementTypes = EngagementTypeAlias;
export type MasteryLevels = MasteryLevelAlias;
export type PathVisibilities = PathVisibilityAlias;
export type ReachLevels = ReachLevelAlias;
export type StepTypes = StepTypeAlias;
