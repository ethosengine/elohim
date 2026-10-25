/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/runtime-manifest.schema.json -- DO NOT EDIT */

/**
 * Device-private declaration or reconstructed observation. No network authority. Source of truth: elohim/ark/core/src/manifest.rs (RuntimeManifest native ABI).
 */
export interface RuntimeManifest {
  schema: 1;
  kind: 'runtime-manifest';
  supersedes: string | null;
  reach: string;
  processes: Record<string, unknown>[];
  envelope?: Record<string, unknown> | null;
  archetype?: string | null;
  follows?: {
    channel: string;
    mode: 'observe' | 'canary' | 'apply';
    pin?: string;
  }[];
  tools?: {
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
