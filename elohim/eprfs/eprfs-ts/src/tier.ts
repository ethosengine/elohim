/**
 * Seam A in TypeScript (interfaces spec §H.1): the tier interface and the tier chain, with the
 * same contract as `eprfs-core`'s `TierChain` (§A.4), ported onto the same vectors.
 *
 * Tiers return named absences and never throw (decision 11). The chain asks its own tier first,
 * then the outer tiers its records name, in a pure order, each only when the policy admits it for
 * the root and within the policy's budget; it verifies every byte against its address before
 * returning or writing it back.
 */
import type { CID } from 'multiformats/cid';

import { verifyAddress as verifiesAddress } from '@elohim/epr';
import type { NamedAbsence, TreePath } from './walk.js';
import {
  absence,
  rewriteOwn,
  TIER_CLASS_ORDER,
  type ItemAbsence,
  type TierRecord,
  type Unanswered,
} from './wire.js';

/** Byte components, as `TreePath` (H14). */
export interface BatchItem {
  cid: CID;
  path: TreePath;
}

export type TierAnswer = Uint8Array | NamedAbsence;
export type TierSink = (item: BatchItem, r: TierAnswer) => void;

export interface Tier {
  readonly record: TierRecord;
  fetchAt(root: CID, item: BatchItem, via: readonly string[], signal?: AbortSignal): Promise<TierAnswer>;
  fetchBlobs(
    root: CID,
    items: readonly BatchItem[],
    via: readonly string[],
    sink: TierSink,
    signal?: AbortSignal,
  ): Promise<void>;
}
export interface WritableTier extends Tier {
  put(cid: CID, bytes: Uint8Array): Promise<void>;
}
/** Rust's TierConnector, so records the page holds become tiers (H8). */
export interface TierConnector {
  connect(record: TierRecord): Tier | null;
}
export type Admission = { release(): void } | { retryAfterMs: number; retryAtEpochSecs: number; rule: string };
export interface ChainPolicy {
  /** Default: `proximityOrder` (§A.2's comparator, H17). */
  order(records: TierRecord[], root: CID): TierRecord[];
  /** Unknown root reach: only own, person-device and household (H7). */
  mayAsk(tier: TierRecord, root: CID, rootReach: number | null): boolean;
  /**
   * The forwarding budget. A refusal gives the absolute come-back time as well: this package
   * reads no clock (the same revision as Rust's `OverBudget.retry_at_epoch_secs`).
   */
  admit(to: string): Admission;
}
export interface ChainStatsSnapshot {
  perTier: Record<string, { asked: number; hit: number; integrity: number }>;
}
/**
 * What the spec's constructor does not carry (revision recorded in the C4 report): whether the
 * chain may leave the device at all (`localOnly`, H20), and the root's reach, which the host's
 * head plane holds beside each root (`rootReach`, as Rust's `BatchOptions.root_reach`).
 */
export interface ChainOptions {
  localOnly?: boolean;
  rootReach?: (root: CID) => number | null;
  /** The id the chain names itself by in its own refusals (over budget). */
  selfId?: string;
  /**
   * The most CIDs one outer ask carries (Rust's `ChainLimits::max_request_items`, the bound a
   * serving endpoint refuses past). Default 4096; a round holds at most this many answers.
   */
  maxRequestItems?: number;
}

/** The per-request bound the chain and every serving tier share by default. */
export const DEFAULT_MAX_REQUEST_ITEMS = 4096;

function classRank(record: TierRecord): number {
  return TIER_CLASS_ORDER.indexOf(record.class);
}

/**
 * §A.2's comparator: class; warm for `root` first; network rank; rtt ascending, unknown last;
 * more recent last-seen first, unknown last; id. An ordering, not an additive score (H17).
 */
export function proximityOrder(records: readonly TierRecord[], root: CID): TierRecord[] {
  const key = root.toString();
  const warm = (r: TierRecord) => (r.proximity?.warmRoots.includes(key) ? 0 : 1);
  const rank = (r: TierRecord) => r.proximity?.networkRank ?? 255;
  const rtt = (r: TierRecord) => (r.proximity?.rttMs == null ? Number.POSITIVE_INFINITY : r.proximity.rttMs);
  const seen = (r: TierRecord) =>
    r.proximity?.lastSeenMs == null ? Number.POSITIVE_INFINITY : -r.proximity.lastSeenMs;
  return [...records].sort(
    (a, b) =>
      classRank(a) - classRank(b) ||
      warm(a) - warm(b) ||
      rank(a) - rank(b) ||
      (rtt(a) === rtt(b) ? 0 : rtt(a) < rtt(b) ? -1 : 1) ||
      (seen(a) === seen(b) ? 0 : seen(a) < seen(b) ? -1 : 1) ||
      (a.id < b.id ? -1 : a.id > b.id ? 1 : 0),
  );
}

const CLOSE: readonly string[] = ['own', 'person-device', 'household'];

/**
 * The host-side `ReachStandingAsk` (§A.6), parameterized by the two openness levels it defaults
 * to, so this package names no reach value. Admits every forward.
 */
export function standingPolicy(levels: { household: number; public: number }): ChainPolicy {
  return {
    order: proximityOrder,
    mayAsk(tier, _root, rootReach) {
      if (rootReach === null) return CLOSE.includes(tier.class);
      const standing = tier.standing ?? (CLOSE.includes(tier.class) ? levels.household : levels.public);
      return rootReach >= standing;
    },
    admit() {
      return { release() {} };
    },
  };
}

interface Open {
  cid: CID;
  key: string;
  items: BatchItem[];
  tried: string[];
  last: Unanswered<ItemAbsence> | null;
  soft: Unanswered<ItemAbsence> | null;
  done: boolean;
}

function groupByCid(items: readonly BatchItem[]): { cid: CID; key: string; items: BatchItem[] }[] {
  const groups: { cid: CID; key: string; items: BatchItem[] }[] = [];
  const index = new Map<string, number>();
  for (const item of items) {
    const key = item.cid.toString();
    const at = index.get(key);
    if (at === undefined) {
      index.set(key, groups.length);
      groups.push({ cid: item.cid, key, items: [item] });
    } else {
      groups[at].items.push(item);
    }
  }
  return groups;
}

function absent(item: BatchItem, unanswered: Unanswered<ItemAbsence>): NamedAbsence {
  return { kind: 'absent', cid: item.cid.toString(), path: item.path, unanswered };
}

/**
 * A later tier's unestablished answer replaces an earlier one, except that an answer every walk
 * reads as an error (bytes that failed their address, the own tier failing) is never degraded by
 * a later tier's silence (B4), as Rust's `keep_soft`.
 */
function isWalkError(why: ItemAbsence | undefined): boolean {
  return why !== undefined && (why.kind === 'integrity' || (why.kind === 'unreachable' && why.tier === 'own'));
}

function keepSoft(o: { soft: Unanswered<ItemAbsence> | null }, why: Unanswered<ItemAbsence>): void {
  if (!isWalkError(o.soft?.why)) o.soft = why;
}

/**
 * A tier's *statement* that its copy failed its address, as Rust's `reported_integrity`: no bytes
 * reached this side, so absence is not established and the chain advances. Only bytes this side
 * verified and found bad are the walk error B4 names.
 */
function reportedIntegrity(tier: string): Unanswered<ItemAbsence> {
  return {
    ...absence.unreachable(tier),
    meanwhile: { local: 'nothing', because: 'the tier reported that its copy failed its address; no bytes reached here' },
  };
}

/** Seam A's contract, ported. Own tier first; verifies before returning or writing back. */
export class TierChain implements Tier {
  readonly record: TierRecord;
  private readonly outer = new Map<string, { record: TierRecord; tier: Tier }>();
  private readonly perTier: ChainStatsSnapshot['perTier'] = {};

  constructor(
    private readonly own: WritableTier,
    records: readonly TierRecord[],
    connector: TierConnector,
    private readonly policy: ChainPolicy,
    private readonly options: ChainOptions = {},
  ) {
    this.record = {
      id: 'chain',
      class: 'own',
      sources: [{ source: 'held-record', record: 'chain' }],
      endpoint: null,
      standing: null,
      proximity: null,
    };
    const merged = new Map<string, TierRecord>();
    for (const r of records) {
      if (r.id === 'own') continue;
      const existing = merged.get(r.id);
      if (existing === undefined) {
        merged.set(r.id, { ...r, sources: [...r.sources] });
      } else {
        for (const s of r.sources) {
          if (!existing.sources.some((e) => JSON.stringify(e) === JSON.stringify(s))) existing.sources.push(s);
        }
        if (classRank(r) < classRank(existing)) existing.class = r.class;
        existing.endpoint ??= r.endpoint;
        existing.proximity ??= r.proximity;
        existing.standing =
          existing.standing === null ? r.standing : r.standing === null ? existing.standing : Math.min(existing.standing, r.standing);
      }
    }
    for (const record of merged.values()) {
      const tier = connector.connect(record);
      if (tier !== null) this.outer.set(record.id, { record, tier });
    }
  }

  /** The connected tiers in the policy's order for `root`: warmth is per root, so the order is
   *  taken per request rather than once at construction. */
  private ordered(root: CID): { record: TierRecord; tier: Tier }[] {
    const records = [...this.outer.values()].map((t) => t.record);
    return this.policy.order(records, root).flatMap((r) => {
      const t = this.outer.get(r.id);
      return t === undefined ? [] : [t];
    });
  }

  stats(): ChainStatsSnapshot {
    return { perTier: structuredClone(this.perTier) };
  }

  private count(id: string, field: 'asked' | 'hit' | 'integrity'): void {
    const c = (this.perTier[id] ??= { asked: 0, hit: 0, integrity: 0 });
    c[field] += 1;
  }

  async fetchAt(root: CID, item: BatchItem, via: readonly string[], signal?: AbortSignal): Promise<TierAnswer> {
    let answer: TierAnswer | undefined;
    await this.fetchBlobs(root, [item], via, (_item, r) => (answer ??= r), signal);
    return answer ?? absent(item, absence.unreachable(this.options.selfId ?? 'own'));
  }

  /** The requester's own reads: `via` is the requester's own list, so it is not a restriction. */
  async fetchBlobs(
    root: CID,
    items: readonly BatchItem[],
    _via: readonly string[],
    sink: TierSink,
    signal?: AbortSignal,
  ): Promise<void> {
    const open: Open[] = [];
    // 1. The own tier first (contract 1).
    for (const group of groupByCid(items)) {
      const r = await this.own.fetchAt(root, group.items[0], [], signal);
      if (r instanceof Uint8Array && (await verifiesAddress(group.cid, r))) {
        for (const item of group.items) sink(item, r);
        continue;
      }
      const soft =
        r instanceof Uint8Array
          ? absence.integrity('own')
          : r.kind === 'absent' && r.unanswered.why.kind !== 'not-held'
            ? r.unanswered
            : null;
      open.push({ ...group, tried: [], last: null, soft, done: false });
    }
    if (open.length === 0) return;

    // 2. Outer tiers, only when the chain may leave the device (contracts 2, 3).
    if (!this.options.localOnly) {
      const rootReach = this.options.rootReach?.(root) ?? null;
      const admitted = this.ordered(root).filter((t) => this.policy.mayAsk(t.record, root, rootReach));
      const perRound = Math.max(1, this.options.maxRequestItems ?? DEFAULT_MAX_REQUEST_ITEMS);
      tiers: for (let position = 0; position < admitted.length; position++) {
        const { record, tier } = admitted[position];
        const delegated = admitted.slice(position + 1).map((t) => t.record.id);
        // Each item is asked of this tier once, in rounds of at most `perRound` CIDs.
        const queue = open.filter((o) => !o.done && o.last === null && !o.tried.includes(record.id));
        for (let start = 0; start < queue.length; start += perRound) {
          const wanting = queue
            .slice(start, start + perRound)
            .filter((o) => !o.done && o.last === null && !o.tried.includes(record.id));
          if (wanting.length === 0) continue;
          const admission = this.policy.admit(record.id);
          if (!('release' in admission)) {
            // Over budget is answered with its come-back time, never routed around (contract 8).
            const refusal = absence.overBudget(this.options.selfId ?? 'own', admission.rule, admission.retryAtEpochSecs);
            for (const o of open) if (!o.done && o.last === null) o.last = refusal;
            break tiers;
          }
          const wanted = new Set(wanting.map((o) => o.key));
          const answers = new Map<string, TierAnswer>();
          this.count(record.id, 'asked');
          try {
            await tier.fetchBlobs(
              root,
              wanting.map((o) => o.items[0]),
              delegated,
              (item, r) => {
                const key = item.cid.toString();
                // Items with unrequested CIDs are dropped; the first answer wins (contract 10).
                if (wanted.has(key) && !answers.has(key)) answers.set(key, r);
              },
              signal,
            );
          } catch {
            // A tier that fails as a whole answers nothing: every item reads as unreachable below.
          } finally {
            admission.release();
          }
          for (const o of wanting) {
            const r = answers.get(o.key);
            if (r === undefined) {
              keepSoft(o, absence.unreachable(record.id));
            } else if (r instanceof Uint8Array) {
              if (await verifiesAddress(o.cid, r)) {
                this.count(record.id, 'hit');
                // Store before return; a failed write-back still delivers (contract 7).
                await this.own.put(o.cid, r).catch(() => undefined);
                o.done = true;
                for (const item of o.items) sink(item, r);
              } else {
                this.count(record.id, 'integrity');
                keepSoft(o, absence.integrity(record.id));
              }
            } else if (r.kind === 'absent') {
              const why = rewriteOwn(r.unanswered.why, record.id);
              const unanswered = { ...r.unanswered, why };
              if (why.kind === 'not-held') {
                // Only the tiers handed to `record` as its `via` count as asked by it (contract 5).
                const asked = [record.id, ...why.tried.filter((id) => delegated.includes(id))];
                for (const id of asked) if (!o.tried.includes(id)) o.tried.push(id);
              } else if (why.kind === 'refused' || why.kind === 'over-budget') {
                o.last = unanswered;
              } else if (why.kind === 'integrity') {
                keepSoft(o, reportedIntegrity(record.id));
              } else {
                keepSoft(o, unanswered);
              }
            } else {
              keepSoft(o, absence.unreachable(record.id));
            }
          }
        }
      }
    }

    // 3. Name what is still absent (contracts 9, 12): a walk error this side observed, else a
    //    final refusal, else an unestablished absence, else an observed one listing every tier
    //    asked. A later refusal never degrades integrity to an absence (B4).
    for (const o of open) {
      if (o.done) continue;
      const why = (isWalkError(o.soft?.why) ? o.soft : null) ?? o.last ?? o.soft ?? absence.notHeld(o.tried);
      for (const item of o.items) sink(item, absent(item, why));
    }
  }
}

/** An in-memory own tier: bytes by CID. It does not verify on read; the chain does. */
export class MemoryTier implements WritableTier {
  readonly record: TierRecord;
  readonly blobs = new Map<string, Uint8Array>();
  readonly asked: BatchItem[][] = [];

  constructor(id = 'own', klass: TierRecord['class'] = 'own') {
    this.record = {
      id,
      class: klass,
      sources: [{ source: 'held-record', record: 'memory' }],
      endpoint: null,
      standing: null,
      proximity: null,
    };
  }

  async put(cid: CID, bytes: Uint8Array): Promise<void> {
    this.blobs.set(cid.toString(), bytes);
  }

  async fetchAt(root: CID, item: BatchItem, via: readonly string[]): Promise<TierAnswer> {
    let answer: TierAnswer | undefined;
    await this.fetchBlobs(root, [item], via, (_i, r) => (answer = r));
    return answer as TierAnswer;
  }

  async fetchBlobs(_root: CID, items: readonly BatchItem[], _via: readonly string[], sink: TierSink): Promise<void> {
    this.asked.push([...items]);
    for (const item of items) {
      const bytes = this.blobs.get(item.cid.toString());
      sink(item, bytes ?? absent(item, absence.notHeld([])));
    }
  }
}
