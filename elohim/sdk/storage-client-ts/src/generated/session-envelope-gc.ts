/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/session-envelope-gc.schema.json -- DO NOT EDIT */

/**
 * Actual epr session gc --json output; private device-local lifecycle result.
 */
export interface SessionEnvelopeGc {
  closed: {
    /**
     * Bound-once generation. Legacy records without this field read as generation 1.
     */
    generation: number;
    /**
     * Raw CID of the exact archived predecessor bytes and its untouched seal; null for the first generation.
     */
    predecessor: {
      recordCid: string;
      covenantSeal: string | null;
    } | null;
    identity: SessionIdentity;
    device: string;
    steward: string;
    model: string | null;
    repository: string;
    openedAt: string;
    closedAt: string | null;
    covenantSeal: string | null;
    witness: string | null;
    /**
     * PID namespace inode from the opening proc root; null means unobserved.
     */
    ownerPidNamespace: string | null;
    /**
     * Boot identity from the opening proc root; null means unobserved.
     */
    ownerBootId: string | null;
    /**
     * Canonical berth authority store bound before the first mooring or lease; null means unused.
     */
    berthStore: string | null;
  }[];
  /**
   * Dead-owner envelopes gc could not close; gc exits non-zero when any are listed.
   */
  failed: {
    session: string;
    error: string;
  }[];
}
/**
 * P1 identity observed at opening and retained as a private binding. Current owner liveness is checked against pid and start time.
 */
export interface SessionIdentity {
  runtime: string;
  session: string;
  ownerPid: number | null;
  /**
   * Linux proc start time in clock ticks since boot. u64 encoded as decimal text to preserve precision; null means no detectable live owner.
   */
  ownerStartTime: string | null;
  source: 'declared' | 'runtimeAncestor' | 'ttyLeader' | 'none';
}
