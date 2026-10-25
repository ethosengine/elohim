/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/session-envelope.schema.json -- DO NOT EDIT */

/**
 * Private (B) device-local P1 binding plus read-only joins of authoritative facet stores. No new grants, DHT entries or public surface.
 */
export interface SessionEnvelope {
  /**
   * Native lifecycle predicate: neither sealed nor closed. Source of truth: Envelope::is_live.
   */
  live: boolean;
  envelope: {
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
  };
  berth: Record<string, unknown>;
  claims: Record<string, unknown>[];
  covenant: Record<string, unknown>;
  standing: Record<string, unknown>;
  capabilityTier: string;
  recallSession: string;
  pinnedTools: {
    [k: string]: string;
  };
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
