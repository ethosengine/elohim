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
  DOORWAY_TIER,
  ENGAGEMENT_TYPES,
  MASTERY_LEVELS,
  PATH_VISIBILITIES,
  REACH_LEVELS,
  STEP_TYPES,
} from './generated/schema-enums.js';

// Import and re-export types
import type {
  CompletionCriteria as CompletionCriteriaType,
  ContentFormats as ContentFormatsType,
  ContentTypes as ContentTypesType,
  DoorwayTier as DoorwayTierType,
  EngagementTypes as EngagementTypesType,
  MasteryLevels as MasteryLevelsType,
  PathVisibilities as PathVisibilitiesType,
  ReachLevels as ReachLevelsType,
  StepTypes as StepTypesType,
} from './generated/schema-enums.js';

// Export types (using original names from generated file)
export type CompletionCriteria = CompletionCriteriaType;
export type ContentFormats = ContentFormatsType;
export type ContentTypes = ContentTypesType;
export type DoorwayTier = DoorwayTierType;
export type EngagementTypes = EngagementTypesType;
export type MasteryLevels = MasteryLevelsType;
export type PathVisibilities = PathVisibilitiesType;
export type ReachLevels = ReachLevelsType;
export type StepTypes = StepTypesType;

// Type aliases for backward compatibility (singular form)
export type ContentType = ContentTypesType;
export type ContentFormat = ContentFormatsType;
export type ReachLevel = ReachLevelsType;
export type StepType = StepTypesType;
export type MasteryLevel = MasteryLevelsType;
export type PathVisibility = PathVisibilitiesType;
export type EngagementType = EngagementTypesType;
