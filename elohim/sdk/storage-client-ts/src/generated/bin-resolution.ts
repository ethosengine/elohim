/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/bin-resolution.schema.json -- DO NOT EDIT */

/**
 * Device-local CLI output; reconstructed from private custody and runtime observations. No network authority.
 */
export interface BinResolution {
  path: string;
  /**
   * Source of truth: device-runtime executable resolution and daemon admission; local operational vocabulary.
   */
  source: 'candidate' | 'explicit-pin' | 'session-pin' | 'current' | 'current-lock-timeout';
  cid: string | null;
}
