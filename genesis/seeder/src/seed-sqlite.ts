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
 *   SEED_IDS - Comma-separated content/path ids; when set, only those rows are
 *              seeded (every phase, blobs included). For scoped verification.
 *
 * Idempotent: each row is read back first and written only when it is missing
 * or genuinely changed (metadata.seedHash, see seed-idempotency.ts). A run
 * over current content writes nothing and re-signs nothing; the per-phase
 * `Seed summary [...]` line reports inserted/updated/unchanged/unverified/failed.
 * DRY_RUN still reads (to report the decisions) but never writes.
 *
 * Phase 3 seeds the content graph: every typed edge a content atom authors
 * (`buildRelationshipInputs`) is read back per source and written only when
 * missing or changed (`decideRelationshipAction`); SEED_IDS scopes edges by
 * their SOURCE atom.
 */

import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';
import { fileURLToPath } from 'url';

import { REACH_OPENNESS, isReach } from './generated/schema-enums.js';
import { earnedReach } from './reach-resolver.js';
import type { CreateContentInput } from './generated/create-content-input.js';
import { waitForDrain } from './wait-for-drain.js';
import { recordBlobUploadOutcome } from './blob-upload-result.js';
import { computeCid } from './doorway-client.js';
import {
  buildStakesDeclaration,
  emitStakesDeclaration,
  loadCorpusDeclaration,
  parsePeerStorageUrls,
  seedNetworkStakes,
  stakesDeclarationLogLine,
} from './corpus-trust.js';
import {
  buildContentInput,
  buildPathInput,
  buildRelationshipInputs,
  contentBodyFor,
  countItems,
  normalizeContentFormat,
  type ConceptJson,
  type PathJson,
  type RelationshipInput,
} from './content-input.js';
import { RelationshipRemapLedger } from './relationship-vocabulary.js';
import {
  addTally,
  decideRelationshipAction,
  decideSeedAction,
  deferToSteward,
  emptyTally,
  formatTally,
  reachReconcileTargets,
  withSeedHash,
  type ContentFieldPatch,
  type SeedDecision,
  type SeedTally,
  type StoredContentRow,
  type StoredHead,
  type StoredLookup,
  type StoredRelationshipRow,
} from './seed-idempotency.js';

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
const SEED_IDS: Set<string> | null = process.env.SEED_IDS
  ? new Set(process.env.SEED_IDS.split(',').map(id => id.trim()).filter(Boolean))
  : null;

/** True when the SEED_IDS scope (if any) includes this id. */
function inSeedScope(id: string): boolean {
  return SEED_IDS === null || SEED_IDS.has(id);
}

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
  // Legacy sha256- prefix ONLY on bare hex — CID-form (`baf…`) passes through
  // untouched (double-wrapping mints an address no peer accepts; backlog
  // blob-fetch-sha256-prefixed-cid-rejection).
  const normalizedHash = existingHash
    ? (/^[0-9a-fA-F]{64}$/.test(existingHash) ? `sha256-${existingHash}` : existingHash)
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
      if (!inSeedScope(json.id)) continue;

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

// `assertReach` / `earnedReach` moved to ./reach-resolver.ts so the doorway
// seed path (seed.ts) resolves reach through the SAME unit. Re-exported here
// because src/__tests__/reach-resolver.test.ts imports it from this module.
export { earnedReach };

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
        // assignment comes from JSON.parse (untyped) — coerce to string so the
        // isReach guard can narrow to Reach for the typed ordinal lookup.
        const candidateReach = String(assignment.reach);
        // Only STORE canonical values: account-package data is external input
        // that degrades gracefully — a non-canonical advisory is dropped here at
        // load time rather than stored and thrown on at resolution. Canonical
        // enforcement for AUTHORED values still HARD-FAILS in earnedReach (and
        // earnedReach keeps its own assertReach on any advisory passed directly).
        const existingOrder = existing && isReach(existing) ? REACH_OPENNESS[existing] : -1;
        const newOrder = isReach(candidateReach) ? REACH_OPENNESS[candidateReach] : 0;

        if (isReach(candidateReach) && newOrder > existingOrder) {
          overrides.set(assignment.contentId, candidateReach);
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

/**
 * The account-package archetype advisory for a content item (only with
 * --use-account-packages). Reach itself is resolved by `buildContentInput`
 * under the inverted burden: default `private`; the authored value and this
 * advisory may RAISE it; the most-open candidate wins (earnedReach, which
 * HARD-FAILS on any non-canonical value).
 */
function advisoryReachFor(contentId: string): string | undefined {
  let advisory: string | undefined;

  if (USE_ACCOUNT_PACKAGES) {
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
    advisory = reachOverrides.get(contentId);
  }

  return advisory;
}

/**
 * Compute a content-derived CIDv1 provenance anchor for a CreateContentInput.
 *
 * The anchor satisfies the storage `require_provenance` read gate
 * (`dht_anchor_hash IS NOT NULL OR p2p_published_at IS NOT NULL`) on
 * hub-optional / peer-starved stacks where the libp2p publish drain never
 * runs. Using a real CIDv1 (raw codec 0x55, sha2-256) is the HONEST
 * alternative to stamping `p2pPublishedAt` (which asserts a DHT publication
 * that never happened). The value will be superseded by the real ActionHash
 * when a `ContentCommitted` notarization later runs.
 *
 * Canonical byte selection (in priority order):
 *   1. `contentBody` UTF-8 bytes — the primary content payload.
 *   2. `blobHash`/`blobCid` as UTF-8 bytes — blob-backed / video items with
 *      no inline body; the hash string itself is canonical per-item.
 *   3. `id + title` UTF-8 — ultimate fallback for placeholder entries.
 *
 * Exported for unit testing.
 */
export async function deriveContentAnchor(input: {
  id: string;
  title: string;
  contentBody?: string;
  blobHash?: string;
  blobCid?: string;
}): Promise<string> {
  let bytes: Uint8Array;
  if (input.contentBody) {
    bytes = new TextEncoder().encode(input.contentBody);
  } else if (input.blobCid) {
    bytes = new TextEncoder().encode(input.blobCid);
  } else if (input.blobHash) {
    bytes = new TextEncoder().encode(input.blobHash);
  } else {
    bytes = new TextEncoder().encode(`${input.id}\x00${input.title}`);
  }
  return computeCid(bytes);
}

// The body rule lives with the one input builder (content-input.ts);
// re-exported because seed-epr-atom.ts reads it from here.
export { contentBodyFor };

/**
 * Build the `deriveContentAnchor` input for a raw content seed JSON, mirroring
 * exactly what `buildContentInput` feeds it during a seed run.
 *
 * CAVEAT: the live seed path injects `blobHash` from `uploadedContentBlobs`
 * (see the `buildContentInput` call site) AFTER building this input; a raw
 * file can't see that. `deriveContentAnchor` prefers `contentBody`, so this
 * only matters for a content JSON with NO inline `content` that relies on an
 * uploaded blob — for such a file this helper's anchor would silently diverge
 * from the seeded row's `dht_anchor_hash`. Callers (seed-epr-atom.ts) must
 * only use it for inline-content files, or plumb the blob hash explicitly.
 */
export function contentAnchorInput(json: ConceptJson): {
  id: string;
  title: string;
  contentBody?: string;
  blobHash?: string;
  blobCid?: string;
} {
  return {
    id: json.id,
    title: json.title,
    contentBody: contentBodyFor(json),
    blobHash: json.blobHash ?? json.blob_hash ?? undefined,
    blobCid: json.blobCid ?? undefined,
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
      if (!inSeedScope(json.id)) continue;

      paths.push(json);
    } catch (err) {
      console.warn(`   Error loading ${file}: ${err}`);
    }
  }

  return paths;
}

// ============================================================================
// API Client
// ============================================================================

/**
 * Retry classification for storage requests.
 *
 * We retry on:
 *  - Network errors (fetch threw) — TCP reset, DNS hiccup, transient pod restart
 *  - HTTP 5xx — storage temporarily unable to serve
 *  - HTTP 429 — rate limit (storage may grow one in future)
 *  - Body containing "database is locked" / "database is busy" — SQLITE_BUSY
 *    surfaces as 500 today; the body text is the canonical signal.
 *
 * We do NOT retry on:
 *  - HTTP 4xx (other than 429) — client-side bug, retry won't help
 *
 * In genesis #956 the storage WAL fix alone eliminated SQLITE_BUSY at the
 * root, so this retry path was never exercised. It remains as
 * defense-in-depth for genuine transient failures.
 */
function isRetryable(status: number | null, bodyText: string): boolean {
  if (status === null) return true; // network error
  if (status === 429) return true;
  if (status >= 500 && status < 600) return true;
  // Be defensive: some upstreams flatten errors to 200 with body — sniff.
  if (/database is (locked|busy)|SQLITE_BUSY/i.test(bodyText)) return true;
  return false;
}

/**
 * Sleep with jitter to spread retry pressure across concurrent batches.
 */
async function backoffSleep(attempt: number): Promise<void> {
  // 500ms, 1.5s, 4s, 8s, then capped at 15s. Plus 0-50% jitter.
  const baseMs = Math.min(500 * Math.pow(3, attempt), 15000);
  const jitter = Math.random() * baseMs * 0.5;
  await new Promise(resolve => setTimeout(resolve, baseMs + jitter));
}

async function seedContent(items: CreateContentInput[]): Promise<{ inserted: number; skipped: number; errors: string[] }> {
  if (DRY_RUN) {
    console.log(`   [DRY RUN] Would seed ${items.length} content items`);
    return { inserted: items.length, skipped: 0, errors: [] };
  }

  const MAX_ATTEMPTS = 6;
  let lastErr: Error | null = null;

  for (let attempt = 0; attempt < MAX_ATTEMPTS; attempt++) {
    let status: number | null = null;
    let bodyText = '';
    try {
      const response = await fetch(`${STORAGE_URL}/db/content/bulk`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
        body: JSON.stringify(items),
      });
      status = response.status;

      if (response.ok) {
        const data = await response.json();
        assertResponseShape<{ inserted: number; skipped: number; errors: string[] }>(
          data, ['inserted', 'skipped', 'errors'], '/db/content/bulk'
        );
        if (attempt > 0) {
          console.log(`     ↻ recovered after ${attempt} retr${attempt === 1 ? 'y' : 'ies'}`);
        }
        return data;
      }

      bodyText = await response.text();
      lastErr = new Error(`HTTP ${status}: ${bodyText}`);
    } catch (err) {
      // Network-level failure (fetch threw)
      lastErr = err instanceof Error ? err : new Error(String(err));
      bodyText = lastErr.message;
    }

    if (!isRetryable(status, bodyText) || attempt === MAX_ATTEMPTS - 1) break;

    console.log(`     ↻ retryable error (attempt ${attempt + 1}/${MAX_ATTEMPTS}): ${
      status !== null ? `HTTP ${status}` : 'network'
    } — backing off`);
    await backoffSleep(attempt);
  }

  throw lastErr ?? new Error('seedContent: unknown failure');
}

/**
 * Re-notarize the authored `reach` onto rows whose stored reach differs.
 *
 * Callers pass ONLY the gated targets (`reachReconcileTargets`): updated rows
 * whose stored reach differs from the authored one. A reach PATCH routes through
 * the conductor and re-authors the entry (update_entry) — it re-signs — so it
 * is never sent for an inserted row (written with the authored reach), an
 * unchanged row, or a row the seeder could not read. It used to be sent for
 * EVERY id in every batch on every run; that was the reseal.
 *
 * Provenance itself is carried at CREATE time via `dhtAnchorHash` in the bulk
 * POST body (see `deriveContentAnchor`); pre-existing NULL-provenance rows are
 * NOT healed here (`UpdateContentInputView` does not expose `dhtAnchorHash`).
 *
 * Best-effort: a failed PATCH is logged but does not abort seeding.
 */
async function stampProvenance(
  items: Array<{ id: string; reach?: string | null }>,
): Promise<{ stamped: number; failed: number; skipped: number }> {
  // Filter to only items that carry a reach value — nothing to PATCH otherwise.
  const reachItems = items.filter(item => Boolean(item.reach));
  if (DRY_RUN || reachItems.length === 0) {
    return { stamped: DRY_RUN ? reachItems.length : 0, failed: 0, skipped: 0 };
  }

  let stamped = 0;
  let failed = 0;

  // Substrate-correct storage routes reach-carrying PATCHes through the
  // conductor (re-notarize in the DHT) — which FAILS for bulk-seeded rows
  // that were never DHT-authored, and EACH failure costs a conductor
  // round-trip. Genesis #1121/#1122 both burned ~60min in this loop
  // (sequential, no timeout, fallback only on non-OK — a thrown/hung fetch
  // skipped the fallback entirely) and hit the pipeline cap. Hardening:
  // per-request timeout, fallback on BOTH non-OK and thrown errors, a
  // circuit breaker that stops paying the conductor cost after it has
  // clearly failed, and bounded concurrency.
  const PATCH_TIMEOUT_MS = 5000;
  const REACH_BREAKER_THRESHOLD = 5;
  const CONCURRENCY = 10;
  let consecutiveReachFailures = 0;
  let reachCircuitOpen = false;
  let reachCircuitSkipped = 0;

  const patchContent = async (id: string, body: Record<string, string>): Promise<boolean> => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), PATCH_TIMEOUT_MS);
    try {
      const response = await fetch(`${STORAGE_URL}/db/content/${encodeURIComponent(id)}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal: controller.signal,
      });
      return response.ok;
    } catch {
      return false;
    } finally {
      clearTimeout(timer);
    }
  };

  const reconcileReach = async (item: { id: string; reach?: string | null }): Promise<void> => {
    if (reachCircuitOpen) {
      reachCircuitSkipped++;
      return;
    }
    const ok = await patchContent(item.id, { reach: item.reach as string });
    if (ok) {
      consecutiveReachFailures = 0;
      stamped++;
      return;
    }
    consecutiveReachFailures++;
    failed++;
    if (consecutiveReachFailures >= REACH_BREAKER_THRESHOLD && !reachCircuitOpen) {
      reachCircuitOpen = true;
      console.warn(
        `   stampProvenance: reach circuit OPEN after ${consecutiveReachFailures} consecutive conductor-path failures — remaining ${reachItems.length - stamped - failed} rows skipped (reach re-notarization needs conductor; not DHT-anchored)`,
      );
    }
  };

  for (let i = 0; i < reachItems.length; i += CONCURRENCY) {
    await Promise.all(reachItems.slice(i, i + CONCURRENCY).map(reconcileReach));
  }

  if (reachCircuitSkipped > 0) {
    console.warn(
      `   stampProvenance: ${reachCircuitSkipped} row(s) skipped after circuit open — reach re-notarization needs the conductor and these rows are not DHT-anchored (bulk-seed anchor gap)`,
    );
  }

  return { stamped, failed, skipped: reachCircuitSkipped };
}

// ============================================================================
// Idempotent seeding — read back, decide, write only what changed
// ============================================================================

const LOOKUP_CONCURRENCY = 16;
const LOOKUP_TIMEOUT_MS = 20_000;
const LOOKUP_MAX_ATTEMPTS = 3;
const UPDATE_CONCURRENCY = 8;

/**
 * Read one row back: GET /db/content/{id} (no identity).
 *
 * 200 → found; 404 → missing (absent, or present below the serving floor —
 * the bulk insert then skips it); any other 4xx (the reach gate's 403 for
 * rows above `public`, device policy, prerequisite gate) → unreadable, left
 * untouched; network / 5xx → retried, then error.
 *
 * NOTE: a 404 on this route also triggers the peer's demand auto-pin and a
 * bounded (≤5s) P2P resolve for the id — the cost of a read-miss here.
 */
/**
 * GET /db/content/{id}/head?election=live — whether a steward earned the row's
 * canonical head, resolved by the peer's own conductor rather than the peer's cached
 * column (a peer that adopted the head through an unordered path can hold a stale
 * "not earned"). Read only for rows the repository has changed. Any failure reads
 * as "not earned", so the seeder falls back to its normal update (which storage
 * itself refuses to let outrank an earned head).
 */
async function lookupStoredHead(id: string): Promise<StoredHead | undefined> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), LOOKUP_TIMEOUT_MS);
  try {
    const response = await fetch(`${STORAGE_URL}/db/content/${encodeURIComponent(id)}/head?election=live`, {
      signal: controller.signal,
    });
    if (response.status !== 200) return undefined;
    const body = (await response.json()) as { earned?: unknown; earnedBy?: unknown };
    return {
      earned: body.earned === true,
      earnedBy: typeof body.earnedBy === 'string' ? body.earnedBy : undefined,
    };
  } catch {
    return undefined;
  } finally {
    clearTimeout(timer);
  }
}

async function lookupStoredRow(id: string): Promise<StoredLookup> {
  let lastMessage = 'unknown failure';
  for (let attempt = 0; attempt < LOOKUP_MAX_ATTEMPTS; attempt++) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), LOOKUP_TIMEOUT_MS);
    let status: number | null = null;
    let bodyText = '';
    try {
      const response = await fetch(`${STORAGE_URL}/db/content/${encodeURIComponent(id)}`, {
        signal: controller.signal,
      });
      status = response.status;
      bodyText = await response.text();
      if (status === 200) {
        const row = JSON.parse(bodyText) as StoredContentRow;
        if (!row || typeof row !== 'object' || row.id !== id || typeof row.title !== 'string') {
          return { kind: 'error', message: `GET ${id}: unexpected body shape` };
        }
        return { kind: 'found', row };
      }
      if (status === 404) return { kind: 'missing' };
      if (status >= 400 && status < 500 && status !== 429) {
        let requiredReach: string | undefined;
        try {
          requiredReach = (JSON.parse(bodyText) as { requiredReach?: string }).requiredReach;
        } catch {
          /* non-JSON refusal body */
        }
        return { kind: 'unreadable', status, requiredReach };
      }
      lastMessage = `GET ${id}: HTTP ${status}: ${bodyText.slice(0, 200)}`;
    } catch (err) {
      lastMessage = `GET ${id}: ${err instanceof Error ? err.message : String(err)}`;
      bodyText = lastMessage;
    } finally {
      clearTimeout(timer);
    }
    if (!isRetryable(status, bodyText) || attempt === LOOKUP_MAX_ATTEMPTS - 1) break;
    await backoffSleep(attempt);
  }
  return { kind: 'error', message: lastMessage };
}

/** Run `fn` over `items` with at most `limit` in flight, preserving order. */
async function mapBounded<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
  const results = new Array<R>(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const i = next++;
      results[i] = await fn(items[i]);
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return results;
}

/**
 * PATCH the authored, non-notarized fields of a changed row (diesel path:
 * title/description/contentBody/contentFormat/tags + metadata merge incl. the
 * new seedHash). Never carries reach/blobHash — see ContentFieldPatch.
 */
async function patchContentFields(id: string, patch: ContentFieldPatch): Promise<string | null> {
  let lastErr = 'unknown failure';
  for (let attempt = 0; attempt < LOOKUP_MAX_ATTEMPTS; attempt++) {
    let status: number | null = null;
    let bodyText = '';
    try {
      const response = await fetch(`${STORAGE_URL}/db/content/${encodeURIComponent(id)}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(patch),
      });
      status = response.status;
      if (response.ok) return null;
      bodyText = await response.text();
      lastErr = `PATCH ${id}: HTTP ${status}: ${bodyText.slice(0, 200)}`;
    } catch (err) {
      lastErr = `PATCH ${id}: ${err instanceof Error ? err.message : String(err)}`;
      bodyText = lastErr;
    }
    if (!isRetryable(status, bodyText) || attempt === LOOKUP_MAX_ATTEMPTS - 1) break;
    await backoffSleep(attempt);
  }
  return lastErr;
}

/**
 * Seed one batch idempotently: read every row back, decide per row, then
 * insert the missing (stamped with seedHash), update the changed, and leave
 * the current and the unreadable alone. Reach is re-notarized only for updated
 * rows whose stored reach differs.
 */
async function seedBatchIdempotent(
  inputs: CreateContentInput[],
): Promise<{ tally: SeedTally; errors: string[] }> {
  const tally = emptyTally();
  const errors: string[] = [];

  const lookups = await mapBounded(inputs, LOOKUP_CONCURRENCY, input => lookupStoredRow(input.id));
  const firstPass: SeedDecision[] = inputs.map((input, i) => decideSeedAction(input, lookups[i]));
  // A changed row whose head a steward earned is the steward's to change.
  const decisions: SeedDecision[] = await mapBounded(firstPass, LOOKUP_CONCURRENCY, async d =>
    d.kind === 'update' ? deferToSteward(d, await lookupStoredHead(d.id)) : d,
  );
  const inputById = new Map(inputs.map(input => [input.id, input]));

  const toInsert: CreateContentInput[] = [];
  const toUpdate: Array<Extract<SeedDecision, { kind: 'update' }>> = [];
  for (const d of decisions) {
    switch (d.kind) {
      case 'insert':
        toInsert.push(withSeedHash(inputById.get(d.id)!, d.seedHash));
        break;
      case 'update':
        toUpdate.push(d);
        break;
      case 'unchanged':
        tally.unchanged++;
        break;
      case 'unverified':
        tally.unverified++;
        break;
      case 'stewarded':
        tally.stewarded++;
        console.log(
          `     = ${d.id}: changed in the repository, but its head was earned` +
            `${d.earnedBy ? ` by ${d.earnedBy}` : ''} — left for steward-publish`,
        );
        break;
      case 'failed':
        tally.failed++;
        errors.push(d.message);
        break;
    }
  }
  for (const d of toUpdate) {
    const note = d.reachPatch ? ` reach→${d.reachPatch}` : '';
    const unpatchable = d.unpatchable.length ? ` (not patchable, left as stored: ${d.unpatchable.join(', ')})` : '';
    console.log(`     ~ ${d.id}: changed (${d.via})${note}${unpatchable}`);
  }

  if (DRY_RUN) {
    tally.inserted += toInsert.length;
    tally.updated += toUpdate.length;
    return { tally, errors };
  }

  if (toInsert.length > 0) {
    const result = await seedContent(toInsert);
    tally.inserted += result.inserted;
    // Read back as missing but already present: the row exists below the
    // external serving floor (no provenance yet). Bulk create never updates, so
    // it is left as it is — the seeder could not read it to compare.
    tally.unverified += result.skipped;
    tally.failed += result.errors.length;
    errors.push(...result.errors);
  }

  const updated = new Set<string>();
  await mapBounded(toUpdate, UPDATE_CONCURRENCY, async d => {
    tally.patches++;
    const err = await patchContentFields(d.id, d.patch);
    if (err) {
      tally.failed++;
      errors.push(err);
      return;
    }
    tally.updated++;
    updated.add(d.id);
  });

  const reachTargets = reachReconcileTargets(toUpdate).filter(t => updated.has(t.id));
  if (reachTargets.length > 0) {
    const stamp = await stampProvenance(reachTargets);
    tally.patches += stamp.stamped + stamp.failed;
    if (stamp.failed + stamp.skipped > 0) {
      console.warn(`     reach re-notarization: ${stamp.stamped} ok, ${stamp.failed} failed, ${stamp.skipped} skipped`);
    }
  }

  return { tally, errors };
}

// ============================================================================
// Content-graph edges — read back per source, write only what changed
// ============================================================================

/**
 * Outgoing-edge page size per source. Storage clamps a list page to 500
 * (`MAX_LIST_LIMIT` in elohim-storage relationships_diesel.rs) and caps an atom's
 * authored edges at 256, so one page holds every edge a source can author.
 */
const RELATIONSHIP_LIST_LIMIT = 500;
const RELATIONSHIP_WRITE_BATCH = 500;

type RelationshipLookup =
  | { kind: 'ok'; rows: StoredRelationshipRow[] }
  /** The reach gate refused the anonymous seeder: it cannot compare, so it writes nothing (as for content). */
  | { kind: 'unreadable'; status: number }
  | { kind: 'error'; message: string };

/** Storage keys an edge on (source, target, type); so does the seeder. */
const edgeKey = (e: { relationshipType: string; targetId: string }) => `${e.relationshipType}|${e.targetId}`;

/** GET /db/relationships?contentId={id}&direction=outgoing — the edges this source already has. */
async function listOutgoingRelationships(sourceId: string): Promise<RelationshipLookup> {
  const url =
    `${STORAGE_URL}/db/relationships?contentId=${encodeURIComponent(sourceId)}` +
    `&direction=outgoing&limit=${RELATIONSHIP_LIST_LIMIT}`;
  let lastMessage = 'unknown failure';
  for (let attempt = 0; attempt < LOOKUP_MAX_ATTEMPTS; attempt++) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), LOOKUP_TIMEOUT_MS);
    let status: number | null = null;
    let bodyText = '';
    try {
      const response = await fetch(url, { signal: controller.signal });
      status = response.status;
      bodyText = await response.text();
      if (status === 200) {
        const body = JSON.parse(bodyText) as { items?: StoredRelationshipRow[]; limit?: number };
        // Storage falls back to a zero-limit default query when it cannot parse
        // the query string — that would read as "no edges" and rewrite them all.
        if (!Array.isArray(body.items) || body.limit !== RELATIONSHIP_LIST_LIMIT) {
          return { kind: 'error', message: `GET relationships ${sourceId}: unexpected body (limit=${body.limit})` };
        }
        return { kind: 'ok', rows: body.items.filter(r => r.sourceId === sourceId) };
      }
      if (status === 401 || status === 403) return { kind: 'unreadable', status };
      lastMessage = `GET relationships ${sourceId}: HTTP ${status}: ${bodyText.slice(0, 200)}`;
    } catch (err) {
      lastMessage = `GET relationships ${sourceId}: ${err instanceof Error ? err.message : String(err)}`;
      bodyText = lastMessage;
    } finally {
      clearTimeout(timer);
    }
    if (!isRetryable(status, bodyText) || attempt === LOOKUP_MAX_ATTEMPTS - 1) break;
    await backoffSleep(attempt);
  }
  return { kind: 'error', message: lastMessage };
}

/** POST /db/relationships/bulk — storage upserts on (source, target, type). */
async function bulkCreateRelationships(
  items: RelationshipInput[],
): Promise<{ created: number; updated: number; errors: string[] }> {
  const MAX_ATTEMPTS = 6;
  let lastErr: Error | null = null;
  for (let attempt = 0; attempt < MAX_ATTEMPTS; attempt++) {
    let status: number | null = null;
    let bodyText = '';
    try {
      const response = await fetch(`${STORAGE_URL}/db/relationships/bulk`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
        body: JSON.stringify(items),
      });
      status = response.status;
      if (response.ok) {
        const data = await response.json();
        assertResponseShape<{ created: number; updated: number; errors: string[] }>(
          data, ['created', 'updated', 'errors'], '/db/relationships/bulk',
        );
        return data;
      }
      bodyText = await response.text();
      lastErr = new Error(`HTTP ${status}: ${bodyText}`);
    } catch (err) {
      lastErr = err instanceof Error ? err : new Error(String(err));
      bodyText = lastErr.message;
    }
    if (!isRetryable(status, bodyText) || attempt === MAX_ATTEMPTS - 1) break;
    await backoffSleep(attempt);
  }
  throw lastErr ?? new Error('bulkCreateRelationships: unknown failure');
}

interface RelationshipTally {
  inserted: number;
  updated: number;
  unchanged: number;
  failed: number;
  /** The source is above the seeder's reach (403): its edges cannot be compared, so none are written. */
  unverified: number;
  /** Stored reach differs from the source atom's — the bulk route cannot carry reach (reported, never written). */
  reachUncarried: number;
}

function formatRelationshipTally(t: RelationshipTally, remaps: RelationshipRemapLedger): string {
  const remapSummary = remaps.summary();
  return (
    `Seed summary [relationships]: inserted=${t.inserted} updated=${t.updated} ` +
    `unchanged=${t.unchanged} unverified=${t.unverified} failed=${t.failed} reachUncarried=${t.reachUncarried} ` +
    `remapped=${remaps.total()}${remapSummary ? ` (${remapSummary})` : ''}`
  );
}

/**
 * Seed every edge the given atoms author: read each source's outgoing edges,
 * decide per edge, bulk-upsert only the missing and changed ones.
 */
async function seedRelationships(
  concepts: ConceptJson[],
  remaps: RelationshipRemapLedger,
): Promise<{ tally: RelationshipTally; errors: string[] }> {
  const tally: RelationshipTally = {
    inserted: 0,
    updated: 0,
    unchanged: 0,
    failed: 0,
    unverified: 0,
    reachUncarried: 0,
  };
  const errors: string[] = [];

  const bySource = concepts
    .map(c => ({ id: c.id, edges: buildRelationshipInputs(c, { advisoryReach: advisoryReachFor(c.id), remaps }) }))
    .filter(s => s.edges.length > 0);
  const edgeCount = bySource.reduce((n, s) => n + s.edges.length, 0);
  console.log(`   ${formatCount(edgeCount)} edges authored by ${formatCount(bySource.length)} atoms`);

  const lookups = await mapBounded(bySource, LOOKUP_CONCURRENCY, s => listOutgoingRelationships(s.id));
  const toWrite: RelationshipInput[] = [];
  let plannedInserts = 0;
  bySource.forEach((source, i) => {
    const lookup = lookups[i];
    if (lookup.kind === 'unreadable') {
      tally.unverified += source.edges.length;
      return;
    }
    if (lookup.kind === 'error') {
      tally.failed += source.edges.length;
      errors.push(lookup.message);
      return;
    }
    const stored = new Map(lookup.rows.map(r => [edgeKey(r), r]));
    for (const edge of source.edges) {
      const decision = decideRelationshipAction(edge, stored.get(edgeKey(edge)));
      if (decision.kind !== 'insert' && decision.reachDiffers) tally.reachUncarried++;
      if (decision.kind === 'unchanged') {
        tally.unchanged++;
        continue;
      }
      if (decision.kind === 'insert') {
        plannedInserts++;
      } else {
        console.log(`     ~ ${edge.sourceId} -${edge.relationshipType}-> ${edge.targetId}: changed (${decision.changed.join(', ')})`);
      }
      toWrite.push(edge);
    }
  });

  if (DRY_RUN) {
    tally.inserted += plannedInserts;
    tally.updated += toWrite.length - plannedInserts;
    return { tally, errors };
  }

  for (let i = 0; i < toWrite.length; i += RELATIONSHIP_WRITE_BATCH) {
    const batch = toWrite.slice(i, i + RELATIONSHIP_WRITE_BATCH);
    try {
      const result = await bulkCreateRelationships(batch);
      tally.inserted += result.created;
      tally.updated += result.updated;
      tally.failed += result.errors.length;
      errors.push(...result.errors);
    } catch (err) {
      tally.failed += batch.length;
      errors.push(`relationships batch ${i / RELATIONSHIP_WRITE_BATCH + 1}: ${err}`);
    }
  }
  return { tally, errors };
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
  let contentTally: SeedTally = emptyTally();
  let pathTally: SeedTally = emptyTally();
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
        const exists = await blobExists(blob.hash);
        if (exists) {
          console.log(`   ✓ ${app.id}: already exists (${blob.hash.slice(0, 16)}...)`);
          blobsSkipped++;
          recordBlobUploadOutcome(uploadedContentBlobs, app.id, { kind: 'existed', hash: blob.hash });
        } else {
          const hash = await uploadBlob(blob.data, 'application/zip', app.id);
          if (hash) {
            console.log(`   ✓ ${app.id}: uploaded ${(blob.data.length / 1024 / 1024).toFixed(2)} MB`);
            blobsUploaded++;
            recordBlobUploadOutcome(uploadedContentBlobs, app.id, { kind: 'uploaded', hash: blob.hash });
          } else {
            console.warn(`   ✗ ${app.id}: upload failed; skipping blobHash linkage`);
            blobsFailed++;
            recordBlobUploadOutcome(uploadedContentBlobs, app.id, { kind: 'failed' });
          }
        }
      } else {
        // Check if there's a hash reference we should verify
        const existingHash = app.blobHash || app.blob_hash;
        if (existingHash) {
          // Legacy sha256- prefix ONLY on bare hex — never double-wrap a CID.
          const normalizedHash = /^[0-9a-fA-F]{64}$/.test(existingHash) ? `sha256-${existingHash}` : existingHash;
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
          recordBlobUploadOutcome(uploadedThumbnails, pathItem.thumbnailUrl, { kind: 'existed', hash: thumbnail.hash });
        } else {
          const hash = await uploadBlob(thumbnail.data, thumbnail.mimeType, `${pathItem.id} thumbnail`);
          if (hash) {
            console.log(`   ✓ ${pathItem.id}: thumbnail uploaded ${(thumbnail.data.length / 1024).toFixed(1)} KB`);
            blobsUploaded++;
            recordBlobUploadOutcome(uploadedThumbnails, pathItem.thumbnailUrl, { kind: 'uploaded', hash: thumbnail.hash });
          } else {
            console.warn(`   ✗ ${pathItem.id}: thumbnail upload failed; skipping blobHash linkage`);
            blobsFailed++;
            recordBlobUploadOutcome(uploadedThumbnails, pathItem.thumbnailUrl, { kind: 'failed' });
          }
        }
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

    if (LIMIT > 0 && content.length > LIMIT) {
      console.log(`   Limiting to ${LIMIT} items`);
      content = content.slice(0, LIMIT);
    }

    console.log(`\nTransforming content...`);
    const contentInputs = await Promise.all(content.map(async c => {
      // One builder (content-input.ts) — the steward publish path builds the
      // same input, so both agree on the seed hash. The linked blob hash is the
      // one Phase 0 verified or uploaded.
      const input = buildContentInput(c, {
        advisoryReach: advisoryReachFor(c.id),
        blobHash: uploadedContentBlobs.get(c.id),
      });
      // Attach a content-derived CIDv1 anchor so the row satisfies the
      // require_provenance read gate at ingest (honest alternative to the
      // p2pPublishedAt false-stamp on hub-optional/peer-starved stacks).
      input.dhtAnchorHash = await deriveContentAnchor(input);
      return input;
    }));
    console.log(`   Transformed ${formatCount(contentInputs.length)} items`);

    // Inverted-burden surprise guard: the `private` default silently migrates
    // ungraded content to private. Warn the operator when a large fraction of
    // resolved reach landed on `private` AND account-packages is off (so they
    // can add reach fields or pass --use-account-packages if unexpected).
    // Robust by construction — a pure count over the transformed inputs.
    if (!USE_ACCOUNT_PACKAGES && contentInputs.length > 0) {
      const privateCount = contentInputs.filter(c => c.reach === 'private').length;
      if (privateCount / contentInputs.length >= 0.5) {
        console.warn(
          `\nWARNING: ${privateCount}/${contentInputs.length} content items resolved to 'private' ` +
          `(inverted-burden default; no authored reach). ` +
          `Add reach fields or use --use-account-packages if unexpected.`
        );
      }
    }

    console.log(`\nSeeding content to database...`);
    const BATCH_SIZE = 100;
    const BATCH_DELAY_MS = 200; // Let SQLite breathe between batches
    const totalBatches = Math.ceil(contentInputs.length / BATCH_SIZE);
    for (let i = 0; i < contentInputs.length; i += BATCH_SIZE) {
      const batch = contentInputs.slice(i, i + BATCH_SIZE);
      const batchNum = Math.floor(i / BATCH_SIZE) + 1;
      try {
        const { tally, errors } = await seedBatchIdempotent(batch);
        contentTally = addTally(contentTally, tally);
        totalErrors.push(...errors);
        console.log(
          `   Batch ${batchNum}/${totalBatches}: ${tally.inserted} inserted, ${tally.updated} updated, ` +
          `${tally.unchanged} unchanged, ${tally.unverified} unverified, ${tally.failed} failed`,
        );
      } catch (err) {
        console.error(`   Batch ${batchNum}/${totalBatches} failed: ${err}`);
        totalErrors.push(`Batch ${batchNum}: ${err}`);
        contentTally.failed += batch.length;
      }
      // Brief pause between batches to avoid SQLite "database is locked" errors
      if (i + BATCH_SIZE < contentInputs.length) {
        await new Promise(resolve => setTimeout(resolve, BATCH_DELAY_MS));
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
    const pathContentInputs = await Promise.all(paths.map(async p => {
      const thumbnailHash =
        p.thumbnailUrl && uploadedThumbnails.has(p.thumbnailUrl)
          ? uploadedThumbnails.get(p.thumbnailUrl)
          : undefined;
      const input = buildPathInput(p, { thumbnailHash });
      // Attach a content-derived CIDv1 anchor for the path row (same
      // require_provenance gate applies to paths seeded as content nodes).
      input.dhtAnchorHash = await deriveContentAnchor(input);
      return input;
    }));

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
      const { tally, errors } = await seedBatchIdempotent(pathContentInputs);
      pathTally = addTally(pathTally, tally);
      totalErrors.push(...errors);
    } catch (err) {
      console.error(`   Path seeding failed: ${err}`);
      totalErrors.push(`Paths: ${err}`);
      pathTally.failed += pathContentInputs.length;
    }

    console.log(`\nPath seeding complete in ${pathTimer.elapsed()}`);
  }

  // ========================================
  // Phase 3: Seed the content graph (typed edges)
  // ========================================
  let relationshipTally: RelationshipTally | null = null;
  const relationshipRemaps = new RelationshipRemapLedger();
  if (!PATHS_ONLY) {
    console.log(`\n${'='.repeat(70)}`);
    console.log(`Phase 3: Seeding Relationships`);
    console.log(`${'='.repeat(70)}`);

    const relTimer = new Timer();
    let sources = loadContentFiles();
    if (LIMIT > 0 && sources.length > LIMIT) sources = sources.slice(0, LIMIT);
    try {
      const { tally, errors } = await seedRelationships(sources, relationshipRemaps);
      relationshipTally = tally;
      totalErrors.push(...errors);
    } catch (err) {
      console.error(`   Relationship seeding failed: ${err}`);
      totalErrors.push(`Relationships: ${err}`);
    }
    console.log(`\nRelationship seeding complete in ${relTimer.elapsed()}`);
  }

  const runTally = addTally(contentTally, pathTally);
  const totalInserted = runTally.inserted;
  // Present on the peer after this run, not newly inserted (was: bulk "skipped").
  const totalSkipped = runTally.updated + runTally.unchanged + runTally.unverified;

  // ========================================
  // Wait for P2P drain
  // ========================================
  // After content and paths are posted to the storage SQLite, the
  // storage node's drain loop (Phase C) publishes rows to the DHT as
  // peers become available. The pipeline must not declare success
  // until drain.pending === 0, otherwise a "caught up" signal can race
  // ahead of actual publish completion. See genesis/plans seeder DHT
  // drain plan, Phase E2.
  if (!DRY_RUN && totalInserted > 0) {
    console.log(`\n${'='.repeat(70)}`);
    console.log(`Waiting for P2P drain to complete`);
    console.log(`${'='.repeat(70)}`);
    try {
      const drainTimeoutMs = parseInt(process.env.DRAIN_TIMEOUT_MS || '', 10) || 10 * 60_000;
      await waitForDrain(STORAGE_URL!, {
        timeoutMs: drainTimeoutMs,
        // totalInserted is the lower bound we expect the storage node's
        // drain queue to observe. Using it here guards against the
        // zero-content edge case and catches drift if the count query
        // is broken upstream.
        expectedMinTotal: totalInserted,
      });
    } catch (err) {
      console.error(`\n❌ DRAIN FAILED: ${err instanceof Error ? err.message : err}\n`);
      throw err;
    }
  } else if (DRY_RUN) {
    console.log(`\nSkipping waitForDrain (dry run)`);
  } else {
    console.log(`\nSkipping waitForDrain (nothing inserted)`);
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

  console.log(`\nSeeding results:${DRY_RUN ? ' (DRY RUN — decisions only, nothing written)' : ''}`);
  if (!PATHS_ONLY) console.log(`   ${formatTally('content', contentTally)}`);
  if (!CONTENT_ONLY) console.log(`   ${formatTally('paths', pathTally)}`);
  if (relationshipTally) console.log(`   ${formatRelationshipTally(relationshipTally, relationshipRemaps)}`);
  console.log(`   Total errors: ${totalErrors.length}`);
  console.log(`   Total time: ${timer.elapsed()}`);

  // ========================================
  // STAKES DECLARATION (deploy-time mint + Q6 write-route fan-out)
  // ========================================
  // Twin of the block in seed.ts (the doorway-path entrypoint): the alpha
  // pipeline seeds THROUGH THIS FILE (scripts/ci/seed-genesis-peer.sh ->
  // seed-sqlite.ts), so the corpus trustBootstrap grant must mint and fan
  // out here too or the deployed fleet never receives it (edge #1361/genesis
  // #1486 proved the gap: grants seeded locally, silence in CI). Membership
  // in the content set does not gate the grant, so this runs regardless of
  // per-item errors; DRY_RUN skips the network fan-out but still mints.
  try {
    const loadedCorpus = loadCorpusDeclaration(DATA_DIR);
    if (!loadedCorpus?.declaration.trustBootstrap) {
      console.log(
        `\n🔒 No corpus trustBootstrap in ${DATA_DIR} — nothing minted (peers stay fail-closed Bootstrap)`,
      );
    } else {
      const artifact = buildStakesDeclaration(loadedCorpus, {
        seedTarget: STORAGE_URL || 'sqlite-direct',
        declaredBy: path.relative(path.resolve(GENESIS_DIR, '..'), loadedCorpus.sourcePath),
        itemsDeclared: totalInserted + totalSkipped,
      })!;
      emitStakesDeclaration(artifact, path.join(SEEDER_DIR, 'stakes-declaration.json'));
      console.log(`\n${stakesDeclarationLogLine(artifact)}`);
      console.log(`   manifestCid: ${artifact.manifestCid}`);
      if (DRY_RUN) {
        console.log('   stakes manifest NOT seeded (dry run)');
      } else {
        let stakesPeers = parsePeerStorageUrls(process.env.PEER_STORAGE_URLS);
        if (stakesPeers.length === 0 && STORAGE_URL) {
          stakesPeers = [{ name: 'seed-target', url: STORAGE_URL }];
        }
        const stakesResult = await seedNetworkStakes(artifact, stakesPeers);
        console.log(
          `   stakes manifest seeded on ${stakesResult.seeded.length}/${stakesPeers.length} peer(s)` +
            (stakesResult.seeded.length > 0 ? `: ${stakesResult.seeded.join(', ')}` : '') +
            (stakesResult.failed.length > 0
              ? ` — failed (stay Bootstrap): ${stakesResult.failed.join(', ')}`
              : ''),
        );
      }
    }
  } catch (err) {
    // The grant is a performance lever, never a seed-correctness gate: a
    // malformed declaration is loud, but content seeding stays authoritative.
    console.warn(
      `   stakes declaration step failed (peers stay fail-closed Bootstrap): ${err instanceof Error ? err.message : String(err)}`,
    );
  }

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

// Standalone execution only — guard so importing this module (e.g. for
// earnedReach in unit tests) does NOT run the seeder. Mirrors the isMain
// pattern in seed-commitments.ts.
const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  main().catch(err => {
    console.error('Fatal error:', err);
    process.exit(1);
  });
}
