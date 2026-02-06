/**
 * Zod Schemas for Seed Data Validation
 *
 * These schemas align with Holochain entry types for lamad content.
 */

import { z } from 'zod';

/**
 * Relationship types for content graph edges
 * Aligned with Holochain DNA Relationship.relationship_type
 */
export const relationshipTypeEnum = z.enum([
  'RELATES_TO',   // General association
  'CONTAINS',     // Parent-child hierarchical
  'DEPENDS_ON',   // Prerequisite dependency
  'IMPLEMENTS',   // Implementation of concept
  'REFERENCES',   // Citation/reference
  'DERIVED_FROM', // This content was derived from source content
]);

export type RelationshipType = z.infer<typeof relationshipTypeEnum>;

/**
 * Content types aligned with Holochain DNA CONTENT_TYPES
 */
export const contentTypeEnum = z.enum([
  'epic',        // High-level narrative/vision document
  'concept',     // Atomic knowledge unit
  'lesson',      // Digestible learning session (AI-derived)
  'scenario',    // Gherkin feature/scenario
  'assessment',  // Quiz or test
  'resource',    // Supporting material
  'reflection',  // Journaling/reflection prompt
  'discussion',  // Discussion topic
  'exercise',    // Practice activity
  'example',     // Illustrative example
  'reference',   // Reference material
  'article',     // Long-form article content
]);

/**
 * Content formats aligned with Holochain DNA CONTENT_FORMATS
 */
export const contentFormatEnum = z.enum([
  'markdown',
  'html',
  'video',
  'audio',
  'interactive',
  'external',
]);

export type ContentType = z.infer<typeof contentTypeEnum>;
export type ContentFormat = z.infer<typeof contentFormatEnum>;

/**
 * Content/Concept Schema - Atomic unit of knowledge
 * Aligned with Holochain DNA Content struct
 */
export const conceptSchema = z.object({
  id: z.string().regex(/^[a-z0-9-]+$/, 'ID must be kebab-case'),
  title: z.string().min(1),
  description: z.string().optional(),
  summary: z.string().optional(),              // Short preview text for cards/lists (AI-generated)
  content: z.string().min(1),
  contentFormat: contentFormatEnum.default('markdown'),
  contentType: contentTypeEnum.optional(),
  sourceDoc: z.string().optional(),
  tags: z.array(z.string()).default([]),
  relationships: z.array(z.object({
    target: z.string(),
    type: relationshipTypeEnum,
  })).default([]),
  // Attention metadata (complexity is learner-relative, computed by AI, not stored)
  estimatedMinutes: z.number().positive().optional(),
  thumbnailUrl: z.string().url().optional(),   // Preview image for visual cards
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.string().datetime().optional(),
  updatedAt: z.string().datetime().optional(),
});

export type Concept = z.infer<typeof conceptSchema>;

/**
 * Content Schema - Alias for concept (legacy compatibility)
 */
export const contentSchema = conceptSchema;
export type Content = Concept;

/**
 * Section Schema - Group of concepts
 */
export const sectionSchema = z.object({
  id: z.string().regex(/^[a-z0-9-]+$/),
  title: z.string().min(1),
  description: z.string().optional(),
  conceptIds: z.array(z.string()).default([]),
  order: z.number().int().nonnegative().optional(),
});

export type Section = z.infer<typeof sectionSchema>;

/**
 * Module Schema - Group of sections
 */
export const moduleSchema = z.object({
  id: z.string().regex(/^[a-z0-9-]+$/),
  title: z.string().min(1),
  description: z.string().optional(),
  sections: z.array(sectionSchema).default([]),
  order: z.number().int().nonnegative().optional(),
});

export type Module = z.infer<typeof moduleSchema>;

/**
 * Chapter Schema - Group of modules
 */
export const chapterSchema = z.object({
  id: z.string().regex(/^[a-z0-9-]+$/),
  title: z.string().min(1),
  description: z.string().optional(),
  modules: z.array(moduleSchema).default([]),
  order: z.number().int().nonnegative().optional(),
});

export type Chapter = z.infer<typeof chapterSchema>;

/**
 * Path Schema - Ordered traversal through content graph
 */
export const pathSchema = z.object({
  id: z.string().regex(/^[a-z0-9-]+$/),
  title: z.string().min(1),
  description: z.string().optional(),
  difficulty: z.enum(['beginner', 'intermediate', 'advanced']).optional(),
  estimatedDuration: z.string().optional(),
  chapters: z.array(chapterSchema).default([]),
  // Flat concept list for simple paths without hierarchy
  conceptIds: z.array(z.string()).optional(),
  tags: z.array(z.string()).default([]),
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.string().datetime().optional(),
  updatedAt: z.string().datetime().optional(),
});

export type Path = z.infer<typeof pathSchema>;

/**
 * Question Schema - Single assessment question
 */
export const questionSchema = z.object({
  id: z.string().regex(/^[a-z0-9-]+$/),
  type: z.enum(['multiple-choice', 'true-false', 'short-answer', 'essay', 'matching']),
  question: z.string().min(1),
  options: z.array(z.string()).optional(),
  correctAnswer: z.union([z.string(), z.array(z.string())]).optional(),
  explanation: z.string().optional(),
  conceptId: z.string().optional(),
  difficulty: z.enum(['easy', 'medium', 'hard']).optional(),
  points: z.number().positive().default(1),
});

export type Question = z.infer<typeof questionSchema>;

/**
 * Assessment Schema - Collection of questions
 */
export const assessmentSchema = z.object({
  id: z.string().regex(/^[a-z0-9-]+$/),
  title: z.string().min(1),
  description: z.string().optional(),
  type: z.enum(['diagnostic', 'formative', 'summative', 'quiz']),
  questions: z.array(questionSchema).default([]),
  conceptIds: z.array(z.string()).default([]),
  passingScore: z.number().min(0).max(100).optional(),
  timeLimit: z.number().positive().optional(),
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.string().datetime().optional(),
  updatedAt: z.string().datetime().optional(),
});

export type Assessment = z.infer<typeof assessmentSchema>;

/**
 * Relationship Schema - Edge in content graph
 */
export const relationshipSchema = z.object({
  source: z.string(),
  target: z.string(),
  type: relationshipTypeEnum,
  metadata: z.record(z.unknown()).optional(),
});

export type Relationship = z.infer<typeof relationshipSchema>;
