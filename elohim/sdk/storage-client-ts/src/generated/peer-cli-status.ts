/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/peer-cli-status.schema.json -- DO NOT EDIT */

/**
 * Device-local CLI output; reconstructed from private custody and runtime observations. No network authority.
 */
export interface PeerCliStatus {
  cliCid: string | null;
  daemonCids: string[];
  daemonDiffers: boolean;
  handle: string;
  chainRoot: string;
  device: string;
  nodeId: string | null;
  ownSeq: number | null;
  rules: null | {
    digest: string;
    short: string;
    state: 'loaded' | 'reloading';
    policy: string;
    reason: string | null;
  };
  devices: {
    device: string;
    nodeId: string;
    seq: number | null;
    root: string | null;
    ageS: number | null;
    eventsFolded: number;
    acked: number | null;
  }[];
  dropped: {
    unadmitted: number;
    forged: number;
    rollback: number;
    replayed: number;
    malformed: number;
  };
  channel: {
    projection_failures: number;
    fold_failures: number;
    oversized_events: number;
    over_budget_folds: number;
    refused_strangers: number;
    last_projection_error: string | null;
    refused_connections: {
      [k: string]: number;
    };
  };
  refusedConnections: {
    nodeId: string;
    device: string | null;
    count: number;
  }[];
  refusedStrangers: number;
}
