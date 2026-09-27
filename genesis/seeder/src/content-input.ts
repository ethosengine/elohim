/**
 * Content-input builders — the ONE place a repository content/path JSON becomes
 * the `CreateContentInput` a content row is written from.
 *
 * Two writers put the same rows onto a peer: the pipeline seeder
 * (`seed-sqlite.ts`) and a steward publishing from their own peer. Both must
 * build the SAME input from the SAME JSON so that `canonicalSeedHash(input)`
 * (seed-hash.ts) agrees between them — that agreement is what lets a later seed
 * run recognise a steward's publish as already current. Neither writer may
 * build its own input: import these.
 *
 * Pure: no filesystem, no network, no environment. Reach is resolved through
 * `earnedReach` (the inverted-burden resolver) from the authored value plus an
 * optional caller-supplied advisory; blob/thumbnail hashes are whatever the
 * caller actually linked (uploaded or already present on the peer).
 */
import { CONTENT_FORMATS } from './validation-constants.js';
import { ALL_STEP_TYPES } from './generated/schema-enums.js';
import type { ContentFormat, ContentType, Reach } from './generated/schema-enums.js';
import type { CreateContentInput } from './generated/create-content-input.js';
import type { ConceptMetadata, PathMetadata } from './generated/metadata-types.js';
import type { Section, Item } from './generated/body-types.js';
import { createHash } from 'crypto';

import { earnedReach } from './reach-resolver.js';
import { applyPathThumbnail } from './path-thumbnail.js';
import { canonicalRelationshipType, type RelationshipRemapLedger } from './relationship-vocabulary.js';

// ============================================================================
// JSON file types from data/lamad/
// ============================================================================

export interface ConceptJson {
  id: string;
  title: string;
  content?: string | object;
  contentFormat?: string;
  contentType?: string;
  reach?: string;           // Authored reach grade (floor — overrides may only raise)
  description?: string;
  summary?: string;
  sourcePath?: string;
  relatedNodeIds?: string[];
  /** Typed, authored edges: `{target, type, role?}` (see relationshipsOf for accepted spellings). */
  relationships?: Array<Record<string, unknown>>;
  tags?: string[];
  estimatedMinutes?: number;
  thumbnailUrl?: string;
  metadata?: Record<string, unknown>;
  // Blob references for html5-app and large content
  blobHash?: string;       // Pre-computed hash (camelCase from JSON) — legacy sha256-<hex> /blob key
  blob_hash?: string;      // Alternative snake_case format
  blobCid?: string;        // Canonical CIDv1 (bafkrei… raw codec) address for the blob bytes
  entryPoint?: string;    // Entry point for html5-app (e.g., "index.html")
  stewardedBy?: Array<{ humanId: string; affinity: number; role: string }>;
}

export interface PathJson {
  id: string;
  title: string;
  description?: string;
  purpose?: string;
  pathType?: string;
  difficulty?: string;
  estimatedDuration?: string;
  estimatedMinutes?: number;
  thumbnailUrl?: string;
  thumbnailAlt?: string;
  version?: string;
  reach?: string;           // Authored reach grade (preferred over visibility)
  visibility?: string;      // Legacy fallback when reach is absent
  tags?: string[];
  chapters?: ChapterJson[];
  conceptIds?: string[];
}

export interface ChapterJson {
  id: string;
  title: string;
  description?: string;
  order?: number;
  estimatedDuration?: string;
  modules?: ModuleJson[];
  conceptIds?: string[];
  steps?: StepJson[];  // Direct steps in chapter (know-thyself format)
}

export interface StepJson {
  order?: number;
  stepType?: string;
  resourceId?: string;
  title?: string;
  stepTitle?: string;
  stepNarrative?: string;
  learningObjectives?: string[];
  optional?: boolean;
  completionCriteria?: string[];
  estimatedTime?: string;
}

export interface ModuleJson {
  id: string;
  title: string;
  description?: string;
  order?: number;
  sections?: SectionJson[];
  steps?: StepJson[];  // Titled steps directly in a module (movement → module → step)
}

export interface SectionJson {
  id: string;
  title: string;
  description?: string;
  order?: number;
  estimatedMinutes?: number;
  conceptIds?: string[];
}

// ============================================================================
// Value Normalizers (map legacy/variant values to valid backend enums)
// ============================================================================

/** Map legacy/variant content formats to canonical values accepted by elohim-storage */
export function normalizeContentFormat(format: string | undefined): ContentFormat {
  if (!format) return 'markdown';

  const normalized = format.toLowerCase();

  // Map variants to canonical values
  const mappings: Record<string, ContentFormat> = {
    'perseus-quiz-json': 'perseus',
    'perseus-quiz': 'perseus',
    'quiz-json': 'perseus',
    'sophia-moment-json': 'sophia',
    'sophia-quiz-json': 'sophia',
    'sophia-mastery': 'sophia',
    'sophia-discovery': 'sophia',
    'md': 'markdown',
    'htm': 'html',
    'txt': 'text',
  };

  if (mappings[normalized]) return mappings[normalized];
  // Validate against the auto-generated constants from healing.rs
  if ((CONTENT_FORMATS as readonly string[]).includes(normalized)) return normalized as ContentFormat;

  // Default to markdown for unknown formats
  console.warn(`   ⚠️ Unknown contentFormat '${format}', defaulting to 'markdown'`);
  return 'markdown';
}

/** Map a step type to an Item role for the epr-composite body schema.
 * Step types (content, assess, video, etc.) are a different concept from
 * Item roles (step, checkpoint, optional, reflection). */
function stepTypeToItemRole(stepType: string | undefined): Item['role'] {
  switch (stepType) {
    case 'assess':
    case 'quiz':
      return 'checkpoint';
    case 'reflection':
    case 'discuss':
      return 'reflection';
    default:
      return 'step';
  }
}

/** Map legacy/variant step types to schema-canonical values.
 * ALL_STEP_TYPES imported from generated schema-enums (single source of truth). */
function normalizeStepType(stepType: string | undefined): string {
  if (!stepType) return 'content';

  const normalized = stepType.toLowerCase();

  // Map legacy values to schema-canonical values
  const mappings: Record<string, string> = {
    'learn': 'content',
    'reading': 'read',
    'quiz': 'assess',
    'assessment': 'assess',
    'discussion': 'reflection',
    'project': 'practice',
    'resource': 'external',
    'test': 'assess',
    'watch': 'video',
  };

  const mapped = mappings[normalized] || normalized;
  if ((ALL_STEP_TYPES as readonly string[]).includes(mapped)) return mapped;

  console.warn(`   ⚠️ Unknown stepType '${stepType}', defaulting to 'content'`);
  return 'content';
}

/**
 * Format a concept ID into a human-readable title.
 * Converts kebab-case to Title Case.
 * Examples:
 *   "manifesto" → "Manifesto"
 *   "quiz-manifesto-foundations" → "Quiz Manifesto Foundations"
 */
function formatConceptTitle(conceptId: string): string {
  return conceptId
    .split('-')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

// ============================================================================
// Content
// ============================================================================

/**
 * Canonical content-body serialization for a content seed JSON: a string
 * `content` is used verbatim; an object `content` is JSON.stringify'd.
 *
 * The body bytes are also the FIRST input to `deriveContentAnchor`
 * (seed-sqlite.ts) — anything computing a content's anchor CID out of band
 * must use this exact rule or it addresses a CID no seeded row carries.
 */
export function contentBodyFor(json: Pick<ConceptJson, 'content'>): string | undefined {
  if (!json.content) return undefined;
  return typeof json.content === 'string' ? json.content : JSON.stringify(json.content);
}

export interface BuildContentOptions {
  /** Account-package archetype advisory; may only RAISE the authored reach. */
  advisoryReach?: string;
  /**
   * The blob hash actually linked for this row (uploaded or already present on
   * the peer). Overrides the JSON's own `blobHash` when set — the seeder links
   * the NORMALIZED hash of the blob it verified, not the raw JSON string.
   */
  blobHash?: string;
}

/**
 * The reach a content row is written at: the inverted-burden `earnedReach` of
 * the authored grade plus the optional account-package advisory. The content
 * row and every edge it authors resolve reach through this one function.
 */
export function resolveContentReach(json: Pick<ConceptJson, 'reach'>, advisoryReach?: string): Reach {
  return earnedReach({ authored: json.reach, advisory: advisoryReach });
}

/** Build the CreateContentInput for a repository content JSON. */
export function buildContentInput(json: ConceptJson, opts: BuildContentOptions = {}): CreateContentInput {
  const contentBody = contentBodyFor(json);
  const contentSizeBytes = contentBody !== undefined ? Buffer.byteLength(contentBody, 'utf-8') : undefined;

  // Build typed metadata (ConceptMetadata from generated schema types)
  const metadata: ConceptMetadata = {};
  if (json.metadata) Object.assign(metadata, json.metadata);
  if (json.estimatedMinutes) metadata.estimatedMinutes = json.estimatedMinutes;
  if (json.thumbnailUrl) metadata.thumbnailUrl = json.thumbnailUrl;
  if (json.relatedNodeIds?.length) metadata.relatedNodeIds = json.relatedNodeIds;
  if (json.summary) metadata.summary = json.summary;
  // The atom's authored edges ride its own signed entry, so they travel, version and
  // disappear with the head that states them. One canonical list (manifest types, roles,
  // deterministic order) replaces whatever legacy shape the file used.
  //
  // Only TYPED edges go here. Bare `relatedNodeIds` already ride the signed entry as
  // `metadata.relatedNodeIds`; folding them in too would change the seed hash of ~3,300
  // corpus atoms and make the next seed run re-sign nearly the whole corpus for no change
  // in meaning. An explicitly authored empty list is kept, because it is how an author
  // says "this atom no longer links anywhere" and storage then removes its edges.
  const typed = buildRelationshipInputs(
    { ...json, relatedNodeIds: undefined },
    { advisoryReach: opts.advisoryReach },
  );
  if (typed.length > 0) {
    metadata.relationships = typed.map(e => ({
      type: e.relationshipType,
      targetId: e.targetId,
      ...(e.metadata ? { role: e.metadata.role } : {}),
    }));
  } else if (Array.isArray(json.relationships) && json.relationships.length === 0) {
    metadata.relationships = [];
  } else {
    delete metadata.relationships;
  }

  return {
    id: json.id,
    title: json.title,
    schemaVersion: 1,
    description: json.description || undefined,
    contentType: (json.contentType ?? 'concept') as ContentType,
    contentFormat: normalizeContentFormat(json.contentFormat),
    contentBody: contentBody ?? undefined,
    blobHash: opts.blobHash ?? json.blobHash ?? json.blob_hash ?? undefined,
    blobCid: json.blobCid ?? undefined,
    contentSizeBytes: contentSizeBytes,
    metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
    reach: resolveContentReach(json, opts.advisoryReach),
    createdBy: undefined,
    tags: json.tags || [],
  };
}

// ============================================================================
// Paths
// ============================================================================

/** One step → one Item. Shared by chapter-level and module-level steps. */
function stepToItem(step: StepJson): Item {
  const item: Item = {
    ref: step.resourceId ?? '',
    role: stepTypeToItemRole(normalizeStepType(step.stepType)),
    title: step.stepTitle || step.title || formatConceptTitle(step.resourceId ?? ''),
  };
  if (step.stepNarrative) item.narrative = step.stepNarrative;
  if (step.learningObjectives) item.learningObjectives = step.learningObjectives;
  if (step.completionCriteria) {
    // Legacy step data stores completionCriteria as string[]; map first to body schema shape
    item.completionCriteria = Array.isArray(step.completionCriteria)
      ? { type: step.completionCriteria[0] as 'view' | 'score' | 'time' | 'interaction' }
      : step.completionCriteria as unknown as Item['completionCriteria'];
  }
  return item;
}

/**
 * Convert path JSON chapters into the sections tree format.
 * Handles these input shapes:
 * 1.  chapters -> modules -> sections -> conceptIds (elohim-protocol)
 * 1b. chapters -> modules -> steps (movements → titled modules → titled steps);
 *     each module becomes one lesson Section carrying its title and its steps.
 *     A module may carry both; its steps come first, then its sections.
 * 2.  chapters -> steps (governance paths, bdd-smoke-tests)
 * 3.  flat conceptIds (no chapters)
 */
export function chaptersToSections(json: PathJson): Section[] {
  // Handle flat conceptIds (no chapters)
  if ((!json.chapters || json.chapters.length === 0) && json.conceptIds?.length) {
    return [{
      id: `${json.id}-default`,
      title: json.title,
      description: json.description,
      level: 'unit',
      items: json.conceptIds.map(id => ({
        ref: id,
        role: 'step',
        title: formatConceptTitle(id),
      })),
    }];
  }

  if (!json.chapters) return [];

  return json.chapters.map(chapter => {
    const section: Section = {
      id: chapter.id,
      title: chapter.title,
      description: chapter.description,
      level: 'unit',
      estimatedDuration: chapter.estimatedDuration,
    };

    // Shape 1 / 1b: chapters -> modules -> (steps | sections -> conceptIds)
    if (chapter.modules?.length) {
      section.sections = [];
      for (const mod of chapter.modules) {
        // Shape 1b: the module is itself a titled lesson of steps.
        if (mod.steps?.length) {
          section.sections.push({
            id: mod.id,
            title: mod.title,
            description: mod.description,
            level: 'lesson',
            items: mod.steps.map(stepToItem),
          });
        }
        // Shape 1: the module's sections are the lessons.
        if (mod.sections) {
          for (const sec of mod.sections) {
            section.sections.push({
              id: sec.id,
              title: sec.title ?? mod.title,
              description: sec.description,
              level: 'lesson',
              items: (sec.conceptIds ?? []).map(id => ({
                ref: id,
                role: 'step',
                title: formatConceptTitle(id),
              })),
            });
          }
        }
      }
      return section;
    }

    // Shape 2: chapters -> steps (flat)
    if (chapter.steps?.length) {
      section.items = chapter.steps.map(stepToItem);
      return section;
    }

    // Shape 2b: chapters -> conceptIds (flat)
    if (chapter.conceptIds?.length) {
      section.items = chapter.conceptIds.map(id => ({
        ref: id,
        role: 'step',
        title: formatConceptTitle(id),
      }));
      return section;
    }

    return section;
  });
}

export interface BuildPathOptions {
  /** Hash of the thumbnail blob actually linked for this path (uploaded or present). */
  thumbnailHash?: string;
}

/**
 * Build the CreateContentInput for a repository path JSON — a content row of
 * contentType 'path' whose body is the epr-composite sections tree
 * (the format parsePathView() in learning-path.model.ts reads).
 */
export function buildPathInput(json: PathJson, opts: BuildPathOptions = {}): CreateContentInput {
  const sections = chaptersToSections(json);

  // Build typed metadata (PathMetadata from generated schema types)
  const metadata: PathMetadata = {};
  if (json.pathType) metadata.pathType = json.pathType;
  if (json.difficulty) metadata.difficulty = json.difficulty;
  if (json.estimatedDuration) metadata.estimatedDuration = json.estimatedDuration;
  if (json.estimatedMinutes) metadata.estimatedDuration = `${json.estimatedMinutes} minutes`;
  if (json.version) metadata.version = json.version;
  if (json.purpose) metadata.purpose = json.purpose;
  if (json.thumbnailUrl) metadata.thumbnailUrl = json.thumbnailUrl;
  if (json.thumbnailAlt) metadata.thumbnailAlt = json.thumbnailAlt;

  const contentBody = JSON.stringify({ sections });

  const input: CreateContentInput = {
    id: json.id,
    title: json.title,
    schemaVersion: 1,
    description: json.description || undefined,
    contentType: 'path',
    contentFormat: 'epr-composite',
    contentBody,
    contentSizeBytes: Buffer.byteLength(contentBody, 'utf-8'),
    metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
    reach: earnedReach({ authored: json.reach ?? json.visibility, advisory: undefined }),
    tags: json.tags || [],
  };
  return applyPathThumbnail(input, opts.thumbnailHash);
}

/** Count total items across a sections tree (for logging). */
export function countItems(sections: Section[]): number {
  let count = 0;
  for (const s of sections) {
    count += s.items?.length ?? 0;
    if (s.sections) count += countItems(s.sections);
  }
  return count;
}

// ============================================================================
// Relationships (typed content-graph edges)
// ============================================================================

/**
 * One storage-ready content-graph edge: the camelCase body item of
 * `POST /db/relationships/bulk` (elohim-views `CreateRelationshipInputView`).
 *
 * `metadata` is stored as the edge's `metadata_json`; it carries the authored
 * `role` (anchor, supporting, story, practice, callback, …) that a learner's
 * elohim will walk when composing a personal path.
 *
 * `reach` is the SOURCE atom's resolved reach — an edge is never more open than
 * the atom that authored it. Storage's input view carries it: set on insert and
 * upsert, and an absent reach never widens an existing edge.
 */
export interface RelationshipInput {
  /** Deterministic: `relationshipId(sourceId, relationshipType, targetId)`. */
  id: string;
  schemaVersion: 1;
  sourceId: string;
  targetId: string;
  /** A manifest relationship id (canonicalRelationshipType), always. */
  relationshipType: string;
  confidence: number;
  inferenceSource: string;
  reach: Reach;
  metadata?: { role: string };
}

export interface BuildRelationshipOptions {
  /** Account-package advisory, resolved exactly as buildContentInput resolves it. */
  advisoryReach?: string;
  /**
   * The reach the source ROW was actually written at, when the caller resolves
   * row reach differently (the doorway seeder's `seedReach`). Wins over
   * `advisoryReach`; either way the edge carries its source row's reach.
   */
  reach?: Reach;
  /** Collects authored-type → manifest-type remaps for the one summary line. */
  remaps?: RelationshipRemapLedger;
}

/**
 * Deterministic edge id from the edge's STORAGE identity. elohim-storage keys a
 * content edge on (h_app_id, source_id, target_id, relationship_type) — its
 * unique index — so the id is derived from exactly that triple (role is
 * metadata on the edge, not part of its identity). Every peer, and every run,
 * mints the same id for the same edge.
 */
export function relationshipId(sourceId: string, relationshipType: string, targetId: string): string {
  const digest = createHash('sha256').update(`${sourceId}|${relationshipType}|${targetId}`, 'utf-8').digest('hex');
  return `rel-${digest.slice(0, 32)}`;
}

function nonEmptyString(v: unknown): string | undefined {
  return typeof v === 'string' && v.trim() !== '' ? v.trim() : undefined;
}

function edgeConfidence(v: unknown): number {
  if (typeof v !== 'number' || !Number.isFinite(v)) return 1.0;
  return Math.min(1, Math.max(0, v));
}

/**
 * Build every storage-ready edge a content JSON authors, from (in precedence
 * order) its top-level `relationships[]`, the legacy `metadata.relationships[]`,
 * and `relatedNodeIds` (bare `RELATES_TO`, role `related`).
 *
 * Accepted item spellings: target | targetId | target_id; type |
 * relationshipType | relationship_type; role; confidence; inferenceSource |
 * inference_source. Every type is canonicalized onto the lamad manifest
 * vocabulary (remaps counted, never dropped). Items with no target and
 * self-edges are skipped.
 *
 * Dedupe: exact (source, type, target, role) repeats collapse. Because storage
 * holds ONE edge per (source, type, target), a later item with the same triple
 * but a different role also collapses onto the first — the higher-precedence
 * origin wins, so an authored `RELATES_TO {role: callback}` is not overwritten
 * by the bare `relatedNodeIds` entry for the same target.
 *
 * Output order is deterministic: by relationshipType, then targetId.
 */
export function buildRelationshipInputs(
  json: Pick<ConceptJson, 'id' | 'reach' | 'relationships' | 'relatedNodeIds' | 'metadata'>,
  opts: BuildRelationshipOptions = {},
): RelationshipInput[] {
  const sourceId = json.id;
  const reach = opts.reach ?? resolveContentReach(json, opts.advisoryReach);
  const edges = new Map<string, RelationshipInput>();

  const add = (target: string | undefined, rawType: unknown, role: string | undefined, item?: Record<string, unknown>) => {
    if (!target || target === sourceId) return;
    const canonical = canonicalRelationshipType(rawType);
    opts.remaps?.note(canonical);
    const key = `${canonical.type}\u0000${target}`;
    if (edges.has(key)) return;
    const edge: RelationshipInput = {
      id: relationshipId(sourceId, canonical.type, target),
      schemaVersion: 1,
      sourceId,
      targetId: target,
      relationshipType: canonical.type,
      confidence: edgeConfidence(item?.confidence),
      inferenceSource: nonEmptyString(item?.inferenceSource) ?? nonEmptyString(item?.inference_source) ?? 'explicit',
      reach,
    };
    if (role) edge.metadata = { role };
    edges.set(key, edge);
  };

  const typed = (items: unknown) => {
    if (!Array.isArray(items)) return;
    for (const raw of items) {
      if (!raw || typeof raw !== 'object') continue;
      const item = raw as Record<string, unknown>;
      const target = nonEmptyString(item.target) ?? nonEmptyString(item.targetId) ?? nonEmptyString(item.target_id);
      const type = item.type ?? item.relationshipType ?? item.relationship_type;
      add(target, type, nonEmptyString(item.role), item);
    }
  };

  typed(json.relationships);
  typed((json.metadata as Record<string, unknown> | undefined)?.relationships);
  for (const target of json.relatedNodeIds ?? []) {
    add(nonEmptyString(target), 'RELATES_TO', 'related');
  }

  const cmp = (x: string, y: string) => (x < y ? -1 : x > y ? 1 : 0);
  return [...edges.values()].sort((a, b) =>
    cmp(a.relationshipType, b.relationshipType) || cmp(a.targetId, b.targetId),
  );
}
