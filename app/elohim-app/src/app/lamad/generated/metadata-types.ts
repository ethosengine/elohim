// AUTO-GENERATED from lamad manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen

export interface ConceptMetadata {
  /** Brief summary of the concept */
  summary?: string;
  /** Original file path from import pipeline */
  sourcePath?: string;
  /** IDs of related content nodes */
  relatedNodeIds?: string[];
  /** Estimated time to consume in minutes */
  estimatedMinutes?: number;
  /** Blob path or resolved URL for concept thumbnail */
  thumbnailUrl?: string;
  /** Bloom's taxonomy level (remember, understand, apply, analyze, evaluate, create) */
  bloomsLevel?: string;
  /** Reference to source document */
  sourceDoc?: string;
  /** Inline relationship declarations from import */
  relationships?: { type?: string; targetId?: string }[];
  /** Decentralized identifier for the concept */
  did?: string;
  /** Open Graph metadata for social sharing */
  openGraphMetadata?: Record<string, unknown>;
  /** JSON-LD or schema.org structured data */
  linkedData?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface AssessmentMetadata {
  /** Reference to the psychometric instrument definition (content ID) */
  instrument?: string;
  /** Assessment mode determining scoring and feedback behavior */
  mode?: 'mastery' | 'discovery' | 'reflection';
  /** Scoring configuration — instrument-specific rules for translating responses to scores */
  scoringRules?: Record<string, unknown>;
  /** Subscale definitions for multi-dimensional assessments */
  subscales?: { id?: string; name?: string; weight?: number }[];
  [key: string]: unknown;
}

export interface ExperienceStoryMetadata {
  /** CID or slug reference to the subject ContentNode (human or collective). */
  subjectRef: string;
  /** CID or slug reference to the role ContentNode. */
  roleRef: string;
  /** CID or slug reference to the feature ContentNode (gherkin feature canonical entry). */
  featureRef: string;
  /** Optional human-readable EPR alias, e.g. epr:experience-story/matthew-manager/as-entrepreneur/learning-journey. */
  aliasEpr?: string;
}

export interface ExperienceMomentMetadata {
  /** CID or slug reference to the subject ContentNode (human or collective). */
  subjectRef: string;
  /** CID or slug reference to the role ContentNode. */
  roleRef: string;
  /** CID or slug reference to the feature ContentNode (gherkin feature canonical entry). */
  featureRef: string;
  /** The scenario name (Scenario: or Scenario Outline: line from the feature file). */
  scenarioName: string;
  /** Relative path from genesis/a2o/features to the feature file, e.g. 'lamad/learning-paths/path-progression.feature'. */
  scenarioUri: string;
  /** Line number where the scenario starts in the feature file. */
  scenarioLine?: number;
  /** Cucumber tags from the scenario (e.g., @regression, @smoke). */
  scenarioTags?: string[];
  /** Scenario execution status. */
  status: 'passed' | 'failed' | 'pending' | 'skipped';
  /** Total execution time in milliseconds. */
  durationMs: number;
  /** Git commit SHA (abbreviated 7 chars or full 40 chars). */
  commit: string;
  /** Unique identifier for this test run (e.g., CI build ID or local run timestamp). */
  runId: string;
  /** Format: {pod}:{deviceArchetype}:{archetypeRevisionHash} — identifies the compute environment. */
  computeFingerprint: string;
  /** Present when status=failed; classifies the error (e.g., 'AssertionError/timeout', 'NetworkError/503'). */
  errorClass?: string;
  /** References to test artifacts (CIDs or paths). */
  sidecarArtifacts?: { cucumber?: string; observation?: string; screenshot?: string; trace?: string; console?: string };
  /** CID or slug reference to the parent experience-story ContentNode. */
  relatedExperienceStory?: string;
}

export interface PathMetadata {
  /** journey | guided | self-paced | assessment */
  pathType?: string;
  /** beginner | intermediate | advanced */
  difficulty?: string;
  /** Human-readable duration (e.g. '6-8 hours') */
  estimatedDuration?: string;
  version?: string;
  purpose?: string;
  /** Blob path or resolved URL for path thumbnail */
  thumbnailUrl?: string;
  thumbnailAlt?: string;
  contributors?: string[];
  prerequisitePaths?: string[];
  attestationsGranted?: string[];
  [key: string]: unknown;
}
