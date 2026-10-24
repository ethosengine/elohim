/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/session-identity.schema.json -- DO NOT EDIT */

export type SessionSource = 'declared' | 'runtimeAncestor' | 'ttyLeader' | 'none';

/**
 * Source of truth: environment and local process table (Ephemeral, Category C). Reconstructed per call; not persisted. Null means no detectable session or live owner at observation time.
 */
export interface SessionIdentity {
  runtime: string;
  session: string | null;
  ownerPid: number | null;
  /**
   * Linux proc start time in clock ticks since boot. u64 encoded as decimal text to preserve precision; null means no detectable live owner.
   */
  ownerStartTime: string | null;
  source: SessionSource;
}
