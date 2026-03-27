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
