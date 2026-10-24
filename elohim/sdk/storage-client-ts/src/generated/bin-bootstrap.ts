/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/bin-bootstrap.schema.json -- DO NOT EDIT */

/**
 * Device-local CLI output; reconstructed from private custody and runtime observations. No network authority.
 */
export type BinBootstrap = BinRelease;

/**
 * Device-local CLI output; reconstructed from private custody and runtime observations. No network authority.
 */
export interface BinRelease {
  cid: string;
  rcid: string;
  tool: string;
  bins: {
    [k: string]: string;
  };
  version: string | null;
  sourceCommit: string;
  treeClean: boolean;
  gateRun: string | null;
  ingestedAt: string;
  sources: {
    sourceCommit: string;
    treeClean: boolean;
    gateRun: string | null;
    ingestedAt: string;
  }[];
  ring: string;
  attestations: {
    [k: string]: unknown;
  };
  /**
   * Adoption route, cache for a verified cached ancestor.
   */
  source?: string;
  [k: string]: unknown;
}
