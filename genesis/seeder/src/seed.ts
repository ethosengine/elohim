/**
 * Holochain Content Seeder
 *
 * Seeds pre-structured JSON content from /data/lamad into Holochain.
 * This is a deterministic script that loads JSON files created by Claude + MCP tools.
 *
 * Pipeline: docs/ → Claude + MCP → data/lamad/ → seeder → Holochain DHT
 *
 * ## Diagnosing Failed Seeds
 *
 * When a seed operation fails or times out, use these endpoints to investigate:
 *
 * ### 1. Get Batch Diagnostics
 * Shows overall batch state including failed items and progress:
 * ```bash
 * curl https://doorway-dev.elohim.host/import/diagnostics/<batch_id>
 * ```
 * Returns: remaining_count, last_completed_chunk, failed_items with error reasons
 *
 * ### 2. Get Remaining Items
 * Re-reads the original blob and shows what wasn't successfully processed:
 * ```bash
 * curl https://doorway-dev.elohim.host/import/remaining/<batch_id>
 * ```
 * Returns: list of items with reasons ("not_attempted" or actual zome error)
 *
 * ### 3. Common Issues
 * - **Schema mismatches**: Check error messages for "invalid content_format" etc.
 * - **Timeouts**: Items marked "not_attempted" were in chunks that timed out
 * - **Validation errors**: Zome error messages show what validation failed
 *
 * ### 4. Finding the batch_id
 * The batch_id is printed in the seeder output, e.g.:
 *   "📤 Queuing import "seed-content-1704307200000"..."
 *
 * Or check all batches:
 * ```bash
 * curl https://doorway-dev.elohim.host/import/batches
 * ```
 *
 * @version 2024-12-24 - Trigger seeder after seed data migration
 */

import * as fs from 'fs';
import * as path from 'path';
import { AdminWebsocket, AppWebsocket, type CellId } from '@holochain/client';
import DoorwayClient, { ImportStatusResponse } from './doorway-client.js';
import StorageClient from './storage-client.js'; // Used for computing blob hash
import { validateBatch, logValidationErrors, isStrictValidation } from './validators.js';
import { ProgressClient, waitForBatchCompletion, waitWithFallback, type ProgressMessage } from './progress-client.js';
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
const APP_ID = 'elohim';
const ZOME_NAME = 'content_store';
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
  const doorwayClient = new DoorwayClient({
    baseUrl: DOORWAY_URL!,
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
    console.log(`   Conductor: ${status.conductor.connected ? '✅ connected' : '❌ disconnected'} (${status.conductor.connected_workers}/${status.conductor.total_workers} workers)`);
    console.log(`   Storage: ${status.storage.healthy ? '✅ healthy' : '❌ unhealthy'} (import=${status.storage.import_enabled ? 'on' : 'off'})`);
    console.log(`   Cell: ${status.diagnostics.cell_discovered ? '✅ discovered' : '⏳ will discover lazily'}`);

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
  if (PATHS_ONLY) {
    console.log('\n⏭️  Skipping content (--paths-only mode)');
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

    // Step 1: Upload items as blob
    const itemsJson = JSON.stringify(itemsToSeed);
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
        total_items: itemsToSeed.length,
        schema_version: 1,
      })
    ) as ImportStatusResponse & { batch_id: string; queued_count?: number; total_items?: number; processing: boolean };

    // Handle both response formats (queued_count from doorway, total_items from storage)
    const queuedCount = queueResult.queued_count ?? queueResult.total_items ?? itemsToSeed.length;
    console.log(`   ✅ Import queued: ${queueResult.batch_id} (${queuedCount} items)`);

    // Step 3: Wait for completion with real-time progress
    const USE_WEBSOCKET = process.env.SEED_USE_WEBSOCKET !== 'false';
    const POLL_STATUS = process.env.SEED_POLL_STATUS !== 'false';
    const POLL_INTERVAL_MS = parseInt(process.env.SEED_POLL_INTERVAL || '5000', 10);
    const POLL_TIMEOUT_MS = parseInt(process.env.SEED_POLL_TIMEOUT || '5400000', 10); // 90 min default

    if (POLL_STATUS) {
      console.log(`   ⏳ Waiting for import to complete...`);

      if (USE_WEBSOCKET) {
        // WebSocket-based real-time progress streaming
        console.log(`      (Using WebSocket progress stream)`);

        try {
          const finalStatus = await waitWithFallback(
            DOORWAY_URL!,
            batchId,
            {
              timeoutMs: POLL_TIMEOUT_MS,
              pollIntervalMs: POLL_INTERVAL_MS,
              onProgress: (update) => {
                if (update.processed_count !== undefined && update.total_items !== undefined) {
                  const pct = ((update.processed_count / update.total_items) * 100).toFixed(1);
                  const rate = update.items_per_second ? ` (${update.items_per_second.toFixed(1)}/s)` : '';
                  console.log(`      Progress: ${update.processed_count}/${update.total_items} (${pct}%)${rate} - ${update.status}`);
                }
              }
            },
            async (pollBatchId) => {
              // Fallback polling function
              const status = await doorwayClient.getImportStatus('content', pollBatchId);
              if (!status) return null;
              return {
                type: 'progress' as const,
                batch_id: pollBatchId,
                status: status.status,
                total_items: status.total_items,
                processed_count: status.processed_count,
                error_count: status.error_count,
                errors: status.errors,
              };
            }
          );

          // Handle final status - CHECK FOR ACTUAL FAILURE
          const succeeded = (finalStatus.processed_count ?? 0) - (finalStatus.error_count ?? 0);
          result.contentSucceeded = succeeded;  // Track for verification
          const elapsed = finalStatus.elapsed_ms ? ` (${(finalStatus.elapsed_ms / 1000).toFixed(1)}s` : '';
          const rate = finalStatus.items_per_second ? `, ${finalStatus.items_per_second.toFixed(0)} items/sec)` : elapsed ? ')' : '';

          // Check for batch failure (failed status or all items errored)
          if (finalStatus.status === 'failed') {
            console.error(`   ❌ Content import FAILED`);
            if (finalStatus.errors?.length) {
              console.error(`   Errors:`);
              for (const err of finalStatus.errors.slice(0, 10)) {
                console.error(`      • ${err}`);
              }
              if (finalStatus.errors.length > 10) {
                console.error(`      ... and ${finalStatus.errors.length - 10} more`);
              }
            }
            console.error(`\n❌ SEEDING FAILED: Batch import failed. Check elohim-storage logs.`);
            process.exit(1);
          }

          // Check for partial success with significant errors
          if (finalStatus.error_count && finalStatus.error_count > 0) {
            console.log(`   ⚠️ ${finalStatus.error_count} errors during import:`);
            if (finalStatus.errors?.length) {
              for (const err of finalStatus.errors.slice(0, 5)) {
                console.error(`      • ${err}`);
              }
              if (finalStatus.errors.length > 5) {
                console.error(`      ... and ${finalStatus.errors.length - 5} more`);
              }
            }

            // Fail if more than 10% of items errored
            const errorRate = (finalStatus.error_count / (finalStatus.total_items ?? 1)) * 100;
            if (errorRate > 10) {
              console.error(`\n❌ SEEDING FAILED: Too many errors (${errorRate.toFixed(1)}%). Check elohim-storage logs.`);
              process.exit(1);
            }
          }

          // Verify we actually processed items (not silent skip!)
          if (succeeded === 0 && (finalStatus.total_items ?? 0) > 0) {
            console.error(`\n❌ SEEDING FAILED: 0 items succeeded out of ${finalStatus.total_items}!`);
            console.error(`   This likely means cell_id discovery failed or conductor rejected all items.`);
            console.error(`   Check elohim-storage logs for "CELL_DISCOVERY" or "CHUNK_ERROR" messages.`);
            process.exit(1);
          }

          console.log(`   ✅ Content import ${finalStatus.status}: ${succeeded} succeeded, ${finalStatus.error_count ?? 0} failed${elapsed}${rate}`);

        } catch (wsError) {
          console.error(`   ❌ Import wait failed: ${wsError}`);
          console.error(`\n❌ SEEDING FAILED: Could not confirm batch completion. Exiting.`);
          process.exit(1);
        }

      } else {
        // HTTP polling fallback (legacy behavior, SEED_USE_WEBSOCKET=false)
        console.log(`      (HTTP polling: interval=${POLL_INTERVAL_MS}ms, timeout=${POLL_TIMEOUT_MS}ms)`);

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
          const succeeded = finalStatus.processed_count - finalStatus.error_count;
          result.contentSucceeded = succeeded;  // Track for verification

          // Check for batch failure
          if (finalStatus.status === 'failed') {
            console.error(`   ❌ Content import FAILED`);
            if (finalStatus.errors.length > 0) {
              for (const err of finalStatus.errors.slice(0, 10)) {
                console.error(`      • ${err}`);
              }
            }
            console.error(`\n❌ SEEDING FAILED: Batch import failed. Check elohim-storage logs.`);
            process.exit(1);
          }

          // Check for partial success with significant errors
          if (finalStatus.error_count > 0) {
            console.log(`   ⚠️ ${finalStatus.errors.length} errors during import:`);
            for (const err of finalStatus.errors.slice(0, 5)) {
              console.error(`      • ${err}`);
            }
            if (finalStatus.errors.length > 5) {
              console.error(`      ... and ${finalStatus.errors.length - 5} more`);
            }

            // Fail if more than 10% of items errored
            const errorRate = (finalStatus.error_count / finalStatus.total_items) * 100;
            if (errorRate > 10) {
              console.error(`\n❌ SEEDING FAILED: Too many errors (${errorRate.toFixed(1)}%). Check elohim-storage logs.`);
              process.exit(1);
            }
          }

          // Verify we actually processed items (not silent skip!)
          if (succeeded === 0 && finalStatus.total_items > 0) {
            console.error(`\n❌ SEEDING FAILED: 0 items succeeded out of ${finalStatus.total_items}!`);
            console.error(`   This likely means cell_id discovery failed or conductor rejected all items.`);
            console.error(`   Check elohim-storage logs for "CELL_DISCOVERY" or "CHUNK_ERROR" messages.`);
            process.exit(1);
          }

          console.log(`   ✅ Content import ${finalStatus.status}: ${succeeded} succeeded, ${finalStatus.error_count} failed`);
        } else {
          console.error(`   ❌ Polling timeout - batch completion could not be confirmed`);
          console.error(`\n❌ SEEDING FAILED: Timeout waiting for batch. Check status manually:`);
          console.error(`      GET ${DOORWAY_URL}/import/content/${batchId}`);
          process.exit(1);
        }
      }
    } else {
      console.log(`   ℹ️ Not polling - batch queued for background processing`);
      console.log(`      Check status: GET ${DOORWAY_URL}/import/content/${batchId}`);
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

  if (allPaths.length > 0) {
    timer.startPhase('Path Import');
    console.log(`\n🚀 Importing ${allPaths.length} paths via doorway...`);

    // Helper to flatten hierarchical path structure into steps
    // Paths can have: chapters → modules → sections → conceptIds
    function extractStepsFromPath(pathData: any): Array<{step_type: string; resource_id: string; order_index: number}> {
      const steps: Array<{step_type: string; resource_id: string; order_index: number}> = [];
      let orderIndex = 0;

      // If path has explicit steps array, use it directly
      if (pathData.steps && Array.isArray(pathData.steps)) {
        return pathData.steps.map((step: any, i: number) => ({
          step_type: step.step_type || step.stepType || 'content',
          resource_id: step.resource_id || step.resourceId || step.id,
          order_index: step.order_index ?? step.orderIndex ?? i,
        }));
      }

      // If path has flat conceptIds array
      if (pathData.conceptIds && Array.isArray(pathData.conceptIds)) {
        return pathData.conceptIds.map((id: string, i: number) => ({
          step_type: 'content',
          resource_id: id,
          order_index: i,
        }));
      }

      // Flatten hierarchical structure: chapters → modules → sections
      // Each level can have either conceptIds (flat) or steps (structured)
      if (pathData.chapters && Array.isArray(pathData.chapters)) {
        for (const chapter of pathData.chapters) {
          // Chapter-level steps array (structured)
          if (chapter.steps && Array.isArray(chapter.steps)) {
            for (const step of chapter.steps) {
              steps.push({
                step_type: step.step_type || step.stepType || 'content',
                resource_id: step.resource_id || step.resourceId || step.id,
                order_index: orderIndex++,
              });
            }
          }
          // Chapter-level conceptIds (flat)
          if (chapter.conceptIds && Array.isArray(chapter.conceptIds)) {
            for (const id of chapter.conceptIds) {
              steps.push({ step_type: 'content', resource_id: id, order_index: orderIndex++ });
            }
          }
          // Modules within chapters
          if (chapter.modules && Array.isArray(chapter.modules)) {
            for (const module of chapter.modules) {
              // Module-level steps array
              if (module.steps && Array.isArray(module.steps)) {
                for (const step of module.steps) {
                  steps.push({
                    step_type: step.step_type || step.stepType || 'content',
                    resource_id: step.resource_id || step.resourceId || step.id,
                    order_index: orderIndex++,
                  });
                }
              }
              // Module-level conceptIds
              if (module.conceptIds && Array.isArray(module.conceptIds)) {
                for (const id of module.conceptIds) {
                  steps.push({ step_type: 'content', resource_id: id, order_index: orderIndex++ });
                }
              }
              // Sections within modules
              if (module.sections && Array.isArray(module.sections)) {
                for (const section of module.sections) {
                  // Section-level steps array
                  if (section.steps && Array.isArray(section.steps)) {
                    for (const step of section.steps) {
                      steps.push({
                        step_type: step.step_type || step.stepType || 'content',
                        resource_id: step.resource_id || step.resourceId || step.id,
                        order_index: orderIndex++,
                      });
                    }
                  }
                  // Section-level conceptIds
                  if (section.conceptIds && Array.isArray(section.conceptIds)) {
                    for (const id of section.conceptIds) {
                      steps.push({ step_type: 'content', resource_id: id, order_index: orderIndex++ });
                    }
                  }
                }
              }
            }
          }
        }
      }

      return steps;
    }

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
      steps: extractStepsFromPath(pathData),
    }));

    // Log step counts for debugging
    for (const path of pathInputs) {
      console.log(`   📍 Path "${path.id}": ${path.steps.length} steps`);
    }
    result.pathsAttempted = pathInputs.length;  // Track for verification

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

        // Wait for paths import to complete (like content import)
        const POLL_TIMEOUT_MS = parseInt(process.env.SEED_POLL_TIMEOUT || '5400000', 10); // 90 min default
        const POLL_INTERVAL_MS = parseInt(process.env.SEED_POLL_INTERVAL || '5000', 10);

        console.log(`   ⏳ Waiting for paths import to complete...`);

        try {
          const finalStatus = await waitWithFallback(
            DOORWAY_URL!,
            pathsBatchId,
            {
              timeoutMs: POLL_TIMEOUT_MS,
              pollIntervalMs: POLL_INTERVAL_MS,
              onProgress: (update) => {
                if (update.processed_count !== undefined && update.total_items !== undefined) {
                  const pct = ((update.processed_count / update.total_items) * 100).toFixed(1);
                  const rate = update.items_per_second ? ` (${update.items_per_second.toFixed(1)}/s)` : '';
                  console.log(`      Progress: ${update.processed_count}/${update.total_items} (${pct}%)${rate} - ${update.status}`);
                }
              }
            },
            async (pollBatchId) => {
              const status = await doorwayClient.getImportStatus('paths', pollBatchId);
              if (!status) return null;
              return {
                type: 'progress' as const,
                batch_id: pollBatchId,
                status: status.status,
                total_items: status.total_items,
                processed_count: status.processed_count,
                error_count: status.error_count,
                errors: status.errors,
              };
            }
          );

          const succeeded = (finalStatus.processed_count ?? 0) - (finalStatus.error_count ?? 0);
          result.pathsSucceeded = succeeded;

          if (finalStatus.status === 'failed') {
            console.error(`   ❌ Paths import FAILED`);
            if (finalStatus.errors?.length) {
              for (const err of finalStatus.errors.slice(0, 5)) {
                console.error(`      • ${err}`);
              }
            }
          } else if (finalStatus.error_count && finalStatus.error_count > 0) {
            console.log(`   ⚠️ ${finalStatus.error_count} path errors during import`);
            console.log(`   ✅ Paths import ${finalStatus.status}: ${succeeded} succeeded, ${finalStatus.error_count} failed`);
          } else {
            console.log(`   ✅ Paths import ${finalStatus.status}: ${succeeded} paths created`);
          }
        } catch (waitErr) {
          console.warn(`   ⚠️ Paths import wait failed: ${waitErr}`);
          console.log(`      Check status: GET ${DOORWAY_URL}/import/paths/${pathsBatchId}`);
        }
      } catch (pathErr: any) {
        console.warn(`   ⚠️ Paths import queue failed: ${pathErr.message}`);
        console.log(`      (Paths import may not be implemented yet in the zome)`);
      }
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
