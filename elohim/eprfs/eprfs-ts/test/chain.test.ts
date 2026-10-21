// Seam A's chain contract (interfaces spec §A.4), ported to vitest: the same items the Rust
// harness (eprfs-serve `assert_tier_contract`) checks, over the TypeScript TierChain.
import { CID } from 'multiformats/cid';
import { describe, expect, it, vi } from 'vitest';

import {
  MemoryTier,
  standingPolicy,
  TierChain,
  type BatchItem,
  type ChainPolicy,
  type Tier,
  type TierAnswer,
  type TierSink,
} from '../src/tier.js';
import { cidOf, RAW_CODEC } from '../src/tree.js';
import { collectWalk, readPath, View, type NamedAbsence } from '../src/walk.js';
import { absence, type TierRecord, type Unanswered, type ItemAbsence } from '../src/wire.js';
import { build, vectors, vpath } from './fixture.js';

type Behaviour =
  | { serve: true }
  | { answerAll: Unanswered<ItemAbsence> }
  | { corrupt: true }
  | { fail: true }
  | { notHeldNaming: string[] };

class ScriptedTier implements Tier {
  readonly record: TierRecord;
  readonly blobs = new Map<string, Uint8Array>();
  readonly batches: { items: BatchItem[]; via: readonly string[] }[] = [];
  behaviour: Behaviour = { serve: true };

  constructor(id: string, klass: TierRecord['class'], standing: number | null = null) {
    this.record = { id, class: klass, sources: [{ source: 'held-record', record: 'roster' }], endpoint: null, standing, proximity: null };
  }

  async fetchAt(root: CID, item: BatchItem, via: readonly string[]): Promise<TierAnswer> {
    let a: TierAnswer | undefined;
    await this.fetchBlobs(root, [item], via, (_i, r) => (a = r));
    return a!;
  }

  async fetchBlobs(_root: CID, items: readonly BatchItem[], via: readonly string[], sink: TierSink): Promise<void> {
    this.batches.push({ items: [...items], via });
    const b = this.behaviour;
    if ('fail' in b) throw new Error('failed as a whole');
    for (const item of items) {
      const absent = (u: Unanswered<ItemAbsence>): NamedAbsence => ({ kind: 'absent', cid: item.cid.toString(), path: item.path, unanswered: u });
      if ('answerAll' in b) sink(item, absent(b.answerAll));
      else if ('corrupt' in b) sink(item, new TextEncoder().encode('not the bytes'));
      else {
        const bytes = this.blobs.get(item.cid.toString());
        sink(item, bytes ?? absent(absence.notHeld('notHeldNaming' in b ? b.notHeldNaming : [])));
      }
    }
  }
}

const enc = new TextEncoder();
const ROOT = CID.parse(vectors.treeView.root);

async function leaf(text: string): Promise<{ cid: CID; bytes: Uint8Array }> {
  const bytes = enc.encode(text);
  return { cid: await cidOf(bytes, RAW_CODEC), bytes };
}

const allowAll: ChainPolicy = {
  order: (records) => records,
  mayAsk: () => true,
  admit: () => ({ release() {} }),
};

function chain(own: MemoryTier, tiers: ScriptedTier[], policy: ChainPolicy = allowAll, localOnly = false, rootReach: number | null = null) {
  const byId = new Map(tiers.map((t) => [t.record.id, t]));
  return new TierChain(own, tiers.map((t) => t.record), { connect: (r) => byId.get(r.id) ?? null }, policy, {
    localOnly,
    rootReach: () => rootReach,
    selfId: 'agent:uhCAkphone',
  });
}

async function ask(c: TierChain, items: BatchItem[]): Promise<[BatchItem, TierAnswer][]> {
  const out: [BatchItem, TierAnswer][] = [];
  await c.fetchBlobs(ROOT, items, [], (i, r) => out.push([i, r]));
  return out;
}

function item(cid: CID, path: string): BatchItem {
  return { cid, path: vpath(path) };
}

function why(r: TierAnswer): ItemAbsence {
  if (r instanceof Uint8Array || r.kind !== 'absent') throw new Error('expected an absence');
  return r.unanswered.why;
}

describe('the TypeScript tier chain', () => {
  it('1. asks its own tier first, and a hit makes zero outer calls', async () => {
    const own = new MemoryTier();
    const x = await leaf('own hit');
    await own.put(x.cid, x.bytes);
    const outer = new ScriptedTier('agent:a', 'household');
    const out = await ask(chain(own, [outer]), [item(x.cid, 'a')]);
    expect(out[0][1]).toEqual(x.bytes);
    expect(outer.batches).toHaveLength(0);
  });

  it('2. local-only stays local and names the miss at once', async () => {
    const outer = new ScriptedTier('agent:a', 'household');
    const x = await leaf('elsewhere');
    outer.blobs.set(x.cid.toString(), x.bytes);
    const out = await ask(chain(new MemoryTier(), [outer], allowAll, true), [item(x.cid, 'a')]);
    expect(why(out[0][1])).toEqual({ kind: 'not-held', tried: [] });
    expect(outer.batches).toHaveLength(0);
  });

  it('4. asks only what the policy admits, with the root reach the host holds', async () => {
    const levels = { household: 2, public: 4 };
    const peer = new ScriptedTier('agent:peer', 'doorway-peer');
    const hub = new ScriptedTier('agent:hub', 'hub', 2);
    const x = await leaf('on both');
    peer.blobs.set(x.cid.toString(), x.bytes);
    hub.blobs.set(x.cid.toString(), x.bytes);
    // An intimate root (1) never reaches a public-standing peer, nor a household-standing hub.
    let out = await ask(chain(new MemoryTier(), [peer, hub], standingPolicy(levels), false, 1), [item(x.cid, 'a')]);
    expect(why(out[0][1]).kind).toBe('not-held');
    expect(peer.batches).toHaveLength(0);
    expect(hub.batches).toHaveLength(0);
    // A household root (2) may ask the hub, still not the public peer; ordered by class.
    out = await ask(chain(new MemoryTier(), [peer, hub], standingPolicy(levels), false, 2), [item(x.cid, 'a')]);
    expect(out[0][1]).toEqual(x.bytes);
    expect(peer.batches).toHaveLength(0);
    // Unknown root reach: only own, person-device and household classes.
    out = await ask(chain(new MemoryTier(), [peer, hub], standingPolicy(levels), false, null), [item(x.cid, 'a')]);
    expect(why(out[0][1]).kind).toBe('not-held');
  });

  it('5. tiers strictly: via is the rest, a tier named in tried is skipped, refusals are final', async () => {
    const [a, b, c] = [new ScriptedTier('agent:a', 'person-device'), new ScriptedTier('agent:b', 'household'), new ScriptedTier('agent:c', 'hub')];
    const x = await leaf('third');
    b.blobs.set(x.cid.toString(), x.bytes);
    c.blobs.set(x.cid.toString(), x.bytes);
    a.behaviour = { notHeldNaming: ['agent:b'] };
    const out = await ask(chain(new MemoryTier(), [a, b, c]), [item(x.cid, 'a')]);
    expect(out[0][1]).toEqual(x.bytes);
    expect(a.batches[0].via).toEqual(['agent:b', 'agent:c']);
    expect(b.batches).toHaveLength(0);

    const refused = absence.refused('agent:a', 'outside-standing');
    const [r1, r2] = [new ScriptedTier('agent:a', 'person-device'), new ScriptedTier('agent:b', 'household')];
    r1.behaviour = { answerAll: refused };
    r2.blobs.set(x.cid.toString(), x.bytes);
    const out2 = await ask(chain(new MemoryTier(), [r1, r2]), [item(x.cid, 'a')]);
    expect(why(out2[0][1])).toEqual(refused.why);
    expect(r2.batches).toHaveLength(0);

    // A tier failing as a whole advances, and absence is then not established.
    const [f1, f2] = [new ScriptedTier('agent:a', 'person-device'), new ScriptedTier('agent:b', 'household')];
    f1.behaviour = { fail: true };
    const out3 = await ask(chain(new MemoryTier(), [f1, f2]), [item(x.cid, 'a')]);
    expect(why(out3[0][1])).toEqual({ kind: 'unreachable', tier: 'agent:a' });
    expect(f2.batches).toHaveLength(1);
  });

  it('6, 7. verifies every hop, writes back before returning, and a failed write-back still delivers', async () => {
    const [bad, good] = [new ScriptedTier('agent:bad', 'person-device'), new ScriptedTier('agent:good', 'household')];
    bad.behaviour = { corrupt: true };
    const x = await leaf('good bytes');
    good.blobs.set(x.cid.toString(), x.bytes);
    const own = new MemoryTier();
    const c = chain(own, [bad, good]);
    expect((await ask(c, [item(x.cid, 'a')]))[0][1]).toEqual(x.bytes);
    expect(own.blobs.has(x.cid.toString())).toBe(true);
    expect(c.stats().perTier['agent:bad'].integrity).toBe(1);
    good.batches.length = 0;
    await ask(c, [item(x.cid, 'a')]);
    expect(good.batches).toHaveLength(0);

    const refusing = new MemoryTier();
    refusing.put = async () => {
      throw new Error('read-only');
    };
    const out = await ask(chain(refusing, [good]), [item(x.cid, 'a')]);
    expect(out[0][1]).toEqual(x.bytes);
  });

  it('8. past the budget, an item is over budget with its come-back time and no tier is dialled', async () => {
    const t = new ScriptedTier('agent:a', 'household');
    const x = await leaf('budget');
    t.blobs.set(x.cid.toString(), x.bytes);
    const closed: ChainPolicy = { ...allowAll, admit: () => ({ retryAfterMs: 2000, retryAtEpochSecs: 1_700_000_002, rule: 'phone-forward' }) };
    const out = await ask(chain(new MemoryTier(), [t], closed), [item(x.cid, 'a')]);
    const r = out[0][1] as Extract<NamedAbsence, { kind: 'absent' }>;
    expect(r.unanswered.why).toEqual({ kind: 'over-budget', by: 'agent:uhCAkphone', rule: 'phone-forward' });
    expect(r.unanswered.comeBack).toEqual({ when: 'at', epochSecs: 1_700_000_002 });
    expect(t.batches).toHaveLength(0);
  });

  it('9, 10. names every tier tried in order, rewrites own, and answers each item once', async () => {
    const tiers = [new ScriptedTier('agent:a', 'person-device'), new ScriptedTier('agent:b', 'household'), new ScriptedTier('agent:c', 'hub')];
    tiers[0].behaviour = { answerAll: absence.notHeld(['own']) };
    const nowhere = await leaf('nowhere');
    const out = await ask(chain(new MemoryTier(), tiers), [item(nowhere.cid, 'a')]);
    expect(why(out[0][1])).toEqual({ kind: 'not-held', tried: ['agent:a', 'agent:b', 'agent:c'] });

    const t = new ScriptedTier('agent:a', 'household');
    const x = await leaf('x');
    t.blobs.set(x.cid.toString(), x.bytes);
    const y = await leaf('y');
    const answered = await ask(chain(new MemoryTier(), [t]), [item(x.cid, 'p'), item(x.cid, 'q'), item(y.cid, 'r')]);
    expect(answered).toHaveLength(3);
    expect(t.batches[0].items.filter((i) => i.cid.equals(x.cid))).toHaveLength(1);
  });

  it('walks over a chain, fetching each node from the outer tier once, and reads an unread path as a named absence without waiting on a timer', async () => {
    const hub = new ScriptedTier('agent:hub', 'household');
    const source = new MemoryTier();
    const { root } = await build(source, vectors.walk.tree);
    for (const [k, v] of source.blobs) hub.blobs.set(k, v);
    const own = new MemoryTier();
    own.blobs.set(root.toString(), source.blobs.get(root.toString())!);
    const c = chain(own, [hub]);
    const walked = await collectWalk(c, root, View.of([vpath('src/lib'), vpath('docs')]));
    expect(walked.nodes.map((n) => n.path.length)).toEqual([0, 1, 1, 2]);
    const asked = hub.batches.flatMap((b) => b.items.map((i) => i.cid.toString()));
    expect(new Set(asked).size).toBe(asked.length);
    expect(hub.batches).toHaveLength(2);

    // Offline: the own tier holds the spine but not this leaf, and the read settles with no
    // timer elapsed (H20).
    vi.useFakeTimers();
    try {
      const offline = chain(own, [hub], allowAll, true);
      const r = await readPath(offline, root, vpath('docs/guide.md'));
      expect(r).not.toBeInstanceOf(Uint8Array);
      const named = r as Extract<NamedAbsence, { kind: 'absent' }>;
      expect(named.kind).toBe('absent');
      expect(named.unanswered.why).toEqual({ kind: 'not-held', tried: [] });
      expect(named.unanswered.comeBack).toEqual({ when: 'on-event', event: 'tier-reachable' });
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });

  it('34. a tampering tier is an integrity error, and a later tier\'s silence does not hide it', async () => {
    const x = await leaf('the real bytes');
    const liar = new ScriptedTier('agent:a', 'household');
    liar.behaviour = { corrupt: true };
    const silent = new ScriptedTier('agent:b', 'household');
    silent.behaviour = { fail: true };
    const out = await ask(chain(new MemoryTier(), [liar, silent]), [item(x.cid, 'a')]);
    expect(why(out[0][1])).toEqual({ kind: 'integrity', from: 'agent:a' });
  });

  it('34, continued. integrity outranks a later refusal and the budget refusing the next dial', async () => {
    const x = await leaf('the real bytes');
    const liar = new ScriptedTier('agent:a', 'household');
    liar.behaviour = { corrupt: true };
    const refusing = new ScriptedTier('agent:b', 'household');
    refusing.behaviour = { answerAll: absence.refused('agent:b', 'outside-standing') };
    const refused = await ask(chain(new MemoryTier(), [liar, refusing]), [item(x.cid, 'a')]);
    expect(why(refused[0][1])).toEqual({ kind: 'integrity', from: 'agent:a' });

    const never = new ScriptedTier('agent:b', 'household');
    const refuseB: ChainPolicy = {
      ...allowAll,
      admit: (to) => (to === 'agent:b' ? { retryAfterMs: 1000, retryAtEpochSecs: 1_700_000_001, rule: 'per-tier' } : { release() {} }),
    };
    const overBudget = await ask(chain(new MemoryTier(), [liar, never], refuseB), [item(x.cid, 'a')]);
    expect(why(overBudget[0][1])).toEqual({ kind: 'integrity', from: 'agent:a' });
    expect(never.batches).toHaveLength(0);
  });

  it('40. a tier\'s integrity statement is withheld, not an error, and a later tier still answers', async () => {
    const x = await leaf('held by b');
    const claiming = new ScriptedTier('agent:a', 'household');
    claiming.behaviour = { answerAll: absence.integrity('agent:a') };
    const alone = await ask(chain(new MemoryTier(), [claiming]), [item(x.cid, 'a')]);
    expect(why(alone[0][1])).toEqual({ kind: 'unreachable', tier: 'agent:a' });

    const holder = new ScriptedTier('agent:b', 'household');
    holder.blobs.set(x.cid.toString(), x.bytes);
    const answered = await ask(chain(new MemoryTier(), [claiming, holder]), [item(x.cid, 'a')]);
    expect(answered[0][1]).toBeInstanceOf(Uint8Array);
  });

  it('36. a forwarder\'s tried strikes only the tiers it was handed as via', async () => {
    const x = await leaf('held by b');
    const liar = new ScriptedTier('agent:a', 'household');
    liar.behaviour = { notHeldNaming: ['agent:z-never-dialled'] };
    const honest = new ScriptedTier('agent:b', 'household');
    const out = await ask(chain(new MemoryTier(), [liar, honest]), [item(x.cid, 'a')]);
    expect(why(out[0][1])).toEqual({ kind: 'not-held', tried: ['agent:a', 'agent:b'] });
    expect(honest.batches).toHaveLength(1);
  });

  it('asks an outer tier in rounds of at most maxRequestItems, each item once', async () => {
    const leaves = await Promise.all([0, 1, 2, 3, 4].map((i) => leaf(`round ${i}`)));
    const hub = new ScriptedTier('agent:hub', 'hub');
    for (const l of leaves) hub.blobs.set(l.cid.toString(), l.bytes);
    const byId = new Map([[hub.record.id, hub]]);
    const c = new TierChain(new MemoryTier(), [hub.record], { connect: (r) => byId.get(r.id) ?? null }, allowAll, {
      maxRequestItems: 2,
    });
    const out = await ask(c, leaves.map((l, i) => item(l.cid, `f${i}`)));
    expect(out.filter(([, r]) => r instanceof Uint8Array)).toHaveLength(5);
    expect(hub.batches.map((b) => b.items.length)).toEqual([2, 2, 1]);

    // A tier answering soft is asked each item once, never again in a later round.
    const soft = new ScriptedTier('agent:soft', 'hub');
    soft.behaviour = { answerAll: absence.membershipUnknown('agent:soft') };
    const bySoft = new Map([[soft.record.id, soft]]);
    const d = new TierChain(new MemoryTier(), [soft.record], { connect: (r) => bySoft.get(r.id) ?? null }, allowAll, {
      maxRequestItems: 2,
    });
    await ask(d, leaves.map((l, i) => item(l.cid, `f${i}`)));
    expect(soft.batches.flatMap((b) => b.items)).toHaveLength(5);
  });

  it('orders its tiers for the root asked: warmth is per root', async () => {
    const x = await leaf('held by both');
    const one = CID.parse(vectors.order.roots[0]);
    const two = CID.parse(vectors.order.roots[1]);
    const warm = (root: CID) => ({ networkRank: 0, warmRoots: [root.toString()], servesExtracted: false, rttMs: null, lastSeenMs: null });
    const a = new ScriptedTier('agent:a', 'hub');
    const b = new ScriptedTier('agent:b', 'hub');
    a.record.proximity = warm(one);
    b.record.proximity = warm(two);
    for (const t of [a, b]) t.blobs.set(x.cid.toString(), x.bytes);
    const own = new MemoryTier();
    // The own tier keeps nothing, so the second read leaves the device too.
    own.put = async () => undefined;
    const c = chain(own, [a, b], standingPolicy({ household: 2, public: 4 }), false, 4);
    await c.fetchBlobs(two, [item(x.cid, 'f')], [], () => undefined);
    expect([a.batches.length, b.batches.length]).toEqual([0, 1]);
    await c.fetchBlobs(one, [item(x.cid, 'f')], [], () => undefined);
    expect([a.batches.length, b.batches.length]).toEqual([1, 1]);
  });
});
