/**
 * Holochain Types for elohim-service CLI
 *
 * These types match the Rust structures in the lamad DNA.
 * Used for communication between Node.js CLI and Holochain conductor.
 */

// =============================================================================
// Content Types - Match Rust Content struct
// =============================================================================

/**
 * Input for creating content via zome call
 * Matches CreateContentInput in coordinator zome
 */
export interface CreateContentInput {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  reach: string;
  metadata_json: string;
}

/**
 * Content entry as stored in Holochain
 * Matches Content struct in integrity zome
 */
export interface HolochainContent {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  author_id: string | null;
  reach: string;
  trust_score: number;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/**
 * Output from content retrieval zome calls
 * Matches ContentOutput in coordinator zome
 */
export interface HolochainContentOutput {
  action_hash: Uint8Array;
  entry_hash: Uint8Array;
  content: HolochainContent;
}

// =============================================================================
// Bulk Import Types
// =============================================================================

/**
 * Input for bulk content creation
 * Matches BulkCreateContentInput in coordinator zome
 */
export interface BulkCreateContentInput {
  import_id: string;
  contents: CreateContentInput[];
}

/**
 * Output from bulk content creation
 * Matches BulkCreateContentOutput in coordinator zome
 */
export interface BulkCreateContentOutput {
  import_id: string;
  created_count: number;
  action_hashes: Uint8Array[];
  errors: string[];
}

// =============================================================================
// Query Types
// =============================================================================

/**
 * Input for querying content by type
 */
export interface QueryByTypeInput {
  content_type: string;
  limit?: number;
}

/**
 * Input for querying content by ID
 */
export interface QueryByIdInput {
  id: string;
}

/**
 * Content statistics from get_content_stats
 */
export interface ContentStats {
  total_count: number;
  by_type: Record<string, number>;
}

// =============================================================================
// Learning Path Types
// =============================================================================

/**
 * Input for creating a learning path
 */
export interface CreatePathInput {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose?: string;
  difficulty: string;
  estimated_duration?: string;
  visibility: string;
  path_type: string;
  tags: string[];
}

/**
 * Input for adding a step to a path
 */
export interface AddPathStepInput {
  path_id: string;
  order_index: number;
  step_type: string;
  resource_id: string;
  step_title?: string;
  step_narrative?: string;
  is_optional: boolean;
}

/**
 * Learning path as stored in Holochain
 */
export interface HolochainLearningPath {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose: string | null;
  created_by: string;
  difficulty: string;
  estimated_duration: string | null;
  visibility: string;
  path_type: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

/**
 * Path step as stored in Holochain
 */
export interface HolochainPathStep {
  id: string;
  path_id: string;
  order_index: number;
  step_type: string;
  resource_id: string;
  step_title: string | null;
  step_narrative: string | null;
  is_optional: boolean;
}

/**
 * Output for path step retrieval
 */
export interface PathStepOutput {
  action_hash: Uint8Array;
  step: HolochainPathStep;
}

/**
 * Output for path with steps retrieval
 */
export interface PathWithSteps {
  action_hash: Uint8Array;
  path: HolochainLearningPath;
  steps: PathStepOutput[];
}

// =============================================================================
// Relationship Types
// =============================================================================

/**
 * Content relationship as stored in Holochain
 */
export interface HolochainContentRelationship {
  id: string;
  source_node_id: string;
  target_node_id: string;
  relationship_type: string;
  confidence: number;
  metadata_json: string | null;
}

// =============================================================================
// Client Configuration Types
// =============================================================================

/**
 * Configuration for Holochain client connection
 */
export interface HolochainClientConfig {
  /** WebSocket URL for admin interface (e.g., ws://localhost:4444 or wss://holochain-dev.elohim.host) */
  adminUrl: string;
  /** Installed app ID (e.g., elohim) */
  appId: string;
  /** Optional path to .happ file for installation */
  happPath?: string;
}

/**
 * Configuration for import operations
 */
export interface HolochainImportConfig extends HolochainClientConfig {
  /** Number of entries per bulk create call */
  batchSize: number;
}

/**
 * Result of an import operation
 */
export interface HolochainImportResult {
  /** Total nodes attempted */
  totalNodes: number;
  /** Successfully created nodes */
  createdNodes: number;
  /** Error messages */
  errors: string[];
  /** Unique import ID for this batch */
  importId: string;
  /** Time taken in milliseconds */
  durationMs?: number;
}

/**
 * Result of a verify operation
 */
export interface HolochainVerifyResult {
  /** IDs that were found */
  found: string[];
  /** IDs that were missing */
  missing: string[];
}

// =============================================================================
// Zome Call Types
// =============================================================================

/**
 * Generic zome call input
 */
export interface ZomeCallInput {
  zomeName: string;
  fnName: string;
  payload: unknown;
}

/**
 * Generic zome call result
 */
export interface ZomeCallResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// =============================================================================
// Constants
// =============================================================================

/** Valid content types matching Rust validation */
export const VALID_CONTENT_TYPES = [
  'source',
  'epic',
  'feature',
  'scenario',
  'concept',
  'role',
  'video',
  'organization',
  'book-chapter',
  'tool',
  'path',
  'assessment',
  'reference',
  'example',
] as const;

/** Valid content formats matching Rust validation */
export const VALID_CONTENT_FORMATS = [
  'markdown',
  'gherkin',
  'html',
  'plaintext',
  'video-embed',
  'external-link',
  'quiz-json',
  'assessment-json',
] as const;

/** Valid reach levels matching Rust validation */
export const VALID_REACH_LEVELS = [
  'private',
  'invited',
  'local',
  'community',
  'federated',
  'commons',
] as const;

/** Valid relationship types matching Rust validation */
export const VALID_RELATIONSHIP_TYPES = [
  'CONTAINS',
  'BELONGS_TO',
  'DESCRIBES',
  'IMPLEMENTS',
  'VALIDATES',
  'RELATES_TO',
  'REFERENCES',
  'DEPENDS_ON',
  'REQUIRES',
  'FOLLOWS',
  'DERIVED_FROM',
  'SOURCE_OF',
] as const;

/** Valid difficulty levels for learning paths */
export const VALID_DIFFICULTY_LEVELS = [
  'beginner',
  'intermediate',
  'advanced',
] as const;

// Type aliases from constants
export type ValidContentType = typeof VALID_CONTENT_TYPES[number];
export type ValidContentFormat = typeof VALID_CONTENT_FORMATS[number];
export type ValidReachLevel = typeof VALID_REACH_LEVELS[number];
export type ValidRelationshipType = typeof VALID_RELATIONSHIP_TYPES[number];
export type ValidDifficultyLevel = typeof VALID_DIFFICULTY_LEVELS[number];
