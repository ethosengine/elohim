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
  ownAuthorshipRefusal,
  widenLanded,
  widenPatch,
  type HeadB64,
  type LineageB64,
} from '../lib/steward-grade-plan.js';

const ME = 'uhCAkMEmeMEmeMEmeMEmeMEmeMEmeMEmeMEmeMEmeMEmeMEme';
const NEEDS = 'needs-authorship';
const OTHER = 'uhCAkOTHERotherOTHERotherOTHERotherOTHERotherOTH';

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

describe('ownAuthorshipRefusal', () => {
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

  it('admits the node when its own agent authored the root and every version', () => {
    assert.equal(ownAuthorshipRefusal(ME, head, lineage()), undefined);
  });

  it('refuses when the id has no version chain on the conductor', () => {
    assert.match(ownAuthorshipRefusal(ME, null, null) ?? '', /no version chain/);
  });

  it('refuses when another agent authored the root', () => {
    assert.match(
      ownAuthorshipRefusal(ME, head, lineage({ rootAuthor: OTHER })) ?? '',
      /root authored by/
    );
  });

  it('refuses when the elected head is another agent’s (an earned election elsewhere)', () => {
    assert.match(ownAuthorshipRefusal(ME, { ...head, author: OTHER }, lineage()) ?? '', /head/);
  });

  it('refuses when a foreign update sits in the chain the PATCH would build on', () => {
    const l = lineage({
      candidates: [
        { actionHash: 'uhCkkROOT', author: ME, fetchOutcome: 'fetched', inRoot: true },
        { actionHash: 'uhCkkFOREIGN', author: OTHER, fetchOutcome: 'fetched', inRoot: true },
      ],
    });
    assert.match(ownAuthorshipRefusal(ME, head, l) ?? '', /authored by/);
  });

  it('refuses when another root claims the id, or the lineage is incomplete', () => {
    assert.ok(ownAuthorshipRefusal(ME, head, lineage({ otherRootCandidates: 1 })));
    assert.ok(ownAuthorshipRefusal(ME, head, lineage({ unfetchableCandidates: 1 })));
    assert.ok(ownAuthorshipRefusal(ME, head, lineage({ invalidLinkTargets: 1 })));
    assert.ok(ownAuthorshipRefusal(ME, head, lineage({ truncated: true })));
    assert.ok(ownAuthorshipRefusal(ME, head, lineage({ candidates: [] })));
    assert.ok(ownAuthorshipRefusal(ME, head, null));
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
