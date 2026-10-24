/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/bin-status.schema.json -- DO NOT EDIT */

/**
 * Ephemeral projection of device-private releases and session pins. CIDv1 addresses; paths are native resolution results, never reconstructed by clients.
 */
export interface BinStatus {
  tool: string;
  store: string;
  session: string | null;
  sessionCid: string | null;
  head: string | null;
  behind: boolean;
  source: string;
  resolvedCid: string | null;
  resolved: string | null;
  verifiedCurrent: boolean;
  candidates: Release[];
}
export interface Release {
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
  [k: string]: unknown;
}
