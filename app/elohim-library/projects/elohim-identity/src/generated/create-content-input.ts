/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: inputs/create-content-input.schema.json -- DO NOT EDIT */

/**
 * Semantic content category
 */
export type ContentType =
  | 'epic'
  | 'concept'
  | 'lesson'
  | 'scenario'
  | 'assessment'
  | 'reflection'
  | 'discussion'
  | 'exercise'
  | 'article'
  | 'path'
  | 'human'
  | 'role'
  | 'collective'
  | 'example'
  | 'reference'
  | 'feature'
  | 'practice'
  | 'contributor'
  | 'video'
  | 'audio'
  | 'book'
  | 'book-chapter'
  | 'documentary'
  | 'bible-verse'
  | 'activity'
  | 'narrative'
  | 'course-module'
  | 'module'
  | 'quiz'
  | 'podcast'
  | 'simulation'
  | 'node-context'
  | 'stewardship-context'
  | 'work-story'
  | 'work-project'
  | 'issue-report'
  | 'application'
  | 'gate-process-declaration'
  | 'universal-band-declaration'
  | 'gate-rules-declaration'
  | 'aggregation-spec'
  | 'escalation-target-spec'
  | 'element-registry';
/**
 * Rendering format hint
 */
export type ContentFormat =
  | 'markdown'
  | 'html'
  | 'video'
  | 'audio'
  | 'interactive'
  | 'external'
  | 'epr-composite'
  | 'plaintext'
  | 'text'
  | 'plain'
  | 'gherkin'
  | 'perseus'
  | 'perseus-json'
  | 'perseus-quiz-json'
  | 'video-embed'
  | 'audio-file'
  | 'html5-app'
  | 'spa-bundle'
  | 'human-json'
  | 'organization-json'
  | 'json'
  | 'sophia'
  | 'sophia-quiz-json'
  | 'element-registry-manifest';
/**
 * Visibility/access level
 */
export type Reach = 'private' | 'self' | 'intimate' | 'trusted' | 'familiar' | 'community' | 'public' | 'commons';

/**
 * Input for creating content records. Must match Rust CreateContentInputView in views.rs.
 */
export interface CreateContentInput {
  /**
   * Unique content identifier
   */
  id: string;
  /**
   * Display title
   */
  title: string;
  /**
   * Schema version for migration tracking
   */
  schemaVersion?: number;
  /**
   * Brief summary
   */
  description?: string;
  contentType?: ContentType;
  contentFormat?: ContentFormat;
  /**
   * Full content body
   */
  contentBody?: string;
  /**
   * SHA-256 hash of associated blob
   */
  blobHash?: string;
  /**
   * CIDv1 content address of associated blob
   */
  blobCid?: string;
  /**
   * Size of content/blob in bytes
   */
  contentSizeBytes?: number;
  /**
   * Domain-specific metadata
   */
  metadata?: Record<string, unknown>;
  reach?: Reach;
  /**
   * Creator agent ID
   */
  createdBy?: string;
  /**
   * Categorization tags
   */
  tags?: string[];
  /**
   * Content-derived provenance anchor set at ingest (seed/import) so the row satisfies the require_provenance read gate on hub-optional/peer-starved stacks where the libp2p publish drain never runs. A CIDv1 (bafkrei…) over the canonical content bytes. Superseded by the real ActionHash when a ContentCommitted notarization later runs. Optional — omit on the peered path where the drain stamps p2pPublishedAt.
   */
  dhtAnchorHash?: string;
}
