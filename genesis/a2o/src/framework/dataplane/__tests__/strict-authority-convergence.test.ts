/* eslint-disable @typescript-eslint/require-await -- injected async fakes resolve synchronously by contract */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  completeOnceWithinDeadline,
  waitForStrictAuthorityConvergence,
} from '../strict-authority-convergence.js';

void describe('waitForStrictAuthorityConvergence', () => {
  void it('accepts delayed convergence within the original shared deadline', async () => {
    let now = 0;
    let probes = 0;
    const result = await waitForStrictAuthorityConvergence(
      ['alpha', 'apex'],
      75,
      async observer => (++probes >= 3 || observer === 'alpha' ? undefined : 'wrong action'),
      {
        intervalMs: 10,
        now: () => now,
        sleep: async ms => {
          now += ms;
        },
      }
    );
    assert.equal(result.converged, true);
  });

  void it('fails when the author-anchored deadline is already expired without probing', async () => {
    let probes = 0;
    const result = await waitForStrictAuthorityConvergence(
      ['alpha'],
      75,
      async () => {
        probes += 1;
        return undefined;
      },
      { now: () => 75 }
    );
    assert.equal(result.converged, false);
    assert.equal(probes, 0);
  });

  void it('never accepts a stable action that differs from the author receipt', async () => {
    let now = 0;
    const result = await waitForStrictAuthorityConvergence(
      ['alpha', 'apex'],
      30,
      async observer =>
        observer === 'apex' ? 'action expected AUTHOR, observed LOCAL' : undefined,
      {
        intervalMs: 10,
        now: () => now,
        sleep: async ms => {
          now += ms;
        },
      }
    );
    assert.equal(result.converged, false);
    assert.deepEqual(result.lastMismatches, {
      apex: 'action expected AUTHOR, observed LOCAL',
    });
  });
});

void describe('completeOnceWithinDeadline', () => {
  void it('runs the full acceptance probe once after convergence', async () => {
    let calls = 0;
    const result = await completeOnceWithinDeadline(
      75,
      async remainingMs => {
        calls += 1;
        assert.equal(remainingMs, 65);
        return 'accepted';
      },
      () => 10
    );
    assert.equal(result, 'accepted');
    assert.equal(calls, 1);
  });

  void it('rejects a browser acceptance probe that completes after the shared deadline', async () => {
    let now = 74;
    await assert.rejects(
      completeOnceWithinDeadline(
        75,
        async () => {
          now = 76;
        },
        () => now
      ),
      /completed after shared authority deadline/
    );
  });
});
