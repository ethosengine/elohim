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

import { AdminWebsocket, AppWebsocket, encodeHashToBase64, CellId } from '@holochain/client';
import * as fs from 'fs';
import * as path from 'path';
import DoorwayClient, { ImportStatusResponse } from './doorway-client.js';
import StorageClient from './storage-client.js';

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
const LOCAL_DEV_DIR = process.env.LOCAL_DEV_DIR || '/projects/elohim/holochain/local-dev';
const HC_PORTS_FILE = process.env.HC_PORTS_FILE || path.join(LOCAL_DEV_DIR, '.hc_ports');
const APP_ID = 'elohim';
// Role name is auto-detected from app's cell_info to support both 'elohim' and legacy 'lamad'
const ZOME_NAME = 'content_store';

// Doorway mode: POST to doorway HTTP routes instead of direct WebSocket calls
// When set, seeder hammers doorway with HTTP calls which then orchestrates conductor writes
const DOORWAY_URL = process.env.DOORWAY_URL; // e.g., 'https://doorway.elohim.host' or 'http://localhost:8080'
const DOORWAY_API_KEY = process.env.DOORWAY_API_KEY;
const USE_DOORWAY = !!DOORWAY_URL;

// Elohim-storage: Blob storage for large import payloads
// When set, seeder uploads blob first, then passes blob_hash to zome
const STORAGE_URL = process.env.STORAGE_URL; // e.g., 'http://localhost:8090' or 'https://storage.elohim.host'
const USE_STORAGE = !!STORAGE_URL;

/**
 * Read Holochain ports from .hc_ports file
 */
function readHcPorts(): { adminPort: number; appPort: number } {
  try {
    const content = fs.readFileSync(HC_PORTS_FILE, 'utf-8');
    const adminMatch = content.match(/admin_port=(\d+)/);
    const appMatch = content.match(/app_port=(\d+)/);

    if (!adminMatch || !appMatch) {
      throw new Error('Could not parse .hc_ports file');
    }

    return {
      adminPort: parseInt(adminMatch[1], 10),
      appPort: parseInt(appMatch[1], 10),
    };
  } catch (error) {
    console.error(`❌ Could not read ${HC_PORTS_FILE}:`, error);
    console.log('   Falling back to default ports (4444, 4445)');
    return { adminPort: 4444, appPort: 4445 };
  }
}

// Read ports from file or env
const ports = readHcPorts();
const ADMIN_WS_URL = process.env.HOLOCHAIN_ADMIN_URL || `ws://localhost:${ports.adminPort}`;
const DEFAULT_APP_WS_URL = process.env.HOLOCHAIN_APP_URL || `ws://localhost:${ports.appPort}`;

/**
 * Resolve app WebSocket URL based on admin URL and dynamic port.
 */
function resolveAppUrl(adminUrl: string, port: number): string {
  if (!adminUrl.includes('localhost') && !adminUrl.includes('127.0.0.1')) {
    const url = new URL(adminUrl);
    const baseUrl = `${url.protocol}//${url.host}`;
    const apiKey = url.searchParams.get('apiKey');
    const apiKeyParam = apiKey ? `?apiKey=${encodeURIComponent(apiKey)}` : '';
    return `${baseUrl}/app/${port}${apiKeyParam}`;
  }
  return `ws://localhost:${port}`;
}

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
 * Main seeding function
 */
async function seed() {
  console.log('🌱 Holochain Content Seeder (JSON mode)');
  console.log(`📁 Data directory: ${DATA_DIR}`);
  console.log(`🔌 Admin WebSocket: ${ADMIN_WS_URL}`);
  if (IDS.length > 0) console.log(`🎯 Filtering to IDs: ${IDS.join(', ')}`);
  if (LIMIT > 0) console.log(`📊 Limit: ${LIMIT}`);

  timer.startPhase('Connection Setup');

  // Connect to admin websocket
  console.log('\n📡 Connecting to Holochain admin...');
  let adminWs: AdminWebsocket;
  try {
    adminWs = await AdminWebsocket.connect({
      url: new URL(ADMIN_WS_URL),
      wsClientOptions: { origin: 'http://localhost' },
    });
    console.log('✅ Connected to admin WebSocket');
  } catch (error) {
    console.error('❌ Failed to connect to admin WebSocket:', error);
    process.exit(1);
  }

  // Get app info to find cell ID
  console.log('\n📱 Getting app info...');
  const apps = await adminWs.listApps({});
  const app = apps.find((a) => a.installed_app_id === APP_ID);

  if (!app) {
    console.error(`❌ App "${APP_ID}" not found. Available apps:`, apps.map((a) => a.installed_app_id));
    process.exit(1);
  }

  // Auto-detect role name from app's cell_info (supports both 'elohim' and legacy 'lamad')
  const availableRoles = Object.keys(app.cell_info);
  const roleName = availableRoles.find(r => r === 'elohim') || availableRoles.find(r => r === 'lamad') || availableRoles[0];

  if (!roleName) {
    console.error(`❌ No roles found in app. cell_info keys:`, availableRoles);
    process.exit(1);
  }

  console.log(`✅ Using role: ${roleName} (available: ${availableRoles.join(', ')})`);

  const cellInfo = app.cell_info[roleName];
  if (!cellInfo || cellInfo.length === 0) {
    console.error(`❌ Role "${roleName}" has no cells`);
    process.exit(1);
  }

  const provisionedCell = cellInfo.find((c: any) => c.type === 'provisioned');
  if (!provisionedCell) {
    console.error('❌ No provisioned cell found');
    process.exit(1);
  }

  const rawCellId = (provisionedCell as any).value.cell_id;

  function toUint8Array(val: any): Uint8Array {
    if (val instanceof Uint8Array) return val;
    if (val?.type === 'Buffer' && Array.isArray(val.data)) {
      return new Uint8Array(val.data);
    }
    if (ArrayBuffer.isView(val)) return new Uint8Array(val.buffer);
    throw new Error(`Cannot convert to Uint8Array: ${JSON.stringify(val)}`);
  }

  const cellId: CellId = [toUint8Array(rawCellId[0]), toUint8Array(rawCellId[1])];
  console.log(`✅ Found cell: ${encodeHashToBase64(cellId[0]).slice(0, 20)}...`);

  // Get app auth token
  console.log('\n🔑 Getting app auth token...');
  const token = await adminWs.issueAppAuthenticationToken({
    installed_app_id: APP_ID,
    single_use: false,
    expiry_seconds: 3600,
  });
  console.log('✅ Got auth token');

  // Authorize signing credentials
  console.log('\n🔏 Authorizing signing credentials...');
  await adminWs.authorizeSigningCredentials(cellId);
  console.log('✅ Signing credentials authorized');

  // Setup app interface
  console.log('\n🔌 Setting up app interface...');
  let appPort: number;
  const existingInterfaces = await adminWs.listAppInterfaces();
  if (existingInterfaces.length > 0) {
    appPort = existingInterfaces[0].port;
    console.log(`✅ Using existing app interface on port ${appPort}`);
  } else {
    const { port } = await adminWs.attachAppInterface({ allowed_origins: '*' });
    appPort = port;
    console.log(`✅ Created app interface on port ${appPort}`);
  }

  const appWsUrl = process.env.HOLOCHAIN_APP_URL || resolveAppUrl(ADMIN_WS_URL, appPort);
  console.log(`🔌 App WebSocket: ${appWsUrl}`);

  // Connect to app websocket
  console.log('\n📡 Connecting to Holochain app...');
  let appWs: AppWebsocket;
  try {
    appWs = await AppWebsocket.connect({
      url: new URL(appWsUrl),
      wsClientOptions: { origin: 'http://localhost' },
      token: token.token,
    });
    console.log('✅ Connected to app WebSocket');
  } catch (error) {
    console.error('❌ Failed to connect to app WebSocket:', error);
    process.exit(1);
  }

  timer.endPhase('Connection Setup');

  // ========================================
  // PRE-FLIGHT VALIDATION
  // ========================================
  timer.startPhase('Pre-flight Validation');
  console.log('\n🔍 Running pre-flight validation...');

  // 1. Test websocket stability and check existing data
  console.log('   Testing websocket stability and checking existing data...');
  let wsHealthy = true;
  let existingContentCount = 0;
  let alreadySeeded = false;

  try {
    const stats = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_content_stats',
      payload: null,
    }) as { total_count: number; by_type: Record<string, number> };

    console.log('   ✅ Websocket connection stable');
    existingContentCount = stats.total_count;

    if (existingContentCount > 0) {
      console.log(`   📊 Remote already has ${existingContentCount} content entries`);

      // Show breakdown by type
      const types = Object.entries(stats.by_type || {})
        .sort((a, b) => b[1] - a[1])
        .slice(0, 5);
      for (const [type, count] of types) {
        console.log(`      • ${type}: ${count}`);
      }

      alreadySeeded = true;
    } else {
      console.log('   📊 Remote database is empty');
    }
  } catch (wsError: any) {
    wsHealthy = false;
    console.error('   ❌ Websocket connection unstable:', cleanErrorMessage(wsError));
  }

  // If already seeded and not forcing, skip seeding
  if (alreadySeeded && !FORCE_SEED) {
    console.log('\n' + '='.repeat(70));
    console.log('⏭️  SKIPPING SEED - Remote already has data');
    console.log('='.repeat(70));
    console.log(`   Content entries: ${existingContentCount}`);
    console.log('');
    console.log('   To force re-seeding, use one of:');
    console.log('     --force              (command line)');
    console.log('     FORCE_SEED=true      (environment variable)');
    console.log('='.repeat(70));

    await adminWs.client.close();
    await appWs.client.close();
    process.exit(0);  // Success exit - this is expected behavior
  }

  if (alreadySeeded && FORCE_SEED) {
    console.log('   ⚠️  --force specified, will seed anyway (may create duplicates!)');
  }

  // 2. Detect supported content formats by attempting a test create
  console.log('   Detecting supported content formats...');
  let supportedFormats: string[] = [];
  try {
    // Try to create a dummy content with an invalid format to get the error message with valid formats
    // NOTE: content_type must be valid (e.g., 'concept') so validation proceeds to content_format check
    await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'create_content',
      payload: {
        id: '__preflight_format_test__',
        content_type: 'concept',  // Must be valid so we get content_format error, not content_type error
        title: 'Format Detection Test',
        description: '',
        summary: null,
        content: 'test',
        content_format: '__invalid_format_to_detect_supported__',
        tags: [],
        source_path: null,
        related_node_ids: [],
        reach: 'private',
        estimated_minutes: null,
        thumbnail_url: null,
        metadata_json: '{}',
      },
    });
    // If it succeeded (unlikely), clean up
    console.log('   ⚠️ Could not detect formats (test succeeded unexpectedly)');
  } catch (formatError: any) {
    const errorMsg = formatError?.message || String(formatError);
    // Parse the supported formats from error like: "Must be one of: [\"markdown\", \"html\", ...]"
    const formatMatch = errorMsg.match(/Must be one of: \[(.*?)\]/);
    if (formatMatch) {
      supportedFormats = formatMatch[1]
        .replace(/\\\"/g, '"')
        .replace(/"/g, '')
        .split(',')
        .map((f: string) => f.trim());
      console.log(`   ✅ DNA supports ${supportedFormats.length} formats: ${supportedFormats.join(', ')}`);
    } else if (errorMsg.includes('already exists')) {
      // Format was accepted, so we can't detect - use a reasonable default
      console.log('   ⚠️ Could not detect formats (test ID already exists)');
    } else {
      console.log(`   ⚠️ Could not detect formats: ${cleanErrorMessage(formatError)}`);
    }
  }

  // 3. Scan content files and check for format mismatches
  console.log('   Scanning content files for format compatibility...');
  const preflightContentDir = path.join(DATA_DIR, 'content');
  const allConceptFiles = findJsonFiles(preflightContentDir);

  const formatIssues: { file: string; format: string; id: string }[] = [];
  const formatCounts: Map<string, number> = new Map();

  for (const file of allConceptFiles) {
    try {
      const content = fs.readFileSync(file, 'utf-8');
      const json = JSON.parse(content) as ConceptJson;
      const format = json.contentFormat || 'markdown';

      formatCounts.set(format, (formatCounts.get(format) || 0) + 1);

      if (supportedFormats.length > 0 && !supportedFormats.includes(format)) {
        formatIssues.push({
          file: path.basename(file),
          format,
          id: json.id || path.basename(file, '.json')
        });
      }
    } catch {
      // Skip parse errors, they'll be caught during seeding
    }
  }

  console.log('   📊 Content format distribution:');
  for (const [format, count] of [...formatCounts.entries()].sort((a, b) => b[1] - a[1])) {
    const supported = supportedFormats.length === 0 || supportedFormats.includes(format);
    const status = supported ? '✅' : '❌';
    console.log(`      ${status} ${format}: ${count} files`);
  }

  // 4. Report pre-flight results and decide whether to proceed
  let preflightPassed = true;
  const preflightIssues: string[] = [];

  if (!wsHealthy) {
    preflightPassed = false;
    preflightIssues.push('Websocket connection is unstable');
  }

  if (formatIssues.length > 0) {
    preflightPassed = false;
    preflightIssues.push(`${formatIssues.length} files use unsupported content formats`);

    // Group by format
    const byFormat = new Map<string, string[]>();
    for (const issue of formatIssues) {
      const list = byFormat.get(issue.format) || [];
      list.push(issue.id);
      byFormat.set(issue.format, list);
    }

    console.log('\n   ⚠️ UNSUPPORTED FORMATS DETECTED:');
    for (const [format, ids] of byFormat) {
      console.log(`      Format "${format}" not in DNA (${ids.length} files):`);
      for (const id of ids.slice(0, 5)) {
        console.log(`         • ${id}`);
      }
      if (ids.length > 5) {
        console.log(`         ... and ${ids.length - 5} more`);
      }
    }
    console.log('\n   💡 FIX: Either redeploy the DNA with the missing format, or');
    console.log('         change the contentFormat in these files to a supported format.');
  }

  timer.endPhase('Pre-flight Validation');

  if (!preflightPassed) {
    console.log('\n' + '='.repeat(70));
    console.log('❌ PRE-FLIGHT VALIDATION FAILED');
    console.log('='.repeat(70));
    for (const issue of preflightIssues) {
      console.log(`   • ${issue}`);
    }
    console.log('\n   Seeding would likely fail. Please fix the issues above first.');
    console.log('   To force seeding anyway, set SKIP_PREFLIGHT=true');
    console.log('='.repeat(70));

    if (process.env.SKIP_PREFLIGHT !== 'true') {
      await adminWs.client.close();
      await appWs.client.close();
      process.exit(1);
    }
    console.log('\n   ⚠️ SKIP_PREFLIGHT=true set, continuing despite failures...\n');
  } else {
    console.log('\n   ✅ Pre-flight validation passed\n');
  }

  // ========================================
  // SEED CONCEPTS (from content/ directory)
  // ========================================
  timer.startPhase('Concept Seeding');
  console.log('\n📚 Seeding concepts from data/lamad/content/...');
  const contentDir = path.join(DATA_DIR, 'content');
  const conceptFiles = findJsonFiles(contentDir);
  console.log(`   Found ${conceptFiles.length} concept files`);

  // OPTIMIZATION: Load all concepts first
  console.log('   Loading concept files...');
  const allConcepts: { concept: ConceptJson; file: string }[] = [];
  let loadErrorCount = 0;

  for (const file of conceptFiles) {
    const concept = loadJson<ConceptJson>(file);
    if (concept) {
      // Validate required fields exist
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
  console.log(`   Loaded ${allConcepts.length} concepts (${loadErrorCount} failed to load)`);

  // Apply ID filter if specified
  let filteredConcepts = allConcepts;
  if (IDS.length > 0) {
    filteredConcepts = allConcepts.filter(({ concept }) => IDS.includes(concept.id));
    console.log(`   Filtered to ${filteredConcepts.length} concepts matching IDs`);
  }

  // Apply limit if specified
  if (LIMIT > 0 && filteredConcepts.length > LIMIT) {
    filteredConcepts = filteredConcepts.slice(0, LIMIT);
    console.log(`   Limited to ${LIMIT} concepts`);
  }

  // For now, skip batch existence check - just try to create all content
  // The bulk_create_content should handle duplicates gracefully
  console.log('   Skipping existence check - will create all content (duplicates handled by zome)');
  const newConcepts = filteredConcepts;
  console.log(`   ${newConcepts.length} concepts to process`);

  // Detect if we're running against a remote conductor
  const isRemote = !ADMIN_WS_URL.includes('localhost') && !ADMIN_WS_URL.includes('127.0.0.1');

  // ========================================
  // BATCH IMPORT: Blob-based import via storage + zome
  // ========================================
  // Architecture:
  // 1. Upload items JSON to elohim-storage → get blob_hash
  // 2. Call queue_import(blob_hash) → zome stores manifest, emits signal
  // 3. Storage receives signal, sends chunks via process_import_chunk()
  // 4. Poll get_import_status() for completion
  //
  // Fallback: If no storage available, use legacy bulk_create_content
  const POLL_STATUS = process.env.SEED_POLL_STATUS !== 'false';  // Default: poll for completion
  const POLL_INTERVAL_MS = parseInt(process.env.SEED_POLL_INTERVAL || '5000', 10);
  const POLL_TIMEOUT_MS = parseInt(process.env.SEED_POLL_TIMEOUT || '300000', 10);  // 5 min default

  // Prepare all content inputs
  const allInputs = newConcepts.map(({ concept, file }) => {
    const relativePath = file.replace(DATA_DIR + '/', '');
    return conceptToInput(concept, relativePath);
  });

  const batchId = `seed-${Date.now()}`;
  let conceptSuccessCount = 0;
  let conceptErrorCount = loadErrorCount;
  let blobImportSucceeded = false;

  // ==============================================
  // DOORWAY IMPORT: All traffic through doorway
  // ==============================================
  // Architecture: Doorway is the single entry point for all external traffic
  // 1. Upload blob to doorway → doorway proxies to storage/peer-shards
  // 2. Call doorway /import/content with blob_hash → doorway calls zome
  // 3. Poll doorway for status → doorway calls zome
  //
  // Requires: DOORWAY_URL only (doorway knows where storage is)
  const USE_DOORWAY_IMPORT = !!DOORWAY_URL;

  if (USE_DOORWAY_IMPORT) {
    console.log(`   🚀 DOORWAY import mode: ${newConcepts.length} concepts`);
    console.log(`      Doorway: ${DOORWAY_URL}`);

    const doorwayClient = new DoorwayClient({
      baseUrl: DOORWAY_URL!,
      apiKey: DOORWAY_API_KEY,
      timeout: 120000, // 2 min for large blob uploads
      retries: 3,
    });

    // Check doorway health (doorway checks storage internally)
    const doorwayHealth = await doorwayClient.checkHealth();
    if (!doorwayHealth.healthy) {
      console.error(`   ❌ Doorway not healthy: ${doorwayHealth.error}`);
      console.log('   Falling back to legacy bulk_create_content...');
    } else {
      console.log(`   ✅ Doorway healthy (v${doorwayHealth.version || 'unknown'}, cache=${doorwayHealth.cacheEnabled ? 'on' : 'off'})`);

      try {
        // Step 1: Upload items JSON blob via doorway (doorway proxies to storage)
        const itemsJson = JSON.stringify(allInputs);
        const itemsBuffer = Buffer.from(itemsJson, 'utf-8');
        console.log(`   📦 Uploading blob via doorway: ${(itemsBuffer.length / 1024 / 1024).toFixed(2)} MB`);

        const blobHashForUpload = StorageClient.computeHash(itemsBuffer);
        const uploadResult = await timer.timeOperation('blob_upload', () =>
          doorwayClient.pushBlob(
            blobHashForUpload,
            itemsBuffer,
            { hash: blobHashForUpload, mimeType: 'application/json', sizeBytes: itemsBuffer.length }
          )
        );

        if (!uploadResult.success) {
          throw new Error(`Blob upload failed: ${uploadResult.error}`);
        }

        const blobHash = uploadResult.hash;
        console.log(`   ✅ Blob uploaded: ${blobHash.slice(0, 30)}...${uploadResult.cached ? ' (cached)' : ''}`);

        // Step 2: Queue import via doorway (not direct WebSocket)
        console.log(`   📤 Queuing import via doorway "${batchId}"...`);
        const queueResult = await timer.timeOperation('queue_import', () =>
          doorwayClient.queueImport('content', {
            batch_id: batchId,
            blob_hash: blobHash,
            total_items: allInputs.length,
            schema_version: 1,
          })
        ) as ImportStatusResponse & { batch_id: string; queued_count: number; processing: boolean };

        console.log(`   ✅ Import queued: ${queueResult.batch_id}`);
        console.log(`      Queued: ${queueResult.queued_count} items`);

        // Step 3: Poll for completion via doorway
        // Note: elohim-storage should be listening for ImportBatchQueued signal
        // and will call process_import_chunk() with the actual items
        if (POLL_STATUS) {
          console.log(`   ⏳ Waiting for storage to process chunks...`);
          console.log(`      (Polling interval=${POLL_INTERVAL_MS}ms, timeout=${POLL_TIMEOUT_MS}ms)`);

          const startTime = Date.now();
          let lastProcessed = 0;

          while (Date.now() - startTime < POLL_TIMEOUT_MS) {
            await new Promise(resolve => setTimeout(resolve, POLL_INTERVAL_MS));

            try {
              const status = await doorwayClient.getImportStatus('content', batchId);

              if (!status) {
                console.error(`   ❌ Batch ${batchId} not found`);
                break;
              }

              // Show progress if changed
              if (status.processed_count !== lastProcessed) {
                const pct = ((status.processed_count / status.total_items) * 100).toFixed(1);
                console.log(`      Progress: ${status.processed_count}/${status.total_items} (${pct}%) - ${status.status}`);
                lastProcessed = status.processed_count;
              }

              if (status.status === 'completed' || status.status === 'failed') {
                conceptSuccessCount = status.processed_count - status.error_count;
                conceptErrorCount = loadErrorCount + status.error_count;
                blobImportSucceeded = true;

                if (status.error_count > 0 && status.errors.length > 0) {
                  console.log(`   ⚠️ ${status.errors.length} errors during import:`);
                  for (const err of status.errors.slice(0, 5)) {
                    console.error(`      • ${err}`);
                  }
                  if (status.errors.length > 5) {
                    console.error(`      ... and ${status.errors.length - 5} more`);
                  }
                }
                break;
              }
            } catch (pollError: any) {
              console.warn(`      ⚠️ Poll failed: ${cleanErrorMessage(pollError)}`);
            }
          }

          if (!blobImportSucceeded && Date.now() - startTime >= POLL_TIMEOUT_MS) {
            console.warn(`   ⚠️ Poll timeout reached, batch may still be processing`);
            console.log(`      Check status via doorway: GET /import/content/${batchId}`);
            conceptSuccessCount = lastProcessed;
          }
        } else {
          console.log(`   ℹ️ Batch queued for background processing (not polling)`);
          console.log(`      Check status via doorway: GET /import/content/${batchId}`);
          blobImportSucceeded = true; // Consider it a success if queued
        }
      } catch (blobError: any) {
        console.error(`   ❌ Doorway import failed: ${cleanErrorMessage(blobError)}`);
        console.log('   Falling back to legacy bulk_create_content...');
      }
    }
  }

  // ==============================================
  // LEGACY MODE: bulk_create_content (inline items)
  // ==============================================
  // Used when:
  // - No DOORWAY_URL configured
  // - Doorway health check failed
  // - Doorway import failed
  if (!blobImportSucceeded) {
    console.log(`   🚀 LEGACY import mode: ${newConcepts.length} concepts via bulk_create_content`);
    if (isRemote) {
      console.log(`   🌐 Remote conductor detected (adding delays between batches)`);
    }

    const BATCH_CREATE_SIZE = 100;
    const BATCH_DELAY_MS = isRemote ? 200 : 0;

    for (let i = 0; i < newConcepts.length; i += BATCH_CREATE_SIZE) {
      const batch = newConcepts.slice(i, i + BATCH_CREATE_SIZE);
      const batchInputs = batch.map(({ concept, file }) => {
        const relativePath = file.replace(DATA_DIR + '/', '');
        return conceptToInput(concept, relativePath);
      });

      const batchNum = Math.floor(i / BATCH_CREATE_SIZE) + 1;
      const totalBatches = Math.ceil(newConcepts.length / BATCH_CREATE_SIZE);
      console.log(`   Creating batch ${batchNum}/${totalBatches} (${batchInputs.length} concepts)...`);

      try {
        const result = await timer.timeOperation('bulk_create_content', () =>
          appWs.callZome({
            cell_id: cellId,
            zome_name: ZOME_NAME,
            fn_name: 'bulk_create_content',
            payload: {
              import_id: `seed-${Date.now()}`,
              contents: batchInputs,
            },
          })
        ) as { created_count: number; errors: string[] };

        conceptSuccessCount += result.created_count;
        if (result.errors.length > 0) {
          conceptErrorCount += result.errors.length;
        }
        console.log(`      ✅ Created ${result.created_count} concepts`);

        if (BATCH_DELAY_MS > 0 && i + BATCH_CREATE_SIZE < newConcepts.length) {
          await new Promise(resolve => setTimeout(resolve, BATCH_DELAY_MS));
        }
      } catch (legacyError: any) {
        console.error(`      ❌ Legacy batch failed: ${cleanErrorMessage(legacyError)}`);
        conceptErrorCount += batch.length;
      }
    }
  }

  console.log('\n📊 Concept Seeding Complete!');
  console.log(`   ✅ Success: ${conceptSuccessCount}`);
  console.log(`   ❌ Errors: ${conceptErrorCount}`);

  timer.endPhase('Concept Seeding');

  // ========================================
  // SEED PATHS (from paths/ directory)
  // ========================================
  timer.startPhase('Path Seeding');
  console.log('\n📖 Seeding paths from data/lamad/paths/...');
  const pathsDir = path.join(DATA_DIR, 'paths');
  const pathFiles = findJsonFiles(pathsDir);
  console.log(`   Found ${pathFiles.length} path files`);

  // Load all paths first (skip index files)
  const allPaths: PathJson[] = [];
  let pathLoadErrorCount = 0;

  for (const file of pathFiles) {
    // Skip index/metadata files
    if (path.basename(file) === 'index.json' || path.basename(file).startsWith('_')) {
      console.log(`   ⏭️  Skipping metadata file: ${path.basename(file)}`);
      continue;
    }

    const pathJson = loadJson<PathJson>(file);
    if (pathJson) {
      // Validate required fields
      if (!pathJson.id || !pathJson.title) {
        const reason = !pathJson.id && !pathJson.title ? 'missing id and title'
          : !pathJson.id ? 'missing id' : 'missing title';
        console.warn(`   ⚠️ Skipping ${path.basename(file)}: ${reason}`);
        timer.recordSkipped(path.basename(file), reason);
        pathLoadErrorCount++;
        continue;
      }
      allPaths.push(pathJson);
    } else {
      pathLoadErrorCount++;
    }
  }
  console.log(`   Loaded ${allPaths.length} paths (${pathLoadErrorCount} failed to load)`);

  // Apply ID filter to paths if specified
  let filteredPaths = allPaths;
  if (IDS.length > 0) {
    filteredPaths = allPaths.filter((p) => IDS.includes(p.id));
    console.log(`   Filtered to ${filteredPaths.length} paths matching IDs`);
  }

  // Skip batch existence check for paths too - just try to create all
  console.log('   Skipping path existence check - will create all paths');
  const newPaths = filteredPaths;
  console.log(`   ${newPaths.length} paths to process`);

  let pathSuccessCount = 0;
  let pathErrorCount = pathLoadErrorCount;

  // Helper to collect steps from a path
  const collectSteps = (pathJson: PathJson): AddPathStepInput[] => {
    const steps: AddPathStepInput[] = [];

    // Legacy steps array
    if (pathJson.steps && pathJson.steps.length > 0) {
      for (const step of pathJson.steps) {
        steps.push({
          path_id: pathJson.id,
          order_index: step.order,
          step_type: 'read',
          resource_id: step.resourceId,
          step_title: step.stepTitle || null,
          step_narrative: step.stepNarrative || null,
          is_optional: step.optional || false,
        });
      }
    }
    // Flat conceptIds (new MCP format)
    else if (pathJson.conceptIds && pathJson.conceptIds.length > 0) {
      for (let i = 0; i < pathJson.conceptIds.length; i++) {
        steps.push({
          path_id: pathJson.id,
          order_index: i,
          step_type: 'read',
          resource_id: pathJson.conceptIds[i],
          step_title: null,
          step_narrative: null,
          is_optional: false,
        });
      }
    }
    // Chapters with direct steps array (e.g., elohim-protocol.json)
    else if (pathJson.chapters && pathJson.chapters.length > 0 && pathJson.chapters[0].steps) {
      let stepIndex = 0;
      for (const chapter of pathJson.chapters) {
        for (const step of chapter.steps || []) {
          steps.push({
            path_id: pathJson.id,
            order_index: stepIndex++,
            step_type: step.stepType || 'read',
            resource_id: step.resourceId,
            step_title: step.stepTitle || null,
            step_narrative: step.stepNarrative || null,
            is_optional: step.optional || false,
          });
        }
      }
    }
    // Chapters/modules/sections hierarchy (MCP generated paths)
    else if (pathJson.chapters && pathJson.chapters.length > 0) {
      let stepIndex = 0;
      for (const chapter of pathJson.chapters) {
        for (const module of chapter.modules || []) {
          for (const section of module.sections || []) {
            for (const conceptId of section.conceptIds || []) {
              steps.push({
                path_id: pathJson.id,
                order_index: stepIndex++,
                step_type: 'read',
                resource_id: conceptId,
                step_title: `${chapter.title} > ${module.title} > ${section.title}`,
                step_narrative: null,
                is_optional: false,
              });
            }
          }
        }
      }
    }

    return steps;
  };

  // Create paths and their steps
  for (const pathJson of newPaths) {
    try {
      console.log(`   📖 ${pathJson.id}: ${pathJson.title}`);

      // Create the path
      // Store chapters and audienceArchetype in metadata_json if present (preserves hierarchy for UI)
      const pathMetadata: Record<string, any> = {};
      if (pathJson.chapters && pathJson.chapters.length > 0) {
        pathMetadata.chapters = pathJson.chapters;
      }
      if (pathJson.audienceArchetype) {
        pathMetadata.audienceArchetype = pathJson.audienceArchetype;
      }

      const metadataJsonValue = Object.keys(pathMetadata).length > 0 ? JSON.stringify(pathMetadata) : null;
      console.log(`      📦 metadata_json: ${metadataJsonValue ? `${pathMetadata.chapters?.length || 0} chapters (${metadataJsonValue.length} bytes)` : 'null'}`);

      const pathInput: CreatePathInput = {
        id: pathJson.id,
        version: pathJson.version || '1.0.0',
        title: pathJson.title,
        description: pathJson.description || '',
        purpose: pathJson.purpose || null,
        difficulty: pathJson.difficulty || 'beginner',
        estimated_duration: pathJson.estimatedDuration || null,
        visibility: pathJson.visibility || 'public',
        path_type: 'learning',
        tags: pathJson.tags || [],
        metadata_json: metadataJsonValue,
      };

      const pathResult = await timer.timeOperation('path_create', () =>
        appWs.callZome({
          cell_id: cellId,
          zome_name: ZOME_NAME,
          fn_name: 'create_path',
          payload: pathInput,
        })
      );
      console.log(`      ✅ Path created: ${encodeHashToBase64(pathResult as Uint8Array).slice(0, 15)}...`);

      // OPTIMIZATION: Batch add steps for this path (in chunks to avoid timeouts)
      const steps = collectSteps(pathJson);
      if (steps.length > 0) {
        // Paths are few and steps are light - use small batches without delays
        const STEP_BATCH_SIZE = parseInt(process.env.SEED_STEP_BATCH_SIZE || '50', 10);
        let totalCreated = 0;
        let totalErrors: string[] = [];

        for (let i = 0; i < steps.length; i += STEP_BATCH_SIZE) {
          const batch = steps.slice(i, i + STEP_BATCH_SIZE);
          try {
            const stepResult = await timer.timeOperation('batch_add_path_steps', () =>
              appWs.callZome({
                cell_id: cellId,
                zome_name: ZOME_NAME,
                fn_name: 'batch_add_path_steps',
                payload: { steps: batch },
              })
            ) as { created_count: number; errors: string[] };

            totalCreated += stepResult.created_count;
            totalErrors.push(...stepResult.errors);
          } catch (batchError: any) {
            console.error(`         ⚠️ Step batch ${Math.floor(i / STEP_BATCH_SIZE) + 1} failed: ${cleanErrorMessage(batchError)}`);
          }
        }

        console.log(`      📝 Added ${totalCreated}/${steps.length} steps`);
        if (totalErrors.length > 0) {
          for (const err of totalErrors.slice(0, 2)) {
            console.error(`         ⚠️ ${err}`);
          }
        }
      }

      pathSuccessCount++;
    } catch (error: any) {
      pathErrorCount++;
      console.error(`      ❌ ${cleanErrorMessage(error)}`);
    }
  }

  console.log('\n📊 Path Seeding Complete!');
  console.log(`   ✅ Success: ${pathSuccessCount}`);
  console.log(`   ❌ Errors: ${pathErrorCount}`);

  timer.endPhase('Path Seeding');

  // ========================================
  // GET STATS
  // ========================================
  timer.startPhase('Final Stats');
  console.log('\n📈 Final stats...');
  try {
    const stats = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_content_stats',
      payload: null,
    });
    console.log('   Content Stats:', stats);

    const allPaths = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_all_paths',
      payload: null,
    }) as any;
    console.log(`   Paths: ${allPaths.total_count}`);
  } catch (error: any) {
    console.log(`   Could not fetch stats: ${cleanErrorMessage(error)}`);
  }

  timer.endPhase('Final Stats');

  // ========================================
  // WARM DOORWAY CACHE
  // ========================================
  timer.startPhase('Cache Warming');
  console.log('\n🔥 Warming doorway cache...');

  // Collect IDs from what we seeded
  const seededContentIds = newConcepts.map(({ concept }) => concept.id);
  const seededPathIds = newPaths.map(p => p.id);

  console.log(`   Content IDs to warm: ${seededContentIds.length}`);
  console.log(`   Path IDs to warm: ${seededPathIds.length} (plus any existing)`);

  try {
    const warmResult = await timer.timeOperation('warm_cache', () =>
      appWs.callZome({
        cell_id: cellId,
        zome_name: ZOME_NAME,
        fn_name: 'warm_cache',
        payload: {
          content_ids: seededContentIds,
          path_ids: null, // null = warm all paths (includes any pre-existing)
        },
      })
    ) as { content_warmed: number; paths_warmed: number; errors: string[] };

    console.log(`   ✅ Cache warmed: ${warmResult.content_warmed} content, ${warmResult.paths_warmed} paths`);

    if (warmResult.errors.length > 0) {
      console.log(`   ⚠️ ${warmResult.errors.length} warmup errors:`);
      for (const err of warmResult.errors.slice(0, 5)) {
        console.log(`      • ${err}`);
      }
      if (warmResult.errors.length > 5) {
        console.log(`      ... and ${warmResult.errors.length - 5} more`);
      }
    }
  } catch (warmError: any) {
    console.error(`   ❌ Cache warming failed: ${cleanErrorMessage(warmError)}`);
    console.log('   (This is non-fatal - cache will warm on first access)');
  }

  timer.endPhase('Cache Warming');

  // Close connections
  await adminWs.client.close();
  await appWs.client.close();

  // Set seed results for the report
  timer.setSeedResults({
    conceptsCreated: conceptSuccessCount,
    conceptErrors: conceptErrorCount,
    pathsCreated: pathSuccessCount,
    pathErrors: pathErrorCount,
  });

  // Print performance report
  timer.printReport();

  // Write report to file for snapshot:save to pick up
  const reportPath = path.join(LOCAL_DEV_DIR, 'last-seed-report.json');
  try {
    const report = timer.exportReport();
    fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
    console.log(`\n📄 Report saved to: ${reportPath}`);
  } catch (err) {
    console.warn(`   Could not save report file: ${err}`);
  }

  console.log('\n✨ Done!');
}

seed().catch((error) => {
  console.error('Fatal error:', cleanErrorMessage(error));
  process.exit(1);
});
