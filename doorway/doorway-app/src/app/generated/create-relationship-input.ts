/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: inputs/create-relationship-input.schema.json -- DO NOT EDIT */

/**
 * The SOURCE atom's reach: an edge is never more open than the atom that authored it. Absent: a new edge is written at commons and an existing edge keeps its stored reach.
 */
export type Reach =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';

/**
 * Input for creating a content edge (POST /db/relationships, POST /db/relationships/bulk items). Must match Rust CreateRelationshipInputView (crates/elohim-views/src/imagodei.rs). Source of truth: an authored edge's truth is its SOURCE atom's signed entry (Content.metadata_json relationships[], F16); storage projects each peer's relationships table from the adopted head. This direct write is seeder scaffold, retired once every atom states its edges in its entry.
 */
export interface CreateRelationshipInput {
  /**
   * Deterministic edge id: 'rel-' + first 32 hex of sha256(source|type|target). Omitted: storage mints one.
   */
  id?: string;
  /**
   * Schema version for migration tracking
   */
  schemaVersion?: number;
  /**
   * The atom that authored the edge
   */
  sourceId: string;
  /**
   * The atom the edge names
   */
  targetId: string;
  /**
   * A lamad manifest relationship id (elohim/sdk/domains/lamad/manifest/relationships.json)
   */
  relationshipType: string;
  /**
   * Edge confidence, default 1.0
   */
  confidence?: number;
  /**
   * How the edge came to be; authored edges are 'explicit'
   */
  inferenceSource?: 'explicit' | 'path' | 'tag' | 'semantic' | 'system';
  reach?: Reach;
  /**
   * Edge metadata, stored as metadata_json. Carries the authored role.
   */
  metadata?: {
    /**
     * The authored role (anchor, supporting, story, practice, callback, related, ...)
     */
    role?: string;
  };
}
