/* eslint-disable @typescript-eslint/no-floating-promises -- node:test describe/it
   return promises that the test runner itself consumes; awaiting them is wrong. */
/**
 * Steward grade decisions (scripts/lib/steward-grade-plan.ts): widen only to the
 * authored reach, never narrow, refuse anything not own-authored, refuse intimate and
 * unknown reaches.
 */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  authoredReachFor,
  classifyRow,
  decideWiden,
  isShedPatch,
  shedDelayMs,
  stewardAuthorshipRefusal,
  stewardSet,
  widenLanded,
  widenPatch,
  type ChainAnswer,
  type HeadB64,
  type LineageB64,
} from '../lib/steward-grade-plan.js';

const ME = 'uhCAkMEmeMEmeMEmeMEmeMEmeMEmeMEmeMEmeMEmeMEmeMEme';
const NEEDS = 'needs-authorship';
const OTHER = 'uhCAkOTHERotherOTHERotherOTHERotherOTHERotherOTH';
const COSTEWARD = 'uhCAkADAMadamADAMadamADAMadamADAMadamADAMadamADA';
const SOLO = stewardSet(ME);
const PAIR = stewardSet(ME, [[COSTEWARD, 'ws://adam-conductor:4444']]);

describe('authoredReachFor', () => {
  it('reads a content atom’s reach only, and a path’s reach else visibility', () => {
    assert.equal(
      authoredReachFor('content', { reach: 'commons', visibility: 'public' }),
      'commons'
    );
    assert.equal(authoredReachFor('content', { visibility: 'public' }), undefined);
    assert.equal(authoredReachFor('path', { visibility: 'public' }), 'public');
    assert.equal(authoredReachFor('path', { reach: 'commons', visibility: 'public' }), 'commons');
    assert.equal(authoredReachFor('path', { reach: '  ' }), undefined);
  });
});

describe('classifyRow', () => {
  it('asks for authorship when a gated row is narrower than the authored reach', () => {
    assert.deepEqual(classifyRow('commons', { kind: 'gated', requiredReach: 'private' }), {
      action: NEEDS,
      from: 'private',
      to: 'commons',
    });
  });

  it('asks for authorship when a served row is narrower (public → commons)', () => {
    assert.deepEqual(classifyRow('commons', { kind: 'served', reach: 'public' }), {
      action: NEEDS,
      from: 'public',
      to: 'commons',
    });
  });

  it('widens only to exactly the authored reach, never past it', () => {
    const v = classifyRow('public', { kind: 'gated', requiredReach: 'private' });
    assert.equal(v.action, NEEDS);
    assert.equal(v.to, 'public');
    assert.deepEqual(widenPatch('public'), { reach: 'public' });
  });

  it('never narrows: a row at or above the authored openness is current', () => {
    assert.equal(classifyRow('commons', { kind: 'served', reach: 'commons' }).action, 'current');
    // public authored, row already commons (more open): left alone, not narrowed.
    const v = classifyRow('public', { kind: 'served', reach: 'commons' });
    assert.deepEqual(v, { action: 'current', from: 'commons', to: 'public' });
  });

  it('refuses intimate, community and every other authored reach outside the fence', () => {
    for (const r of ['intimate', 'community', 'private', 'self', 'trusted', 'familiar']) {
      const v = classifyRow(r, { kind: 'gated', requiredReach: 'private' });
      assert.equal(v.action, 'refused', r);
    }
  });

  it('refuses an unknown or missing authored reach', () => {
    const unknown = classifyRow('everyone', { kind: 'gated', requiredReach: 'private' });
    assert.equal(unknown.action, 'refused');
    assert.match((unknown as { reason: string }).reason, /unknown authored reach/);
    assert.equal(classifyRow(undefined, { kind: 'served', reach: 'private' }).action, 'refused');
  });

  it('refuses an unknown stored reach, a reachless row, and an absent row', () => {
    assert.equal(classifyRow('commons', { kind: 'served', reach: 'secret' }).action, 'refused');
    assert.equal(classifyRow('commons', { kind: 'served', reach: null }).action, 'refused');
    assert.equal(classifyRow('commons', { kind: 'gated', requiredReach: null }).action, 'refused');
    assert.equal(classifyRow('commons', { kind: 'absent' }).action, 'refused');
  });
});

const head: HeadB64 = { headActionHash: 'uhCkkHEAD', author: ME };
const lineage = (over: Partial<LineageB64> = {}): LineageB64 => ({
  rootAuthor: ME,
  contentId: 'c1',
  candidates: [
    { actionHash: 'uhCkkROOT', author: ME, fetchOutcome: 'fetched', inRoot: true },
    { actionHash: 'uhCkkHEAD', author: ME, fetchOutcome: 'fetched', inRoot: true },
  ],
  otherRootCandidates: 0,
  unfetchableCandidates: 0,
  invalidLinkTargets: 0,
  truncated: false,
  ...over,
});

describe('stewardAuthorshipRefusal', () => {
  it('admits the node when its own agent authored the root and every version', () => {
    assert.equal(stewardAuthorshipRefusal(SOLO, head, lineage()), undefined);
  });

  it('admits a co-steward’s root, head and versions alongside this node’s', () => {
    const l = lineage({
      rootAuthor: COSTEWARD,
      candidates: [
        { actionHash: 'uhCkkROOT', author: COSTEWARD, fetchOutcome: 'fetched', inRoot: true },
        { actionHash: 'uhCkkMINE', author: ME, fetchOutcome: 'fetched', inRoot: true },
        { actionHash: 'uhCkkHEAD', author: COSTEWARD, fetchOutcome: 'fetched', inRoot: true },
      ],
    });
    assert.equal(stewardAuthorshipRefusal(PAIR, { ...head, author: COSTEWARD }, l), undefined);
    // …but without that co-steward in the set, the same chain is refused.
    assert.match(
      stewardAuthorshipRefusal(SOLO, { ...head, author: COSTEWARD }, l) ?? '',
      /head uhCkkHEAD authored by uhCAkADAMadamADA…, not this node$/
    );
  });

  it('refuses a key outside the set and names it, even with co-stewards', () => {
    assert.match(
      stewardAuthorshipRefusal(PAIR, head, lineage({ rootAuthor: OTHER })) ?? '',
      /root authored by uhCAkOTHERotherO…, not this node or one of its 1 co-steward/
    );
    assert.match(
      stewardAuthorshipRefusal(PAIR, { ...head, author: OTHER }, lineage()) ?? '',
      /head/
    );
    const foreign = lineage({
      candidates: [
        { actionHash: 'uhCkkROOT', author: ME, fetchOutcome: 'fetched', inRoot: true },
        { actionHash: 'uhCkkFOREIGN', author: OTHER, fetchOutcome: 'fetched', inRoot: true },
      ],
    });
    assert.match(stewardAuthorshipRefusal(PAIR, head, foreign) ?? '', /version .* authored by/);
    const anon = lineage({
      candidates: [
        { actionHash: 'uhCkkROOT', author: null, fetchOutcome: 'fetched', inRoot: true },
      ],
    });
    assert.ok(stewardAuthorshipRefusal(PAIR, head, anon));
  });

  it('keeps every lineage-completeness check with co-stewards admitted', () => {
    assert.ok(stewardAuthorshipRefusal(PAIR, head, lineage({ otherRootCandidates: 1 })));
    assert.ok(stewardAuthorshipRefusal(PAIR, head, lineage({ unfetchableCandidates: 1 })));
    assert.ok(stewardAuthorshipRefusal(PAIR, head, lineage({ invalidLinkTargets: 1 })));
    assert.ok(stewardAuthorshipRefusal(PAIR, head, lineage({ truncated: true })));
    assert.ok(stewardAuthorshipRefusal(PAIR, head, lineage({ candidates: [] })));
    assert.ok(stewardAuthorshipRefusal(PAIR, head, null));
    const outOfRoot = lineage({
      candidates: [{ actionHash: 'uhCkkX', author: ME, fetchOutcome: 'fetched', inRoot: false }],
    });
    assert.ok(stewardAuthorshipRefusal(PAIR, head, outOfRoot));
  });

  it('never lets a co-steward entry stand in for this node', () => {
    assert.equal(stewardSet(ME, [[ME, 'ws://self']]).coStewards.size, 0);
  });
});

describe('decideWiden', () => {
  const needs = classifyRow('commons', { kind: 'gated', requiredReach: 'private' });
  const chain = (over: Partial<LineageB64> = {}, h: HeadB64 = head): ChainAnswer => ({
    kind: 'chain',
    head: h,
    lineage: lineage(over),
  });

  it('widens over a chain the stewards authored', () => {
    assert.deepEqual(decideWiden(needs, PAIR, chain({ rootAuthor: COSTEWARD })), {
      action: 'widen',
      mode: 'chain',
      from: 'private',
      to: 'commons',
    });
  });

  it('refuses a chain with an author outside the set', () => {
    const d = decideWiden(needs, PAIR, chain({ rootAuthor: OTHER }));
    assert.equal(d.action, 'refused');
    assert.match((d as { reason: string }).reason, /root authored by uhCAkOTHER/);
  });

  it('widens as a new root when the row exists but no version chain does', () => {
    assert.deepEqual(decideWiden(needs, SOLO, { kind: 'no-chain' }), {
      action: 'widen',
      mode: 'new-root',
      from: 'private',
      to: 'commons',
    });
  });

  it('refuses (retry later) when the zome answers PENDING — never read as no chain', () => {
    const d = decideWiden(needs, PAIR, { kind: 'pending', detail: 'PENDING: not retrievable' });
    assert.equal(d.action, 'refused');
    assert.match((d as { reason: string }).reason, /pending/);
  });

  it('never narrows: a current row stays current whatever the conductor says', () => {
    const current = classifyRow('public', { kind: 'served', reach: 'commons' });
    for (const a of [{ kind: 'no-chain' } as ChainAnswer, chain()]) {
      assert.deepEqual(decideWiden(current, PAIR, a), current);
    }
  });

  it('keeps the commons fence: an intimate or unknown authored reach is refused', () => {
    for (const r of ['intimate', 'community', 'everyone']) {
      const v = classifyRow(r, { kind: 'gated', requiredReach: 'private' });
      const d = decideWiden(v, PAIR, { kind: 'no-chain' });
      assert.equal(d.action, 'refused', r);
    }
    // An absent row never becomes a new root.
    const absent = classifyRow('commons', { kind: 'absent' });
    assert.equal(decideWiden(absent, PAIR, { kind: 'no-chain' }).action, 'refused');
  });
});

describe('PATCH shed ladder', () => {
  it('waits out catching-up and a zome websocket timeout, nothing else', () => {
    assert.equal(isShedPatch(503, '{"status":"catching-up","retryAfter":2}'), true);
    assert.equal(isShedPatch(503, '{"error":"Zome call failed: Websocket error: Timeout"}'), true);
    assert.equal(
      isShedPatch(
        503,
        '{"error":"Zome call failed: Source chain error: Attempted to commit a bundle to the source chain, but the source chain head has moved since the bundle began"}'
      ),
      true
    );
    assert.equal(isShedPatch(503, '{"error":"Zome call failed: source chain head moved"}'), false);
    assert.equal(isShedPatch(500, '{"status":"catching-up"}'), false);
    assert.equal(isShedPatch(503, 'not json'), false);
  });

  it('honours Retry-After (header, else body), default 5 s, within 2–15 s', () => {
    assert.equal(shedDelayMs('7', ''), 7000);
    assert.equal(shedDelayMs(null, '{"status":"catching-up","retryAfter":2}'), 2000);
    assert.equal(shedDelayMs(null, '{"error":"timeout"}'), 5000);
    assert.equal(shedDelayMs('600', ''), 15_000);
    assert.equal(shedDelayMs('0', ''), 2000);
  });
});

describe('widenLanded', () => {
  it('confirms only an anonymous 200 at exactly the authored reach', () => {
    assert.equal(widenLanded('commons', { kind: 'served', reach: 'commons' }), true);
    assert.equal(widenLanded('commons', { kind: 'served', reach: 'public' }), false);
    assert.equal(widenLanded('commons', { kind: 'gated', requiredReach: 'private' }), false);
    assert.equal(widenLanded('commons', { kind: 'absent' }), false);
  });
});
