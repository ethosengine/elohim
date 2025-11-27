import { ContentNode } from './content-node.model';

/**
 * LearningPath - A curated journey through Territory resources.
 *
 * Holochain mapping:
 * - Entry type: "learning_path"
 * - Steps stored inline (they're small)
 * - resourceId in each step links to content_node entries via action hash
 */
export interface LearningPath {
  // Identity - becomes action hash in Holochain
  id: string;
  version: string;

  // Descriptive metadata
  title: string;
  description: string;
  purpose: string;

  // Authorship - agent public keys in Holochain
  createdBy: string;
  contributors: string[];
  forkedFrom?: string;
  createdAt: string;  // ISO 8601
  updatedAt: string;

  // The journey structure
  steps: PathStep[];

  // Classification
  tags: string[];
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  estimatedDuration: string;

  // Access control
  visibility: 'public' | 'organization' | 'private';

  // Prerequisites and outcomes
  prerequisitePaths?: string[];
  attestationsGranted?: string[];
}

/**
 * PathStep - A single step in a learning path.
 * Links to Territory (ContentNode) and adds journey context.
 */
export interface PathStep {
  order: number;
  resourceId: string;  // Links to ContentNode.id

  // Path-specific context (not in the content node itself)
  stepTitle: string;
  stepNarrative: string;
  learningObjectives: string[];
  reflectionPrompts?: string[];
  practiceExercises?: string[];

  // Metadata
  estimatedTime?: string;
  optional: boolean;

  // Alternatives and gating
  alternativeResourceIds?: string[];
  completionCriteria: string[];
  attestationRequired?: string;
  attestationGranted?: string;

  // Phase 6 Extension (Post-MVP) - Concepts addressed by this step
  conceptsAddressed?: string[];
}

/**
 * PathStepView - Composite returned by PathService.getPathStep()
 * Combines step context with resolved content.
 */
export interface PathStepView {
  step: PathStep;
  content: ContentNode;

  // Navigation context
  hasPrevious: boolean;
  hasNext: boolean;
  previousStepIndex?: number;
  nextStepIndex?: number;

  // Progress for authenticated user
  isCompleted?: boolean;
  affinity?: number;
  notes?: string;
}

/**
 * PathIndex - Catalog entry for path discovery (lightweight)
 */
export interface PathIndexEntry {
  id: string;
  title: string;
  description: string;
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  estimatedDuration: string;
  stepCount: number;
  tags: string[];
}

/**
 * PathIndex - Response from path catalog endpoint
 */
export interface PathIndex {
  lastUpdated: string;
  totalCount: number;
  paths: PathIndexEntry[];
}
