/**
 * Pre-flight Validators for Seeder
 *
 * These validation functions mirror the Rust validation in:
 * holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs
 *
 * Running validation BEFORE uploading blobs catches issues immediately
 * instead of failing during zome calls after 20+ minutes of seeding.
 *
 * Usage:
 *   import { validateBatch, validateContent } from './validators';
 *
 *   const result = validateBatch(items);
 *   if (result.totalInvalid > 0) {
 *     // Handle invalid items
 *   }
 */

import {
  CONTENT_TYPES,
  CONTENT_FORMATS,
  REACH_LEVELS,
  PATH_VISIBILITIES,
  STEP_TYPES,
  MASTERY_LEVELS,
  ENGAGEMENT_TYPES,
} from './validation-constants.js';

// =============================================================================
// Input Types (matching seed.ts CreateContentInput)
// =============================================================================

export interface CreateContentInput {
  id: string;
  content_type: string;
  title: string;
  description: string;
  summary: string | null;
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  reach: string;
  estimated_minutes: number | null;
  thumbnail_url: string | null;
  metadata_json: string;
}

export interface CreateLearningPathInput {
  id: string;
  title: string;
  description: string;
  created_by: string;
  visibility: string;
  tags: string[];
}

export interface CreatePathStepInput {
  id: string;
  path_id: string;
  resource_id: string;
  step_type: string;
  order_index: number;
  completion_criteria?: string;
}

export interface CreateContentMasteryInput {
  id: string;
  human_id: string;
  content_id: string;
  mastery_level: string;
  mastery_level_index: number;
  freshness_score: number;
  last_engagement_type: string;
}

// =============================================================================
// Validation Result Types
// =============================================================================

export interface ValidationResult {
  valid: boolean;
  errors: string[];
}

export interface BatchValidationResult<T> {
  validItems: T[];
  invalidItems: { item: T; errors: string[] }[];
  totalValid: number;
  totalInvalid: number;
}

// =============================================================================
// Content Validation
// =============================================================================

/**
 * Validate a single content item
 *
 * Mirrors Rust validation in healing.rs:151-197
 */
export function validateContent(content: CreateContentInput): ValidationResult {
  const errors: string[] = [];

  // Required fields (deterministic)
  if (!content.id || content.id.trim() === '') {
    errors.push('Content id is required');
  }
  if (!content.title || content.title.trim() === '') {
    errors.push('Content title is required');
  }
  if (!content.content_type || content.content_type.trim() === '') {
    errors.push('Content type is required');
  }

  // Enum validation (deterministic)
  if (content.content_type && !CONTENT_TYPES.includes(content.content_type as any)) {
    errors.push(
      `Invalid content_type '${content.content_type}'. Must be one of: ${CONTENT_TYPES.join(', ')}`
    );
  }

  if (content.reach && !REACH_LEVELS.includes(content.reach as any)) {
    errors.push(
      `Invalid reach '${content.reach}'. Must be one of: ${REACH_LEVELS.join(', ')}`
    );
  }

  if (content.content_format && !CONTENT_FORMATS.includes(content.content_format as any)) {
    errors.push(
      `Invalid content_format '${content.content_format}'. Must be one of: ${CONTENT_FORMATS.join(', ')}`
    );
  }

  // Related ID validation (structure only, not existence)
  for (const relatedId of content.related_node_ids || []) {
    if (!relatedId || relatedId.trim() === '') {
      errors.push('Related content ID cannot be empty');
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

// =============================================================================
// LearningPath Validation
// =============================================================================

/**
 * Validate a learning path
 *
 * Mirrors Rust validation in healing.rs:227-252
 */
export function validateLearningPath(path: CreateLearningPathInput): ValidationResult {
  const errors: string[] = [];

  // Required fields
  if (!path.id || path.id.trim() === '') {
    errors.push('LearningPath id is required');
  }
  if (!path.title || path.title.trim() === '') {
    errors.push('LearningPath title is required');
  }
  if (!path.created_by || path.created_by.trim() === '') {
    errors.push('LearningPath created_by is required');
  }

  // Enum validation
  if (path.visibility && !PATH_VISIBILITIES.includes(path.visibility as any)) {
    errors.push(
      `Invalid visibility '${path.visibility}'. Must be one of: ${PATH_VISIBILITIES.join(', ')}`
    );
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

// =============================================================================
// PathStep Validation
// =============================================================================

/**
 * Validate a path step
 *
 * Mirrors Rust validation in healing.rs:282-323
 */
export function validatePathStep(step: CreatePathStepInput): ValidationResult {
  const errors: string[] = [];

  // Required fields
  if (!step.id || step.id.trim() === '') {
    errors.push('PathStep id is required');
  }
  if (!step.path_id || step.path_id.trim() === '') {
    errors.push('PathStep path_id is required');
  }
  if (!step.resource_id || step.resource_id.trim() === '') {
    errors.push('PathStep resource_id is required');
  }

  // Enum validation
  if (step.step_type && !STEP_TYPES.includes(step.step_type as any)) {
    errors.push(
      `Invalid step_type '${step.step_type}'. Must be one of: ${STEP_TYPES.join(', ')}`
    );
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

// =============================================================================
// ContentMastery Validation
// =============================================================================

/**
 * Validate content mastery
 *
 * Mirrors Rust validation in healing.rs:353-402
 */
export function validateContentMastery(mastery: CreateContentMasteryInput): ValidationResult {
  const errors: string[] = [];

  // Required fields
  if (!mastery.id || mastery.id.trim() === '') {
    errors.push('ContentMastery id is required');
  }
  if (!mastery.human_id || mastery.human_id.trim() === '') {
    errors.push('ContentMastery human_id is required');
  }
  if (!mastery.content_id || mastery.content_id.trim() === '') {
    errors.push('ContentMastery content_id is required');
  }

  // Enum validation
  if (mastery.mastery_level && !MASTERY_LEVELS.includes(mastery.mastery_level as any)) {
    errors.push(
      `Invalid mastery_level '${mastery.mastery_level}'. Must be one of: ${MASTERY_LEVELS.join(', ')}`
    );
  }

  // Range validation
  if (mastery.freshness_score < 0.0 || mastery.freshness_score > 1.0) {
    errors.push(
      `freshness_score ${mastery.freshness_score} out of range (0.0-1.0)`
    );
  }

  // Engagement type validation
  if (mastery.last_engagement_type && !ENGAGEMENT_TYPES.includes(mastery.last_engagement_type as any)) {
    errors.push(
      `Invalid last_engagement_type '${mastery.last_engagement_type}'. Must be one of: ${ENGAGEMENT_TYPES.join(', ')}`
    );
  }

  // Cross-field validation: mastery_level_index should match mastery_level
  if (mastery.mastery_level) {
    const expectedIndex = MASTERY_LEVELS.indexOf(mastery.mastery_level as any);
    if (expectedIndex !== -1 && mastery.mastery_level_index !== expectedIndex) {
      errors.push(
        `mastery_level_index ${mastery.mastery_level_index} doesn't match mastery_level '${mastery.mastery_level}' (expected ${expectedIndex})`
      );
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

// =============================================================================
// Batch Validation
// =============================================================================

/**
 * Validate a batch of content items
 *
 * Returns separate arrays of valid and invalid items.
 * Invalid items include their error messages for logging.
 */
export function validateBatch(items: CreateContentInput[]): BatchValidationResult<CreateContentInput> {
  const validItems: CreateContentInput[] = [];
  const invalidItems: { item: CreateContentInput; errors: string[] }[] = [];

  for (const item of items) {
    const result = validateContent(item);
    if (result.valid) {
      validItems.push(item);
    } else {
      invalidItems.push({ item, errors: result.errors });
    }
  }

  return {
    validItems,
    invalidItems,
    totalValid: validItems.length,
    totalInvalid: invalidItems.length,
  };
}

/**
 * Validate a batch of learning paths
 */
export function validatePathBatch(items: CreateLearningPathInput[]): BatchValidationResult<CreateLearningPathInput> {
  const validItems: CreateLearningPathInput[] = [];
  const invalidItems: { item: CreateLearningPathInput; errors: string[] }[] = [];

  for (const item of items) {
    const result = validateLearningPath(item);
    if (result.valid) {
      validItems.push(item);
    } else {
      invalidItems.push({ item, errors: result.errors });
    }
  }

  return {
    validItems,
    invalidItems,
    totalValid: validItems.length,
    totalInvalid: invalidItems.length,
  };
}

/**
 * Validate a batch of path steps
 */
export function validateStepBatch(items: CreatePathStepInput[]): BatchValidationResult<CreatePathStepInput> {
  const validItems: CreatePathStepInput[] = [];
  const invalidItems: { item: CreatePathStepInput; errors: string[] }[] = [];

  for (const item of items) {
    const result = validatePathStep(item);
    if (result.valid) {
      validItems.push(item);
    } else {
      invalidItems.push({ item, errors: result.errors });
    }
  }

  return {
    validItems,
    invalidItems,
    totalValid: validItems.length,
    totalInvalid: invalidItems.length,
  };
}

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Log validation errors to console
 */
export function logValidationErrors(
  invalidItems: { item: { id: string }; errors: string[] }[],
  maxToShow: number = 10
): void {
  const count = invalidItems.length;
  const showCount = Math.min(count, maxToShow);

  for (let i = 0; i < showCount; i++) {
    const { item, errors } = invalidItems[i];
    console.warn(`   - ${item.id}: ${errors.join('; ')}`);
  }

  if (count > maxToShow) {
    console.warn(`   ... and ${count - maxToShow} more`);
  }
}

/**
 * Check if STRICT_VALIDATION environment variable is set
 */
export function isStrictValidation(): boolean {
  return process.env.STRICT_VALIDATION === 'true';
}
