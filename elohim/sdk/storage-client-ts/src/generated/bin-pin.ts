/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/bin-pin.schema.json -- DO NOT EDIT */

/**
 * Device-local CLI output; reconstructed from private custody and runtime observations. No network authority.
 */
export interface BinPin {
  cid: string;
  session: string;
  pinnedAt: string;
  ownerPid: number | null;
  ownerStartTime: string | null;
  provisional: boolean;
  runningCid?: string;
  /**
   * Source of truth: device-runtime executable resolution and daemon admission; local operational vocabulary.
   */
  source?: 'candidate' | 'current';
}
