/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/bin-tools.schema.json -- DO NOT EDIT */

/**
 * Device-private declaration or reconstructed observation. No network authority. Source of truth: elohim/ark/core/src/declaration.rs (render_bin_tools).
 */
export interface BinTools {
  version: 1;
  tools: {
    [k: string]: {
      bins: string[];
      aliases: {
        [k: string]: string;
      };
      candidateEnv: {
        [k: string]: string[];
      };
      longRunning: string[][];
      source: {
        kind: 'repository' | 'gitlink';
        manifest?: string;
        path?: string;
        publishedRef: string[];
        repo?: string;
        releasePrefix?: string;
        attestation?: {
          provider: string;
          repo: string;
          check: string;
        };
      };
    };
  };
}
