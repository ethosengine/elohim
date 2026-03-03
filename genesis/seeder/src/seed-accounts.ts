/**
 * Seed Accounts — import account packages per-human via doorway /account/import.
 *
 * Reads generated account packages from genesis/data/account-packages/ and
 * POSTs each to the doorway API. Each package sets content reach levels,
 * relationships, and stewardship allocations for that human.
 *
 * Must run AFTER:
 *   - seed-sqlite.ts  (content must exist before reach can be set)
 *   - seed-humans.ts  (humans must exist before relationships can reference them)
 *
 * Usage:
 *   npx tsx src/seed-accounts.ts                             # Import all packages
 *   npx tsx src/seed-accounts.ts --human matthew              # Single human
 *   npx tsx src/seed-accounts.ts --dry-run                    # Preview without importing
 *   npx tsx src/seed-accounts.ts --doorway-url http://...     # Override doorway URL
 *
 * Environment variables:
 *   DOORWAY_URL   Doorway URL (default: https://doorway-alpha.elohim.host)
 */

import { readFileSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { AccountAccountImportResultViewView, AccountPackageInputViewInputView } from '@elohim/storage-client';

// =============================================================================
// Package loading
// =============================================================================

function loadPackages(packagesDir: string, humanFilter?: string): AccountPackageInputView[] {
  const files = readdirSync(packagesDir).filter(
    f => f.endsWith('.json') && f !== 'index.json' && f !== 'conductor-groups.json'
  );

  const packages: AccountPackageInputView[] = [];
  for (const file of files) {
    const pkg: AccountPackageInputView = JSON.parse(readFileSync(resolve(packagesDir, file), 'utf-8'));

    if (humanFilter) {
      const id = pkg.identity.humanId.toLowerCase();
      const name = pkg.identity.displayName.toLowerCase();
      if (!id.includes(humanFilter.toLowerCase()) && !name.includes(humanFilter.toLowerCase())) {
        continue;
      }
    }

    packages.push(pkg);
  }

  return packages;
}

// =============================================================================
// Import
// =============================================================================

type Outcome = 'imported' | 'failed';

interface SeedResult {
  humanId: string;
  displayName: string;
  outcome: Outcome;
  contentUpdated: number;
  relationshipsCreated: number;
  stewardshipCreated: number;
  collectivesJoined: number;
  errors: string[];
}

async function importPackage(doorwayUrl: string, pkg: AccountPackageInputView): Promise<SeedResult> {
  const humanId = pkg.identity.humanId;
  const displayName = pkg.identity.displayName;

  try {
    const res = await fetch(`${doorwayUrl}/account/import`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(pkg),
    });

    if (!res.ok) {
      const errorText = await res.text();
      return {
        humanId,
        displayName,
        outcome: 'failed',
        contentUpdated: 0,
        relationshipsCreated: 0,
        stewardshipCreated: 0,
        collectivesJoined: 0,
        errors: [`HTTP ${res.status}: ${errorText}`],
      };
    }

    const result: AccountImportResultView = await res.json();
    return {
      humanId,
      displayName,
      outcome: 'imported',
      contentUpdated: result.contentUpdated,
      relationshipsCreated: result.relationshipsCreated,
      stewardshipCreated: result.stewardshipCreated,
      collectivesJoined: result.collectivesJoined ?? 0,
      errors: result.errors ?? [],
    };
  } catch (err) {
    return {
      humanId,
      displayName,
      outcome: 'failed',
      contentUpdated: 0,
      relationshipsCreated: 0,
      stewardshipCreated: 0,
      collectivesJoined: 0,
      errors: [err instanceof Error ? err.message : String(err)],
    };
  }
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const DRY_RUN = args.includes('--dry-run');
  const humanFilter = args.find(a => a.startsWith('--human='))?.split('=')[1]
    ?? args.find(a => a.startsWith('--human'))
      ? args[args.indexOf('--human') + 1]
      : undefined;
  const doorwayUrlArg = args.find(a => a.startsWith('--doorway-url='))?.split('=')[1]
    ?? (args.includes('--doorway-url') ? args[args.indexOf('--doorway-url') + 1] : undefined);

  const doorwayUrl = (doorwayUrlArg ?? process.env.DOORWAY_URL ?? 'https://doorway-alpha.elohim.host').replace(
    /\/$/,
    ''
  );

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const packagesDir = resolve(__dirname, '../../data/account-packages');

  console.log('=== Seed Accounts ===\n');
  console.log(`Doorway:   ${doorwayUrl}`);
  console.log(`Packages:  ${packagesDir}`);
  if (DRY_RUN) console.log('Mode:      DRY RUN');
  if (humanFilter) console.log(`Filter:    ${humanFilter}`);
  console.log('');

  const packages = loadPackages(packagesDir, humanFilter);
  console.log(`Found ${packages.length} account packages\n`);

  if (packages.length === 0) {
    console.log('No packages to import.');
    return;
  }

  if (DRY_RUN) {
    for (const pkg of packages) {
      console.log(
        `  [DRY] ${pkg.identity.displayName.padEnd(18)} ` +
        `content=${pkg.content.length} rels=${pkg.relationships.length} stew=${pkg.stewardship.length} coll=${pkg.collectives.length}`
      );
    }
    console.log('\nDry run complete. No imports performed.');
    return;
  }

  // Import sequentially to avoid overwhelming the doorway
  const results: SeedResult[] = [];
  let totalContent = 0;
  let totalRels = 0;
  let totalStew = 0;
  let totalColl = 0;

  for (const pkg of packages) {
    const result = await importPackage(doorwayUrl, pkg);
    results.push(result);

    const icon = result.outcome === 'imported' ? '+' : 'X';
    const warnings = result.errors.length > 0 ? ` (${result.errors.length} warnings)` : '';
    console.log(
      `  [${icon}] ${result.displayName.padEnd(18)} ` +
      `content=${result.contentUpdated} rels=${result.relationshipsCreated} stew=${result.stewardshipCreated} coll=${result.collectivesJoined}${warnings}`
    );

    totalContent += result.contentUpdated;
    totalRels += result.relationshipsCreated;
    totalStew += result.stewardshipCreated;
    totalColl += result.collectivesJoined;
  }

  // Summary
  const imported = results.filter(r => r.outcome === 'imported').length;
  const failed = results.filter(r => r.outcome === 'failed').length;
  const withErrors = results.filter(r => r.errors.length > 0);

  console.log('');
  console.log(`=== Results: ${imported} imported, ${failed} failed ===`);
  console.log(`    Content updated: ${totalContent}`);
  console.log(`    Relationships created: ${totalRels}`);
  console.log(`    Stewardship created: ${totalStew}`);
  console.log(`    Collectives joined: ${totalColl}`);

  if (withErrors.length > 0) {
    console.log('\nWarnings/Errors:');
    for (const r of withErrors) {
      for (const err of r.errors) {
        console.log(`  ${r.displayName}: ${err}`);
      }
    }
  }

  if (failed > 0) {
    console.error(`\n${failed} package(s) failed to import.`);
    process.exit(1);
  }

  process.exit(0);
}

main();
