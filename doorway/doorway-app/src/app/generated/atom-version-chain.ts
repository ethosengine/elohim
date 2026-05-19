/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/atom-version-chain.schema.json -- DO NOT EDIT */

/**
 * Ordered chain of EPR atom CIDs linked by SUPERSEDES edges, walked forward from the given start CID. Source of truth: CozoDB graph projection (Operational, Category C). Reconstructed per request by walking epr_edge SUPERSEDES arcs.
 */
export interface AtomVersionChain {
  /**
   * The CID the walk starts from.
   */
  currentCid: string;
  /**
   * Ordered successor CIDs (each supersedes the previous).
   */
  chain: {
    cid: string;
    version: number;
    /**
     * ISO-8601 timestamp when this entry was superseded.
     */
    supersededAt?: string | null;
  }[];
  /**
   * The final (most recent) CID in the chain; null when chain is empty.
   */
  canonicalCid?: string | null;
}
