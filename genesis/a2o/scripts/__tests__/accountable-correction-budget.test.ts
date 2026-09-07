/* eslint-disable @typescript-eslint/no-floating-promises -- node:test describe/it
   return promises that the test runner itself consumes; awaiting them is wrong. */
/**
 * T7b (2026-09-08) — station 4 (crash window)'s post-restart deadline, derived from the
 * live rotation after T6, not a constant.
 *
 * `feedback_projector.rs`'s `SweepScheduler` is deliberately in-memory (its own doc
 * comment: "a restart forgets it, every member starts Hot") — a storage restart throws
 * away T6's cold-retire saving, so the first post-restart rotation costs a full
 * `ceil(N/8)` sweeps again, the same bound the pre-T6 projector always paid. This test
 * covers only the pure arithmetic (`deriveCrashRestartBudgetMs`); the peer-env/DB
 * plumbing around it (`crashRestartBudget`) needs a live mesh and is covered by the
 * mesh receipt instead.
 */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  deriveCrashRestartBudgetMs,
  DHT_PROPAGATION_MS,
  MEMBERS_PER_SWEEP,
  RESTART_SETTLE_MS,
} from '../../steps/dataplane/accountable-correction.helpers.js';

describe('deriveCrashRestartBudgetMs', () => {
  it('rounds N up to a whole sweep and adds the +2 publish margin', () => {
    // N=25 needs ceil(25/8)=4 sweeps to visit everyone once, plus 2 to confirm
    // nothing pending before publish_generation republishes.
    const b = deriveCrashRestartBudgetMs(25, 60_000);
    assert.equal(b.sweeps, 4 + 2);
    assert.equal(b.nTotal, 25);
    assert.equal(b.sweepMs, 60_000);
  });

  it('an exact multiple of MEMBERS_PER_SWEEP does not round up an extra sweep', () => {
    const b = deriveCrashRestartBudgetMs(MEMBERS_PER_SWEEP * 3, 60_000);
    assert.equal(b.sweeps, 3 + 2);
  });

  it('budgets restart settle + sweeps*sweepMs, with no DHT floor by default', () => {
    const b = deriveCrashRestartBudgetMs(8, 60_000);
    assert.equal(b.dhtFloorMs, 0);
    assert.equal(b.restartSettleMs, RESTART_SETTLE_MS);
    assert.equal(b.budgetMs, RESTART_SETTLE_MS + b.sweeps * 60_000);
  });

  it('adds the 300 s DHT propagation floor only when the arm names a fresh act', () => {
    const cold = deriveCrashRestartBudgetMs(8, 60_000, false);
    const fresh = deriveCrashRestartBudgetMs(8, 60_000, true);
    assert.equal(fresh.dhtFloorMs, DHT_PROPAGATION_MS);
    assert.equal(fresh.budgetMs - cold.budgetMs, DHT_PROPAGATION_MS);
  });

  it('never divides by zero or goes negative for N=0 (a brand-new mesh)', () => {
    const b = deriveCrashRestartBudgetMs(0, 60_000);
    assert.equal(b.sweeps, 2);
    assert.ok(b.budgetMs > 0);
  });

  it('a negative N (a caller bug, never a real count) clamps to zero members', () => {
    const b = deriveCrashRestartBudgetMs(-5, 60_000);
    assert.equal(b.nTotal, -5);
    assert.equal(b.sweeps, 2);
  });

  it('honours the peer-reported sweep interval, not the product default', () => {
    // The lane pins ELOHIM_FEEDBACK_SWEEP_SECONDS=5 for measurement rounds; the
    // derivation must scale with the LIVE peer's sweep, never the 60 s default.
    const fast = deriveCrashRestartBudgetMs(40, 5_000);
    const slow = deriveCrashRestartBudgetMs(40, 60_000);
    assert.ok(fast.budgetMs < slow.budgetMs);
    assert.equal(fast.sweeps, slow.sweeps); // same N -> same sweep COUNT
  });

  it('scales with N: more subscriptions never yields a smaller budget', () => {
    const small = deriveCrashRestartBudgetMs(8, 60_000);
    const large = deriveCrashRestartBudgetMs(127, 60_000);
    assert.ok(large.budgetMs > small.budgetMs);
    assert.equal(large.sweeps, Math.ceil(127 / MEMBERS_PER_SWEEP) + 2);
  });
});
