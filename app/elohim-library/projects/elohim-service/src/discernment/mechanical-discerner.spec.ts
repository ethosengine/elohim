import { describe, expect, it } from 'vitest';

import { discernMechanical } from './mechanical-discerner.js';
import { momentFixture, priorFixture } from './fixtures.js';

describe('discernMechanical — rule 5 (witness)', () => {
  it('mints witness/meaningful/cross-fingerprint-attestation when a passing scenario is validated by a NEW compute fingerprint', () => {
    const moment = momentFixture({
      status: 'passed',
      computeFingerprint: 'adam-alpha:device-family-laptop-small:def456',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'passed',
            computeFingerprint: 'matthew-alpha:device-family-node-base:abc123',
          }),
          knownErrorClasses: new Set<string>(),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'witness',
      magnitude: 'meaningful',
      evidenceType: 'cross-fingerprint-attestation',
    });
  });

  it('mints witness for a FAILING scenario confirmed by a new fingerprint (structural failure, not flake)', () => {
    const moment = momentFixture({
      status: 'failed',
      errorClass: 'AssertionError/timeout',
      computeFingerprint: 'jessica-alpha:device-family-mobile:ghi789',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'failed',
            valence: 'discovery',
            evidenceType: 'novel-failure-class',
            errorClass: 'AssertionError/timeout',
            computeFingerprint: 'matthew-alpha:device-family-node-base:abc123',
          }),
          knownErrorClasses: new Set(['AssertionError/timeout']),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'witness',
      evidenceType: 'cross-fingerprint-attestation',
    });
  });
});

describe('discernMechanical — rule 4 (recovery)', () => {
  it('mints progress/meaningful/recovery when a scenario passes after a prior-failed attestation', () => {
    const moment = momentFixture({ status: 'passed' });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'failed',
            valence: 'regression',
            evidenceType: 'known-cause-recurrence',
            errorClass: 'NetworkError/503',
          }),
          knownErrorClasses: new Set(['NetworkError/503']),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'progress',
      magnitude: 'meaningful',
      evidenceType: 'recovery',
    });
  });
});

describe('discernMechanical — rule 3 (validation)', () => {
  it('mints validation/meaningful/failure-mode-confirmed when a @validates-failure-mode scenario fails and there is no prior-passed attestation', () => {
    const moment = momentFixture({
      status: 'failed',
      scenarioTags: ['@e2e', '@lamad', '@validates-failure-mode'],
      errorClass: 'ExpectedFailure/unauthorized',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: { knownErrorClasses: new Set(['ExpectedFailure/unauthorized']) },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'validation',
      magnitude: 'meaningful',
      evidenceType: 'failure-mode-confirmed',
    });
  });

  it('yields to rule 2 when a @validates-failure-mode scenario fails AFTER a prior-passed attestation (the validation itself regressed)', () => {
    const moment = momentFixture({
      status: 'failed',
      scenarioTags: ['@e2e', '@validates-failure-mode'],
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

    expect(tag).toMatchObject({ valence: 'discovery' });
  });
});

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
