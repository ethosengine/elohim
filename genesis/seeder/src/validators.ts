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
  contentType: string;
  title: string;
  description: string;
  summary: string | null;
  content: string;
  contentFormat: string;
  tags: string[];
  sourcePath: string | null;
  relatedNodeIds: string[];
  reach: string;
  estimatedMinutes: number | null;
  thumbnailUrl: string | null;
  metadataJson: string;
  // Content manifest fields (sparse DHT - blob storage)
  blobCid: string | null;           // CID pointing to elohim-storage blob
  contentSizeBytes: number | null; // Size of content body
  contentHash: string | null;       // SHA256 of content body
  blobHash?: string;                // SHA256 hash of ZIP blob for html5-app content
}

export interface CreateLearningPathInput {
  id: string;
  title: string;
  description: string;
  createdBy: string;
  visibility: string;
  tags: string[];
}

export interface CreatePathStepInput {
  id: string;
  pathId: string;
  resourceId: string;
  stepType: string;
  orderIndex: number;
  completionCriteria?: string;
}

export interface CreateContentMasteryInput {
  id: string;
  humanId: string;
  contentId: string;
  masteryLevel: string;
  masteryLevelIndex: number;
  freshnessScore: number;
  lastEngagementType: string;
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
  if (!content.contentType || content.contentType.trim() === '') {
    errors.push('Content type is required');
  }

  // Enum validation (deterministic)
  if (content.contentType && !CONTENT_TYPES.includes(content.contentType as any)) {
    errors.push(
      `Invalid contentType '${content.contentType}'. Must be one of: ${CONTENT_TYPES.join(', ')}`
    );
  }

  if (content.reach && !REACH_LEVELS.includes(content.reach as any)) {
    errors.push(
      `Invalid reach '${content.reach}'. Must be one of: ${REACH_LEVELS.join(', ')}`
    );
  }

  if (content.contentFormat && !CONTENT_FORMATS.includes(content.contentFormat as any)) {
    errors.push(
      `Invalid contentFormat '${content.contentFormat}'. Must be one of: ${CONTENT_FORMATS.join(', ')}`
    );
  }

  // Related ID validation (structure only, not existence)
  for (const relatedId of content.relatedNodeIds || []) {
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
  if (!path.createdBy || path.createdBy.trim() === '') {
    errors.push('LearningPath createdBy is required');
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
  if (!step.pathId || step.pathId.trim() === '') {
    errors.push('PathStep pathId is required');
  }
  if (!step.resourceId || step.resourceId.trim() === '') {
    errors.push('PathStep resourceId is required');
  }

  // Enum validation
  if (step.stepType && !STEP_TYPES.includes(step.stepType as any)) {
    errors.push(
      `Invalid stepType '${step.stepType}'. Must be one of: ${STEP_TYPES.join(', ')}`
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
  if (!mastery.humanId || mastery.humanId.trim() === '') {
    errors.push('ContentMastery humanId is required');
  }
  if (!mastery.contentId || mastery.contentId.trim() === '') {
    errors.push('ContentMastery contentId is required');
  }

  // Enum validation
  if (mastery.masteryLevel && !MASTERY_LEVELS.includes(mastery.masteryLevel as any)) {
    errors.push(
      `Invalid masteryLevel '${mastery.masteryLevel}'. Must be one of: ${MASTERY_LEVELS.join(', ')}`
    );
  }

  // Range validation
  if (mastery.freshnessScore < 0.0 || mastery.freshnessScore > 1.0) {
    errors.push(
      `freshnessScore ${mastery.freshnessScore} out of range (0.0-1.0)`
    );
  }

  // Engagement type validation
  if (mastery.lastEngagementType && !ENGAGEMENT_TYPES.includes(mastery.lastEngagementType as any)) {
    errors.push(
      `Invalid lastEngagementType '${mastery.lastEngagementType}'. Must be one of: ${ENGAGEMENT_TYPES.join(', ')}`
    );
  }

  // Cross-field validation: masteryLevelIndex should match mastery_level
  if (mastery.masteryLevel) {
    const expectedIndex = MASTERY_LEVELS.indexOf(mastery.masteryLevel as any);
    if (expectedIndex !== -1 && mastery.masteryLevelIndex !== expectedIndex) {
      errors.push(
        `masteryLevelIndex ${mastery.masteryLevelIndex} doesn't match masteryLevel '${mastery.masteryLevel}' (expected ${expectedIndex})`
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
