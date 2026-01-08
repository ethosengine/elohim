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
 */

import * as fs from 'fs';
import * as path from 'path';
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

// ============================================================================
// Types (matching elohim-storage db schema)
// ============================================================================

interface CreateContentInput {
  id: string;
  title: string;
  description?: string;
  content_type: string;
  content_format: string;
  /** Inline content body (markdown, JSON quiz data, etc.) */
  content_body?: string;
  blob_hash?: string;
  blob_cid?: string;
  content_size_bytes?: number;
  metadata_json?: string;
  reach: string;
  created_by?: string;
  tags: string[];
}

interface CreatePathInput {
  id: string;
  title: string;
  description?: string;
  path_type: string;
  difficulty?: string;
  estimated_duration?: string;
  thumbnail_url?: string;
  thumbnail_alt?: string;
  metadata_json?: string;
  visibility: string;
  created_by?: string;
  tags: string[];
  chapters: CreateChapterInput[];
}

interface CreateChapterInput {
  id: string;
  title: string;
  description?: string;
  order_index: number;
  estimated_duration?: string;
  steps: CreateStepInput[];
}

interface CreateStepInput {
  id: string;
  path_id: string;
  chapter_id?: string;
  title: string;
  description?: string;
  step_type: string;
  resource_id?: string;
  resource_type?: string;
  order_index: number;
  estimated_duration?: string;
  metadata_json?: string;
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
    content_type: json.contentType || 'concept',
    content_format: json.contentFormat || 'markdown',
    content_body: contentBody,
    content_size_bytes: contentSizeBytes,
    metadata_json: Object.keys(metadata).length > 0 ? JSON.stringify(metadata) : undefined,
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
                    path_id: json.id,
                    chapter_id: chapter.id,
                    // Use concept ID as title - each step gets unique title
                    // Section title is stored in metadata for grouping context
                    title: formatConceptTitle(conceptId),
                    description: section.description,
                    step_type: 'learn',
                    resource_id: conceptId,
                    resource_type: 'content',
                    order_index: stepIndex++,
                    estimated_duration: section.estimatedMinutes
                      ? `${section.estimatedMinutes} minutes`
                      : undefined,
                    // Store section context in metadata for UI grouping
                    metadata_json: JSON.stringify({
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
            path_id: json.id,
            chapter_id: chapter.id,
            title: formatConceptTitle(conceptId),
            step_type: 'learn',
            resource_id: conceptId,
            resource_type: 'content',
            order_index: stepIndex++,
          });
        }
      }

      // Handle direct steps in chapter (know-thyself format)
      if (chapter.steps) {
        for (const step of chapter.steps) {
          chapterSteps.push({
            id: `${json.id}-step-${stepIndex}`,
            path_id: json.id,
            chapter_id: chapter.id,
            title: step.stepTitle || step.resourceId || `Step ${stepIndex + 1}`,
            description: step.stepNarrative,
            step_type: step.stepType === 'content' ? 'learn' : (step.stepType || 'learn'),
            resource_id: step.resourceId,
            resource_type: 'content',
            order_index: step.order ?? stepIndex,
            estimated_duration: step.estimatedTime,
            metadata_json: step.learningObjectives || step.completionCriteria
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
        order_index: chapter.order ?? ci,
        estimated_duration: chapter.estimatedDuration,
        steps: chapterSteps,
      });
    }
  }

  // Handle flat conceptIds format (no chapters)
  if (json.conceptIds && json.conceptIds.length > 0 && chapters.length === 0) {
    // Create a default chapter to hold steps
    const defaultSteps: CreateStepInput[] = json.conceptIds.map((conceptId, i) => ({
      id: `${json.id}-step-${i}`,
      path_id: json.id,
      title: formatConceptTitle(conceptId),
      step_type: 'learn',
      resource_id: conceptId,
      resource_type: 'content',
      order_index: i,
    }));

    chapters.push({
      id: `${json.id}-default-chapter`,
      title: json.title,
      description: json.description,
      order_index: 0,
      steps: defaultSteps,
    });
  }

  return {
    id: json.id,
    title: json.title,
    description: json.description,
    path_type: 'guided',
    difficulty: json.difficulty,
    estimated_duration: json.estimatedDuration,
    thumbnail_url: json.thumbnailUrl,
    thumbnail_alt: json.thumbnailAlt,
    metadata_json: Object.keys(metadata).length > 0 ? JSON.stringify(metadata) : undefined,
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
    const contentInputs = content.map(transformContent);
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
    const pathInputs = paths.map(transformPath);
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
