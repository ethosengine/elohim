/**
 * Seed Collectives — create collective definitions via doorway /db/collectives.
 *
 * Reads collective definitions from genesis/data/collectives/collectives.json,
 * validates them against the schema, and POSTs each to the doorway API.
 * Must run BEFORE seed-accounts.ts so that collectives exist before
 * participations are created.
 *
 * Usage:
 *   npx tsx src/seed-collectives.ts                             # Seed all collectives
 *   npx tsx src/seed-collectives.ts --dry-run                   # Preview without seeding
 *   npx tsx src/seed-collectives.ts --validate-only             # Validate and exit
 *   npx tsx src/seed-collectives.ts --doorway-url http://...    # Override doorway URL
 *
 * Environment variables:
 *   DOORWAY_URL   Doorway URL (default: https://doorway-alpha.elohim.host)
 */

import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { CreateCollectiveInputView, JsonValue } from '@elohim/storage-client';

import { validateCollectivesFile, validateReferentialIntegrity } from './validate-collectives.js';

// =============================================================================
// Types
// =============================================================================

interface CollectiveEntry {
  id: string;
  name: string;
  governanceLayer: string;
  reach?: string | null;
  constitutionalParentId?: string | null;
  description?: string | null;
  governanceModel?: string | null;
  domain?: string | null;
  place?: string | null;
  coupling?: {
    lamad?: string;
    shefa?: string;
    qahal?: string;
  } | null;
}

// =============================================================================
// Mapping
// =============================================================================

function toInputView(entry: CollectiveEntry): CreateCollectiveInputView {
  const metadata: Record<string, JsonValue> = {};
  if (entry.governanceModel) metadata.governanceModel = entry.governanceModel;
  if (entry.domain) metadata.domain = entry.domain;
  if (entry.place) metadata.place = entry.place;
  if (entry.coupling) metadata.coupling = entry.coupling as JsonValue;

  return {
    id: entry.id,
    name: entry.name,
    description: entry.description ?? null,
    governanceLayer: entry.governanceLayer,
    constitutionalParentId: entry.constitutionalParentId ?? null,
    reach: entry.reach ?? null,
    metadata: Object.keys(metadata).length > 0 ? metadata : null,
    createdBy: null,
  };
}

// =============================================================================
// Topological Sort
// =============================================================================

function topologicalSort(collectives: CollectiveEntry[]): CollectiveEntry[] {
  const byId = new Map(collectives.map(c => [c.id, c]));
  const sorted: CollectiveEntry[] = [];
  const visited = new Set<string>();

  function visit(c: CollectiveEntry): void {
    if (visited.has(c.id)) return;
    visited.add(c.id);
    if (c.constitutionalParentId) {
      const parent = byId.get(c.constitutionalParentId);
      if (parent) visit(parent);
    }
    sorted.push(c);
  }

  for (const c of collectives) visit(c);
  return sorted;
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const DRY_RUN = args.includes('--dry-run');
  const VALIDATE_ONLY = args.includes('--validate-only');
  const doorwayUrlArg = args.find(a => a.startsWith('--doorway-url='))?.split('=')[1]
    ?? (args.includes('--doorway-url') ? args[args.indexOf('--doorway-url') + 1] : undefined);

  const doorwayUrl = (doorwayUrlArg ?? process.env.DOORWAY_URL ?? 'https://doorway-alpha.elohim.host').replace(
    /\/$/,
    ''
  );

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const collectivesFile = resolve(__dirname, '../../data/collectives/collectives.json');

  console.log('=== Seed Collectives ===\n');
  console.log(`Collectives:  ${collectivesFile}`);
  if (!VALIDATE_ONLY) console.log(`Doorway:      ${doorwayUrl}`);
  if (DRY_RUN) console.log('Mode:         DRY RUN');
  if (VALIDATE_ONLY) console.log('Mode:         VALIDATE ONLY');
  console.log('');

  // ---------------------------------------------------------------------------
  // Step 1: Validate before any HTTP calls
  // ---------------------------------------------------------------------------
  console.log('Validating collectives file...');
  const validationResult = validateCollectivesFile(collectivesFile);
  const integrityErrors = validateReferentialIntegrity(validationResult);
  const allErrors = [...validationResult.errors, ...integrityErrors];

  console.log(`  Collectives:    ${validationResult.collectiveCount}`);
  console.log(`  Relationships:  ${validationResult.relationshipCount}`);
  console.log(`  Errors:         ${allErrors.length}`);
  if (validationResult.warnings.length > 0) {
    console.log(`  Warnings:       ${validationResult.warnings.length}`);
  }
  console.log('');

  if (allErrors.length > 0) {
    console.error('Validation FAILED:');
    for (const e of allErrors) console.error(`  x ${e}`);
    process.exit(1);
  }

  if (validationResult.warnings.length > 0) {
    console.log('Warnings:');
    for (const w of validationResult.warnings) console.log(`  ! ${w}`);
    console.log('');
  }

  console.log('Validation passed.\n');

  if (VALIDATE_ONLY) {
    process.exit(0);
  }

  // ---------------------------------------------------------------------------
  // Step 2: Topological sort + map to input views
  // ---------------------------------------------------------------------------
  const rawCollectives = (validationResult.data?.collectives ?? []) as CollectiveEntry[];

  if (rawCollectives.length === 0) {
    console.log('No collectives to seed.');
    process.exit(0);
  }

  const sorted = topologicalSort(rawCollectives);
  console.log(`Found ${sorted.length} collective definitions (topologically sorted)\n`);

  // ---------------------------------------------------------------------------
  // Step 3: Dry run
  // ---------------------------------------------------------------------------
  if (DRY_RUN) {
    const byId = new Map(sorted.map(c => [c.id, c]));

    function parentChain(c: CollectiveEntry): string {
      const chain: string[] = [];
      let current: CollectiveEntry | undefined = c;
      while (current?.constitutionalParentId) {
        chain.unshift(current.constitutionalParentId);
        current = byId.get(current.constitutionalParentId);
      }
      return chain.length > 0 ? ` (parent: ${chain.join(' > ')})` : '';
    }

    for (const coll of sorted) {
      const reach = coll.reach ? ` reach=${coll.reach}` : '';
      console.log(
        `  [DRY] ${coll.id.padEnd(35)} ${coll.governanceLayer.padEnd(12)} ${coll.name}${reach}${parentChain(coll)}`
      );
    }
    console.log('\nDry run complete. No collectives seeded.');
    return;
  }

  // ---------------------------------------------------------------------------
  // Step 4: POST each collective
  // ---------------------------------------------------------------------------
  let created = 0;
  let failed = 0;

  for (const coll of sorted) {
    const inputView = toInputView(coll);
    try {
      const res = await fetch(`${doorwayUrl}/db/collectives`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(inputView),
      });

      if (res.ok) {
        console.log(`  [+] ${coll.id.padEnd(35)} ${coll.governanceLayer.padEnd(12)} ${coll.name}`);
        created++;
      } else {
        const errorText = await res.text();
        console.log(`  [X] ${coll.id.padEnd(35)} HTTP ${res.status}: ${errorText}`);
        failed++;
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.log(`  [X] ${coll.id.padEnd(35)} ${msg}`);
      failed++;
    }
  }

  console.log(`\n=== Results: ${created} created, ${failed} failed ===`);

  if (failed > 0) {
    console.error(`\n${failed} collective(s) failed to seed.`);
    process.exit(1);
  }

  process.exit(0);
}

main();
