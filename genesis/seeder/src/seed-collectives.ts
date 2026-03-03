/**
 * Seed Collectives — create collective definitions via doorway /db/collectives.
 *
 * Reads collective definitions from genesis/data/collectives/collectives.json
 * and POSTs each to the doorway API. Must run BEFORE seed-accounts.ts so that
 * collectives exist before participations are created.
 *
 * Usage:
 *   npx tsx src/seed-collectives.ts                             # Seed all collectives
 *   npx tsx src/seed-collectives.ts --dry-run                   # Preview without seeding
 *   npx tsx src/seed-collectives.ts --doorway-url http://...    # Override doorway URL
 *
 * Environment variables:
 *   DOORWAY_URL   Doorway URL (default: https://doorway-alpha.elohim.host)
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { CreateCollectiveInputView } from '@elohim/storage-client';

// =============================================================================
// Types
// =============================================================================

interface CollectivesData {
  collectives: CreateCollectiveInputView[];
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const DRY_RUN = args.includes('--dry-run');
  const doorwayUrlArg = args.find(a => a.startsWith('--doorway-url='))?.split('=')[1]
    ?? (args.includes('--doorway-url') ? args[args.indexOf('--doorway-url') + 1] : undefined);

  const doorwayUrl = (doorwayUrlArg ?? process.env.DOORWAY_URL ?? 'https://doorway-alpha.elohim.host').replace(
    /\/$/,
    ''
  );

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const collectivesFile = resolve(__dirname, '../../data/collectives/collectives.json');

  console.log('=== Seed Collectives ===\n');
  console.log(`Doorway:      ${doorwayUrl}`);
  console.log(`Collectives:  ${collectivesFile}`);
  if (DRY_RUN) console.log('Mode:         DRY RUN');
  console.log('');

  const data: CollectivesData = JSON.parse(readFileSync(collectivesFile, 'utf-8'));
  console.log(`Found ${data.collectives.length} collective definitions\n`);

  if (data.collectives.length === 0) {
    console.log('No collectives to seed.');
    return;
  }

  if (DRY_RUN) {
    for (const coll of data.collectives) {
      console.log(`  [DRY] ${coll.id.padEnd(35)} ${coll.governanceLayer.padEnd(12)} ${coll.name}`);
    }
    console.log('\nDry run complete. No collectives seeded.');
    return;
  }

  let created = 0;
  let failed = 0;

  for (const coll of data.collectives) {
    try {
      const res = await fetch(`${doorwayUrl}/db/collectives`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(coll),
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
