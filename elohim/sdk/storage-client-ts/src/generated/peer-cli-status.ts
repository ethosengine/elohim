/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/peer-cli-status.schema.json -- DO NOT EDIT */

/**
 * Source of truth: device-private bin stores, head table and process table (Private, Category B). Device-local CLI output; reconstructed from private custody and runtime observations per invocation. No network authority.
 */
export interface PeerCliStatus {
  cliCid: string | null;
  daemonCids: string[];
  daemonDiffers: boolean;
  handle: string;
  chainRoot: string;
  device: string;
  nodeId: string | null;
  ownSeq: number | null;
  rules: null | {
    digest: string;
    short: string;
    state: 'loaded' | 'reloading';
    policy: string;
    reason: string | null;
  };
  devices: {
    device: string;
    nodeId: string;
    seq: number | null;
    root: string | null;
    ageS: number | null;
    /**
     * Seconds on this device's clock since any verified head was heard from the peer, including a replay; null when no verified head has been heard (one provenance).
     */
    heardAgeS?: number | null;
    /**
     * Seconds on this device's clock since a head from the peer folded new records; null when no head has folded records (one provenance).
     */
    foldedAgeS?: number | null;
    /**
     * Whether the most recent recovery probe reached the peer; null when no recovery probe has been recorded (one provenance).
     */
    probeOk?: boolean | null;
    /**
     * Whether the most recent recovery probe found a peer that does not support probing; null when no recovery probe has been recorded (one provenance).
     */
    probeUnsupported?: boolean | null;
    /**
     * Seconds on this device's clock since the most recent recovery probe; null when no recovery probe has been recorded (one provenance).
     */
    probeAgeS?: number | null;
    /**
     * Number of recovery resubscriptions attempted for this peer since the daemon started.
     */
    resubscriptions?: number;
    /**
     * Number of recovery connection resets attempted for this peer since the daemon started.
     */
    connectionResets?: number;
    eventsFolded: number;
    acked: number | null;
  }[];
  dropped: {
    unadmitted: number;
    forged: number;
    rollback: number;
    replayed: number;
    malformed: number;
  };
  channel: {
    projection_failures: number;
    fold_failures: number;
    oversized_events: number;
    over_budget_folds: number;
    refused_strangers: number;
    last_projection_error: string | null;
    refused_connections: {
      [k: string]: number;
    };
  };
  refusedConnections: {
    nodeId: string;
    device: string | null;
    count: number;
  }[];
  refusedStrangers: number;
  /**
   * This device's own crossing-intent records that the policy file the daemon loaded does not cross. Empty when every one crosses, or when no daemon has recorded its store and rules.
   */
  withheldCrossings: {
    measure: string;
    kind: string;
    events: number;
  }[];
  /**
   * The artifact gate's exit counts over the daemon's recorded flow store for the last 14 days (K12 design section 4): serves by reach, serves by method row, and outputs whose latest own custody record in the window has no reach slot. Null when no daemon has recorded its store, or the store cannot be read.
   */
  artifactGate: null | {
    since: string;
    reachDraws: number;
    rowDraws: number;
    slotlessDesignated: number;
  };
  /**
   * The last `epr bin follow` attempt per registered tool, one row each, read from that tool store's private follow.json. Optional: absent in output from a CLI that predates follow outcomes; every current CLI emits it. `state` makes every absence explicit (views CONVENTIONS rule 11): `never` = the store holds no follow record; `recorded` = a terminal record; `in-progress` = a pending record whose owner process was observed running (not a zombie); `unknown` = a pending record whose owner could not be observed, with `detail` saying why; `interrupted` = a pending record that will never finish (owner exited, is a zombie, its PID was reused, or the outcome could not be written), with `detail` saying which; `corrupt` = the record exists but cannot be read or fails validation; `unavailable` = the tool's store could not be opened, or, with tool `*`, the tool registry itself.
   */
  followOutcomes?: (
    | {
        state: 'never';
        tool: string;
      }
    | {
        state: 'recorded' | 'in-progress';
        tool: string;
        record: FollowRecord;
      }
    | {
        state: 'unknown' | 'interrupted';
        tool: string;
        record: FollowRecord;
        detail: string;
      }
    | {
        state: 'corrupt' | 'unavailable';
        tool: string;
        detail: string;
      }
  )[];
}
/**
 * One `epr bin follow` attempt as the tool store's private follow.json holds it. Device operational state: never notarized or synced, and it never names the installed head (current.json does).
 */
export interface FollowRecord {
  version: number;
  attempt: string;
  startedAt: string;
  /**
   * Null only while the attempt is pending (one provenance).
   */
  endedAt: string | null;
  ownerPid: number;
  /**
   * Owner start time in clock ticks as decimal text (bigint-as-string); null when the process table could not be read at start.
   */
  ownerStartTime: string | null;
  /**
   * The source checkout followed, as the caller named it; URL userinfo, query and fragment are removed.
   */
  source: string;
  /**
   * The checkout HEAD the attempt observed; null when it stopped before reading one.
   */
  commit: string | null;
  mode: 'apply' | 'canary' | 'observe';
  /**
   * Device head when the attempt began; null when the device had none.
   */
  priorCid: string | null;
  /**
   * Release the attempt staged or tried to land; null when it stopped before choosing one.
   */
  candidateCid: string | null;
  /**
   * Device head when the attempt ended; null when the device had none.
   */
  resultCid: string | null;
  /**
   * pending is written before any work; the other four are terminal.
   */
  outcome: 'pending' | 'landed' | 'refused' | 'unchanged' | 'observed';
  /**
   * Stable reason category (a refusal kind, or why nothing was done). Null for a clean landing or an observation; a landing whose head committed before a later step failed carries that step's kind.
   */
  kind: string | null;
  /**
   * Bounded (240 characters), single-line, sanitised reason: URL credentials and secret-named assignments are removed. Null exactly when there is no reason to give.
   */
  reason: string | null;
}
