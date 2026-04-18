import type {
  DiscernmentInput,
  EvidenceType,
  Magnitude,
  StoryPointTag,
  Valence,
} from './types.js';

function mkTag(
  input: DiscernmentInput,
  momentEntryHash: string,
  valence: Valence,
  magnitude: Magnitude,
  evidenceType: EvidenceType,
): StoryPointTag {
  return {
    v: 1,
    valence,
    magnitude,
    evidenceType,
    computeFingerprint: input.moment.computeFingerprint,
    runId: input.moment.runId,
    commit: input.moment.commit,
    momentEntryHash,
    discernerId: 'discernment-service-v1-mechanical',
  };
}

/**
 * v1 mechanical discernment gate. Pure function.
 * See spec §7.3 for rule ordering and rationale.
 */
export function discernMechanical(
  input: DiscernmentInput,
  momentEntryHash: string,
): StoryPointTag | null {
  const { moment, priors } = input;

  // Rule 1 — first-pass-green
  if (moment.status === 'passed' && !priors.latestAny) {
    return mkTag(input, momentEntryHash, 'progress', 'meaningful', 'first-pass-green');
  }

  // Rule 2 — failed after prior-passed → discovery (novel) or regression (known)
  // NOTE: Rule 2 must come BEFORE rule 3 so that a @validates-failure-mode scenario
  // that regressed (had prior-passed) gets classified by rule 2, not rule 3.
  if (moment.status === 'failed' && priors.latestAny?.status === 'passed') {
    const isNovel =
      !moment.errorClass || !priors.knownErrorClasses.has(moment.errorClass);
    return mkTag(
      input,
      momentEntryHash,
      isNovel ? 'discovery' : 'regression',
      'meaningful',
      isNovel ? 'novel-failure-class' : 'known-cause-recurrence',
    );
  }

  // Rule 3 — @validates-failure-mode scenario failing with no prior-passed attestation
  if (moment.status === 'failed' && moment.scenarioTags.includes('@validates-failure-mode')) {
    return mkTag(input, momentEntryHash, 'validation', 'meaningful', 'failure-mode-confirmed');
  }

  // Rule 4 — passed after prior-failed → recovery
  if (moment.status === 'passed' && priors.latestAny?.status === 'failed') {
    return mkTag(input, momentEntryHash, 'progress', 'meaningful', 'recovery');
  }

  // Rule 5 — witness (same status, DIFFERENT fingerprint)
  if (
    priors.latestAny &&
    priors.latestAny.status === moment.status &&
    priors.latestAny.computeFingerprint !== moment.computeFingerprint
  ) {
    return mkTag(
      input,
      momentEntryHash,
      'witness',
      'meaningful',
      'cross-fingerprint-attestation',
    );
  }

  return null;
}
