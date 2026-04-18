/**
 * Types for the v1 mechanical discernment gate.
 *
 * The gate is a pure function; these types define its input and output.
 * Backed by spec 2026-04-18-experience-story-epr-design.md §5–§7.
 */

export type Valence =
  | 'progress'
  | 'discovery'
  | 'regression'
  | 'validation'
  | 'witness'
  | 'refinement'
  | 'confirmation';

export type Magnitude = 'small' | 'meaningful' | 'significant';

export type EvidenceType =
  | 'first-pass-green'
  | 'novel-failure-class'
  | 'known-cause-recurrence'
  | 'failure-mode-confirmed'
  | 'recovery'
  | 'cross-fingerprint-attestation'
  | 'evidence-enriched';

export type ScenarioStatus = 'passed' | 'failed' | 'pending' | 'skipped';

export type SidecarName = 'cucumber' | 'observation' | 'screenshot' | 'trace' | 'console';

/** Format: {pod}:{deviceArchetype}:{archetypeRevisionHash}. */
export type ComputeFingerprint = string;

/**
 * A moment as passed to the discerner. Mirrors the experience-moment
 * frontmatter (spec §6.2). Sidecar values are blob references; their
 * presence (keys) is what refinement rule checks.
 */
export interface ExperienceMomentPayload {
  recordedAt: string;
  subjectRef: string;
  roleRef: string;
  featureRef: string;
  scenarioName: string;
  scenarioUri: string;
  scenarioLine?: number;
  scenarioTags: readonly string[];
  status: ScenarioStatus;
  durationMs: number;
  commit: string;
  runId: string;
  computeFingerprint: ComputeFingerprint;
  errorClass?: string;
  sidecarArtifacts: Partial<Record<SidecarName, string>>;
}

/** A prior attestation, as queried from storage projection. */
export interface PriorAttestation {
  momentEntryHash: string;
  status: ScenarioStatus;
  valence: Valence;
  magnitude: Magnitude;
  evidenceType: EvidenceType;
  computeFingerprint: ComputeFingerprint;
  errorClass?: string;
  durationMs: number;
  sidecarArtifactNames: readonly SidecarName[];
}

/** The input bundle the gate receives. */
export interface DiscernmentInput {
  moment: ExperienceMomentPayload;
  priors: {
    /** Most recent attestation on this experience-story from ANY compute fingerprint. */
    latestAny?: PriorAttestation;
    /** Most recent attestation from THIS moment's compute fingerprint. */
    latestSameFingerprint?: PriorAttestation;
    /** Every error class ever attested on this experience-story. */
    knownErrorClasses: ReadonlySet<string>;
  };
}

/** The gate's output — attached to the Holochain Link and mirrored on the EconomicEvent. */
export interface StoryPointTag {
  v: 1;
  valence: Valence;
  magnitude: Magnitude;
  evidenceType: EvidenceType;
  computeFingerprint: ComputeFingerprint;
  runId: string;
  commit: string;
  momentEntryHash: string;
  discernerId: 'discernment-service-v1-mechanical';
}
