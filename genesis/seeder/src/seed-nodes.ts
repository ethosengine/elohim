/**
 * Seed Nodes — register stewarded nodes and stewardship relationships.
 *
 * Reads genesis/data/shefa/nodes.json and:
 *   1. Creates ContentNodes for node context text (type: node-context)
 *   2. Creates ContentNodes for stewardship context text (type: stewardship-context)
 *   3. Registers each node via POST /db/nodes with contextEprId
 *   4. Creates stewardship relationships via POST /db/nodes/{id}/stewardship
 *
 * Must run AFTER:
 *   - seed-humans.ts (humans must exist for stewardship references)
 *
 * Environment variables:
 *   STORAGE_URL   elohim-storage URL (default: http://localhost:8090)
 *
 * Exit codes:
 *   0 — all nodes and stewardship seeded or already exist
 *   1 — one or more operations failed
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { ContentFormat, ContentType, Reach } from './generated/schema-enums.js';

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
    const payload: Array<{
      id: string;
      title: string;
      contentType: ContentType;
      contentFormat: ContentFormat;
      contentBody: string;
      reach: Reach;
    }> = [
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
// Node registration
// =============================================================================

async function registerNode(
  storageUrl: string,
  node: NodeEntry,
  contextEprId: string | undefined,
): Promise<'created' | 'exists' | 'failed'> {
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
): Promise<'created' | 'exists' | 'failed'> {
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
      stewContextEprIds.set(`${entry.nodeId}:${entry.humanId}`, contentId);
    }
  }

  // --- Phase 3: Register nodes ---
  console.log('\nPhase 3: Register nodes');

  for (const node of nodesJson.nodes) {
    const contextEprId = nodeContextEprIds.get(node.id);
    const result = await registerNode(storageUrl, node, contextEprId);

    const icon = result === 'created' ? '+' : result === 'exists' ? '=' : 'X';
    const ctxNote = contextEprId ? ` (ctx: ${contextEprId})` : '';
    console.log(`  [${icon}] ${node.displayName.padEnd(20)} ${node.id}${ctxNote}`);

    if (result === 'failed') failed++;
  }

  // --- Phase 4: Create stewardship relationships ---
  console.log('\nPhase 4: Stewardship relationships');

  for (const entry of nodesJson.stewardship) {
    const contextEprId = stewContextEprIds.get(`${entry.nodeId}:${entry.humanId}`);
    const result = await createStewardship(storageUrl, entry, contextEprId);

    const icon = result === 'created' ? '+' : result === 'exists' ? '=' : 'X';
    const ctxNote = contextEprId ? ' (with context)' : '';
    console.log(
      `  [${icon}] ${entry.nodeId.padEnd(20)} <- ${entry.humanId} (${entry.relationship}, ${entry.affinityScore})${ctxNote}`,
    );

    if (result === 'failed') failed++;
  }

  // --- Summary ---
  console.log('');
  if (failed > 0) {
    console.error(`=== Done with ${failed} failure(s) ===`);
    process.exit(1);
  } else {
    console.log('=== Done: all nodes and stewardship seeded ===');
    process.exit(0);
  }
}

main();
