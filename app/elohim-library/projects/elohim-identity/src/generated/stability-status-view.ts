/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/stability-status-view.schema.json -- DO NOT EDIT */

/**
 * Unified self-healing read model served at doorway GET /admin/self-healing. Source of truth: doorway in-process runtime state + storage projector status (Operational, Category C — node-local, never notarized). A node serves its OWN state. NOTE: the TS codegen registration (codegen-ts.mjs INTERFACE_FILES) + the Angular consumer service are a flagged frontend/TS sibling follow-on; this file is the authored wire contract, kept in sync with doorway's Rust SelfHealingView (self_healing.rs).
 */
export interface StabilityStatusView {
  /**
   * Resource snapshot + derived Auto config + reasons. Null until the auto-config sibling plan lands.
   */
  autoPreset?: Record<string, unknown> | null;
  /**
   * Inbound admission state. Null until the inbound-admission wire-up follow-on exposes inbound_semaphore state.
   */
  admission?: {
    maxInflight: number;
    available: number;
    shedTotal: number;
  } | null;
  /**
   * Per-upstream circuit/health. Empty until the upstream-self-protection wire-up follow-on exposes a breaker snapshot.
   */
  upstreams: {
    endpoint: string;
    circuit: 'closed' | 'half-open' | 'open';
    errorStreak: number;
    lastGood: string | null;
    skipped: boolean;
  }[];
  projector: {
    lagSeconds: number | null;
    caughtUp: boolean | null;
    divergentAnchor: number | null;
  };
  peers: {
    peer: string;
    status: string;
    lastSeen: string | null;
  }[];
  render: {
    total: number;
    degenerateRate: number;
  };
  warmup: {
    inProgress: boolean;
    attempts: number;
    completed: boolean;
    lastError: string | null;
  };
  conductor: {
    connected: boolean;
    connectedWorkers: number;
    totalWorkers: number;
  };
}
