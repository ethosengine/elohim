/**
 * SQLite Content Seeder
 *
 * Seeds content and paths directly to elohim-storage SQLite database.
 * This is the fast alternative to DHT seeding - <1 minute vs 50+ minutes.
 *
 * Usage:
 *   STORAGE_URL=http://localhost:8090 npx tsx src/seed-sqlite.ts
 *
 * Environment variables:
 *   STORAGE_URL - elohim-storage HTTP endpoint (required)
 *   DATA_DIR - Path to lamad data directory (optional, defaults to ../data/lamad)
 *   LIMIT - Maximum items to seed (optional, for testing)
 *   DRY_RUN - If "true", validate but don't write (optional)
 *   SKIP_BLOB_UPLOAD - Skip uploading blobs (for debugging)
 */

import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';
import { fileURLToPath } from 'url';

import { CONTENT_FORMATS } from './validation-constants.js';
import { ALL_STEP_TYPES } from './generated/schema-enums.js';
import type { ContentFormat, ContentType, Reach } from './generated/schema-enums.js';
import type { CreateContentInput } from './generated/create-content-input.js';

// Directory setup
const __filename = fileURLToPath(import.meta.url);
const SEEDER_DIR = path.dirname(path.dirname(__filename));
const GENESIS_DIR = path.resolve(SEEDER_DIR, '..');
const DATA_DIR = process.env.DATA_DIR || path.join(GENESIS_DIR, 'data', 'lamad');
const STORAGE_URL = process.env.STORAGE_URL;

// Parse arguments
const args = process.argv.slice(2);
const LIMIT = parseInt(process.env.LIMIT || args.find(a => a.startsWith('--limit='))?.split('=')[1] || '0', 10);
const DRY_RUN = process.env.DRY_RUN === 'true' || args.includes('--dry-run');
const CONTENT_ONLY = args.includes('--content-only') || process.env.CONTENT_ONLY === 'true';
const PATHS_ONLY = args.includes('--paths-only') || process.env.PATHS_ONLY === 'true';
const SKIP_BLOB_UPLOAD = process.env.SKIP_BLOB_UPLOAD === 'true' || args.includes('--skip-blob-upload');
const USE_ACCOUNT_PACKAGES = args.includes('--use-account-packages') || process.env.USE_ACCOUNT_PACKAGES === 'true';
const ACCOUNT_PACKAGES_DIR = process.env.ACCOUNT_PACKAGES_DIR || path.join(GENESIS_DIR, 'data', 'account-packages');
const CONDUCTOR_FOR = args.find(a => a.startsWith('--conductor-for='))?.split('=')[1];

// ============================================================================
// Canonical Human Registry (single source of truth: humans.json)
// ============================================================================

const HUMANS_JSON_PATH = path.join(GENESIS_DIR, 'docs', 'humans', 'humans.json');

function loadValidHumanIds(): Set<string> {
  if (!fs.existsSync(HUMANS_JSON_PATH)) {
    console.warn(`Warning: humans.json not found at ${HUMANS_JSON_PATH} — skipping humanId validation`);
    return new Set();
  }
  const data = JSON.parse(fs.readFileSync(HUMANS_JSON_PATH, 'utf-8'));
  return new Set((data.humans as Array<{ id: string }>).map(h => h.id));
}

const VALID_HUMAN_IDS = loadValidHumanIds();

// Content formats that require blob upload
const BLOB_FORMATS = ['html5-app', 'perseus-quiz-json'];

// ============================================================================
// Value Normalizers (map legacy/variant values to valid backend enums)
// ============================================================================

/** Map legacy/variant content formats to canonical values accepted by elohim-storage */
function normalizeContentFormat(format: string | undefined): ContentFormat {
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

// ============================================================================
// Types — from protocol schema (generated by codegen-ts.mjs)
// ============================================================================
// CreateContentInput imported from ./generated/create-content-input.js
// Already has ContentType, ContentFormat, Reach enum types — no narrowing needed.

// ============================================================================
// JSON file types from data/lamad/
// ============================================================================

interface ConceptJson {
  id: string;
  title: string;
  content?: string | object;
  contentFormat?: string;
  contentType?: string;
  description?: string;
  summary?: string;
  sourcePath?: string;
  relatedNodeIds?: string[];
  tags?: string[];
  estimatedMinutes?: number;
  thumbnailUrl?: string;
  metadata?: Record<string, unknown>;
  // Blob references for html5-app and large content
  blobHash?: string;       // Pre-computed hash (camelCase from JSON)
  blob_hash?: string;      // Alternative snake_case format
  entryPoint?: string;    // Entry point for html5-app (e.g., "index.html")
  stewardedBy?: StewardAnnotation[];
}

interface PathJson {
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
  visibility?: string;
  tags?: string[];
  chapters?: ChapterJson[];
  conceptIds?: string[];
}

interface ChapterJson {
  id: string;
  title: string;
  description?: string;
  order?: number;
  estimatedDuration?: string;
  modules?: ModuleJson[];
  conceptIds?: string[];
  steps?: StepJson[];  // Direct steps in chapter (know-thyself format)
}

interface StepJson {
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

interface ModuleJson {
  id: string;
  title: string;
  description?: string;
  order?: number;
  sections?: SectionJson[];
}

interface SectionJson {
  id: string;
  title: string;
  description?: string;
  order?: number;
  estimatedMinutes?: number;
  conceptIds?: string[];
}

// ============================================================================
// Utilities
// ============================================================================

class Timer {
  private start = Date.now();

  elapsed(): string {
    const ms = Date.now() - this.start;
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  }
}

function formatCount(n: number | undefined): string {
  return n != null ? n.toLocaleString() : '0';
}

/**
 * Validate HTTP response shape at the wire boundary.
 * TypeScript types disappear at runtime — this catches snake_case, missing fields,
 * and unexpected shapes before they cause cryptic errors downstream.
 */
function assertResponseShape<T>(
  data: unknown,
  requiredFields: (keyof T)[],
  endpoint: string,
): asserts data is T {
  if (typeof data !== 'object' || data === null) {
    throw new Error(
      `[${endpoint}] Expected object, got ${typeof data}. ` +
      `Response: ${JSON.stringify(data).slice(0, 200)}`
    );
  }
  const obj = data as Record<string, unknown>;
  const missing = requiredFields.filter(f => !(f as string in obj));
  if (missing.length > 0) {
    const snakeCase = missing.map(f => String(f).replace(/[A-Z]/g, c => `_${c.toLowerCase()}`));
    const hasSnake = snakeCase.some(s => s in obj);
    const hint = hasSnake
      ? ` (found snake_case equivalents — storage may need updating to camelCase)`
      : '';
    throw new Error(
      `[${endpoint}] Response missing required fields: ${missing.join(', ')}${hint}. ` +
      `Got keys: ${Object.keys(obj).join(', ')}`
    );
  }
}

/**
 * Compute SHA256 hash of data (matching elohim-storage format).
 */
function computeHash(data: Buffer): string {
  const hash = crypto.createHash('sha256').update(data).digest('hex');
  return `sha256-${hash}`;
}

/**
 * Upload a blob to elohim-storage.
 * Returns the hash on success, null on failure.
 */
async function uploadBlob(data: Buffer, mimeType: string, description?: string): Promise<string | null> {
  if (DRY_RUN) {
    const hash = computeHash(data);
    console.log(`   [DRY RUN] Would upload blob: ${hash} (${data.length} bytes)`);
    return hash;
  }

  const hash = computeHash(data);

  try {
    const response = await fetch(`${STORAGE_URL}/blob/${hash}`, {
      method: 'PUT',
      headers: {
        'Content-Type': mimeType,
      },
      body: new Uint8Array(data),
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error(`   ✗ Failed to upload ${description || hash}: ${response.status} - ${errorText}`);
      return null;
    }

    return hash;
  } catch (error) {
    console.error(`   ✗ Failed to upload ${description || hash}: ${error}`);
    return null;
  }
}

/**
 * Check if a blob exists in storage.
 */
async function blobExists(hash: string): Promise<boolean> {
  try {
    const response = await fetch(`${STORAGE_URL}/blob/${hash}`, {
      method: 'HEAD',
    });
    return response.status === 200;
  } catch {
    return false;
  }
}

/**
 * Find and load HTML5 app ZIP blob for content.
 * Returns the blob data and hash, or null if not found.
 */
function findHtml5AppBlob(concept: ConceptJson, contentDir: string): { data: Buffer; hash: string } | null {
  // Get existing hash (supports both camelCase and snake_case)
  const existingHash = concept.blobHash || concept.blobHash;
  const normalizedHash = existingHash
    ? (existingHash.startsWith('sha256-') ? existingHash : `sha256-${existingHash}`)
    : null;

  // Check metadata.localZipPath first
  const metadata = concept.metadata as Record<string, unknown> | undefined;
  if (metadata?.localZipPath) {
    const zipPath = path.join(GENESIS_DIR, metadata.localZipPath as string);
    if (fs.existsSync(zipPath)) {
      const data = fs.readFileSync(zipPath);
      const hash = normalizedHash || computeHash(data);
      console.log(`   📦 Found ZIP via metadata.localZipPath: ${metadata.localZipPath}`);
      return { data, hash };
    }
  }

  // Try to find a zip file with same ID in content directory
  const zipPath = path.join(contentDir, `${concept.id}.zip`);
  if (fs.existsSync(zipPath)) {
    const data = fs.readFileSync(zipPath);
    const hash = normalizedHash || computeHash(data);
    return { data, hash };
  }

  // If we have a hash reference but no local file, the blob should already be uploaded
  if (normalizedHash) {
    return null; // No local file to upload
  }

  return null;
}

/**
 * Find and load thumbnail image for a path.
 * Searches in genesis/assets/images/ directory.
 */
function findThumbnailBlob(thumbnailUrl: string | undefined): { data: Buffer; hash: string; mimeType: string } | null {
  if (!thumbnailUrl) return null;

  // Handle various path formats
  let imagePath: string | null = null;

  if (thumbnailUrl.startsWith('/images/')) {
    // Map /images/xxx to assets/images/xxx
    imagePath = path.join(GENESIS_DIR, 'assets', thumbnailUrl.slice(1));
  } else if (thumbnailUrl.startsWith('images/')) {
    imagePath = path.join(GENESIS_DIR, 'assets', thumbnailUrl);
  } else if (thumbnailUrl.startsWith('assets/')) {
    imagePath = path.join(GENESIS_DIR, thumbnailUrl);
  } else if (thumbnailUrl.startsWith('/assets/')) {
    imagePath = path.join(GENESIS_DIR, thumbnailUrl.slice(1));
  } else if (thumbnailUrl.startsWith('blob/') || thumbnailUrl.startsWith('/blob/')) {
    // Already a blob reference
    return null;
  }

  if (!imagePath || !fs.existsSync(imagePath)) {
    return null;
  }

  const data = fs.readFileSync(imagePath);
  const hash = computeHash(data);

  // Determine MIME type from extension
  const ext = path.extname(imagePath).toLowerCase();
  const mimeTypes: Record<string, string> = {
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.jpeg': 'image/jpeg',
    '.gif': 'image/gif',
    '.webp': 'image/webp',
    '.svg': 'image/svg+xml',
  };
  const mimeType = mimeTypes[ext] || 'application/octet-stream';

  return { data, hash, mimeType };
}

/**
 * Format a concept ID into a human-readable title.
 * Converts kebab-case to Title Case.
 * Examples:
 *   "manifesto" → "Manifesto"
 *   "quiz-manifesto-foundations" → "Quiz Manifesto Foundations"
 *   "elohim-lamad" → "Elohim Lamad"
 */
function formatConceptTitle(conceptId: string): string {
  return conceptId
    .split('-')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

// ============================================================================
// Stewardship Filtering
// ============================================================================

interface StewardAnnotation {
  humanId: string;
  affinity: number;
  role: string;
}

/**
 * Filter content nodes to those stewarded by a specific human.
 * Returns content where the given humanId is the highest-affinity steward.
 * If no stewardedBy field exists, defaults to the operator (backwards compat).
 */
function filterBySteward(
  concepts: ConceptJson[],
  humanId: string,
  operatorId: string = 'human-matthew-manager',
): ConceptJson[] {
  return concepts.filter(concept => {
    const stewards = concept.stewardedBy;

    if (!stewards || stewards.length === 0) {
      return humanId === operatorId;
    }

    const primary = stewards.reduce((max, s) => (s.affinity > max.affinity ? s : max), stewards[0]);
    return primary.humanId === humanId;
  });
}

// ============================================================================
// Content Loading
// ============================================================================

function loadContentFiles(): ConceptJson[] {
  const contentDir = path.join(DATA_DIR, 'content');
  if (!fs.existsSync(contentDir)) {
    console.error(`Content directory not found: ${contentDir}`);
    return [];
  }

  const files = fs.readdirSync(contentDir).filter(f => f.endsWith('.json'));
  const concepts: ConceptJson[] = [];

  for (const file of files) {
    try {
      const filePath = path.join(contentDir, file);
      const raw = fs.readFileSync(filePath, 'utf-8');
      const json = JSON.parse(raw);

      // Skip index files
      if (file === 'index.json') continue;

      // Ensure required fields
      if (!json.id || !json.title) {
        console.warn(`   Skipping ${file}: missing id or title`);
        continue;
      }

      concepts.push(json);
    } catch (err) {
      console.warn(`   Error loading ${file}: ${err}`);
    }
  }

  return concepts;
}

// ============================================================================
// Account Package Reach Override
//
// When --use-account-packages is set, loads account packages from
// genesis/data/account-packages/ and uses the maximum reach level assigned
// to each content item across all humans. This replaces the hardcoded
// 'public' reach with per-content reach levels derived from human affinities,
// stewardship, and relationship graphs.
// ============================================================================

/** Reach levels ordered from most restrictive to most permissive */
const REACH_ORDER: Record<string, number> = {
  private: 0,
  invited: 1,
  local: 2,
  neighborhood: 3,
  municipal: 4,
  commons: 5,
  public: 6, // Legacy compatibility
};

/**
 * Load account packages and build a map of content ID → maximum reach level.
 * The "maximum reach" is the most permissive reach assigned to this content
 * across all humans. This determines what reach the content is seeded at —
 * the P2P replication layer then restricts delivery based on per-human reach.
 */
function loadReachOverrides(): Map<string, string> {
  const overrides = new Map<string, string>();

  if (!fs.existsSync(ACCOUNT_PACKAGES_DIR)) {
    console.warn(`   Account packages directory not found: ${ACCOUNT_PACKAGES_DIR}`);
    return overrides;
  }

  const files = fs.readdirSync(ACCOUNT_PACKAGES_DIR).filter(
    f => f.endsWith('.json') && f !== 'index.json' && f !== 'conductor-groups.json'
  );

  for (const file of files) {
    try {
      const pkg = JSON.parse(fs.readFileSync(path.join(ACCOUNT_PACKAGES_DIR, file), 'utf-8'));
      if (!pkg.content || !Array.isArray(pkg.content)) continue;

      for (const assignment of pkg.content) {
        const existing = overrides.get(assignment.contentId);
        const existingOrder = existing ? (REACH_ORDER[existing] ?? 0) : -1;
        const newOrder = REACH_ORDER[assignment.reach] ?? 0;

        if (newOrder > existingOrder) {
          overrides.set(assignment.contentId, assignment.reach);
        }
      }
    } catch {
      // Skip malformed packages
    }
  }

  return overrides;
}

/** Global reach overrides — loaded once if --use-account-packages is set */
let reachOverrides: Map<string, string> | null = null;

function getReachForContent(contentId: string): Reach {
  if (!USE_ACCOUNT_PACKAGES) return 'public';

  if (!reachOverrides) {
    console.log('Loading reach overrides from account packages...');
    reachOverrides = loadReachOverrides();
    console.log(`   Loaded reach for ${reachOverrides.size} content items`);

    // Show distribution
    const dist = new Map<string, number>();
    for (const reach of reachOverrides.values()) {
      dist.set(reach, (dist.get(reach) || 0) + 1);
    }
    for (const [reach, count] of [...dist.entries()].sort((a, b) => b[1] - a[1])) {
      console.log(`     ${reach}: ${count}`);
    }
  }

  return (reachOverrides.get(contentId) as Reach | undefined) ?? 'commons';
}

function transformContent(json: ConceptJson): CreateContentInput {
  // Serialize content body to string
  let contentBody: string | undefined;
  let contentSizeBytes: number | undefined;
  if (json.content) {
    contentBody = typeof json.content === 'string'
      ? json.content
      : JSON.stringify(json.content);
    contentSizeBytes = Buffer.byteLength(contentBody, 'utf-8');
  }

  // Build metadata object (matches Rust JsonVal — parsed, not stringified)
  const metadata: Record<string, unknown> = {};
  if (json.metadata) Object.assign(metadata, json.metadata);
  if (json.estimatedMinutes) metadata.estimatedMinutes = json.estimatedMinutes;
  if (json.thumbnailUrl) metadata.thumbnailUrl = json.thumbnailUrl;
  if (json.relatedNodeIds?.length) metadata.relatedNodeIds = json.relatedNodeIds;
  if (json.summary) metadata.summary = json.summary;

  return {
    id: json.id,
    title: json.title,
    schemaVersion: 1,
    description: json.description || undefined,
    contentType: (json.contentType ?? 'concept') as ContentType,
    contentFormat: normalizeContentFormat(json.contentFormat),
    contentBody: contentBody ?? undefined,
    blobHash: json.blobHash ?? json.blob_hash ?? undefined,
    blobCid: undefined,
    contentSizeBytes: contentSizeBytes,
    metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
    reach: getReachForContent(json.id),
    createdBy: undefined,
    tags: json.tags || [],
  };
}

// ============================================================================
// Path Loading
// ============================================================================

function loadPathFiles(): PathJson[] {
  const pathsDir = path.join(DATA_DIR, 'paths');
  if (!fs.existsSync(pathsDir)) {
    console.error(`Paths directory not found: ${pathsDir}`);
    return [];
  }

  const files = fs.readdirSync(pathsDir).filter(f => f.endsWith('.json'));
  const paths: PathJson[] = [];

  for (const file of files) {
    try {
      const filePath = path.join(pathsDir, file);
      const raw = fs.readFileSync(filePath, 'utf-8');
      const json = JSON.parse(raw);

      // Skip index files
      if (file === 'index.json') continue;

      // Ensure required fields
      if (!json.id || !json.title) {
        console.warn(`   Skipping ${file}: missing id or title`);
        continue;
      }

      paths.push(json);
    } catch (err) {
      console.warn(`   Error loading ${file}: ${err}`);
    }
  }

  return paths;
}

interface SectionNode {
  id?: string;
  title?: string;
  description?: string;
  level?: string;
  sections?: SectionNode[];
  items?: SectionItem[];
  estimatedDuration?: string;
  optional?: boolean;
}

interface SectionItem {
  ref: string;
  role?: string;
  title?: string;
  narrative?: string;
  learningObjectives?: string[];
  completionCriteria?: { type: string; threshold?: number };
}

/**
 * Convert path JSON chapters into the sections tree format.
 * Handles three input shapes:
 * 1. chapters -> modules -> sections -> conceptIds (elohim-protocol)
 * 2. chapters -> steps (governance paths, bdd-smoke-tests)
 * 3. flat conceptIds (no chapters)
 */
function chaptersToSections(json: PathJson): SectionNode[] {
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

  return json.chapters.map((chapter, ci) => {
    const section: SectionNode = {
      id: chapter.id,
      title: chapter.title,
      description: chapter.description,
      level: 'unit',
      estimatedDuration: chapter.estimatedDuration,
    };

    // Shape 1: chapters -> modules -> sections -> conceptIds
    if (chapter.modules?.length) {
      section.sections = [];
      for (const mod of chapter.modules) {
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
      section.items = chapter.steps.map(step => {
        const item: SectionItem = {
          ref: step.resourceId ?? '',
          role: normalizeStepType(step.stepType),
          title: step.stepTitle || step.title || formatConceptTitle(step.resourceId ?? ''),
        };
        if (step.stepNarrative) item.narrative = step.stepNarrative;
        if (step.learningObjectives) item.learningObjectives = step.learningObjectives;
        if (step.completionCriteria) {
          item.completionCriteria = Array.isArray(step.completionCriteria)
            ? { type: step.completionCriteria.join(', ') }
            : step.completionCriteria;
        }
        return item;
      });
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

/**
 * Transform a path JSON file into a CreateContentInput for /db/content/bulk.
 * Chapters/modules/sections/conceptIds -> recursive RawSection[] tree with RawItem[] leaves.
 * This is the format parsePathView() in learning-path.model.ts expects.
 */
function transformPathToContent(json: PathJson): CreateContentInput {
  const sections = chaptersToSections(json);

  // Build metadata
  const metadata: Record<string, unknown> = {};
  if (json.pathType) metadata.pathType = json.pathType;
  if (json.difficulty) metadata.difficulty = json.difficulty;
  if (json.estimatedDuration) metadata.estimatedDuration = json.estimatedDuration;
  if (json.estimatedMinutes) metadata.estimatedDuration = `${json.estimatedMinutes} minutes`;
  if (json.version) metadata.version = json.version;
  if (json.purpose) metadata.purpose = json.purpose;
  if (json.thumbnailUrl) metadata.thumbnailUrl = json.thumbnailUrl;
  if (json.thumbnailAlt) metadata.thumbnailAlt = json.thumbnailAlt;

  const contentBody = JSON.stringify({ sections });

  return {
    id: json.id,
    title: json.title,
    schemaVersion: 1,
    description: json.description || undefined,
    contentType: 'path',
    contentFormat: 'epr-composite',
    contentBody,
    contentSizeBytes: Buffer.byteLength(contentBody, 'utf-8'),
    metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
    reach: (json.visibility as Reach | undefined) ?? 'public',
    tags: json.tags || [],
  };
}

// ============================================================================
// API Client
// ============================================================================

async function seedContent(items: CreateContentInput[]): Promise<{ inserted: number; skipped: number; errors: string[] }> {
  if (DRY_RUN) {
    console.log(`   [DRY RUN] Would seed ${items.length} content items`);
    return { inserted: items.length, skipped: 0, errors: [] };
  }

  const response = await fetch(`${STORAGE_URL}/db/content/bulk`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(items),
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }

  const data = await response.json();
  assertResponseShape<{ inserted: number; skipped: number; errors: string[] }>(
    data, ['inserted', 'skipped', 'errors'], '/db/content/bulk'
  );
  return data;
}

/** Count total items across a sections tree (for logging) */
function countItems(sections: SectionNode[]): number {
  let count = 0;
  for (const s of sections) {
    count += s.items?.length ?? 0;
    if (s.sections) count += countItems(s.sections);
  }
  return count;
}

async function getStats(): Promise<{ contentCount: number; uniqueTags: number }> {
  const response = await fetch(`${STORAGE_URL}/db/stats`);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }
  const data = await response.json();
  assertResponseShape<{ contentCount: number; uniqueTags: number }>(
    data, ['contentCount', 'uniqueTags'], '/db/stats'
  );
  return data;
}

// ============================================================================
// Main
// ============================================================================

async function main() {
  console.log('='.repeat(70));
  console.log('SQLite Content Seeder');
  console.log('='.repeat(70));

  // Validate environment
  if (!STORAGE_URL) {
    console.error('\nError: STORAGE_URL environment variable is required');
    console.error('Example: STORAGE_URL=http://localhost:8090 npx tsx src/seed-sqlite.ts');
    process.exit(1);
  }

  console.log(`\nConfiguration:`);
  console.log(`   Storage URL: ${STORAGE_URL}`);
  console.log(`   Data directory: ${DATA_DIR}`);
  console.log(`   Limit: ${LIMIT || 'none'}`);
  console.log(`   Dry run: ${DRY_RUN}`);
  console.log(`   Content only: ${CONTENT_ONLY}`);
  console.log(`   Paths only: ${PATHS_ONLY}`);
  console.log(`   Skip blob upload: ${SKIP_BLOB_UPLOAD}`);
  if (CONDUCTOR_FOR) {
    console.log(`   Conductor for: ${CONDUCTOR_FOR}`);
    if (VALID_HUMAN_IDS.size > 0 && !VALID_HUMAN_IDS.has(CONDUCTOR_FOR)) {
      console.error(`\nError: --conductor-for="${CONDUCTOR_FOR}" is not a valid humanId`);
      console.error(`Valid humanIds (from ${HUMANS_JSON_PATH}):`);
      for (const id of [...VALID_HUMAN_IDS].sort()) {
        console.error(`   ${id}`);
      }
      process.exit(1);
    }
  }

  // Check storage is available
  console.log(`\nChecking storage availability...`);
  try {
    const stats = await getStats();
    console.log(`   Current database: ${formatCount(stats.contentCount)} content, ${formatCount(stats.uniqueTags)} tags`);
  } catch (err) {
    console.error(`\nError: Cannot connect to storage at ${STORAGE_URL}`);
    console.error(`   ${err}`);
    console.error(`\nMake sure elohim-storage is running with ENABLE_CONTENT_DB=true`);
    process.exit(1);
  }

  const timer = new Timer();
  let totalInserted = 0;
  let totalSkipped = 0;
  let totalErrors: string[] = [];

  // Map to store uploaded blob hashes for content (id -> hash)
  const uploadedContentBlobs = new Map<string, string>();
  // Map to store uploaded thumbnail hashes for paths (thumbnailUrl -> hash)
  const uploadedThumbnails = new Map<string, string>();

  // ========================================
  // Phase 0: Upload Blobs (HTML5 apps, thumbnails)
  // ========================================
  if (!SKIP_BLOB_UPLOAD) {
    console.log(`\n${'='.repeat(70)}`);
    console.log(`Phase 0: Uploading Blobs`);
    console.log(`${'='.repeat(70)}`);

    const blobTimer = new Timer();
    const contentDir = path.join(DATA_DIR, 'content');
    let blobsUploaded = 0;
    let blobsSkipped = 0;
    let blobsFailed = 0;

    // Load content to find HTML5 apps
    console.log(`\nScanning for HTML5 app blobs...`);
    const content = loadContentFiles();
    const html5Apps = content.filter(c =>
      normalizeContentFormat(c.contentFormat) === 'html5-app' ||
      c.contentFormat === 'html5-app'
    );
    console.log(`   Found ${html5Apps.length} HTML5 app content items`);

    for (const app of html5Apps) {
      const blob = findHtml5AppBlob(app, contentDir);
      if (blob) {
        // Check if already exists
        const exists = await blobExists(blob.hash);
        if (exists) {
          console.log(`   ✓ ${app.id}: already exists (${blob.hash.slice(0, 16)}...)`);
          blobsSkipped++;
        } else {
          const hash = await uploadBlob(blob.data, 'application/zip', app.id);
          if (hash) {
            console.log(`   ✓ ${app.id}: uploaded ${(blob.data.length / 1024 / 1024).toFixed(2)} MB`);
            blobsUploaded++;
          } else {
            blobsFailed++;
          }
        }
        uploadedContentBlobs.set(app.id, blob.hash);
      } else {
        // Check if there's a hash reference we should verify
        const existingHash = app.blobHash || app.blob_hash;
        if (existingHash) {
          const normalizedHash = existingHash.startsWith('sha256-') ? existingHash : `sha256-${existingHash}`;
          const exists = await blobExists(normalizedHash);
          if (!exists) {
            console.warn(`   ⚠️ ${app.id}: blob_hash exists but blob not found in storage`);
          }
          uploadedContentBlobs.set(app.id, normalizedHash);
        }
      }
    }

    // Scan for path thumbnails
    console.log(`\nScanning for path thumbnails...`);
    const paths = loadPathFiles();
    const pathsWithThumbnails = paths.filter(p => p.thumbnailUrl);
    console.log(`   Found ${pathsWithThumbnails.length} paths with thumbnails`);

    for (const pathItem of pathsWithThumbnails) {
      if (!pathItem.thumbnailUrl) continue;

      // Skip if already processed
      if (uploadedThumbnails.has(pathItem.thumbnailUrl)) continue;

      const thumbnail = findThumbnailBlob(pathItem.thumbnailUrl);
      if (thumbnail) {
        const exists = await blobExists(thumbnail.hash);
        if (exists) {
          console.log(`   ✓ ${pathItem.id}: thumbnail already exists`);
          blobsSkipped++;
        } else {
          const hash = await uploadBlob(thumbnail.data, thumbnail.mimeType, `${pathItem.id} thumbnail`);
          if (hash) {
            console.log(`   ✓ ${pathItem.id}: thumbnail uploaded ${(thumbnail.data.length / 1024).toFixed(1)} KB`);
            blobsUploaded++;
          } else {
            blobsFailed++;
          }
        }
        uploadedThumbnails.set(pathItem.thumbnailUrl, thumbnail.hash);
      }
    }

    console.log(`\nBlob upload complete in ${blobTimer.elapsed()}`);
    console.log(`   Uploaded: ${blobsUploaded}, Skipped: ${blobsSkipped}, Failed: ${blobsFailed}`);
  }

  // ========================================
  // Phase 1: Seed Content
  // ========================================
  if (!PATHS_ONLY) {
    console.log(`\n${'='.repeat(70)}`);
    console.log(`Phase 1: Seeding Content`);
    console.log(`${'='.repeat(70)}`);

    const contentTimer = new Timer();
    console.log(`\nLoading content files...`);
    let content = loadContentFiles();
    console.log(`   Loaded ${formatCount(content.length)} content items`);

    // Validate stewardedBy humanIds against canonical registry
    if (VALID_HUMAN_IDS.size > 0) {
      const invalidIds = new Map<string, number>();
      for (const concept of content) {
        const stewards = concept.stewardedBy;
        if (stewards) {
          for (const s of stewards) {
            if (!VALID_HUMAN_IDS.has(s.humanId)) {
              invalidIds.set(s.humanId, (invalidIds.get(s.humanId) || 0) + 1);
            }
          }
        }
      }
      if (invalidIds.size > 0) {
        console.error(`\n   ❌ ERROR: Content references unknown humanIds not in humans.json:`);
        for (const [id, count] of [...invalidIds.entries()].sort((a, b) => b[1] - a[1])) {
          console.error(`      ${id}: ${count} content nodes`);
        }
        console.error(`   Re-run genesis/scripts/annotate-stewardship.py to fix.`);
        process.exit(1);
      }
      console.log(`   [stewardship] All stewardedBy humanIds validated against humans.json ✓`);
    }

    if (CONDUCTOR_FOR) {
      const beforeCount = content.length;
      content = filterBySteward(content, CONDUCTOR_FOR);
      console.log(`   [stewardship] Filtered to ${content.length}/${beforeCount} content nodes for ${CONDUCTOR_FOR}`);
    }

    if (LIMIT > 0 && content.length > LIMIT) {
      console.log(`   Limiting to ${LIMIT} items`);
      content = content.slice(0, LIMIT);
    }

    console.log(`\nTransforming content...`);
    const contentInputs = content.map(c => {
      const input = transformContent(c);
      // Add blob_hash if we uploaded one for this content
      const blobHash = uploadedContentBlobs.get(c.id);
      if (blobHash) {
        input.blobHash = blobHash;
      }
      return input;
    });
    console.log(`   Transformed ${formatCount(contentInputs.length)} items`);

    console.log(`\nSeeding content to database...`);
    const BATCH_SIZE = 500;
    for (let i = 0; i < contentInputs.length; i += BATCH_SIZE) {
      const batch = contentInputs.slice(i, i + BATCH_SIZE);
      try {
        const result = await seedContent(batch);
        totalInserted += result.inserted;
        totalSkipped += result.skipped;
        totalErrors.push(...result.errors);

        console.log(`   Batch ${Math.floor(i / BATCH_SIZE) + 1}: ${result.inserted} inserted, ${result.skipped} skipped`);
      } catch (err) {
        console.error(`   Batch ${Math.floor(i / BATCH_SIZE) + 1} failed: ${err}`);
        totalErrors.push(`Batch ${Math.floor(i / BATCH_SIZE) + 1}: ${err}`);
      }
    }

    console.log(`\nContent seeding complete in ${contentTimer.elapsed()}`);
  }

  // ========================================
  // Phase 2: Seed Paths
  // ========================================
  if (!CONTENT_ONLY) {
    console.log(`\n${'='.repeat(70)}`);
    console.log(`Phase 2: Seeding Paths as Content`);
    console.log(`${'='.repeat(70)}`);

    const pathTimer = new Timer();
    console.log(`\nLoading path files...`);
    let paths = loadPathFiles();
    console.log(`   Loaded ${formatCount(paths.length)} paths`);

    if (LIMIT > 0 && paths.length > LIMIT) {
      console.log(`   Limiting to ${LIMIT} items`);
      paths = paths.slice(0, LIMIT);
    }

    console.log(`\nTransforming paths to content nodes...`);
    const pathContentInputs = paths.map(p => {
      const input = transformPathToContent(p);
      // Update thumbnailUrl to blob reference if we uploaded one
      if (p.thumbnailUrl && uploadedThumbnails.has(p.thumbnailUrl)) {
        const blobHash = uploadedThumbnails.get(p.thumbnailUrl)!;
        const meta = (typeof input.metadata === 'object' && input.metadata !== null ? input.metadata : {}) as Record<string, unknown>;
        meta.thumbnailUrl = `/blob/${blobHash}`;
        input.metadata = meta;
      }
      return input;
    });

    // Count steps for logging
    const totalSteps = pathContentInputs.reduce((sum, p) => {
      try {
        const body = JSON.parse(p.contentBody || '{}');
        return sum + countItems(body.sections || []);
      } catch { return sum; }
    }, 0);
    console.log(`   Transformed ${formatCount(pathContentInputs.length)} paths with ${formatCount(totalSteps)} steps`);

    console.log(`\nSeeding paths to database...`);
    try {
      const result = await seedContent(pathContentInputs);
      totalInserted += result.inserted;
      totalSkipped += result.skipped;
      totalErrors.push(...result.errors);
      console.log(`   ${result.inserted} paths inserted, ${result.skipped} skipped`);
    } catch (err) {
      console.error(`   Path seeding failed: ${err}`);
      totalErrors.push(`Paths: ${err}`);
    }

    console.log(`\nPath seeding complete in ${pathTimer.elapsed()}`);
  }

  // ========================================
  // Summary
  // ========================================
  console.log(`\n${'='.repeat(70)}`);
  console.log(`Summary`);
  console.log(`${'='.repeat(70)}`);

  try {
    const finalStats = await getStats();
    console.log(`\nFinal database state:`);
    console.log(`   Content: ${formatCount(finalStats.contentCount)} items`);
    console.log(`   Tags: ${formatCount(finalStats.uniqueTags)} unique`);
  } catch (err) {
    console.log(`\nCould not get final stats: ${err}`);
  }

  console.log(`\nSeeding results:`);
  console.log(`   Total inserted: ${formatCount(totalInserted)}`);
  console.log(`   Total skipped: ${formatCount(totalSkipped)}`);
  console.log(`   Total errors: ${totalErrors.length}`);
  console.log(`   Total time: ${timer.elapsed()}`);

  if (totalErrors.length > 0) {
    console.log(`\nErrors (first 10):`);
    for (const err of totalErrors.slice(0, 10)) {
      console.log(`   - ${err}`);
    }
    if (totalErrors.length > 10) {
      console.log(`   ... and ${totalErrors.length - 10} more`);
    }
  }

  console.log(`\n${'='.repeat(70)}`);
  if (totalErrors.length > 0) {
    process.exit(1);
  }
}

main().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});
