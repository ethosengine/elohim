import type {
  ExperienceMomentPayload,
  PriorAttestation,
  SidecarName,
} from './types.js';

export function momentFixture(
  overrides: Partial<ExperienceMomentPayload> = {},
): ExperienceMomentPayload {
  return {
    recordedAt: '2026-04-18T14:32:11Z',
    subjectRef: 'human:matthew-manager',
    roleRef: 'role:as-entrepreneur',
    featureRef: 'feature:learning-journey',
    scenarioName: 'Welcome flow loads in under 2s',
    scenarioUri: 'features/lamad/learning-journey.feature',
    scenarioLine: 47,
    scenarioTags: ['@e2e', '@lamad'],
    status: 'passed',
    durationMs: 1842,
    commit: 'abc123d',
    runId: 'pipeline-42',
    computeFingerprint: 'matthew-alpha:device-family-node-base:abc123',
    sidecarArtifacts: { cucumber: 'blob:bafkrei-cucumber/xyz.json' },
    ...overrides,
  };
}

export function priorFixture(
  overrides: Partial<PriorAttestation> = {},
): PriorAttestation {
  return {
    momentEntryHash: 'uhCEk-prior-moment-hash',
    status: 'passed',
    valence: 'progress',
    magnitude: 'meaningful',
    evidenceType: 'first-pass-green',
    computeFingerprint: 'matthew-alpha:device-family-node-base:abc123',
    durationMs: 2100,
    sidecarArtifactNames: ['cucumber'] as readonly SidecarName[],
    ...overrides,
  };
}
