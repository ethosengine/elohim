/**
 * Unit tests for the pure household-projection settle predicate in
 * seed-household-formation.ts.
 *
 * Encodes genesis #1182 Cluster B: the formation ceremony writes memberships on
 * the conductors, but the SQL projection (collective_cid stamp + participant
 * list) lags via DHT → gossip → projection_reconcile. The seeder must wait for
 * the projection to reflect the affirmed triad before it exits, or downstream
 * a2o reads "collective_cid not stamped" / "triad member missing from
 * participants". `householdProjectionSatisfied` is the pure decision the
 * (network) settle-wait loops on.
 */
import { describe, expect, it } from 'vitest';

import { describeProjectionGap, householdProjectionSatisfied } from './seed-household-formation.js';

const TRIAD = ['human-matthew-manager', 'human-jessica-spouse', 'human-james-son'];

describe('householdProjectionSatisfied', () => {
  it('is false when the collective is absent (projection has not run)', () => {
    expect(householdProjectionSatisfied(null, [], TRIAD)).toBe(false);
  });

  it('is false when collective_cid is not yet stamped', () => {
    expect(householdProjectionSatisfied({ slug: 'family-dowell' }, TRIAD, TRIAD)).toBe(false);
  });

  it('is false when a triad member is still missing from participants', () => {
    const collective = { collectiveCid: 'collective:uhCkkAAA' };
    const partial = ['human-matthew-manager', 'human-jessica-spouse']; // james missing
    expect(householdProjectionSatisfied(collective, partial, TRIAD)).toBe(false);
  });

  it('is true when cid is stamped AND all expected members are participants', () => {
    const collective = { collective_cid: 'collective:uhCkkAAA' };
    expect(householdProjectionSatisfied(collective, TRIAD, TRIAD)).toBe(true);
  });

  it('accepts participant rows as objects carrying an id/humanId', () => {
    const collective = { collectiveCid: 'collective:uhCkkAAA' };
    const rows = TRIAD.map((id) => ({ humanId: id, role: 'member' }));
    expect(householdProjectionSatisfied(collective, rows, TRIAD)).toBe(true);
  });

  it('ignores a present-but-empty collective_cid (not a real stamp)', () => {
    expect(householdProjectionSatisfied({ collectiveCid: '' }, TRIAD, TRIAD)).toBe(false);
  });
});

describe('describeProjectionGap', () => {
  const STAMPED = { collectiveCid: 'collective:uhCkkAAA' };

  it('names an absent collective row', () => {
    expect(describeProjectionGap(null, [], TRIAD)).toContain('absent');
  });

  it('names the unstamped cid, and what it saw instead', () => {
    const gap = describeProjectionGap({ slug: 'family-dowell' }, TRIAD, TRIAD);
    expect(gap).toContain('collective_cid unstamped');
    expect(gap).toContain('null');
  });

  it('names EVERY missing member, not just the first', () => {
    const gap = describeProjectionGap(STAMPED, ['human-james-son'], TRIAD);
    expect(gap).toContain('human-matthew-manager');
    expect(gap).toContain('human-jessica-spouse');
    expect(gap).toContain('present: human-james-son');
  });

  it('reports both legs when both are short', () => {
    const gap = describeProjectionGap({}, [], TRIAD);
    expect(gap).toContain('collective_cid unstamped');
    expect(gap).toContain('participants missing');
  });

  it('reads object participant rows the same way the predicate does', () => {
    const rows = TRIAD.map(id => ({ humanId: id }));
    expect(describeProjectionGap(STAMPED, rows, TRIAD)).toBe(
      'no gap detected on the final poll (race with settle)',
    );
  });

  it('never claims a gap the settle predicate would have accepted', () => {
    // The two must agree: satisfied ⇒ no gap named.
    expect(householdProjectionSatisfied(STAMPED, TRIAD, TRIAD)).toBe(true);
    expect(describeProjectionGap(STAMPED, TRIAD, TRIAD)).not.toContain('missing');
  });
});
