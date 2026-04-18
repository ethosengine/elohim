import { describe, expect, it } from 'vitest';

import { discernMechanical } from './mechanical-discerner.js';
import { momentFixture, priorFixture } from './fixtures.js';

describe('discernMechanical — rule 2 (failed after prior-passed)', () => {
  it('mints discovery/meaningful/novel-failure-class when error class is new', () => {
    const moment = momentFixture({
      status: 'failed',
      errorClass: 'AssertionError/timeout',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({ status: 'passed' }),
          knownErrorClasses: new Set<string>(),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'discovery',
      magnitude: 'meaningful',
      evidenceType: 'novel-failure-class',
    });
  });

  it('mints regression/meaningful/known-cause-recurrence when error class was seen before', () => {
    const moment = momentFixture({
      status: 'failed',
      errorClass: 'NetworkError/503',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({ status: 'passed' }),
          knownErrorClasses: new Set(['NetworkError/503']),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'regression',
      magnitude: 'meaningful',
      evidenceType: 'known-cause-recurrence',
    });
  });
});

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
