/**
 * Seed Nodes — register stewarded nodes and stewardship relationships.
 *
 * Reads genesis/data/shefa/nodes.json and:
 *   1. Creates ContentNodes for node context text (type: node-context)
 *   2. Creates ContentNodes for stewardship context text (type: stewardship-context)
 *   3. Registers each node via GET-before-POST /db/nodes with contextEprId
 *   4. Creates stewardship relationships via GET-before-POST
 *      /db/nodes/{id}/stewardship
 *
 * Idempotency (Phases 3-4): a `GET /db/nodes/{id}` runs before every node
 * registration, and its joined `stewards` array is consulted before every
 * stewardship write — a row that already exists is counted 'exists' and
 * never re-POSTed. This is deliberately independent of how storage responds
 * to a duplicate POST (200/409/500) — elohim-storage's own dedup behavior
 * for these tables is being hardened separately; this seeder's idempotency
 * does not depend on that landing. Phases 1-2 (bulk content) do not need a
 * GET-before-POST leg: `POST /db/content/bulk` already reports duplicates
 * as `skipped` (not an error) in its own response body.
 *
 * Must run AFTER:
 *   - seed-humans.ts — `node_stewardship.human_id` carries a `NOT NULL
 *     REFERENCES humans(id)` foreign key (deployed SQLite enforces FK
 *     constraints — see the genesis #1105 FK-storm regression test in
 *     elohim-storage's `db/collectives.rs`), so every stewardship entry's
 *     `humanId` must already have a `humans` row on THIS storage peer.
 *     seed-humans.ts only populates that row for household members (via its
 *     best-effort `seedProjectionRow` bridge, gated on `householdId` being
 *     set) — a human referenced by nodes.json's stewardship list with no
 *     household membership, or whose bridge write failed, leaves the FK
 *     unsatisfiable and the affected stewardship POST fails.
 *
 * Environment variables:
 *   STORAGE_URL   elohim-storage URL (default: http://localhost:8090)
 *
 * Exit codes (partial-readiness aware — matches the runProbedSeeder
 * contract other seeders in this chain report against, e.g.
 * seed-conductor-identities.ts / seed-agent-bindings.ts):
 *   0 — clean: every node/stewardship/context row created or already exists
 *   2 — partial: at least one row succeeded (created/exists), at least one failed
 *   1 — total failure: nothing succeeded (or a fatal top-level error)
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { ContentFormat, ContentType, Reach } from './generated/schema-enums.js';
import type { CreateContentInput } from './generated/create-content-input.js';
import type { StewardedNodeView } from '@elohim/storage-client';

// =============================================================================
// Types (mirrors nodes.json schema)
// =============================================================================

interface NodeEntry {
  id: string;
  displayName: string;
  cpuCores: number;
  memoryGb: number;
  storageTb: number;
  bandwidthMbps: number;
  stewardTier: string;
  region: string;
  context?: string;
}

interface StewardshipEntry {
  nodeId: string;
  humanId: string;
  affinityScore: number;
  relationship: string;
  context?: string;
}

interface NodesJson {
  nodes: NodeEntry[];
  stewardship: StewardshipEntry[];
}

// =============================================================================
// Content creation (context text -> ContentNode with EPR ID)
// =============================================================================

interface BulkContentResult {
  inserted: number;
  skipped: number;
  errors: string[];
}

async function createContextContent(
  storageUrl: string,
  id: string,
  title: string,
  body: string,
  contentType: ContentType,
): Promise<'created' | 'exists' | 'failed'> {
  try {
    const payload: Array<CreateContentInput> = [
      {
        id,
        title,
        contentType,
        contentFormat: 'text',
        contentBody: body,
        reach: 'intimate',
      },
    ];
    const res = await fetch(`${storageUrl}/db/content/bulk`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Schema-Version': '1',
      },
      body: JSON.stringify(payload),
    });

    if (!res.ok) {
      const errorText = await res.text();
      console.error(`    Content creation failed for ${id}: HTTP ${res.status}: ${errorText}`);
      return 'failed';
    }

    const result: BulkContentResult = await res.json();
    if (result.inserted > 0) return 'created';
    if (result.skipped > 0) return 'exists';
    if (result.errors.length > 0) {
      console.error(`    Content creation errors for ${id}: ${result.errors.join(', ')}`);
      return 'failed';
    }
    return 'exists';
  } catch (err) {
    console.error(
      `    Content creation error for ${id}: ${err instanceof Error ? err.message : String(err)}`,
    );
    return 'failed';
  }
}

// =============================================================================
// GET-before-POST idempotency helper
// =============================================================================

/**
 * `GET /db/nodes/{id}` — the node with its joined `stewards` array, or null
 * when the node doesn't exist (404) or the probe itself fails (network
 * error, non-2xx/404 status). A probe failure is NOT distinguished from
 * "doesn't exist" here — the caller falls through to POST either way, which
 * is the same best-effort posture the rest of this seeder already takes on
 * network errors (registerNode/createStewardship below).
 */
async function getNode(storageUrl: string, id: string): Promise<StewardedNodeView | null> {
  try {
    const res = await fetch(`${storageUrl}/db/nodes/${id}`);
    if (res.status === 404) return null;
    if (!res.ok) {
      console.error(`    GET /db/nodes/${id} failed: HTTP ${res.status}`);
      return null;
    }
    return (await res.json()) as StewardedNodeView;
  } catch (err) {
    console.error(
      `    GET /db/nodes/${id} error: ${err instanceof Error ? err.message : String(err)}`,
    );
    return null;
  }
}

// =============================================================================
// Node registration
// =============================================================================

async function registerNode(
  storageUrl: string,
  node: NodeEntry,
  contextEprId: string | undefined,
  existing: StewardedNodeView | null,
): Promise<'created' | 'exists' | 'failed'> {
  // GET-before-POST: a node the GET already found is 'exists' — never
  // re-POSTed, regardless of how storage would itself respond to a
  // duplicate insert.
  if (existing) return 'exists';

  try {
    const res = await fetch(`${storageUrl}/db/nodes`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: node.id,
        displayName: node.displayName,
        cpuCores: node.cpuCores,
        memoryGb: node.memoryGb,
        storageTb: node.storageTb,
        bandwidthMbps: node.bandwidthMbps,
        stewardTier: node.stewardTier,
        region: node.region,
        contextEprId: contextEprId ?? null,
      }),
    });

    if (res.ok || res.status === 201) return 'created';
    // Belt-and-suspenders: the GET above is the primary idempotency guard,
    // but a race (two seed runs overlapping) could still land a duplicate
    // POST between our GET and this write — treat storage's own conflict
    // signaling as 'exists' too, whichever shape it currently answers in.
    if (res.status === 409) return 'exists';

    const errorText = await res.text();
    console.error(`    Node registration failed for ${node.id}: HTTP ${res.status}: ${errorText}`);
    return 'failed';
  } catch (err) {
    console.error(
      `    Node registration error for ${node.id}: ${err instanceof Error ? err.message : String(err)}`,
    );
    return 'failed';
  }
}

// =============================================================================
// Stewardship creation
// =============================================================================

async function createStewardship(
  storageUrl: string,
  entry: StewardshipEntry,
  contextEprId: string | undefined,
  alreadySteward: boolean,
): Promise<'created' | 'exists' | 'failed'> {
  // GET-before-POST: the node's joined `stewards` array (from getNode) is
  // the idempotency source — a human already listed as a steward is
  // 'exists' and never re-POSTed.
  if (alreadySteward) return 'exists';

  try {
    const res = await fetch(`${storageUrl}/db/nodes/${entry.nodeId}/stewardship`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        nodeId: entry.nodeId,
        humanId: entry.humanId,
        affinityScore: entry.affinityScore,
        relationship: entry.relationship,
        contextEprId: contextEprId ?? null,
      }),
    });

    if (res.ok || res.status === 201) return 'created';
    // Belt-and-suspenders — see registerNode's matching comment.
    if (res.status === 409) return 'exists';

    const errorText = await res.text();
    console.error(
      `    Stewardship failed for ${entry.nodeId}/${entry.humanId}: HTTP ${res.status}: ${errorText}`,
    );
    return 'failed';
  } catch (err) {
    console.error(
      `    Stewardship error for ${entry.nodeId}/${entry.humanId}: ${err instanceof Error ? err.message : String(err)}`,
    );
    return 'failed';
  }
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const storageUrl = (process.env.STORAGE_URL || 'http://localhost:8090').replace(/\/$/, '');

  // Load nodes.json
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const jsonPath = resolve(__dirname, '../../data/shefa/nodes.json');
  const nodesJson: NodesJson = JSON.parse(readFileSync(jsonPath, 'utf-8'));

  console.log('=== Seed Nodes ===\n');
  console.log(`Storage:       ${storageUrl}`);
  console.log(`Nodes:         ${nodesJson.nodes.length}`);
  console.log(`Stewardship:   ${nodesJson.stewardship.length}`);
  console.log('');

  let created = 0;
  let existsCount = 0;
  let failed = 0;

  // --- Phase 1: Create context ContentNodes for nodes ---
  console.log('Phase 1: Node context content');
  const nodeContextEprIds: Map<string, string> = new Map();

  for (const node of nodesJson.nodes) {
    if (!node.context) {
      console.log(`  [—] ${node.displayName.padEnd(20)} (no context)`);
      continue;
    }

    const contentId = `ctx-${node.id}`;
    const result = await createContextContent(
      storageUrl,
      contentId,
      `${node.displayName} context`,
      node.context,
      'node-context',
    );

    const icon = result === 'created' ? '+' : result === 'exists' ? '=' : 'X';
    console.log(`  [${icon}] ${node.displayName.padEnd(20)} -> ${contentId}`);

    if (result === 'failed') {
      failed++;
    } else {
      if (result === 'created') created++;
      else existsCount++;
      nodeContextEprIds.set(node.id, contentId);
    }
  }

  // --- Phase 2: Create context ContentNodes for stewardship ---
  console.log('\nPhase 2: Stewardship context content');
  const stewContextEprIds: Map<string, string> = new Map();

  for (const entry of nodesJson.stewardship) {
    if (!entry.context) {
      console.log(
        `  [—] ${entry.nodeId.padEnd(20)} / ${entry.humanId.padEnd(24)} (no context)`,
      );
      continue;
    }

    const contentId = `ctx-stew-${entry.nodeId}-${entry.humanId}`;
    const result = await createContextContent(
      storageUrl,
      contentId,
      `Stewardship: ${entry.humanId} -> ${entry.nodeId}`,
      entry.context,
      'stewardship-context',
    );

    const icon = result === 'created' ? '+' : result === 'exists' ? '=' : 'X';
    console.log(`  [${icon}] ${entry.nodeId.padEnd(20)} / ${entry.humanId}`);

    if (result === 'failed') {
      failed++;
    } else {
      if (result === 'created') created++;
      else existsCount++;
      stewContextEprIds.set(`${entry.nodeId}:${entry.humanId}`, contentId);
    }
  }

  // --- Phase 3: Register nodes (GET-before-POST) ---
  console.log('\nPhase 3: Register nodes');

  // Cache the joined node views Phase 3 fetches — Phase 4 reuses them
  // (freshened per-node on write below) so it never re-GETs a node it just
  // saw here.
  const nodeViewCache: Map<string, StewardedNodeView | null> = new Map();

  for (const node of nodesJson.nodes) {
    const contextEprId = nodeContextEprIds.get(node.id);
    const existing = await getNode(storageUrl, node.id);
    const result = await registerNode(storageUrl, node, contextEprId, existing);
    // Re-fetch after a fresh create so Phase 4 sees the (still steward-less)
    // joined view rather than the pre-POST null.
    nodeViewCache.set(node.id, result === 'created' ? await getNode(storageUrl, node.id) : existing);

    const icon = result === 'created' ? '+' : result === 'exists' ? '=' : 'X';
    const ctxNote = contextEprId ? ` (ctx: ${contextEprId})` : '';
    console.log(`  [${icon}] ${node.displayName.padEnd(20)} ${node.id}${ctxNote}`);

    if (result === 'failed') failed++;
    else if (result === 'created') created++;
    else existsCount++;
  }

  // --- Phase 4: Create stewardship relationships (GET-before-POST) ---
  console.log('\nPhase 4: Stewardship relationships');

  for (const entry of nodesJson.stewardship) {
    const contextEprId = stewContextEprIds.get(`${entry.nodeId}:${entry.humanId}`);
    // Reuse Phase 3's cache when present; a stewardship entry whose node
    // Phase 3 didn't touch (defensive — nodes.json's own entries always
    // pair a node with its stewardship rows) falls back to a fresh GET.
    let nodeView = nodeViewCache.get(entry.nodeId);
    if (nodeView === undefined) {
      nodeView = await getNode(storageUrl, entry.nodeId);
      nodeViewCache.set(entry.nodeId, nodeView);
    }
    const alreadySteward = nodeView?.stewards.some(s => s.humanId === entry.humanId) ?? false;
    const result = await createStewardship(storageUrl, entry, contextEprId, alreadySteward);

    const icon = result === 'created' ? '+' : result === 'exists' ? '=' : 'X';
    const ctxNote = contextEprId ? ' (with context)' : '';
    console.log(
      `  [${icon}] ${entry.nodeId.padEnd(20)} <- ${entry.humanId} (${entry.relationship}, ${entry.affinityScore})${ctxNote}`,
    );

    if (result === 'failed') failed++;
    else if (result === 'created') created++;
    else existsCount++;
  }

  // --- Summary ---
  // Partial-readiness aware exit, matching the runProbedSeeder contract
  // other seeders in this chain report against: 0 clean, 2 partial (some
  // succeeded, some failed), 1 total failure (nothing succeeded).
  const succeeded = created + existsCount;
  console.log('');
  console.log(
    `=== Results: ${created} created, ${existsCount} existing, ${failed} failed ===`,
  );
  if (failed === 0) {
    process.exit(0);
  } else if (succeeded > 0) {
    console.error(`=== Done: partial (${succeeded} succeeded, ${failed} failed) ===`);
    process.exit(2);
  } else {
    console.error(`=== Done: total failure (0 succeeded, ${failed} failed) ===`);
    process.exit(1);
  }
}

// Top-level catch, matching every sibling seeder in this chain
// (seed-humans.ts / seed-conductor-identities.ts / seed-household-formation.ts
// / seed-commitments.ts / seed-delegates-compute.ts) — this file used to
// call `main();` bare, so an exception escaping the per-item try/catch
// blocks (or thrown before Phase 1 starts, e.g. reading/parsing
// nodes.json) crashed the process as an unhandled rejection with NO
// `=== Done ===` summary line and no per-item [+]/[X] output at all,
// making a genuine first-run failure indistinguishable from this leg
// simply never running.
main().catch(err => {
  console.error('FATAL:', err instanceof Error ? (err.stack ?? err.message) : err);
  process.exit(1);
});
