/**
 * Path types — derived from ContentNode with contentType 'path'.
 *
 * Paths are no longer a separate entity. They are ContentNodes with
 * contentFormat 'epr-composite' whose body is a recursive sections tree.
 * This file defines the parsed view types and backward-compatible aliases.
 *
 * Migration (2026-03-25): LearningPath → PathView over ContentNode.
 */

import { ContentNode, ContentMetadata } from './content-node.model';

// =========================================================================
// Core PathView Types (new canonical model)
// =========================================================================

/**
 * Parsed view of a ContentNode with contentType 'path'.
 *
 * Created by parsePathView(node). Contains the underlying ContentNode
 * plus parsed fields from its body and metadata.
 */
export interface PathView {
  /** The underlying ContentNode */
  node: ContentNode;

  /** Path type hint for UI rendering (from metadata) */
  pathType: string;

  /** Difficulty level (from metadata) */
  difficulty?: string;

  /** Human-readable estimated duration (from metadata) */
  estimatedDuration?: string;

  /** Thumbnail image URL (from metadata) */
  thumbnailUrl?: string;

  /** Alt text for thumbnail (from metadata) */
  thumbnailAlt?: string;

  /** Recursive sections tree parsed from body */
  sections: PathSection[];

  // =========================================================================
  // Convenience accessors — mirror old LearningPath fields for compat
  // =========================================================================

  /** Same as node.id */
  id: string;
  /** Same as node.title */
  title: string;
  /** Same as node.description */
  description: string;
  /** Version from metadata (default '1.0.0') */
  version: string;
  /** Tags from node */
  tags: string[];
  /** Visibility mapped from node.reach */
  visibility: PathVisibility;
  /** Created by (node.authorId) */
  createdBy: string;
  /** Contributors list */
  contributors: string[];
  /** Created at timestamp */
  createdAt?: string;
  /** Updated at timestamp */
  updatedAt?: string;

  /**
   * Flat steps array — synthesized from sections tree for backward compat.
   * Contains one PathStep per leaf item across all sections.
   */
  steps: PathStep[];

  /**
   * Chapters — synthesized from top-level sections for backward compat.
   * Each top-level section becomes a PathChapter.
   */
  chapters?: PathChapter[];

  /** Purpose from metadata or description */
  purpose: string;

  /** Pre-requisite path IDs */
  prerequisitePaths?: string[];
  /** Attestations granted on completion */
  attestationsGranted?: string[];

  // Social metadata (from node)
  thumbnailUrl2?: string; // unused, for structural compat

  /** Step count (computed) */
  stepCount?: number;
  /** Chapter count (computed) */
  chapterCount?: number;
}

/**
 * PathSection — a node in the recursive sections tree.
 *
 * Sections can nest (sections within sections) to support
 * scope-and-sequence hierarchy: course → unit → lesson → items.
 */
export interface PathSection {
  /** Section identifier */
  id: string;
  /** Section title */
  title: string;
  /** What this section covers */
  description?: string;
  /** Hierarchy level: course, unit, lesson, etc. */
  level?: string;
  /** Order within parent */
  order: number;
  /** Nested sub-sections */
  sections?: PathSection[];
  /** Leaf-level content items (EPR references) */
  items?: PathItem[];

  // Legacy compat fields (synthesized for old code)
  /** Concept IDs — flattened from items[].ref */
  conceptIds: string[];
  /** Estimated duration */
  estimatedDuration?: string;
  /** Whether optional */
  optional?: boolean;
  /** Estimated minutes */
  estimatedMinutes?: number;
  /** Assessments in this section */
  assessments?: SectionAssessment[];
  /** Modules within this section (synthesized for 4-level compat) */
  modules?: PathModule[];
  /** Steps within this section (synthesized for 2-level compat) */
  steps?: PathStep[];
  /** Attestation granted on completion */
  attestationGranted?: string;
}

/**
 * PathItem — a single content reference within a section.
 *
 * Items use EPR references (epr:content-id) to point at ContentNodes.
 * Pedagogical context (narrative, objectives) lives here — it's the
 * teacher's editorial arrangement, not graph metadata.
 */
export interface PathItem {
  /** EPR reference (e.g., "epr:concept-concentric-circles" or just "concept-id") */
  ref: string;
  /** Role in the section: step, checkpoint, reflection */
  role: string;
  /** Display title override */
  title?: string;
  /** Narrative context for this item within the path */
  narrative?: string;
  /** Learning objectives */
  learningObjectives?: string[];
  /** Completion criteria */
  completionCriteria?: { type: string; threshold?: number };
}

// =========================================================================
// Legacy Types — backward-compatible aliases and synthesized types
// =========================================================================

/**
 * Difficulty level for learning paths and content.
 */
export type DifficultyLevel = 'beginner' | 'intermediate' | 'advanced';

/**
 * Path type hint for UI rendering.
 */
export type PathType = 'journey' | 'quest' | 'expedition' | 'practice';

/**
 * @deprecated Use PathView instead. This alias exists for backward compatibility.
 */
export type LearningPath = PathView;

/**
 * PathChapter — synthesized from top-level sections for backward compat.
 *
 * @deprecated Use PathSection with level='course' instead.
 */
export interface PathChapter {
  id: string;
  title: string;
  description?: string;
  order: number;
  modules?: PathModule[];
  steps?: PathStep[];
  estimatedDuration?: string;
  attestationGranted?: string;
  optional?: boolean;
}

/**
 * PathModule — synthesized from mid-level sections for backward compat.
 *
 * @deprecated Use PathSection with level='unit' instead.
 */
export interface PathModule {
  id: string;
  title: string;
  description?: string;
  order: number;
  sections: PathSection[];
  estimatedDuration?: string;
  learningObjectives?: string[];
  optional?: boolean;
}

/**
 * SectionAssessment - assessment within a section.
 */
export interface SectionAssessment {
  id: string;
  title: string;
  type: 'core' | 'applied' | 'synthesis';
  description?: string;
  assessmentId?: string;
}

/**
 * PathStep — synthesized from leaf items for backward compat.
 *
 * Future: migrate consumers to use PathItem instead.
 */
export interface PathStep {
  order: number;
  stepType?: 'content' | 'path' | 'external' | 'checkpoint';
  resourceId: string;
  chapterId?: string;
  moduleId?: string;
  sectionId?: string;
  pathId?: string;
  externalUrl?: string;
  stepTitle: string;
  stepNarrative: string;
  learningObjectives: string[];
  reflectionPrompts?: string[];
  practiceExercises?: string[];
  estimatedTime?: string;
  optional: boolean;
  alternativeResourceIds?: string[];
  completionCriteria: string[];
  attestationRequired?: string;
  attestationGranted?: string;
  conceptsAddressed?: string[];
  /** Shared concepts for cross-path mastery (used by getConceptProgressForPath) */
  sharedConcepts?: string[];
}

/**
 * PathStepView - Composite returned by PathService.getPathStep()
 */
export interface PathStepView {
  step: PathStep;
  content: ContentNode;
  nestedPath?: PathView;
  chapter?: {
    id: string;
    title: string;
    order: number;
    stepIndexWithinChapter: number;
    totalStepsInChapter: number;
  };
  hasPrevious: boolean;
  hasNext: boolean;
  previousStepIndex?: number;
  nextStepIndex?: number;
  isCompleted?: boolean;
  affinity?: number;
  notes?: string;
}

/**
 * PathOverviewView - Rich view for path landing/overview pages.
 */
export interface PathOverviewView {
  path: PathView;
  chapterSummaries?: ChapterSummary[];
  totalStepCount: number;
  progress?: {
    completedSteps: number;
    totalRequiredSteps: number;
    completionPercentage: number;
    currentStepIndex: number;
    currentChapterIndex?: number;
    attestationsEarned: string[];
  };
  nestedPathSummaries?: PathIndexEntry[];
  prerequisites?: {
    pathId: string;
    title: string;
    isCompleted: boolean;
  }[];
}

/**
 * ChapterSummary - Lightweight chapter info for overview display.
 */
export interface ChapterSummary {
  id: string;
  title: string;
  description?: string;
  order: number;
  stepCount: number;
  estimatedDuration?: string;
  attestationGranted?: string;
  completedSteps?: number;
  isComplete?: boolean;
}

/**
 * PathIndexEntry - Catalog entry for path discovery (lightweight)
 */
export interface PathIndexEntry {
  id: string;
  title: string;
  description: string;
  difficulty: DifficultyLevel;
  estimatedDuration: string;
  stepCount: number;
  tags: string[];
  chapterCount?: number;
  pathType?: PathType;
  attestationsGranted?: string[];
  category?: string;
  thumbnailUrl?: string;
  thumbnailAlt?: string;
}

/**
 * PathIndex - Response from path catalog endpoint
 */
export interface PathIndex {
  lastUpdated: string;
  totalCount: number;
  paths: PathIndexEntry[];
}

// =========================================================================
// Path Visibility (Graduated Intimacy Model)
// =========================================================================

export type PathVisibility =
  | 'public'
  | 'connections'
  | 'trusted'
  | 'intimate'
  | 'organization'
  | 'private';

export const VISIBILITY_MIGRATION: Record<string, PathVisibility> = {
  organization: 'connections',
  private: 'intimate',
};

export function requiresConsent(visibility: PathVisibility): boolean {
  return visibility !== 'public';
}

export function requiresAttestation(visibility: PathVisibility): boolean {
  return visibility === 'intimate';
}

// =========================================================================
// Legacy types kept for backward compat (re-exported from old model)
// =========================================================================

/** @deprecated Use PathView */
export interface PathContentMetadata {
  pathId: string;
  difficulty: DifficultyLevel;
  estimatedDuration: string;
  stepCount: number;
  chapterCount?: number;
  contentNodeIds: string[];
  nestedPathIds?: string[];
  creatorInfo?: {
    presenceId: string;
    brandName?: string;
    brandLogoUrl?: string;
  };
  forkedFromPathId?: string;
  forkGeneration?: number;
  canonicalStatus?: 'draft' | 'community' | 'canonical';
  pathType?: PathType;
  attestationsGranted?: string[];
}

/** @deprecated Use ContentNode relationship queries */
export interface PathReference {
  nodeId: string;
  pathId: string;
  title: string;
  relationship: 'contains' | 'references' | 'requires';
}

// =========================================================================
// Parser — ContentNode → PathView
// =========================================================================

/**
 * Resolve an EPR reference to a plain content ID.
 * Strips "epr:" prefix if present.
 */
function resolveRef(ref: string): string {
  return ref.startsWith('epr:') ? ref.slice(4) : ref;
}

/**
 * Map a reach level to a PathVisibility value for backward compat.
 */
function reachToVisibility(reach: string | undefined): PathVisibility {
  switch (reach) {
    case 'commons':
      return 'public';
    case 'community':
    case 'federated':
      return 'connections';
    case 'local':
    case 'neighborhood':
    case 'municipal':
    case 'bioregional':
    case 'regional':
      return 'trusted';
    case 'invited':
      return 'intimate';
    case 'private':
      return 'private';
    default:
      return 'public';
  }
}

/**
 * Recursively collect all leaf items from a sections tree, producing flat PathSteps.
 */
function collectSteps(sections: PathSection[], stepOffset: number): PathStep[] {
  const steps: PathStep[] = [];
  for (const section of sections) {
    // Collect items at this level
    if (section.items) {
      for (const item of section.items) {
        const contentId = resolveRef(item.ref);
        steps.push({
          order: stepOffset + steps.length,
          stepType: item.role === 'checkpoint' ? 'checkpoint' : 'content',
          resourceId: contentId,
          sectionId: section.id,
          stepTitle: item.title ?? contentId,
          stepNarrative: item.narrative ?? '',
          learningObjectives: item.learningObjectives ?? [],
          optional: false,
          completionCriteria: item.completionCriteria
            ? [JSON.stringify(item.completionCriteria)]
            : [],
        });
      }
    }
    // Recurse into sub-sections
    if (section.sections) {
      const childSteps = collectSteps(section.sections, stepOffset + steps.length);
      steps.push(...childSteps);
    }
  }
  return steps;
}

/**
 * Enrich a raw section from JSON body with backward-compat fields.
 */
function enrichSection(raw: RawSection, index: number): PathSection {
  const items = (raw.items ?? []).map(
    (item: RawItem): PathItem => ({
      ref: item.ref,
      role: item.role ?? 'step',
      title: item.title,
      narrative: item.narrative,
      learningObjectives: item.learningObjectives,
      completionCriteria: item.completionCriteria,
    })
  );

  const childSections = (raw.sections ?? []).map((s: RawSection, i: number) =>
    enrichSection(s, i)
  );

  // Flatten concept IDs from items + child sections
  const conceptIds = [
    ...items.map((item: PathItem) => resolveRef(item.ref)),
    ...childSections.flatMap((s: PathSection) => s.conceptIds),
  ];

  // Build legacy steps from items
  const steps = items.map(
    (item: PathItem, i: number): PathStep => ({
      order: i,
      stepType: item.role === 'checkpoint' ? 'checkpoint' : 'content',
      resourceId: resolveRef(item.ref),
      stepTitle: item.title ?? resolveRef(item.ref),
      stepNarrative: item.narrative ?? '',
      learningObjectives: item.learningObjectives ?? [],
      optional: false,
      completionCriteria: item.completionCriteria
        ? [JSON.stringify(item.completionCriteria)]
        : [],
    })
  );

  return {
    id: raw.id ?? `section-${index}`,
    title: raw.title ?? '',
    description: raw.description,
    level: raw.level ?? 'lesson',
    order: index,
    sections: childSections.length > 0 ? childSections : undefined,
    items: items.length > 0 ? items : undefined,
    conceptIds,
    estimatedDuration: raw.estimatedDuration,
    optional: raw.optional,
    steps: steps.length > 0 ? steps : undefined,
  };
}

/**
 * Synthesize PathChapter[] from top-level sections.
 */
function sectionsToChapters(sections: PathSection[]): PathChapter[] {
  return sections.map((section, i) => {
    // Convert child sections to PathModule[]
    // If a child section has no further sub-sections but has its own conceptIds/items,
    // wrap it as its own section in the module so conceptIds are accessible.
    const modules: PathModule[] = (section.sections ?? []).map(
      (childSection, j): PathModule => ({
        id: childSection.id,
        title: childSection.title,
        description: childSection.description,
        order: j,
        sections: (childSection.sections?.length ?? 0) > 0
          ? childSection.sections!
          : [childSection],
        estimatedDuration: childSection.estimatedDuration,
        learningObjectives: [],
      })
    );

    return {
      id: section.id,
      title: section.title,
      description: section.description,
      order: i,
      modules: modules.length > 0 ? modules : undefined,
      steps: section.steps,
      estimatedDuration: section.estimatedDuration,
      attestationGranted: section.attestationGranted,
      optional: section.optional,
    };
  });
}

/** Raw section shape from JSON body */
interface RawSection {
  id?: string;
  title?: string;
  description?: string;
  level?: string;
  sections?: RawSection[];
  items?: RawItem[];
  estimatedDuration?: string;
  optional?: boolean;
}

/** Raw item shape from JSON body */
interface RawItem {
  ref: string;
  role?: string;
  title?: string;
  narrative?: string;
  learningObjectives?: string[];
  completionCriteria?: { type: string; threshold?: number };
}

/**
 * Parse a ContentNode into a PathView.
 *
 * Extracts sections from the composite body, path-specific metadata,
 * and synthesizes backward-compatible steps/chapters arrays.
 */
export function parsePathView(node: ContentNode): PathView {
  // Parse body — may be string JSON or already-parsed object
  let body: Record<string, unknown>;
  if (typeof node.content === 'string') {
    try {
      body = JSON.parse(node.content) as Record<string, unknown>;
    } catch {
      body = {};
    }
  } else {
    body = (node.content as Record<string, unknown>) ?? {};
  }

  const meta: ContentMetadata = node.metadata ?? {};

  // Parse sections from body
  const rawSections = (body['sections'] as RawSection[]) ?? [];
  const sections = rawSections.map((s, i) => enrichSection(s, i));

  // Synthesize flat steps from sections tree
  const steps = collectSteps(sections, 0);

  // Synthesize chapters from top-level sections
  const chapters = sections.length > 0 ? sectionsToChapters(sections) : undefined;

  return {
    node,
    pathType: (meta as Record<string, unknown>)['pathType'] as string ?? body['pathType'] as string ?? 'journey',
    difficulty: (meta as Record<string, unknown>)['difficulty'] as string ?? undefined,
    estimatedDuration: (meta as Record<string, unknown>)['estimatedDuration'] as string ?? undefined,
    // Prefer resolved thumbnailUrl from ContentService (set at runtime on the node)
    // over raw metadata value (which is an unresolved /blob/sha256-... path)
    thumbnailUrl: (node as unknown as Record<string, unknown>)['thumbnailUrl'] as string
      ?? (meta as Record<string, unknown>)['thumbnailUrl'] as string
      ?? undefined,
    thumbnailAlt: (meta as Record<string, unknown>)['thumbnailAlt'] as string ?? undefined,
    sections,

    // Convenience accessors for backward compat
    id: node.id,
    title: node.title,
    description: node.description,
    version: ((meta as Record<string, unknown>)['version'] as string) ?? '1.0.0',
    tags: node.tags ?? [],
    visibility: reachToVisibility(node.reach),
    createdBy: node.authorId ?? '',
    contributors: ((meta as Record<string, unknown>)['contributors'] as string[]) ?? [],
    createdAt: node.createdAt,
    updatedAt: node.updatedAt,
    steps,
    chapters,
    purpose: ((meta as Record<string, unknown>)['purpose'] as string) ?? node.description,
    prerequisitePaths: (meta as Record<string, unknown>)['prerequisitePaths'] as string[] | undefined,
    attestationsGranted: (meta as Record<string, unknown>)['attestationsGranted'] as string[] | undefined,
    stepCount: steps.length,
    chapterCount: chapters?.length,
  };
}
