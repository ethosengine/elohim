/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/epr-head-view.schema.json -- DO NOT EDIT */

/**
 * content.reach passed through verbatim (Category A). Omitted only if the underlying column value fails to parse as a known reach class.
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
 * Source of truth: DHT (Notarized HEAD projection, Category A) — a read projection of `content.declared_head_action_hash` / `declared_head_at`. The envelope's cid is a function of the declared head alone: `version`, `id`, `content`, `lamad`, `shefa`, `qahal`, `relationships`, `author` and `updated` are exactly the canonical dag-cbor bytes served verbatim at the `Accept: application/vnd.ipld.dag-cbor` arm of GET /epr-head/{id}; `cid` is that address (CIDv1, codec 0x71), minted per request over those same bytes — two peers agreeing on the declared head mint identical bytes and therefore identical cid, regardless of local row mtime. `updated` is rendered from `content.declared_head_at` (the DHT declaration act's own Timestamp) — NEVER the row's `updated_at` mtime, which is bumped by every stamp including no-ops and cannot be a fact about the content. `updated` is OMITTED (never a substitute value) when `declared_head_at` is NULL — honest absence, per Rule 11's single-provenance case (this row has not recorded a declaration timestamp), not the answer-envelope pattern. Per-peer operational facts (replica counts, projector counts, diversity, caller role) are NOT carried here — a document whose whole point is to be the same everywhere must not embed a fact two honest peers can honestly disagree on. Those are served separately at GET /api/v1/blob/{hash}/distribution/summary, which genuinely differs between honest peers and is intentionally NOT peer-invariant.
 */
export interface EprHeadView {
  /**
   * Schema version for forward compatibility.
   */
  version: number;
  /**
   * Stable content ID (slug) — a lookup key, not an address. Deliberately not the same identifier class as `cid`.
   */
  id: string;
  /**
   * CID of the current content bytes (IPLD link to Tier 3), from content.blob_cid.
   */
  content: string;
  /**
   * Knowledge pillar context — A-projected from the content row.
   */
  lamad: {
    title: string;
    contentType: string;
    description?: string;
    contentFormat?: string;
    /**
     * Omitted when empty. Projected via a Category C join table (content_tags); can lag in steady state.
     */
    tags?: string[];
  };
  /**
   * Value pillar context. Always empty on this HTTP arm (derived with enrich_pillars=false). Classified A2 but never anchored — no create_stewardship_allocation zome function exists to notarize it.
   */
  shefa: {
    /**
     * Omitted when empty.
     */
    stewards?: string[];
    /**
     * Omitted when empty.
     */
    allocations?: number[];
  };
  /**
   * Governance pillar context.
   */
  qahal: {
    reach?: Reach;
    /**
     * Always absent today — a dead field, never populated (named follow-up F4; not addressed by this envelope's cid-invariance fix).
     */
    layer?: string;
    /**
     * Always empty (omitted) on this HTTP arm. The P2P enrich=true caller populates it downstream from content-graph PREREQUISITE edges; format 'prerequisite-mastery:{prereq_id}'.
     */
    attestationRequirements?: string[];
  };
  /**
   * Typed relationships to other content. Always present (serializes as `[]` when empty, unlike the omit-when-empty fields above). Always empty on this HTTP arm.
   */
  relationships: {
    /**
     * Relationship type: PREREQUISITE, TEACHES, CONTAINS, REFERENCES, etc.
     */
    type: string;
    /**
     * Target content ID.
     */
    target: string;
    /**
     * Target content address (CID). Omitted when unknown.
     */
    targetCid?: string;
  }[];
  /**
   * Author DID/agent identifier, from content.created_by. Omitted when unset.
   */
  author?: string;
  /**
   * RFC3339 UTC, seconds precision, rendered from content.declared_head_at (microseconds since the Unix epoch) — the DHT declaration act's own Timestamp. NEVER content.updated_at (a local row mtime bumped by every stamp, including no-ops). Omitted — never a substitute value — when declared_head_at is NULL.
   */
  updated?: string;
  /**
   * CIDv1 dag-cbor (codec 0x71, Sha2_256) address of this envelope's canonical bytes: version, id, content, lamad, shefa, qahal, relationships, author, updated — nothing else. Recomputed per request; a function of the declared head alone. Omitted only on the (should-not-occur) internal encoding failure that also collapses distribution-free bytes to nothing servable via the dag-cbor arm.
   */
  cid?: string;
}
