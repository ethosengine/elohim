/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/navigation-context-view.schema.json -- DO NOT EDIT */

/**
 * Graph-traversal contextual view for an EPR atom — includes the resolved atom, the navigation origin, and neighbourhood atoms reachable within 2 hops. Source of truth: CozoDB graph projection (Operational, Category C). Reconstructed per request from graph primitives (NEIGHBORHOOD rule).
 */
export interface NavigationContextView {
  atom: ResolvedAtomView;
  originContext: {
    /**
     * CID of the navigation origin atom.
     */
    originCid: string;
    /**
     * EPR type of the origin (e.g. 'epr').
     */
    originType: string;
    /**
     * Optional structured path from origin to this atom.
     */
    originPath?: string | null;
  };
  /**
   * Atoms reachable from this atom within max_hops edges.
   */
  neighborhood?: {
    cid: string;
    relType: string;
    hops: number;
  }[];
}
/**
 * Three-pillar EPR atom view assembled from graph projection. Source of truth: CozoDB graph projection (Operational, Category C). Reconstructed per request from DHT-canonical epr_atoms relational projection.
 */
export interface ResolvedAtomView {
  /**
   * Content-addressed identifier for this EPR atom.
   */
  cid: string;
  /**
   * Human-readable slug (the EPR id field).
   */
  slug: string;
  /**
   * CID of the associated content blob.
   */
  contentCid: string;
  version: number;
  /**
   * Author DID, null when not specified.
   */
  authorDid?: string | null;
  /**
   * ISO-8601 timestamp of last update.
   */
  updatedAt: string;
  lamad: {
    title: string;
    contentType: string;
    description?: string | null;
    contentFormat?: string | null;
    tags: string[];
  };
  shefa?: {
    stewards?: string[];
    allocations?: number[];
  } | null;
  qahal: {
    reach?: string | null;
    layer?: string | null;
    attestationRequirements: string[];
  };
}
