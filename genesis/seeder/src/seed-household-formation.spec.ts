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

import { householdProjectionSatisfied } from './seed-household-formation.js';

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
