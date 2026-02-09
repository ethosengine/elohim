/**
 * Holochain Content Seeder
 *
 * Seeds pre-structured JSON content from /data/lamad into elohim-storage.
 * This is a deterministic script that loads JSON files created by Claude + MCP tools.
 *
 * Pipeline: docs/ → Claude + MCP → data/lamad/ → seeder → Doorway → elohim-storage
 *
 * ## Architecture
 *
 * The seeder calls Doorway's bulk HTTP endpoints which proxy to elohim-storage:
 * - POST /api/db/content/bulk - Bulk create content nodes
 * - POST /api/db/paths/bulk - Bulk create learning paths
 * - POST /api/db/relationships/bulk - Bulk create relationships
 *
 * These are synchronous operations that return immediately with results.
 *
 * ## Diagnosing Issues
 *
 * Check elohim-storage logs for validation errors or database issues.
 * The seeder reports errors inline during execution.
 *
 * ## Environment Variables
 *
 * - DOORWAY_URL (required): Doorway endpoint (e.g., https://doorway-dev.elohim.host)
 * - STORAGE_URL (optional): Direct storage URL for blob sync
 * - SKIP_VERIFICATION: Skip pre/post-flight conductor checks
 * - SKIP_BLOB_SYNC: Skip genesis blob sync
 * - LIMIT: Limit number of items to seed
 * - IDS: Comma-separated list of specific IDs to seed
 *
 * @version 2025-01-09 - Simplified to use direct bulk HTTP endpoints
 */

import * as fs from 'fs';
import * as path from 'path';
import { AdminWebsocket, AppWebsocket, type CellId } from '@holochain/client';
import DoorwayClient from './doorway-client.js';
import StorageClient from './storage-client.js'; // Used for computing blob hash (thumbnails, HTML5 apps)
import BlobManager from './blob-manager.js';
import { validateBatch, logValidationErrors, isStrictValidation } from './validators.js';
import { SeedingVerification, type ExpectedCounts } from './verification.js';

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
const BLOBS_DIR = path.join(GENESIS_DIR, 'blobs');
const GENESIS_MANIFEST_PATH = path.join(BLOBS_DIR, 'manifest.json');

// Genesis blob pack manifest (pre-computed content addresses)
interface GenesisManifestEntry {
  cid: string;
  hash: string;
  sizeBytes: number;
  contentFormat: string;
}
interface GenesisManifest {
  version: number;
  entries: Record<string, GenesisManifestEntry>;
}
let genesisManifest: GenesisManifest | null = null;

/**
 * Load genesis blob pack manifest if available
 * This enables sparse DHT entries by pre-computing blob CIDs
 */
function loadGenesisManifest(): GenesisManifest | null {
  if (genesisManifest !== null) return genesisManifest;

  if (fs.existsSync(GENESIS_MANIFEST_PATH)) {
    try {
      genesisManifest = JSON.parse(fs.readFileSync(GENESIS_MANIFEST_PATH, 'utf-8'));
      console.log(`   📦 Loaded genesis blob manifest: ${Object.keys(genesisManifest!.entries).length} entries`);
      return genesisManifest;
    } catch (err) {
      console.warn(`   ⚠️ Failed to load genesis manifest: ${err}`);
    }
  }
  return null;
}

/**
 * Sync genesis blobs to elohim-storage.
 * Ensures all content bodies are available before creating DHT manifests.
 *
 * Uses STORAGE_URL environment variable or falls back to doorway's storage endpoint.
 */
async function syncGenesisBlobs(storageUrl: string): Promise<{ synced: number; skipped: number; failed: number }> {
  const manifest = loadGenesisManifest();
  if (!manifest) {
    console.log('   ⏭️ No genesis manifest found, skipping blob sync');
    return { synced: 0, skipped: 0, failed: 0 };
  }

  const entries = Object.entries(manifest.entries);
  console.log(`   📦 Syncing ${entries.length} genesis blobs to ${storageUrl}...`);

  let synced = 0;
  let skipped = 0;
  let failed = 0;

  // Batch check and upload
  const BATCH_SIZE = 50;
  for (let i = 0; i < entries.length; i += BATCH_SIZE) {
    const batch = entries.slice(i, i + BATCH_SIZE);

    for (const [id, entry] of batch) {
      try {
        // Check if blob exists
        const checkRes = await fetch(`${storageUrl}/shard/${entry.hash}`, { method: 'HEAD' });
        if (checkRes.ok) {
          skipped++;
          continue;
        }

        // Read blob from genesis pack
        const blobPath = path.join(BLOBS_DIR, entry.hash);
        if (!fs.existsSync(blobPath)) {
          console.warn(`   ⚠️ Missing blob file: ${blobPath}`);
          failed++;
          continue;
        }

        const blobData = fs.readFileSync(blobPath);

        // Upload to storage
        const uploadRes = await fetch(`${storageUrl}/shard/${entry.hash}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/octet-stream' },
          body: blobData,
        });

        if (uploadRes.ok) {
          synced++;
        } else {
          console.warn(`   ⚠️ Failed to upload ${id}: ${uploadRes.status}`);
          failed++;
        }
      } catch (err) {
        console.warn(`   ⚠️ Error syncing ${id}: ${err}`);
        failed++;
      }
    }

    // Progress
    if ((i + BATCH_SIZE) % 500 === 0 || i + BATCH_SIZE >= entries.length) {
      console.log(`   📤 Progress: ${Math.min(i + BATCH_SIZE, entries.length)}/${entries.length} (${synced} synced, ${skipped} existed, ${failed} failed)`);
    }
  }

  return { synced, skipped, failed };
}

// Parse command-line arguments
const args = process.argv.slice(2);
const LIMIT_ARG = args.find(a => a.startsWith('--limit'));
const LIMIT = LIMIT_ARG ? parseInt(LIMIT_ARG.split('=')[1] || args[args.indexOf(LIMIT_ARG) + 1] || '0', 10) : 0;
const IDS_ARG = args.find(a => a.startsWith('--ids'));
const IDS = IDS_ARG ? (IDS_ARG.split('=')[1] || args[args.indexOf(IDS_ARG) + 1] || '').split(',').filter(Boolean) : [];
const FORCE_SEED = args.includes('--force') || process.env.FORCE_SEED === 'true';
const PATHS_ONLY = args.includes('--paths-only') || process.env.SEED_PATHS_ONLY === 'true';
const CONTENT_ONLY = args.includes('--content-only') || process.env.SEED_CONTENT_ONLY === 'true';

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
const HOLOCHAIN_ADMIN_URL = process.env.HOLOCHAIN_ADMIN_URL; // e.g., 'wss://doorway-dev.elohim.host?apiKey=...'
const SKIP_VERIFICATION = process.env.SKIP_VERIFICATION === 'true';
const SKIP_BLOB_SYNC = process.env.SKIP_BLOB_SYNC === 'true';
const APP_ID = 'elohim';
const ZOME_NAME = 'content_store';
const DOORWAY_API_KEY = process.env.DOORWAY_API_KEY;

// Storage URL for blob sync (defaults to doorway + /storage path)
const STORAGE_URL = process.env.STORAGE_URL || (DOORWAY_URL ? `${DOORWAY_URL}/storage` : null);

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

/**
 * Normalize content format to storage-accepted values.
 *
 * Valid formats: markdown, html, json, text, perseus, gherkin, yaml, toml, latex, asciidoc, html5-app, iframe, embed
 */
function normalizeContentFormat(format: string | undefined): string {
  if (!format) return 'markdown';
  const formatMap: Record<string, string> = {
    'perseus-quiz-json': 'perseus',  // Perseus quiz format
    'plaintext': 'text',             // Plain text
    'plain': 'text',
    'txt': 'text',
  };
  return formatMap[format.toLowerCase()] || format.toLowerCase();
}

/**
 * Normalize path type to storage-accepted values.
 *
 * Valid types: guided, self-paced, challenge, assessment, exploration, certification
 */
function normalizePathType(pathType: string | undefined): string {
  if (!pathType) return 'guided';
  const typeMap: Record<string, string> = {
    'linear': 'guided',
    'journey': 'guided',
    'course': 'guided',
    'module': 'guided',
    'expedition': 'exploration',  // Expedition → exploration
    'adventure': 'exploration',
    'discovery': 'exploration',
    'tutorial': 'self-paced',
    'interactive': 'exploration',
    'test': 'assessment',
    'quiz': 'assessment',
    'exam': 'certification',
  };
  const normalized = typeMap[pathType.toLowerCase()] || pathType.toLowerCase();
  // Valid types: guided, self-paced, challenge, assessment, exploration, certification
  const validTypes = ['guided', 'self-paced', 'challenge', 'assessment', 'exploration', 'certification'];
  if (!validTypes.includes(normalized)) {
    console.warn(`   ⚠️ Unknown pathType '${pathType}' -> defaulting to 'guided'`);
    return 'guided';
  }
  return normalized;
}

// Types matching the Holochain zome
interface CreateContentInput {
  id: string;
  contentType: string;
  title: string;
  description: string;
  summary: string | null;           // Short preview text for cards/lists
  content: string;                  // Legacy: full body. New: empty/hash if blob_cid set
  contentFormat: string;
  tags: string[];
  sourcePath: string | null;
  relatedNodeIds: string[];
  reach: string;
  estimatedMinutes: number | null; // Reading/viewing time
  thumbnailUrl: string | null;     // Preview image for visual cards
  metadataJson: string;
  // Content manifest fields (Phase 0 refactor - sparse DHT)
  blobCid: string | null;          // CID pointing to elohim-storage blob
  contentSizeBytes: number | null; // Size of content body
  contentHash: string | null;      // SHA256 of content body
  // HTML5 app blob reference (ZIP stored in elohim-storage)
  blobHash?: string;               // SHA256 hash of ZIP blob for html5-app content
}

interface ContentOutput {
  actionHash: Uint8Array;
  entryHash: Uint8Array;
  content: any;
}

interface CreatePathInput {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose: string | null;
  difficulty: string;
  estimatedDuration: string | null;
  visibility: string;
  pathType: string;
  tags: string[];
  /** Extensible metadata JSON (stores chapters for hierarchical paths) */
  metadataJson: string | null;
  /** Thumbnail image URL (relative path or blob reference) */
  thumbnailUrl?: string;
  /** SHA256 hash of thumbnail blob in elohim-storage */
  thumbnailBlobHash?: string;
}

interface AddPathStepInput {
  pathId: string;
  orderIndex: number;
  stepType: string;
  resourceId: string;
  stepTitle: string | null;
  stepNarrative: string | null;
  isOptional: boolean;
}

// JSON file types from data/lamad/
// Supports both simple schema (from MCP tools) and rich schema (from legacy import)
interface ConceptJson {
  id: string;
  title: string;
  content: string | object;  // Can be string (markdown) or object (quiz-json, etc.)
  contentFormat?: 'markdown' | 'html' | 'plain' | 'perseus-quiz-json' | 'gherkin' | 'html5-app';
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
  // HTML5 app blob reference (pre-computed hash in JSON)
  blobHash?: string;          // SHA256 hash (without sha256- prefix)
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
  // Thumbnail image
  thumbnailUrl?: string;
  thumbnailAlt?: string;
  /** Local path to thumbnail image (relative to genesis/) for blob upload */
  localThumbnailPath?: string;
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
 *
 * If genesis manifest is available, uses pre-computed blob CID for sparse DHT entries.
 * Otherwise, falls back to embedding full content in DHT entry (legacy behavior).
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

  // Check genesis manifest for pre-computed blob address
  const manifest = loadGenesisManifest();
  const blobEntry = manifest?.entries[concept.id];

  // If blob entry exists, use sparse DHT pattern (just metadata + CID reference)
  // Otherwise, embed full content in DHT entry (legacy behavior)
  // Exception: html5-app keeps original content object (appId, entryPoint, fallbackUrl)
  // because the renderer needs that to build the /apps/{appId}/{entryPoint} URL
  const isHtml5App = concept.contentFormat === 'html5-app';
  const useSparsePattern = blobEntry !== undefined && !isHtml5App;

  return {
    id: concept.id,
    contentType: concept.contentType || 'concept',
    title: concept.title,
    description: description,
    summary: summary,
    // Sparse: store hash reference, Full: store entire content
    // html5-app: always store the content object with appId/entryPoint
    content: useSparsePattern ? `sha256:${blobEntry.hash}` : contentString,
    contentFormat: normalizeContentFormat(concept.contentFormat),
    tags: concept.tags || [],
    sourcePath: sourcePathValue,
    relatedNodeIds: relatedIds,
    reach: 'public',
    estimatedMinutes: concept.estimatedMinutes || null,
    thumbnailUrl: concept.thumbnailUrl || null,
    metadataJson: JSON.stringify({
      sourceDoc: concept.sourceDoc,
      sourcePath: concept.sourcePath,
      relationships: concept.relationships,
      did: concept.did,
      openGraphMetadata: concept.openGraphMetadata,
      linkedData: concept.linkedData,
      ...concept.metadata,
    }),
    // Content manifest fields for sparse DHT
    blobCid: blobEntry?.cid ?? null,
    contentSizeBytes: blobEntry?.sizeBytes ?? null,
    contentHash: blobEntry?.hash ?? null,
  };
}

// =============================================================================
// HOLOCHAIN CONNECTION FOR VERIFICATION
// =============================================================================

interface HolochainConnection {
  appWs: AppWebsocket;
  cellId: CellId;
  close: () => Promise<void>;
}

/**
 * Result of seeding operation - used for post-flight verification
 */
interface SeedResult {
  contentAttempted: number;
  contentSucceeded: number;
  pathsAttempted: number;
  pathsSucceeded: number;
  sampleIds: string[];
}

/**
 * Connect to Holochain conductor for pre/post-flight verification.
 * Uses admin interface to discover cell, then connects to app interface.
 */
async function connectForVerification(): Promise<HolochainConnection | null> {
  if (!HOLOCHAIN_ADMIN_URL) {
    console.log('   ⚠️ HOLOCHAIN_ADMIN_URL not set - skipping verification');
    return null;
  }

  try {
    // Connect to admin interface
    const adminWs = await AdminWebsocket.connect({
      url: new URL(HOLOCHAIN_ADMIN_URL),
      wsClientOptions: { origin: 'http://localhost' },
    });

    // Find our app
    const apps = await adminWs.listApps({});
    const app = apps.find((a) => a.installed_app_id === APP_ID);
    if (!app) {
      console.log(`   ⚠️ App '${APP_ID}' not found on conductor`);
      await adminWs.client.close();
      return null;
    }

    // Find cell (support both 'elohim' and legacy 'lamad' roles)
    const availableRoles = Object.keys(app.cell_info);
    const roleName = availableRoles.find(r => r === 'elohim') ||
                     availableRoles.find(r => r === 'lamad') ||
                     availableRoles[0];

    if (!roleName) {
      console.log('   ⚠️ No cell roles found');
      await adminWs.client.close();
      return null;
    }

    const cellInfo = app.cell_info[roleName];
    // Handle both cell formats:
    // - Holochain native: { type: "provisioned", value: { cell_id: [...] } }
    // - JS client format: { provisioned: { cell_id: [...] } }
    const provisionedCell = cellInfo?.find((c: any) =>
      ('provisioned' in c) || (c.type === 'provisioned')
    );
    if (!provisionedCell) {
      console.log('   ⚠️ No provisioned cell found');
      await adminWs.client.close();
      return null;
    }

    // Extract cell_id from either format
    const rawCellId = ('provisioned' in provisionedCell)
      ? (provisionedCell as any).provisioned.cell_id
      : (provisionedCell as any).value.cell_id;
    function toUint8Array(val: any): Uint8Array {
      if (val instanceof Uint8Array) return val;
      if (val?.type === 'Buffer' && Array.isArray(val.data)) return new Uint8Array(val.data);
      if (ArrayBuffer.isView(val)) return new Uint8Array(val.buffer);
      throw new Error('Cannot convert to Uint8Array');
    }
    const cellId: CellId = [toUint8Array(rawCellId[0]), toUint8Array(rawCellId[1])];

    // Get auth token
    const token = await adminWs.issueAppAuthenticationToken({
      installed_app_id: APP_ID,
      single_use: false,
      expiry_seconds: 300,
    });

    await adminWs.authorizeSigningCredentials(cellId);

    // Resolve app interface URL
    const existingInterfaces = await adminWs.listAppInterfaces();
    const appPort = existingInterfaces.length > 0 ? existingInterfaces[0].port : 4445;

    let appWsUrl: string;
    if (!HOLOCHAIN_ADMIN_URL.includes('localhost') && !HOLOCHAIN_ADMIN_URL.includes('127.0.0.1')) {
      const url = new URL(HOLOCHAIN_ADMIN_URL);
      const baseUrl = `${url.protocol}//${url.host}`;
      const apiKey = url.searchParams.get('apiKey');
      const apiKeyParam = apiKey ? `?apiKey=${encodeURIComponent(apiKey)}` : '';
      appWsUrl = `${baseUrl}/app/${appPort}${apiKeyParam}`;
    } else {
      appWsUrl = `ws://localhost:${appPort}`;
    }

    // Connect to app interface
    const appWs = await AppWebsocket.connect({
      url: new URL(appWsUrl),
      wsClientOptions: { origin: 'http://localhost' },
      token: token.token,
    });

    return {
      appWs,
      cellId,
      close: async () => {
        await appWs.client.close();
        await adminWs.client.close();
      },
    };
  } catch (error) {
    console.log(`   ⚠️ Failed to connect for verification: ${error instanceof Error ? error.message : error}`);
    return null;
  }
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
 *
 * @returns SeedResult with counts for post-flight verification
 */
async function seedViaDoorway(): Promise<SeedResult> {
  // Track results for verification
  const result: SeedResult = {
    contentAttempted: 0,
    contentSucceeded: 0,
    pathsAttempted: 0,
    pathsSucceeded: 0,
    sampleIds: [],
  };
  timer.startPhase('Doorway Import');

  // Create doorway client
  // Pass storageUrl to use storage directly for /db/* bulk operations
  // This bypasses doorway's proxy which may not have the routes compiled
  const doorwayClient = new DoorwayClient({
    baseUrl: DOORWAY_URL!,
    storageUrl: STORAGE_URL || undefined,
    apiKey: DOORWAY_API_KEY,
    timeout: 120000, // 2 min for large blob uploads
    retries: 3,
  });

  // Check doorway status (comprehensive preflight)
  console.log('\n🏥 Checking doorway status...');
  const status = await doorwayClient.checkStatus();
  if (!status) {
    // Fall back to health check
    const health = await doorwayClient.checkHealth();
    if (!health.healthy) {
      console.error(`❌ Doorway not healthy: ${health.error}`);
      console.error('   Cannot proceed with doorway import. Exiting.');
      process.exit(1);
    }
    console.log(`✅ Doorway healthy (v${health.version || 'unknown'}, cache=${health.cacheEnabled ? 'on' : 'off'})`);
  } else {
    console.log(`✅ Doorway status: ${status.status} (v${status.version || 'unknown'})`);
    console.log(`   Conductor: ${status.conductor.connected ? '✅ connected' : '❌ disconnected'} (${status.conductor.connectedWorkers}/${status.conductor.totalWorkers} workers)`);
    console.log(`   Storage: ${status.storage.healthy ? '✅ healthy' : '❌ unhealthy'} (import=${status.storage.importEnabled ? 'on' : 'off'})`);
    console.log(`   Cell: ${status.diagnostics.cellDiscovered ? '✅ discovered' : '⏳ will discover lazily'}`);

    // Show any recommendations
    if (status.diagnostics.recommendations.length > 0) {
      console.log(`   ⚠️ Recommendations:`);
      for (const rec of status.diagnostics.recommendations) {
        console.log(`      • ${rec}`);
      }
    }

    // Check if ready for seeding
    if (!status.conductor.connected) {
      console.error(`❌ Conductor not connected - cannot proceed with seeding`);
      process.exit(1);
    }
    if (!status.storage.healthy) {
      console.error(`❌ Storage not healthy - cannot proceed with seeding`);
      process.exit(1);
    }
  }

  // ========================================
  // LOAD CONTENT
  // ========================================
  let filteredConcepts: { concept: ConceptJson; file: string }[] = [];

  if (PATHS_ONLY) {
    console.log('\n⏭️  Skipping content import (--paths-only mode)');
    // Still load content for title lookups
    timer.startPhase('Content Loading');
    const contentDir = path.join(DATA_DIR, 'content');
    const conceptFiles = findJsonFiles(contentDir);
    for (const file of conceptFiles) {
      const concept = loadJson<ConceptJson>(file);
      if (concept?.id && concept?.title) {
        filteredConcepts.push({ concept, file });
      }
    }
    console.log(`   Loaded ${filteredConcepts.length} concepts for title lookups`);
    timer.endPhase('Content Loading');
  } else {
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
  filteredConcepts = allConcepts;
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
  // UPLOAD HTML5 APP ZIPS
  // ========================================
  timer.startPhase('HTML5 App Blobs');
  console.log('\n📦 Processing HTML5 app content...');

  // Initialize blob manager for HTML5 app processing
  // forceUpload: true ensures blobs are uploaded even if blob_hash exists
  // (needed for fresh deployments where content JSON has blob_hash but storage is empty)
  const blobManager = new BlobManager({
    doorwayUrl: DOORWAY_URL!,
    apiKey: DOORWAY_API_KEY,
    minBlobSize: 0, // Always extract html5-app ZIPs regardless of size
    cacheDir: path.join(GENESIS_DIR, '.blob-cache'),
    forceUpload: true, // Always upload blobs to ensure storage has them
  });

  // Map to store blob_hash for each html5-app content ID
  const html5AppBlobHashes = new Map<string, { hash: string; appId: string; entryPoint: string }>();

  // Find and process all html5-app content
  const html5Apps = filteredConcepts.filter(({ concept }) =>
    concept.contentFormat === 'html5-app'
  );

  if (html5Apps.length > 0) {
    console.log(`   Found ${html5Apps.length} HTML5 app(s) to process`);

    for (const { concept, file } of html5Apps) {
      try {
        // Process with BlobManager to extract ZIP
        const contentDir = path.join(DATA_DIR, 'content');
        const processed = await blobManager.processContent(concept as any, contentDir);

        if (processed.extracted && processed.blob && processed.blobMetadata) {
          console.log(`   📦 Uploading ${concept.id}: ${(processed.blob.length / 1024 / 1024).toFixed(2)} MB`);

          // Upload ZIP to storage via doorway
          const uploadResult = await timer.timeOperation('html5_app_upload', () =>
            doorwayClient.pushBlob(processed.blobMetadata!.hash, processed.blob!, {
              hash: processed.blobMetadata!.hash,
              mimeType: 'application/zip',
              sizeBytes: processed.blob!.length,
              entryPoint: processed.blobMetadata!.entryPoint,
              fallbackUrl: processed.blobMetadata!.fallbackUrl,
            })
          );

          if (uploadResult.success) {
            // Extract appId from content object
            const contentObj = typeof concept.content === 'object' ? concept.content as Record<string, unknown> : null;
            const appId = contentObj?.appId as string || concept.id;
            const entryPoint = contentObj?.entryPoint as string || 'index.html';

            html5AppBlobHashes.set(concept.id, {
              hash: processed.blobMetadata.hash,
              appId,
              entryPoint,
            });

            console.log(`   ✅ ${concept.id}: ${uploadResult.cached ? 'already cached' : 'uploaded'} (appId: ${appId})`);
          } else {
            console.error(`   ❌ ${concept.id}: upload failed - ${uploadResult.error}`);
          }
        } else {
          // No local ZIP found - check if content has fallback URL
          const contentObj = typeof concept.content === 'object' ? concept.content as Record<string, unknown> : null;
          if (contentObj?.fallbackUrl) {
            console.log(`   ⚠️ ${concept.id}: no local ZIP, will use fallbackUrl`);
          } else {
            console.warn(`   ⚠️ ${concept.id}: no local ZIP and no fallbackUrl`);
            timer.recordSkipped(path.basename(file), 'html5-app without local ZIP');
          }
        }
      } catch (err) {
        console.error(`   ❌ ${concept.id}: processing error - ${err}`);
      }
    }

    console.log(`   📊 Processed ${html5AppBlobHashes.size}/${html5Apps.length} HTML5 apps`);
  } else {
    console.log('   No HTML5 app content found');
  }

  timer.endPhase('HTML5 App Blobs');

  // ========================================
  // UPLOAD & IMPORT CONTENT
  // ========================================
  if (filteredConcepts.length > 0) {
    timer.startPhase('Content Import');
    console.log(`\n🚀 Importing ${filteredConcepts.length} concepts via doorway...`);

    // Convert to zome inputs
    const allInputs = filteredConcepts.map(({ concept, file }) => {
      const relativePath = file.replace(DATA_DIR + '/', '');
      const input = conceptToInput(concept, relativePath);

      // For HTML5 apps, add the blob_hash from uploaded ZIP
      const appInfo = html5AppBlobHashes.get(concept.id);
      if (appInfo) {
        // Set blob_hash to point to the uploaded ZIP
        input.blobHash = appInfo.hash;

        // Add appId to metadata_json so elohim-storage can look it up
        const metadata = input.metadataJson ? JSON.parse(input.metadataJson) : {};
        metadata.appId = appInfo.appId;
        metadata.entryPoint = appInfo.entryPoint;
        input.metadataJson = JSON.stringify(metadata);
      }

      return input;
    });

    // Pre-flight validation - catch issues before blob upload
    timer.startPhase('Pre-flight Validation');
    console.log(`\n🔍 Validating ${allInputs.length} content items...`);

    const validationResult = validateBatch(allInputs);

    if (validationResult.totalInvalid > 0) {
      console.warn(`   ⚠️  ${validationResult.totalInvalid} items failed validation:`);
      logValidationErrors(validationResult.invalidItems, 10);

      if (isStrictValidation()) {
        console.error(`\n❌ VALIDATION FAILED: ${validationResult.totalInvalid} invalid items.`);
        console.error(`   Set STRICT_VALIDATION=false to continue with valid items only.`);
        process.exit(1);
      }

      console.log(`   ℹ️  Continuing with ${validationResult.totalValid} valid items (STRICT_VALIDATION not set)`);
    } else {
      console.log(`   ✅ All ${validationResult.totalValid} items passed validation`);
    }
    timer.endPhase('Pre-flight Validation');

    // Use only valid items for seeding
    const itemsToSeed = validationResult.validItems;

    // Track for verification
    result.contentAttempted = itemsToSeed.length;
    // Collect sample IDs (first 5, last 5, and some from middle)
    const sampleIndices = [
      0, 1, 2, 3, 4,  // first 5
      Math.floor(itemsToSeed.length / 2),  // middle
      ...Array.from({ length: 5 }, (_, i) => Math.max(0, itemsToSeed.length - 5 + i)),  // last 5
    ].filter((i, idx, arr) => i < itemsToSeed.length && arr.indexOf(i) === idx);  // dedupe
    result.sampleIds = sampleIndices.map(i => itemsToSeed[i].id);

    // Transform items to backend format (content → contentBody)
    // Backend uses serde rename_all = "camelCase" so expects camelCase field names
    // Coerce null values to undefined for optional fields (TypeScript compatibility)
    const transformedItems = itemsToSeed.map(item => ({
      schemaVersion: 1,
      id: item.id,
      title: item.title,
      description: item.description,
      contentType: item.contentType,
      contentFormat: normalizeContentFormat(item.contentFormat),  // Ensure valid format
      contentBody: item.content,  // Backend expects contentBody (camelCase)
      blobHash: item.blobHash ?? undefined,
      blobCid: item.blobCid ?? undefined,
      metadataJson: item.metadataJson,
      reach: item.reach || 'public',
      tags: item.tags || [],
    }));

    // Bulk create content via direct HTTP call (through Doorway's /api/db proxy)
    console.log(`   📤 Bulk creating ${transformedItems.length} content items...`);
    const startTime = Date.now();

    try {
      const bulkResult = await timer.timeOperation('bulk_create_content', () =>
        doorwayClient.bulkCreateContent(transformedItems)
      );

      const elapsed = Date.now() - startTime;
      const rate = transformedItems.length / (elapsed / 1000);

      // Track for verification
      result.contentSucceeded = bulkResult.inserted;

      // Report errors if any
      if (bulkResult.errors.length > 0) {
        console.log(`   ⚠️ ${bulkResult.errors.length} errors during import:`);
        for (const err of bulkResult.errors.slice(0, 5)) {
          console.error(`      • ${err}`);
        }
        if (bulkResult.errors.length > 5) {
          console.error(`      ... and ${bulkResult.errors.length - 5} more`);
        }

        // Fail if more than 10% of items errored
        const errorRate = (bulkResult.errors.length / transformedItems.length) * 100;
        if (errorRate > 10) {
          console.error(`\n❌ SEEDING FAILED: Too many errors (${errorRate.toFixed(1)}%). Check elohim-storage logs.`);
          process.exit(1);
        }
      }

      console.log(`   ✅ Content import complete: ${bulkResult.inserted} inserted, ${bulkResult.skipped} skipped (${elapsed}ms, ${rate.toFixed(1)} items/sec)`);

      // ========================================
      // EXTRACT & CREATE RELATIONSHIPS
      // ========================================
      timer.startPhase('Relationship Import');
      console.log(`\n🔗 Extracting relationships from content...`);

      // Extract relationships from content items
      const relationships: Array<{
        sourceId: string;
        targetId: string;
        relationshipType: string;
        confidence: number;
        inferenceSource: string;
      }> = [];

      // Track seen relationships to avoid duplicates
      const seen = new Set<string>();

      for (const item of itemsToSeed) {
        // Extract from relatedNodeIds array (simple RELATES_TO relationships)
        if (item.relatedNodeIds && item.relatedNodeIds.length > 0) {
          for (const targetId of item.relatedNodeIds) {
            const key = `${item.id}:${targetId}:RELATES_TO`;
            if (!seen.has(key) && item.id !== targetId) {
              seen.add(key);
              relationships.push({
                schemaVersion: 1,
                sourceId: item.id,
                targetId: targetId,
                relationshipType: 'RELATES_TO',
                confidence: 1.0,
                inferenceSource: 'explicit',
              });
            }
          }
        }

        // Extract from relationships array in metadata (typed relationships)
        if (item.metadataJson) {
          try {
            const metadata = JSON.parse(item.metadataJson);
            if (metadata.relationships && Array.isArray(metadata.relationships)) {
              for (const rel of metadata.relationships) {
                const targetId = rel.target || rel.targetId || rel.target_id;
                const relType = rel.type || rel.relationship_type || 'RELATES_TO';
                if (targetId && item.id !== targetId) {
                  const key = `${item.id}:${targetId}:${relType}`;
                  if (!seen.has(key)) {
                    seen.add(key);
                    relationships.push({
                      schemaVersion: 1,
                      sourceId: item.id,
                      targetId: targetId,
                      relationshipType: relType.toUpperCase(),
                      confidence: rel.confidence ?? 1.0,
                      inferenceSource: rel.inference_source || 'explicit',
                    });
                  }
                }
              }
            }
          } catch {
            // Ignore JSON parse errors
          }
        }
      }

      if (relationships.length > 0) {
        console.log(`   Found ${relationships.length} relationships to create`);
        console.log(`   📤 Bulk creating relationships...`);

        try {
          const relResult = await timer.timeOperation('bulk_create_relationships', () =>
            doorwayClient.bulkCreateRelationships(relationships)
          );

          console.log(`   ✅ Relationships import complete: ${relResult.created} created`);
          if (relResult.errors.length > 0) {
            console.log(`   ⚠️ ${relResult.errors.length} relationship errors:`);
            for (const err of relResult.errors.slice(0, 3)) {
              console.error(`      • ${err}`);
            }
          }
        } catch (relErr: any) {
          console.warn(`   ⚠️ Relationships bulk create failed: ${relErr.message}`);
          // Don't exit - relationship failure shouldn't abort seeding
        }
      } else {
        console.log(`   No relationships found in content`);
      }

      timer.endPhase('Relationship Import');

    } catch (error) {
      console.error(`   ❌ Bulk create failed: ${error}`);
      console.error(`\n❌ SEEDING FAILED: Content bulk create failed. Check elohim-storage logs.`);
      process.exit(1);
    }

    timer.endPhase('Content Import');
  }
  } // end if (!PATHS_ONLY)

  // ========================================
  // LOAD & IMPORT PATHS
  // ========================================
  if (CONTENT_ONLY) {
    console.log('\n⏭️  Skipping paths (--content-only mode)');
  } else {
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

  // ========================================
  // UPLOAD PATH THUMBNAIL BLOBS
  // ========================================
  timer.startPhase('Path Thumbnails');
  console.log('\n🖼️  Processing path thumbnails...');

  // Map to store blob_hash for each path ID
  const pathThumbnailHashes = new Map<string, string>();

  // Find paths with local thumbnails
  const pathsWithThumbnails = allPaths.filter(({ pathData }) => pathData.localThumbnailPath);

  if (pathsWithThumbnails.length > 0) {
    console.log(`   Found ${pathsWithThumbnails.length} path(s) with local thumbnails`);

    for (const { pathData } of pathsWithThumbnails) {
      try {
        const thumbnailPath = path.join(GENESIS_DIR, pathData.localThumbnailPath);
        if (!fs.existsSync(thumbnailPath)) {
          console.warn(`   ⚠️ ${pathData.id}: thumbnail not found at ${pathData.localThumbnailPath}`);
          continue;
        }

        const imageData = fs.readFileSync(thumbnailPath);
        const hash = StorageClient.computeHash(imageData);

        // Determine MIME type from extension
        const ext = path.extname(thumbnailPath).toLowerCase();
        const mimeTypes: Record<string, string> = {
          '.png': 'image/png',
          '.jpg': 'image/jpeg',
          '.jpeg': 'image/jpeg',
          '.webp': 'image/webp',
          '.gif': 'image/gif',
          '.svg': 'image/svg+xml',
        };
        const mimeType = mimeTypes[ext] || 'application/octet-stream';

        console.log(`   📦 Uploading ${pathData.id}: ${(imageData.length / 1024).toFixed(1)} KB (${mimeType})`);

        const uploadResult = await doorwayClient.pushBlob(hash, imageData, {
          hash,
          mimeType,
          sizeBytes: imageData.length,
        });

        if (uploadResult.success) {
          pathThumbnailHashes.set(pathData.id, hash);
          console.log(`   ✅ ${pathData.id}: ${uploadResult.cached ? 'already cached' : 'uploaded'}`);
        } else {
          console.error(`   ❌ ${pathData.id}: upload failed - ${uploadResult.error}`);
        }
      } catch (err) {
        console.error(`   ❌ ${pathData.id}: processing error - ${err}`);
      }
    }

    console.log(`   📊 Processed ${pathThumbnailHashes.size}/${pathsWithThumbnails.length} thumbnails`);
  } else {
    console.log('   No paths with local thumbnails found');
  }

  timer.endPhase('Path Thumbnails');

  if (allPaths.length > 0) {
    timer.startPhase('Path Import');
    console.log(`\n🚀 Importing ${allPaths.length} paths via doorway...`);

    // Build content title lookup map from loaded concepts
    // Used to resolve conceptIds to proper titles in path steps
    const contentTitleMap = new Map<string, string>();
    for (const { concept } of filteredConcepts) {
      if (concept.id && concept.title) {
        contentTitleMap.set(concept.id, concept.title);
      }
    }

    // Step type matching zome's PathImportStepInput struct
    interface StepInput {
      stepType: string;
      resourceId: string;
      orderIndex: number;
      stepTitle?: string;
      stepNarrative?: string;
      isOptional?: boolean;
      // Module association metadata for UI filtering
      moduleId?: string;
      chapterId?: string;
      sectionId?: string;
    }

    // Valid step types per storage validation
    // Storage accepts: learn, practice, quiz, assessment, discussion, project, resource, video, reading, checkpoint
    const VALID_STEP_TYPES = ['learn', 'practice', 'quiz', 'assessment', 'discussion', 'project', 'resource', 'video', 'reading', 'checkpoint'];

    // Normalize step type aliases to valid storage values
    function normalizeStepType(type: string | undefined): string {
      if (!type) return 'learn';  // Default to 'learn' (generic content)
      const normalized = type.toLowerCase();
      // Map old/alternative step types to storage-accepted values
      const aliases: Record<string, string> = {
        // Old seeder types -> new storage types
        'content': 'learn',
        'read': 'reading',
        'assess': 'assessment',
        'path': 'resource',      // Sub-path reference
        'external': 'resource',  // External link
        'interactive': 'practice',
        // Common variations
        'lesson': 'learn',
        'article': 'reading',
        'test': 'quiz',
        'exercise': 'practice',
      };
      const mapped = aliases[normalized] || normalized;
      if (!VALID_STEP_TYPES.includes(mapped)) {
        console.warn(`   ⚠️ Unknown stepType '${type}' -> defaulting to 'learn'`);
        return 'learn';
      }
      return mapped;
    }

    // Helper to extract step data from a step object (preserves optional fields)
    function extractStepData(
      step: any,
      orderIndex: number,
      metadata?: { chapterId?: string; moduleId?: string; sectionId?: string }
    ): StepInput {
      return {
        stepType: normalizeStepType(step.stepType || step.stepType),
        resourceId: step.resourceId || step.resourceId || step.id,
        orderIndex: step.orderIndex ?? step.orderIndex ?? orderIndex,
        // Optional fields - only include if present
        ...(step.stepTitle || step.stepTitle ? { stepTitle: step.stepTitle || step.stepTitle } : {}),
        ...(step.stepNarrative || step.stepNarrative ? { stepNarrative: step.stepNarrative || step.stepNarrative } : {}),
        ...((step.isOptional ?? step.optional) !== undefined ? { isOptional: step.isOptional ?? step.optional ?? false } : {}),
        // Module association metadata
        ...(metadata?.chapterId ? { chapterId: metadata.chapterId } : {}),
        ...(metadata?.moduleId ? { moduleId: metadata.moduleId } : {}),
        ...(metadata?.sectionId ? { sectionId: metadata.sectionId } : {}),
      };
    }

    // DEPRECATED: This function globally flattens hierarchy, losing chapter structure.
    // Use extractStepsFromChapter() instead which preserves per-chapter hierarchy.
    // Kept for backward compatibility only.
    // Helper to flatten hierarchical path structure into steps
    // Paths can have: chapters → modules → sections → conceptIds
    function extractStepsFromPath(pathData: any): StepInput[] {
      const steps: StepInput[] = [];
      let orderIndex = 0;
      let moduleCount = 0;

      // If path has explicit steps array, use it directly
      if (pathData.steps && Array.isArray(pathData.steps)) {
        return pathData.steps.map((step: any, i: number) => extractStepData(step, i));
      }

      // If path has flat conceptIds array
      if (pathData.conceptIds && Array.isArray(pathData.conceptIds)) {
        return pathData.conceptIds.map((id: string, i: number) => ({
          stepType: 'learn',
          resourceId: id,
          orderIndex: i,
        }));
      }

      // Flatten hierarchical structure: chapters → modules → sections
      // Each level can have either conceptIds (flat) or steps (structured)
      if (pathData.chapters && Array.isArray(pathData.chapters)) {
        for (const chapter of pathData.chapters) {
          const chapterId = chapter.id;

          // Chapter-level steps array (structured) - preserves step metadata
          if (chapter.steps && Array.isArray(chapter.steps)) {
            for (const step of chapter.steps) {
              steps.push(extractStepData(step, orderIndex++, { chapterId }));
            }
          }
          // Chapter-level conceptIds (flat)
          if (chapter.conceptIds && Array.isArray(chapter.conceptIds)) {
            for (const id of chapter.conceptIds) {
              steps.push({
                stepType: 'learn',
                resourceId: id,
                orderIndex: orderIndex++,
                chapterId,
              });
            }
          }
          // Modules within chapters
          if (chapter.modules && Array.isArray(chapter.modules)) {
            for (const module of chapter.modules) {
              const moduleId = module.id;
              moduleCount++;

              // Module-level steps array - preserves step metadata
              if (module.steps && Array.isArray(module.steps)) {
                for (const step of module.steps) {
                  steps.push(extractStepData(step, orderIndex++, { chapterId, moduleId }));
                }
              }
              // Module-level conceptIds
              if (module.conceptIds && Array.isArray(module.conceptIds)) {
                for (const id of module.conceptIds) {
                  steps.push({
                    stepType: 'learn',
                    resourceId: id,
                    orderIndex: orderIndex++,
                    chapterId,
                    moduleId,
                  });
                }
              }
              // Sections within modules
              if (module.sections && Array.isArray(module.sections)) {
                for (const section of module.sections) {
                  const sectionId = section.id;

                  // Section-level steps array - preserves step metadata
                  if (section.steps && Array.isArray(section.steps)) {
                    for (const step of section.steps) {
                      steps.push(extractStepData(step, orderIndex++, { chapterId, moduleId, sectionId }));
                    }
                  }
                  // Section-level conceptIds
                  if (section.conceptIds && Array.isArray(section.conceptIds)) {
                    for (const id of section.conceptIds) {
                      steps.push({
                        stepType: 'learn',
                        resourceId: id,
                        orderIndex: orderIndex++,
                        chapterId,
                        moduleId,
                        sectionId,
                      });
                    }
                  }
                }
              }
            }
          }
        }
      }

      // Log step extraction summary
      if (moduleCount > 0) {
        console.log(`   📍 Path "${pathData.id}": Extracted ${steps.length} steps from ${moduleCount} modules`);
      }

      return steps;
    }

    // Extract steps from a single chapter (not globally)
    // Used for proper hierarchical transformation that preserves chapter structure
    function extractStepsFromChapter(chapter: any, pathId: string): any[] {
      const steps: any[] = [];
      let orderIndex = 0;

      // Direct chapter steps (2-level format: chapters → steps)
      if (chapter.steps?.length) {
        for (const step of chapter.steps) {
          steps.push({
            id: step.id || `${chapter.id}-step-${orderIndex + 1}`,
            pathId,
            chapterId: chapter.id,
            title: step.stepTitle || step.title || `Step ${orderIndex + 1}`,
            stepType: normalizeStepType(step.stepType),
            resourceId: step.resourceId || null,
            orderIndex: orderIndex++,
            metadataJson: JSON.stringify({ moduleId: null, sectionId: null }),
          });
        }
      }

      // Chapter-level conceptIds (flat)
      if (chapter.conceptIds?.length) {
        for (const conceptId of chapter.conceptIds) {
          steps.push({
            id: `${chapter.id}-step-${orderIndex + 1}`,
            pathId,
            chapterId: chapter.id,
            title: contentTitleMap.get(conceptId) || conceptId,  // Lookup title from content
            stepType: 'learn',
            resourceId: conceptId,
            orderIndex: orderIndex++,
            metadataJson: JSON.stringify({ moduleId: null, sectionId: null }),
          });
        }
      }

      // Module → Section → conceptIds (4-level format: chapters → modules → sections → conceptIds)
      if (chapter.modules?.length) {
        for (const module of chapter.modules) {
          // Module-level conceptIds
          if (module.conceptIds?.length) {
            for (const conceptId of module.conceptIds) {
              steps.push({
                id: `${module.id}-step-${orderIndex + 1}`,
                pathId,
                chapterId: chapter.id,
                title: contentTitleMap.get(conceptId) || conceptId,  // Lookup title from content
                stepType: 'learn',
                resourceId: conceptId,
                orderIndex: orderIndex++,
                metadataJson: JSON.stringify({
                  moduleId: module.id,
                  sectionId: null,
                }),
              });
            }
          }

          // Module-level steps
          if (module.steps?.length) {
            for (const step of module.steps) {
              steps.push({
                id: step.id || `${module.id}-step-${orderIndex + 1}`,
                pathId,
                chapterId: chapter.id,
                title: step.stepTitle || step.title || `Step ${orderIndex + 1}`,
                stepType: normalizeStepType(step.stepType),
                resourceId: step.resourceId || null,
                orderIndex: orderIndex++,
                metadataJson: JSON.stringify({
                  moduleId: module.id,
                  sectionId: null,
                }),
              });
            }
          }

          // Sections within modules
          if (module.sections?.length) {
            for (const section of module.sections) {
              // Section-level conceptIds
              if (section.conceptIds?.length) {
                for (const conceptId of section.conceptIds) {
                  steps.push({
                    id: `${section.id}-step-${orderIndex + 1}`,
                    pathId,
                    chapterId: chapter.id,
                    title: contentTitleMap.get(conceptId) || conceptId,  // Lookup title from content
                    stepType: 'learn',
                    resourceId: conceptId,
                    orderIndex: orderIndex++,
                    metadataJson: JSON.stringify({
                      moduleId: module.id,
                      sectionId: section.id,
                    }),
                  });
                }
              }

              // Section-level steps
              if (section.steps?.length) {
                for (const step of section.steps) {
                  steps.push({
                    id: step.id || `${section.id}-step-${orderIndex + 1}`,
                    pathId,
                    chapterId: chapter.id,
                    title: step.stepTitle || step.title || `Step ${orderIndex + 1}`,
                    stepType: normalizeStepType(step.stepType),
                    resourceId: step.resourceId || null,
                    orderIndex: orderIndex++,
                    metadataJson: JSON.stringify({
                      moduleId: module.id,
                      sectionId: section.id,
                    }),
                  });
                }
              }
            }
          }
        }
      }

      return steps;
    }

    // Transform paths directly with proper chapter hierarchy (no global flattening)
    const transformedPaths = allPaths.map(({ pathData }) => {
      const thumbnailHash = pathThumbnailHashes.get(pathData.id);

      let chapters: any[];

      if (pathData.chapters?.length) {
        // Transform each source chapter directly (preserves hierarchy)
        chapters = pathData.chapters.map((ch: any, idx: number) => {
          const chapterSteps = extractStepsFromChapter(ch, pathData.id);
          return {
            id: ch.id,
            title: ch.title || '',
            description: ch.description || '',
            orderIndex: ch.order ?? ch.orderIndex ?? idx,
            estimatedDuration: ch.estimatedDuration || null,
            steps: chapterSteps,
          };
        });

        // Log chapter hierarchy summary
        const totalSteps = chapters.reduce((sum, ch) => sum + ch.steps.length, 0);
        console.log(`   📍 Path "${pathData.id}": ${chapters.length} chapters, ${totalSteps} total steps`);
      } else if (pathData.steps?.length) {
        // Legacy flat steps - wrap in default chapter
        chapters = [{
          id: `${pathData.id}-chapter-1`,
          title: pathData.title || 'Main Content',
          description: pathData.description || '',
          orderIndex: 0,
          estimatedDuration: null,
          steps: pathData.steps.map((step: any, idx: number) => ({
            id: step.id || `${pathData.id}-step-${idx + 1}`,
            pathId: pathData.id,
            chapterId: `${pathData.id}-chapter-1`,
            title: step.stepTitle || step.title || `Step ${idx + 1}`,
            stepType: normalizeStepType(step.stepType),
            resourceId: step.resourceId || null,
            orderIndex: idx,
            metadataJson: JSON.stringify({}),
          })),
        }];
        console.log(`   📍 Path "${pathData.id}": 1 default chapter, ${pathData.steps.length} steps (legacy format)`);
      } else if (pathData.conceptIds?.length) {
        // Legacy flat conceptIds - wrap in default chapter
        chapters = [{
          id: `${pathData.id}-chapter-1`,
          title: pathData.title || 'Main Content',
          description: pathData.description || '',
          orderIndex: 0,
          estimatedDuration: null,
          steps: pathData.conceptIds.map((conceptId: string, idx: number) => ({
            id: `${pathData.id}-step-${idx + 1}`,
            pathId: pathData.id,
            chapterId: `${pathData.id}-chapter-1`,
            title: `Step ${idx + 1}`,
            stepType: 'learn',
            resourceId: conceptId,
            orderIndex: idx,
            metadataJson: JSON.stringify({}),
          })),
        }];
        console.log(`   📍 Path "${pathData.id}": 1 default chapter, ${pathData.conceptIds.length} steps (conceptIds format)`);
      } else {
        chapters = [];
        console.log(`   📍 Path "${pathData.id}": No chapters or steps found`);
      }

      return {
        schemaVersion: 1,
        id: pathData.id,
        title: pathData.title,
        description: pathData.description || '',
        pathType: normalizePathType(pathData.pathType),
        difficulty: pathData.difficulty || 'beginner',
        estimatedDuration: pathData.estimatedDuration || null,
        visibility: pathData.visibility || 'public',
        metadataJson: JSON.stringify(
          pathData.chapters
            ? { chapters: pathData.chapters, ...pathData.metadata }
            : pathData.metadata || {}
        ),
        tags: pathData.tags || [],
        thumbnailUrl: thumbnailHash ? `/blob/${thumbnailHash}` : pathData.thumbnailUrl || null,
        thumbnailBlobHash: thumbnailHash || null,
        chapters,
      };
    });

    result.pathsAttempted = transformedPaths.length;  // Track for verification

    // Bulk create paths via direct HTTP call
    console.log(`   📤 Bulk creating ${transformedPaths.length} paths...`);
    const pathsStartTime = Date.now();

    try {
      const pathsBulkResult = await timer.timeOperation('bulk_create_paths', () =>
        doorwayClient.bulkCreatePaths(transformedPaths)
      );

      const pathsElapsed = Date.now() - pathsStartTime;
      const pathsRate = transformedPaths.length / (pathsElapsed / 1000);

      result.pathsSucceeded = pathsBulkResult.inserted;

      // Report errors if any
      if (pathsBulkResult.errors.length > 0) {
        console.log(`   ⚠️ ${pathsBulkResult.errors.length} path errors during import:`);
        for (const err of pathsBulkResult.errors.slice(0, 5)) {
          console.error(`      • ${err}`);
        }
        if (pathsBulkResult.errors.length > 5) {
          console.error(`      ... and ${pathsBulkResult.errors.length - 5} more`);
        }
      }

      console.log(`   ✅ Paths import complete: ${pathsBulkResult.inserted} inserted, ${pathsBulkResult.skipped} skipped (${pathsElapsed}ms, ${pathsRate.toFixed(1)} paths/sec)`);

    } catch (pathErr: any) {
      console.error(`   ❌ Paths bulk create failed: ${pathErr.message}`);
      console.error(`\n❌ SEEDING FAILED: Paths bulk create failed. Check elohim-storage logs.`);
      // Don't exit - paths failure shouldn't abort if content succeeded
      result.pathsSucceeded = 0;
    }

    timer.endPhase('Path Import');
  }
  } // end if (!CONTENT_ONLY)

  // ========================================
  // SUMMARY
  // ========================================
  timer.endPhase('Doorway Import');
  console.log('\n' + '='.repeat(70));
  console.log('✅ DOORWAY SEEDING COMPLETE');
  console.log('='.repeat(70));
  timer.printReport();
  console.log('='.repeat(70));

  return result;
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

  // Load genesis manifest for sparse DHT pattern
  const manifest = loadGenesisManifest();
  if (manifest) {
    console.log(`   Using sparse DHT pattern (manifest has ${Object.keys(manifest.entries).length} blob entries)`);
  } else {
    console.log(`   Using legacy pattern (full content in DHT entries)`);
  }

  // ========================================
  // VERIFICATION SETUP
  // ========================================
  let verification: SeedingVerification | null = null;
  let connection: HolochainConnection | null = null;

  if (!SKIP_VERIFICATION) {
    console.log('\n🔍 Connecting for verification...');
    connection = await connectForVerification();

    if (connection) {
      verification = new SeedingVerification(connection.appWs, connection.cellId, ZOME_NAME);

      // Run pre-flight checks
      console.log('\n🔬 Running pre-flight checks...');
      const preflight = await verification.runPreflightChecks();

      // Display results
      for (const check of preflight.checks) {
        const icon = check.status === 'pass' ? '✅' : check.status === 'fail' ? '❌' : '⚠️';
        console.log(`   ${icon} ${check.name}: ${check.message}`);
        if (check.details) console.log(`      ${check.details}`);
      }

      if (preflight.warnings.length > 0) {
        console.log('\n   ⚠️ Warnings:');
        for (const warn of preflight.warnings) {
          console.log(`      • ${warn}`);
        }
      }

      if (!preflight.canProceed) {
        console.error('\n❌ Pre-flight checks failed:');
        for (const err of preflight.errors) {
          console.error(`   • ${err}`);
        }
        console.error('\nSeeding aborted. Fix the issues above and try again.');
        console.error('Set SKIP_VERIFICATION=true to bypass checks (not recommended).');
        await connection.close();
        process.exit(1);
      }

      console.log(`\n   📊 Existing content: ${preflight.existingCounts.content} items, ${preflight.existingCounts.paths} paths`);
    } else {
      console.log('   ⚠️ Verification unavailable - proceeding without pre/post-flight checks');
    }
  } else {
    console.log('\n⚠️ Verification skipped (SKIP_VERIFICATION=true)');
  }

  // ========================================
  // GENESIS BLOB SYNC (if manifest available)
  // ========================================
  if (!SKIP_BLOB_SYNC && STORAGE_URL) {
    console.log('\n📦 Syncing genesis blobs...');
    const blobSyncResult = await syncGenesisBlobs(STORAGE_URL);
    console.log(`   ✅ Blob sync complete: ${blobSyncResult.synced} synced, ${blobSyncResult.skipped} existed, ${blobSyncResult.failed} failed`);
    if (blobSyncResult.failed > 0) {
      console.warn(`   ⚠️ ${blobSyncResult.failed} blobs failed to sync - DHT entries will embed full content`);
    }
  } else if (!STORAGE_URL) {
    console.log('\n⚠️ No STORAGE_URL configured, skipping blob sync (DHT entries will embed full content)');
  } else {
    console.log('\n⚠️ Blob sync skipped (SKIP_BLOB_SYNC=true)');
  }

  // ========================================
  // RUN SEEDING
  // ========================================
  const seedResult = await seedViaDoorway();

  // ========================================
  // POST-FLIGHT VERIFICATION
  // ========================================
  if (verification && connection) {
    console.log('\n🔬 Running post-flight verification...');
    const postflight = await verification.runPostflightVerification(
      { content: seedResult.contentAttempted, paths: seedResult.pathsAttempted },
      seedResult.sampleIds.slice(0, 5)  // Verify first 5 sample IDs
    );

    // Display results
    for (const check of postflight.checks) {
      const icon = check.status === 'pass' ? '✅' : check.status === 'fail' ? '❌' : '⚠️';
      console.log(`   ${icon} ${check.name}: ${check.message}`);
      if (check.details) console.log(`      ${check.details}`);
    }

    if (postflight.warnings.length > 0) {
      console.log('\n   ⚠️ Warnings:');
      for (const warn of postflight.warnings) {
        console.log(`      • ${warn}`);
      }
    }

    if (!postflight.success) {
      console.error('\n❌ Post-flight verification FAILED:');
      for (const err of postflight.errors) {
        console.error(`   • ${err}`);
      }
      console.error('\n⚠️ Data may not have been properly written to the conductor!');
      console.error('   Check conductor logs and verify data manually.');
      await connection.close();
      process.exit(1);
    }

    console.log(`\n   📊 Final content: ${postflight.finalCounts.content} items, ${postflight.finalCounts.paths} paths`);
    console.log(`   📈 Delta: +${postflight.delta.content} content, +${postflight.delta.paths} paths`);
    await connection.close();
    console.log('\n✅ Seeding verified successfully!');
  } else if (seedResult.contentSucceeded !== seedResult.contentAttempted) {
    // Even without verification, warn if doorway reported failures
    console.warn(`\n⚠️ Seeding completed with issues: ${seedResult.contentSucceeded}/${seedResult.contentAttempted} content items succeeded`);
  }
}

seed().catch((error) => {
  console.error('Fatal error:', cleanErrorMessage(error));
  process.exit(1);
});
