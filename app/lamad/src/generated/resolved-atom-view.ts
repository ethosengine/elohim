/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/resolved-atom-view.schema.json -- DO NOT EDIT */

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
