import { describe, expect, it } from 'vitest';

import { discernMechanical } from './mechanical-discerner.js';
import { momentFixture } from './fixtures.js';

describe('discernMechanical — rule 1 (first-pass-green)', () => {
  it('mints progress/meaningful/first-pass-green for a passing moment with no prior', () => {
    const moment = momentFixture({ status: 'passed' });

    const tag = discernMechanical(
      {
        moment,
        priors: { knownErrorClasses: new Set<string>() },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).not.toBeNull();
    expect(tag).toMatchObject({
      v: 1,
      valence: 'progress',
      magnitude: 'meaningful',
      evidenceType: 'first-pass-green',
      computeFingerprint: moment.computeFingerprint,
      runId: moment.runId,
      commit: moment.commit,
      momentEntryHash: 'uhCEk-moment-hash',
      discernerId: 'discernment-service-v1-mechanical',
    });
  });
});
