/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/runtime-declaration.schema.json -- DO NOT EDIT */

/**
 * Device-private declaration or reconstructed observation. No network authority. Source of truth: elohim/ark/core/src/declaration.rs (ManifestView).
 */
export interface RuntimeDeclaration {
  manifestCid: string;
  supersedes: string | null;
  follows: {
    channel: string;
    mode: 'observe' | 'canary' | 'apply';
    pin?: string;
  }[];
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
  processes: {
    name: string;
    channelId: string | null;
    artifactCid: string | null;
    artifactSha256: string | null;
    argv: string[];
  }[];
}
