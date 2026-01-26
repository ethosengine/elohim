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

// Content formats that require blob upload
const BLOB_FORMATS = ['html5-app', 'perseus-quiz-json'];

// ============================================================================
// Value Normalizers (map legacy/variant values to valid backend enums)
// ============================================================================

/** Valid content formats accepted by elohim-storage */
const VALID_CONTENT_FORMATS = [
  'markdown', 'html', 'json', 'text', 'perseus', 'sophia', 'gherkin',
  'yaml', 'toml', 'latex', 'asciidoc', 'html5-app', 'iframe', 'embed'
];

/** Map legacy/variant content formats to valid values */
function normalizeContentFormat(format: string | undefined): string {
  if (!format) return 'markdown';

  const normalized = format.toLowerCase();

  // Map variants to canonical values
  const mappings: Record<string, string> = {
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
  if (VALID_CONTENT_FORMATS.includes(normalized)) return normalized;

  // Default to markdown for unknown formats
  console.warn(`   ⚠️ Unknown contentFormat '${format}', defaulting to 'markdown'`);
  return 'markdown';
}

/** Valid step types accepted by elohim-storage */
const VALID_STEP_TYPES = [
  'learn', 'practice', 'quiz', 'assessment', 'discussion',
  'project', 'resource', 'video', 'reading', 'checkpoint'
];

/** Map legacy/variant step types to valid values */
function normalizeStepType(stepType: string | undefined): string {
  if (!stepType) return 'learn';

  const normalized = stepType.toLowerCase();

  // Map variants to canonical values
  const mappings: Record<string, string> = {
    'content': 'learn',
    'assess': 'assessment',
    'test': 'quiz',
    'watch': 'video',
    'read': 'reading',
  };

  if (mappings[normalized]) return mappings[normalized];
  if (VALID_STEP_TYPES.includes(normalized)) return normalized;

  // Default to learn for unknown types
  console.warn(`   ⚠️ Unknown stepType '${stepType}', defaulting to 'learn'`);
  return 'learn';
}

// ============================================================================
// Types (matching elohim-storage db schema)
// ============================================================================

interface CreateContentInput {
  id: string;
  title: string;
  description?: string;
  contentType: string;
  contentFormat: string;
  /** Inline content body (markdown, JSON quiz data, etc.) */
  contentBody?: string;
  blobHash?: string;
  blobCid?: string;
  contentSizeBytes?: number;
  metadataJson?: string;
  reach: string;
  createdBy?: string;
  tags: string[];
}

interface CreatePathInput {
  id: string;
  title: string;
  description?: string;
  pathType: string;
  difficulty?: string;
  estimatedDuration?: string;
  thumbnailUrl?: string;
  thumbnailAlt?: string;
  metadataJson?: string;
  visibility: string;
  createdBy?: string;
  tags: string[];
  chapters: CreateChapterInput[];
}

interface CreateChapterInput {
  id: string;
  title: string;
  description?: string;
  orderIndex: number;
  estimatedDuration?: string;
  steps: CreateStepInput[];
}

interface CreateStepInput {
  id: string;
  pathId: string;
  chapterId?: string;
  title: string;
  description?: string;
  stepType: string;
  resourceId?: string;
  resourceType?: string;
  orderIndex: number;
  estimatedDuration?: string;
  metadataJson?: string;
}

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
}

interface PathJson {
  id: string;
  title: string;
  description?: string;
  purpose?: string;
  difficulty?: string;
  estimatedDuration?: string;
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

function formatCount(n: number): string {
  return n.toLocaleString();
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

  // Build metadata JSON
  const metadata: Record<string, unknown> = {};
  if (json.metadata) Object.assign(metadata, json.metadata);
  if (json.estimatedMinutes) metadata.estimatedMinutes = json.estimatedMinutes;
  if (json.thumbnailUrl) metadata.thumbnailUrl = json.thumbnailUrl;
  if (json.relatedNodeIds?.length) metadata.relatedNodeIds = json.relatedNodeIds;
  if (json.summary) metadata.summary = json.summary;

  return {
    id: json.id,
    title: json.title,
    description: json.description || undefined,
    contentType: json.contentType || 'concept',
    contentFormat: normalizeContentFormat(json.contentFormat),
    contentBody: contentBody,
    contentSizeBytes: contentSizeBytes,
    metadataJson: Object.keys(metadata).length > 0 ? JSON.stringify(metadata) : undefined,
    reach: 'public',
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

function transformPath(json: PathJson): CreatePathInput {
  const chapters: CreateChapterInput[] = [];
  let stepIndex = 0;

  // Build metadata JSON
  const metadata: Record<string, unknown> = {};
  if (json.purpose) metadata.purpose = json.purpose;
  if (json.version) metadata.version = json.version;
  if (json.thumbnailAlt) metadata.thumbnailAlt = json.thumbnailAlt;

  // Handle hierarchical chapters format
  if (json.chapters && json.chapters.length > 0) {
    for (let ci = 0; ci < json.chapters.length; ci++) {
      const chapter = json.chapters[ci];
      const chapterSteps: CreateStepInput[] = [];

      // Flatten modules/sections into steps
      if (chapter.modules) {
        for (const mod of chapter.modules) {
          if (mod.sections) {
            for (const section of mod.sections) {
              if (section.conceptIds) {
                for (const conceptId of section.conceptIds) {
                  chapterSteps.push({
                    id: `${json.id}-step-${stepIndex}`,
                    pathId: json.id,
                    chapterId: chapter.id,
                    // Use concept ID as title - each step gets unique title
                    // Section title is stored in metadata for grouping context
                    title: formatConceptTitle(conceptId),
                    description: section.description,
                    stepType: 'learn',
                    resourceId: conceptId,
                    resourceType: 'content',
                    orderIndex: stepIndex++,
                    estimatedDuration: section.estimatedMinutes
                      ? `${section.estimatedMinutes} minutes`
                      : undefined,
                    // Store section context in metadata for UI grouping
                    metadataJson: JSON.stringify({
                      sectionTitle: section.title,
                      sectionOrder: section.order,
                    }),
                  });
                }
              }
            }
          }
        }
      }

      // Handle flat conceptIds in chapter
      if (chapter.conceptIds) {
        for (const conceptId of chapter.conceptIds) {
          chapterSteps.push({
            id: `${json.id}-step-${stepIndex}`,
            pathId: json.id,
            chapterId: chapter.id,
            title: formatConceptTitle(conceptId),
            stepType: 'learn',
            resourceId: conceptId,
            resourceType: 'content',
            orderIndex: stepIndex++,
          });
        }
      }

      // Handle direct steps in chapter (know-thyself format)
      if (chapter.steps) {
        for (const step of chapter.steps) {
          chapterSteps.push({
            id: `${json.id}-step-${stepIndex}`,
            pathId: json.id,
            chapterId: chapter.id,
            title: step.stepTitle || step.resourceId || `Step ${stepIndex + 1}`,
            description: step.stepNarrative,
            stepType: normalizeStepType(step.stepType),
            resourceId: step.resourceId,
            resourceType: 'content',
            orderIndex: step.order ?? stepIndex,
            estimatedDuration: step.estimatedTime,
            metadataJson: step.learningObjectives || step.completionCriteria
              ? JSON.stringify({
                  learningObjectives: step.learningObjectives,
                  completionCriteria: step.completionCriteria,
                  optional: step.optional,
                })
              : undefined,
          });
          stepIndex++;
        }
      }

      chapters.push({
        id: chapter.id,
        title: chapter.title,
        description: chapter.description,
        orderIndex: chapter.order ?? ci,
        estimatedDuration: chapter.estimatedDuration,
        steps: chapterSteps,
      });
    }
  }

  // Handle flat conceptIds format (no chapters)
  if (json.conceptIds && json.conceptIds.length > 0 && chapters.length === 0) {
    // Create a default chapter to hold steps
    const defaultSteps: CreateStepInput[] = json.conceptIds.map((conceptId, i) => ({
      id: `${json.id}-step-${i}`,
      pathId: json.id,
      title: formatConceptTitle(conceptId),
      stepType: 'learn',
      resourceId: conceptId,
      resourceType: 'content',
      orderIndex: i,
    }));

    chapters.push({
      id: `${json.id}-default-chapter`,
      title: json.title,
      description: json.description,
      orderIndex: 0,
      steps: defaultSteps,
    });
  }

  return {
    id: json.id,
    title: json.title,
    description: json.description,
    pathType: 'guided',
    difficulty: json.difficulty,
    estimatedDuration: json.estimatedDuration,
    thumbnailUrl: json.thumbnailUrl,
    thumbnailAlt: json.thumbnailAlt,
    metadataJson: Object.keys(metadata).length > 0 ? JSON.stringify(metadata) : undefined,
    visibility: json.visibility || 'public',
    tags: json.tags || [],
    chapters,
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

  return response.json();
}

async function seedPaths(items: CreatePathInput[]): Promise<{ inserted: number; skipped: number; errors: string[] }> {
  if (DRY_RUN) {
    console.log(`   [DRY RUN] Would seed ${items.length} paths`);
    return { inserted: items.length, skipped: 0, errors: [] };
  }

  const response = await fetch(`${STORAGE_URL}/db/paths/bulk`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(items),
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }

  return response.json();
}

async function getStats(): Promise<{ content_count: number; path_count: number; step_count: number; unique_tags: number }> {
  const response = await fetch(`${STORAGE_URL}/db/stats`);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }
  return response.json();
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

  // Check storage is available
  console.log(`\nChecking storage availability...`);
  try {
    const stats = await getStats();
    console.log(`   Current database: ${formatCount(stats.content_count)} content, ${formatCount(stats.path_count)} paths`);
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
    console.log(`Phase 2: Seeding Paths`);
    console.log(`${'='.repeat(70)}`);

    const pathTimer = new Timer();
    console.log(`\nLoading path files...`);
    let paths = loadPathFiles();
    console.log(`   Loaded ${formatCount(paths.length)} paths`);

    if (LIMIT > 0 && paths.length > LIMIT) {
      console.log(`   Limiting to ${LIMIT} items`);
      paths = paths.slice(0, LIMIT);
    }

    console.log(`\nTransforming paths...`);
    const pathInputs = paths.map(p => {
      const input = transformPath(p);
      // Update thumbnail_url to blob reference if we uploaded one
      if (p.thumbnailUrl && uploadedThumbnails.has(p.thumbnailUrl)) {
        const blobHash = uploadedThumbnails.get(p.thumbnailUrl)!;
        input.thumbnailUrl = `/blob/${blobHash}`;
      }
      return input;
    });
    const totalSteps = pathInputs.reduce((sum, p) =>
      sum + p.chapters.reduce((csum, c) => csum + c.steps.length, 0), 0);
    console.log(`   Transformed ${formatCount(pathInputs.length)} paths with ${formatCount(totalSteps)} steps`);

    console.log(`\nSeeding paths to database...`);
    try {
      const result = await seedPaths(pathInputs);
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
    console.log(`   Content: ${formatCount(finalStats.content_count)} items`);
    console.log(`   Paths: ${formatCount(finalStats.path_count)} items`);
    console.log(`   Steps: ${formatCount(finalStats.step_count)} items`);
    console.log(`   Tags: ${formatCount(finalStats.unique_tags)} unique`);
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
