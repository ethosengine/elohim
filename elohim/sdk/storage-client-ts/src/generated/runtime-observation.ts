/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/runtime-observation.schema.json -- DO NOT EDIT */

/**
 * Device-private declaration or reconstructed observation. No network authority. Source of truth: elohim/ark/core/src/passport.rs (RuntimeObservation).
 */
export interface RuntimeObservation {
  follows: {
    channel: string;
    mode: 'observe' | 'canary' | 'apply';
    appliedRelease: string | null;
  }[];
  tools: {
    tool: string;
    appliedRelease: string | null;
  }[];
}
