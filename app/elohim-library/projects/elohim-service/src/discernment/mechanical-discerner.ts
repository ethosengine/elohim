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

  return null;
}
