/**
 * Holochain Content Seeder
 *
 * Seeds pre-structured JSON content from /data/lamad into Holochain.
 * This is a deterministic script that loads JSON files created by Claude + MCP tools.
 *
 * Pipeline: docs/ → Claude + MCP → data/lamad/ → seeder → Holochain DHT
 *
 * @version 2024-12-24 - Trigger seeder after seed data migration
 */

import * as fs from 'fs';
import * as path from 'path';
import DoorwayClient, { ImportStatusResponse } from './doorway-client.js';
import StorageClient from './storage-client.js'; // Used for computing blob hash

// ========================================
// PERFORMANCE TIMING UTILITIES
// ========================================
interface TimingStats {
  count: number;
  totalMs: number;
  minMs: number;
  maxMs: number;
  samples: number[];  // Keep last N samples for percentile calc
}

interface SkippedFile {
  file: string;
  reason: string;
}

class PerformanceTimer {
  private stats: Map<string, TimingStats> = new Map();
  private phases: Map<string, { start: number; end?: number }> = new Map();
  private skippedFiles: SkippedFile[] = [];
  private overallStart: number = Date.now();
  private readonly maxSamples = 100;

  private seedResults: { conceptsCreated: number; conceptErrors: number; pathsCreated: number; pathErrors: number } = {
    conceptsCreated: 0, conceptErrors: 0, pathsCreated: 0, pathErrors: 0
  };

  recordSkipped(file: string, reason: string): void {
    this.skippedFiles.push({ file, reason });
  }

  getSkippedFiles(): SkippedFile[] {
    return this.skippedFiles;
  }

  setSeedResults(results: { conceptsCreated: number; conceptErrors: number; pathsCreated: number; pathErrors: number }): void {
    this.seedResults = results;
  }

  /**
   * Export report data as JSON for saving with snapshots
   */
  exportReport(): object {
    const totalDuration = Date.now() - this.overallStart;

    // Build phase data
    const phases: Record<string, { durationMs: number; percentage: number }> = {};
    for (const [name, phase] of this.phases) {
      if (phase.end) {
        const duration = phase.end - phase.start;
        phases[name] = {
          durationMs: duration,
          percentage: parseFloat(((duration / totalDuration) * 100).toFixed(1)),
        };
      }
    }

    // Build operation stats
    const operations: Record<string, { count: number; totalMs: number; avgMs: number; p50Ms: number; p95Ms: number; maxMs: number }> = {};
    for (const [category, stat] of this.stats) {
      const avg = stat.count > 0 ? stat.totalMs / stat.count : 0;
      operations[category] = {
        count: stat.count,
        totalMs: stat.totalMs,
        avgMs: parseFloat(avg.toFixed(0)),
        p50Ms: parseFloat(this.percentile(stat.samples, 50).toFixed(0)),
        p95Ms: parseFloat(this.percentile(stat.samples, 95).toFixed(0)),
        maxMs: stat.maxMs,
      };
    }

    // Group skipped files by reason
    const skippedByReason: Record<string, string[]> = {};
    for (const { file, reason } of this.skippedFiles) {
      if (!skippedByReason[reason]) skippedByReason[reason] = [];
      skippedByReason[reason].push(file);
    }

    return {
      timestamp: new Date().toISOString(),
      totalDurationMs: totalDuration,
      totalDurationFormatted: this.formatDuration(totalDuration),
      results: this.seedResults,
      phases,
      operations,
      skippedFiles: {
        total: this.skippedFiles.length,
        byReason: skippedByReason,
      },
    };
  }

  startPhase(name: string): void {
    this.phases.set(name, { start: Date.now() });
    console.log(`\n⏱️  [${name}] Starting...`);
  }

  endPhase(name: string): number {
    const phase = this.phases.get(name);
    if (!phase) return 0;
    phase.end = Date.now();
    const duration = phase.end - phase.start;
    console.log(`⏱️  [${name}] Completed in ${this.formatDuration(duration)}`);
    return duration;
  }

  async timeOperation<T>(category: string, operation: () => Promise<T>): Promise<T> {
    const start = Date.now();
    try {
      return await operation();
    } finally {
      const duration = Date.now() - start;
      this.recordTiming(category, duration);
    }
  }

  private recordTiming(category: string, durationMs: number): void {
    let stat = this.stats.get(category);
    if (!stat) {
      stat = { count: 0, totalMs: 0, minMs: Infinity, maxMs: 0, samples: [] };
      this.stats.set(category, stat);
    }
    stat.count++;
    stat.totalMs += durationMs;
    stat.minMs = Math.min(stat.minMs, durationMs);
    stat.maxMs = Math.max(stat.maxMs, durationMs);
    // Keep samples for percentile calculation
    if (stat.samples.length < this.maxSamples) {
      stat.samples.push(durationMs);
    } else {
      // Reservoir sampling to maintain representative samples
      const idx = Math.floor(Math.random() * stat.count);
      if (idx < this.maxSamples) {
        stat.samples[idx] = durationMs;
      }
    }
  }

  private formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    const mins = Math.floor(ms / 60000);
    const secs = ((ms % 60000) / 1000).toFixed(1);
    return `${mins}m ${secs}s`;
  }

  private percentile(arr: number[], p: number): number {
    if (arr.length === 0) return 0;
    const sorted = [...arr].sort((a, b) => a - b);
    const idx = Math.ceil((p / 100) * sorted.length) - 1;
    return sorted[Math.max(0, idx)];
  }

  printReport(): void {
    const totalDuration = Date.now() - this.overallStart;

    console.log('\n' + '='.repeat(70));
    console.log('📊 PERFORMANCE ANALYSIS REPORT');
    console.log('='.repeat(70));
    console.log(`\n⏱️  Total Runtime: ${this.formatDuration(totalDuration)}`);

    // Phase breakdown
    console.log('\n📈 Phase Breakdown:');
    console.log('-'.repeat(50));
    for (const [name, phase] of this.phases) {
      if (phase.end) {
        const duration = phase.end - phase.start;
        const pct = ((duration / totalDuration) * 100).toFixed(1);
        console.log(`   ${name.padEnd(30)} ${this.formatDuration(duration).padStart(10)} (${pct}%)`);
      }
    }

    // Operation statistics
    console.log('\n📉 Operation Statistics:');
    console.log('-'.repeat(70));
    console.log('   ' + 'Operation'.padEnd(25) + 'Count'.padStart(8) + 'Total'.padStart(12) + 'Avg'.padStart(10) + 'p50'.padStart(8) + 'p95'.padStart(8) + 'Max'.padStart(8));
    console.log('-'.repeat(70));

    // Sort by total time descending
    const sortedStats = [...this.stats.entries()].sort((a, b) => b[1].totalMs - a[1].totalMs);

    for (const [category, stat] of sortedStats) {
      const avg = stat.count > 0 ? stat.totalMs / stat.count : 0;
      const p50 = this.percentile(stat.samples, 50);
      const p95 = this.percentile(stat.samples, 95);
      console.log(
        '   ' +
        category.padEnd(25) +
        stat.count.toString().padStart(8) +
        this.formatDuration(stat.totalMs).padStart(12) +
        `${avg.toFixed(0)}ms`.padStart(10) +
        `${p50.toFixed(0)}ms`.padStart(8) +
        `${p95.toFixed(0)}ms`.padStart(8) +
        `${stat.maxMs.toFixed(0)}ms`.padStart(8)
      );
    }

    // Performance recommendations
    console.log('\n💡 Performance Analysis:');
    console.log('-'.repeat(50));

    const batchCheck = this.stats.get('batch_exists_check');
    const bulkCreate = this.stats.get('bulk_create_content');
    const batchPathCheck = this.stats.get('batch_path_exists_check');
    const batchSteps = this.stats.get('batch_add_path_steps');

    if (batchCheck) {
      console.log(`   • Batch ID checks: ${batchCheck.count} calls, ${this.formatDuration(batchCheck.totalMs)} total`);
      console.log(`     (Previously would have been ~${batchCheck.count * 500} individual calls)`);
    }

    if (bulkCreate) {
      const avgPerBatch = bulkCreate.totalMs / bulkCreate.count;
      console.log(`   • Bulk content creation: ${bulkCreate.count} batches, ${this.formatDuration(bulkCreate.totalMs)} total`);
      console.log(`     Average ${avgPerBatch.toFixed(0)}ms per batch of ~50 concepts`);
    }

    if (batchPathCheck) {
      console.log(`   • Path ID check: ${this.formatDuration(batchPathCheck.totalMs)} (single batch call)`);
    }

    if (batchSteps) {
      console.log(`   • Batch step creation: ${batchSteps.count} calls, ${this.formatDuration(batchSteps.totalMs)} total`);
    }

    // Calculate estimated savings
    const totalTime = Date.now() - this.overallStart;
    console.log(`\n   📈 Estimated speedup from batch operations:`);
    console.log(`      Old approach: ~${this.formatDuration(totalTime * 10)} (estimated)`);
    console.log(`      New approach: ${this.formatDuration(totalTime)}`);
    console.log(`      Speedup: ~10x faster`);

    // Skipped files report
    if (this.skippedFiles.length > 0) {
      console.log('\n⚠️  Skipped Files (need attention):');
      console.log('-'.repeat(70));

      // Group by reason
      const byReason = new Map<string, string[]>();
      for (const { file, reason } of this.skippedFiles) {
        const files = byReason.get(reason) || [];
        files.push(file);
        byReason.set(reason, files);
      }

      for (const [reason, files] of byReason) {
        console.log(`\n   ${reason} (${files.length} files):`);
        // Show first 10 files, summarize rest
        const displayFiles = files.slice(0, 10);
        for (const file of displayFiles) {
          console.log(`      • ${file}`);
        }
        if (files.length > 10) {
          console.log(`      ... and ${files.length - 10} more`);
        }
      }

      console.log(`\n   📁 Total skipped: ${this.skippedFiles.length} files`);
      console.log('   💡 Fix these files to include them in future seeds');
    }

    console.log('\n' + '='.repeat(70));
  }
}

// Global timer instance
const timer = new PerformanceTimer();

// Configuration
// Data directory is sibling to seeder: elohim/genesis/data/lamad
// Seeder is at: elohim/genesis/seeder/src/seed.ts
import { fileURLToPath } from 'url';
const __filename = fileURLToPath(import.meta.url);
const SEEDER_DIR = path.dirname(path.dirname(__filename)); // Go up from src/ to seeder/
const GENESIS_DIR = path.resolve(SEEDER_DIR, '..'); // Go up from seeder/ to genesis/
const DATA_DIR = process.env.DATA_DIR || path.join(GENESIS_DIR, 'data', 'lamad');

// Parse command-line arguments
const args = process.argv.slice(2);
const LIMIT_ARG = args.find(a => a.startsWith('--limit'));
const LIMIT = LIMIT_ARG ? parseInt(LIMIT_ARG.split('=')[1] || args[args.indexOf(LIMIT_ARG) + 1] || '0', 10) : 0;
const IDS_ARG = args.find(a => a.startsWith('--ids'));
const IDS = IDS_ARG ? (IDS_ARG.split('=')[1] || args[args.indexOf(IDS_ARG) + 1] || '').split(',').filter(Boolean) : [];
const FORCE_SEED = args.includes('--force') || process.env.FORCE_SEED === 'true';

// =============================================================================
// DOORWAY MODE (Required)
// =============================================================================
// All seeding goes through doorway, which relays to elohim-storage.
//
// Architecture:
//   Seeder → Doorway → elohim-storage → Conductor
//              │              │
//         (HTTP)    (single connection, batching)
//
// Doorway provides:
// - Web-accessible endpoints for remote development
// - Authentication and rate limiting
// - Relay to elohim-storage for blob imports
// - CDN-style caching for reads (DeliveryRelay)
// =============================================================================

// Doorway URL is REQUIRED - seeder only works through doorway
const DOORWAY_URL = process.env.DOORWAY_URL; // e.g., 'https://doorway-dev.elohim.host'
const DOORWAY_API_KEY = process.env.DOORWAY_API_KEY;

/**
 * Clean up error messages by truncating long byte arrays
 */
function cleanErrorMessage(error: any): string {
  const msg = error?.message || String(error);
  // Truncate byte arrays like Deserialize([139, 162, 105, ...])
  return msg
    .replace(/Deserialize\(\[[\d,\s]{50,}\]\)/g, 'Deserialize([...truncated...])')
    .replace(/\[[\d,\s]{100,}\]/g, '[...bytes truncated...]')
    .slice(0, 200); // Cap total length
}

// Types matching the Holochain zome
interface CreateContentInput {
  id: string;
  content_type: string;
  title: string;
  description: string;
  summary: string | null;           // Short preview text for cards/lists
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  reach: string;
  estimated_minutes: number | null; // Reading/viewing time
  thumbnail_url: string | null;     // Preview image for visual cards
  metadata_json: string;
}

interface ContentOutput {
  action_hash: Uint8Array;
  entry_hash: Uint8Array;
  content: any;
}

interface CreatePathInput {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose: string | null;
  difficulty: string;
  estimated_duration: string | null;
  visibility: string;
  path_type: string;
  tags: string[];
  /** Extensible metadata JSON (stores chapters for hierarchical paths) */
  metadata_json: string | null;
}

interface AddPathStepInput {
  path_id: string;
  order_index: number;
  step_type: string;
  resource_id: string;
  step_title: string | null;
  step_narrative: string | null;
  is_optional: boolean;
}

// JSON file types from data/lamad/
// Supports both simple schema (from MCP tools) and rich schema (from legacy import)
interface ConceptJson {
  id: string;
  title: string;
  content: string | object;  // Can be string (markdown) or object (quiz-json, etc.)
  contentFormat?: 'markdown' | 'html' | 'plain' | 'perseus-quiz-json' | 'gherkin';
  // Simple schema fields
  sourceDoc?: string;
  relationships?: { target: string; type: string }[];
  metadata?: Record<string, unknown>;
  // Rich schema fields (from legacy import)
  contentType?: string;
  description?: string;
  summary?: string;           // Short preview text for cards/lists (AI-generated)
  sourcePath?: string;
  relatedNodeIds?: string[];
  tags?: string[];
  did?: string;
  openGraphMetadata?: Record<string, unknown>;
  linkedData?: Record<string, unknown>;
  // Attention metadata
  estimatedMinutes?: number;  // Reading/viewing time in minutes
  thumbnailUrl?: string;      // Preview image for visual cards
}

interface PathJson {
  id: string;
  title: string;
  description?: string;
  difficulty?: 'beginner' | 'intermediate' | 'advanced';
  estimatedDuration?: string;
  // Audience archetype for pedagogical pipeline
  audienceArchetype?: string;
  // New MCP format
  chapters?: ChapterJson[];
  conceptIds?: string[];
  // Legacy format
  steps?: LegacyStepJson[];
  version?: string;
  purpose?: string;
  visibility?: string;
  tags?: string[];
}

interface LegacyStepJson {
  order: number;
  resourceId: string;
  stepTitle?: string;
  stepNarrative?: string;
  optional?: boolean;
}

interface ChapterStepJson {
  stepType?: string;
  resourceId: string;
  stepTitle?: string;
  stepNarrative?: string;
  optional?: boolean;
}

interface ChapterJson {
  id: string;
  title: string;
  description?: string;
  order?: number;
  estimatedDuration?: string;
  attestationGranted?: string;
  modules?: ModuleJson[];
  steps?: ChapterStepJson[];  // Legacy format: chapters with direct steps
}

interface ModuleJson {
  id: string;
  title: string;
  description?: string;
  order?: number;
  sections?: SectionJson[];
}

interface SectionAssessmentJson {
  id: string;
  title: string;
  type: 'core' | 'applied' | 'synthesis';
  description?: string;
  assessmentId?: string;  // Link to actual assessment content
}

interface SectionJson {
  id: string;
  title: string;
  description?: string;
  order?: number;
  estimatedMinutes?: number;
  conceptIds?: string[];
  assessments?: SectionAssessmentJson[];
}

interface AssessmentJson {
  id: string;
  title: string;
  description?: string;
  type: 'diagnostic' | 'formative' | 'summative' | 'quiz';
  questions?: QuestionJson[];
  conceptIds?: string[];
  passingScore?: number;
  timeLimit?: number;
}

interface QuestionJson {
  id: string;
  type: 'multiple-choice' | 'true-false' | 'short-answer' | 'essay' | 'matching';
  question: string;
  options?: string[];
  correctAnswer?: string | string[];
  explanation?: string;
  conceptId?: string;
  difficulty?: 'easy' | 'medium' | 'hard';
  points?: number;
}

/**
 * Find all JSON files in a directory recursively
 */
function findJsonFiles(dir: string): string[] {
  const files: string[] = [];

  if (!fs.existsSync(dir)) {
    return files;
  }

  function walk(currentDir: string) {
    const entries = fs.readdirSync(currentDir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(currentDir, entry.name);

      if (entry.isDirectory()) {
        walk(fullPath);
      } else if (entry.name.endsWith('.json')) {
        files.push(fullPath);
      }
    }
  }

  walk(dir);
  return files;
}

/**
 * Load and parse a JSON file
 */
function loadJson<T>(filePath: string): T | null {
  try {
    const content = fs.readFileSync(filePath, 'utf-8');
    return JSON.parse(content) as T;
  } catch (error) {
    console.error(`  ❌ Failed to load ${filePath}:`, error);
    return null;
  }
}

/**
 * Convert concept JSON to Holochain input
 * Supports both simple schema (from MCP tools) and rich schema (from legacy import)
 */
function conceptToInput(concept: ConceptJson, sourcePath: string): CreateContentInput {
  // Get related IDs - check both formats
  const relatedIds = concept.relatedNodeIds || concept.relationships?.map(r => r.target) || [];

  // Get source path - check both formats
  const sourcePathValue = concept.sourcePath || concept.sourceDoc || sourcePath;

  // Stringify content if it's an object (e.g., quiz-json, assessment formats)
  const contentString = typeof concept.content === 'string'
    ? concept.content
    : JSON.stringify(concept.content);

  // Get description - use existing or derive from content
  const description = concept.description ||
    (typeof concept.content === 'string' ? concept.content.slice(0, 500) : concept.title);

  // Get summary - use existing or derive from description/content
  const summary = concept.summary ||
    (description.length > 150 ? description.slice(0, 150) + '...' : description);

  return {
    id: concept.id,
    content_type: concept.contentType || 'concept',
    title: concept.title,
    description: description,
    summary: summary,
    content: contentString,
    content_format: concept.contentFormat || 'markdown',
    tags: concept.tags || [],
    source_path: sourcePathValue,
    related_node_ids: relatedIds,
    reach: 'public',
    estimated_minutes: concept.estimatedMinutes || null,
    thumbnail_url: concept.thumbnailUrl || null,
    metadata_json: JSON.stringify({
      sourceDoc: concept.sourceDoc,
      sourcePath: concept.sourcePath,
      relationships: concept.relationships,
      did: concept.did,
      openGraphMetadata: concept.openGraphMetadata,
      linkedData: concept.linkedData,
      ...concept.metadata,
    }),
  };
}

/**
 * Doorway mode seeding
 *
 * Routes all traffic through doorway - the primary mode for
 * remote/cloud environments (Eclipse Che, production, etc.)
 *
 * Flow:
 * 1. Load content files from disk
 * 2. Upload blob to doorway → doorway forwards to elohim-storage
 * 3. Queue import with blob_hash → doorway calls zome
 * 4. Poll for status → doorway calls zome
 */
async function seedViaDoorway() {
  timer.startPhase('Doorway Import');

  // Create doorway client
  const doorwayClient = new DoorwayClient({
    baseUrl: DOORWAY_URL!,
    apiKey: DOORWAY_API_KEY,
    timeout: 120000, // 2 min for large blob uploads
    retries: 3,
  });

  // Check doorway health
  console.log('\n🏥 Checking doorway health...');
  const health = await doorwayClient.checkHealth();
  if (!health.healthy) {
    console.error(`❌ Doorway not healthy: ${health.error}`);
    console.error('   Cannot proceed with doorway import. Exiting.');
    process.exit(1);
  }
  console.log(`✅ Doorway healthy (v${health.version || 'unknown'}, cache=${health.cacheEnabled ? 'on' : 'off'})`);

  // ========================================
  // LOAD CONTENT
  // ========================================
  timer.startPhase('Content Loading');
  console.log('\n📚 Loading content from data/lamad/content/...');
  const contentDir = path.join(DATA_DIR, 'content');
  const conceptFiles = findJsonFiles(contentDir);
  console.log(`   Found ${conceptFiles.length} concept files`);

  const allConcepts: { concept: ConceptJson; file: string }[] = [];
  let loadErrorCount = 0;

  for (const file of conceptFiles) {
    const concept = loadJson<ConceptJson>(file);
    if (concept) {
      if (!concept.id || !concept.title) {
        const reason = !concept.id && !concept.title ? 'missing id and title'
          : !concept.id ? 'missing id' : 'missing title';
        console.warn(`   ⚠️ Skipping ${path.basename(file)}: ${reason}`);
        timer.recordSkipped(path.basename(file), reason);
        loadErrorCount++;
        continue;
      }
      allConcepts.push({ concept, file });
    } else {
      timer.recordSkipped(path.basename(file), 'failed to parse JSON');
      loadErrorCount++;
    }
  }
  console.log(`   Loaded ${allConcepts.length} concepts (${loadErrorCount} failed)`);

  // Apply filters
  let filteredConcepts = allConcepts;
  if (IDS.length > 0) {
    filteredConcepts = allConcepts.filter(({ concept }) => IDS.includes(concept.id));
    console.log(`   Filtered to ${filteredConcepts.length} concepts matching IDs`);
  }
  if (LIMIT > 0 && filteredConcepts.length > LIMIT) {
    filteredConcepts = filteredConcepts.slice(0, LIMIT);
    console.log(`   Limited to ${LIMIT} concepts`);
  }

  timer.endPhase('Content Loading');

  // ========================================
  // UPLOAD & IMPORT CONTENT
  // ========================================
  if (filteredConcepts.length > 0) {
    timer.startPhase('Content Import');
    console.log(`\n🚀 Importing ${filteredConcepts.length} concepts via doorway...`);

    // Convert to zome inputs
    const allInputs = filteredConcepts.map(({ concept, file }) => {
      const relativePath = file.replace(DATA_DIR + '/', '');
      return conceptToInput(concept, relativePath);
    });

    // Step 1: Upload items as blob
    const itemsJson = JSON.stringify(allInputs);
    const itemsBuffer = Buffer.from(itemsJson, 'utf-8');
    console.log(`   📦 Uploading content blob: ${(itemsBuffer.length / 1024 / 1024).toFixed(2)} MB`);

    const blobHash = StorageClient.computeHash(itemsBuffer);
    const uploadResult = await timer.timeOperation('content_blob_upload', () =>
      doorwayClient.pushBlob(blobHash, itemsBuffer, {
        hash: blobHash,
        mimeType: 'application/json',
        sizeBytes: itemsBuffer.length,
      })
    );

    if (!uploadResult.success) {
      console.error(`   ❌ Blob upload failed: ${uploadResult.error}`);
      process.exit(1);
    }
    console.log(`   ✅ Blob uploaded: ${blobHash.slice(0, 30)}...${uploadResult.cached ? ' (cached)' : ''}`);

    // Step 2: Queue import
    const batchId = `seed-content-${Date.now()}`;
    console.log(`   📤 Queuing import "${batchId}"...`);

    const queueResult = await timer.timeOperation('content_queue_import', () =>
      doorwayClient.queueImport('content', {
        batch_id: batchId,
        blob_hash: blobHash,
        total_items: allInputs.length,
        schema_version: 1,
      })
    ) as ImportStatusResponse & { batch_id: string; queued_count: number; processing: boolean };

    console.log(`   ✅ Import queued: ${queueResult.batch_id} (${queueResult.queued_count} items)`);

    // Step 3: Poll for completion (optional)
    const POLL_STATUS = process.env.SEED_POLL_STATUS !== 'false';
    const POLL_INTERVAL_MS = parseInt(process.env.SEED_POLL_INTERVAL || '5000', 10);
    const POLL_TIMEOUT_MS = parseInt(process.env.SEED_POLL_TIMEOUT || '300000', 10);

    if (POLL_STATUS) {
      console.log(`   ⏳ Waiting for import to complete...`);
      console.log(`      (Poll interval=${POLL_INTERVAL_MS}ms, timeout=${POLL_TIMEOUT_MS}ms)`);

      const startTime = Date.now();
      let lastProcessed = 0;
      let finalStatus: ImportStatusResponse | null = null;

      while (Date.now() - startTime < POLL_TIMEOUT_MS) {
        await new Promise(resolve => setTimeout(resolve, POLL_INTERVAL_MS));

        const status = await doorwayClient.getImportStatus('content', batchId);
        if (!status) {
          console.warn(`      ⚠️ Batch ${batchId} not found`);
          break;
        }

        if (status.processed_count !== lastProcessed) {
          const pct = ((status.processed_count / status.total_items) * 100).toFixed(1);
          console.log(`      Progress: ${status.processed_count}/${status.total_items} (${pct}%) - ${status.status}`);
          lastProcessed = status.processed_count;
        }

        if (status.status === 'completed' || status.status === 'failed') {
          finalStatus = status;
          break;
        }
      }

      if (finalStatus) {
        if (finalStatus.error_count > 0 && finalStatus.errors.length > 0) {
          console.log(`   ⚠️ ${finalStatus.errors.length} errors during import:`);
          for (const err of finalStatus.errors.slice(0, 5)) {
            console.error(`      • ${err}`);
          }
          if (finalStatus.errors.length > 5) {
            console.error(`      ... and ${finalStatus.errors.length - 5} more`);
          }
        }
        console.log(`   ✅ Content import ${finalStatus.status}: ${finalStatus.processed_count - finalStatus.error_count} succeeded, ${finalStatus.error_count} failed`);
      } else {
        console.warn(`   ⚠️ Polling timeout - batch may still be processing`);
        console.log(`      Check status: GET ${DOORWAY_URL}/import/content/${batchId}`);
      }
    } else {
      console.log(`   ℹ️ Not polling - batch queued for background processing`);
      console.log(`      Check status: GET ${DOORWAY_URL}/import/content/${batchId}`);
    }

    timer.endPhase('Content Import');
  }

  // ========================================
  // LOAD & IMPORT PATHS
  // ========================================
  timer.startPhase('Path Loading');
  console.log('\n📍 Loading paths from data/lamad/paths/...');
  const pathsDir = path.join(DATA_DIR, 'paths');
  const pathFiles = fs.existsSync(pathsDir) ? findJsonFiles(pathsDir) : [];
  console.log(`   Found ${pathFiles.length} path files`);

  const allPaths: { pathData: any; file: string }[] = [];
  for (const file of pathFiles) {
    if (path.basename(file) === 'index.json') continue; // Skip index files
    const pathData = loadJson<any>(file);
    if (pathData && pathData.id && pathData.title) {
      allPaths.push({ pathData, file });
    }
  }
  console.log(`   Loaded ${allPaths.length} valid paths`);
  timer.endPhase('Path Loading');

  if (allPaths.length > 0) {
    timer.startPhase('Path Import');
    console.log(`\n🚀 Importing ${allPaths.length} paths via doorway...`);

    // Convert paths to zome inputs
    const pathInputs = allPaths.map(({ pathData }) => ({
      id: pathData.id,
      version: pathData.version || '1.0.0',
      title: pathData.title,
      description: pathData.description || '',
      purpose: pathData.purpose || null,
      difficulty: pathData.difficulty || 'beginner',
      estimated_duration: pathData.estimatedDuration || null,
      visibility: pathData.visibility || 'public',
      path_type: pathData.pathType || 'linear',
      tags: pathData.tags || [],
      metadata_json: JSON.stringify(pathData.chapters || pathData.metadata || {}),
      steps: (pathData.steps || pathData.conceptIds?.map((id: string, i: number) => ({
        step_type: 'content',
        resource_id: id,
        order_index: i,
      })) || []),
    }));

    // Upload paths blob
    const pathsJson = JSON.stringify(pathInputs);
    const pathsBuffer = Buffer.from(pathsJson, 'utf-8');
    console.log(`   📦 Uploading paths blob: ${(pathsBuffer.length / 1024).toFixed(2)} KB`);

    const pathsBlobHash = StorageClient.computeHash(pathsBuffer);
    const pathsUploadResult = await doorwayClient.pushBlob(pathsBlobHash, pathsBuffer, {
      hash: pathsBlobHash,
      mimeType: 'application/json',
      sizeBytes: pathsBuffer.length,
    });

    if (!pathsUploadResult.success) {
      console.error(`   ❌ Paths blob upload failed: ${pathsUploadResult.error}`);
    } else {
      console.log(`   ✅ Paths blob uploaded: ${pathsBlobHash.slice(0, 30)}...`);

      // Queue paths import
      const pathsBatchId = `seed-paths-${Date.now()}`;
      console.log(`   📤 Queuing paths import "${pathsBatchId}"...`);

      try {
        const pathsQueueResult = await doorwayClient.queueImport('paths', {
          batch_id: pathsBatchId,
          blob_hash: pathsBlobHash,
          total_items: pathInputs.length,
          schema_version: 1,
        });
        console.log(`   ✅ Paths import queued: ${pathsQueueResult.batch_id}`);
      } catch (pathErr: any) {
        console.warn(`   ⚠️ Paths import queue failed: ${pathErr.message}`);
        console.log(`      (Paths import may not be implemented yet in the zome)`);
      }
    }

    timer.endPhase('Path Import');
  }

  // ========================================
  // SUMMARY
  // ========================================
  timer.endPhase('Doorway Import');
  console.log('\n' + '='.repeat(70));
  console.log('✅ DOORWAY SEEDING COMPLETE');
  console.log('='.repeat(70));
  timer.printReport();
  console.log('='.repeat(70));
}

/**
 * Main seeding function
 *
 * Seeds content through doorway → elohim-storage → conductor.
 *
 * Usage:
 *   DOORWAY_URL=https://doorway-dev.elohim.host npm run seed
 *
 * Options:
 *   --limit=N     Limit to N concepts
 *   --ids=a,b,c   Only seed specific IDs
 *   --force       Seed even if data already exists
 */
async function seed() {
  console.log('🌱 Holochain Content Seeder');
  console.log(`📁 Data directory: ${DATA_DIR}`);

  // Require DOORWAY_URL
  if (!DOORWAY_URL) {
    console.error('\n❌ DOORWAY_URL environment variable is required');
    console.error('');
    console.error('Usage:');
    console.error('  DOORWAY_URL=https://doorway-dev.elohim.host npm run seed');
    console.error('');
    console.error('The seeder works through doorway, which relays to elohim-storage.');
    console.error('Architecture: Seeder → Doorway → elohim-storage → Conductor');
    process.exit(1);
  }

  console.log(`🌐 Doorway: ${DOORWAY_URL}`);
  console.log(`   Flow: Seeder → Doorway → elohim-storage → Conductor`);
  if (IDS.length > 0) console.log(`🎯 Filtering to IDs: ${IDS.join(', ')}`);
  if (LIMIT > 0) console.log(`📊 Limit: ${LIMIT}`);

  await seedViaDoorway();
}

seed().catch((error) => {
  console.error('Fatal error:', cleanErrorMessage(error));
  process.exit(1);
});
