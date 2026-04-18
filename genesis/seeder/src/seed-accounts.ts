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
 *   npx tsx src/seed-accounts.ts --doorway-url http://...     # Override single target
 *   npx tsx src/seed-accounts.ts --target-peers url1,url2     # Split across peers
 *
 * Environment variables:
 *   DOORWAY_URL           Single target (default: https://doorway-alpha.elohim.host)
 *   SEEDER_TARGET_PEERS   Comma-separated storage URLs; when set, accounts are
 *                         deterministically distributed across peers by humanId hash.
 *                         Overrides DOORWAY_URL when present.
 *                         Example:
 *                           SEEDER_TARGET_PEERS=http://elohim-matthew-alpha...:8090,http://elohim-adam-alpha...:8090
 */

import { readFileSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { AccountImportResultView, AccountPackageInputView } from '@elohim/storage-client';

import type { DeploymentRegistry } from './deployment-registry.js';
import { loadDeploymentRegistry, parseDeployedHumansArg } from './deployment-registry.js';

// =============================================================================
// Package loading
// =============================================================================

function loadPackages(packagesDir: string, humanFilter?: string): AccountPackageInputView[] {
  const files = readdirSync(packagesDir).filter(
    f => f.endsWith('.json') && f !== 'index.json' && f !== 'conductor-groups.json' && !f.endsWith('.schema.json')
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
// Registry-based partitioning
// =============================================================================

export interface PartitionResult {
  toSeed: AccountPackageInputView[];
  staged: AccountPackageInputView[];
  orphanRegistryEntries: string[];
}

export function partitionByRegistry(
  packages: AccountPackageInputView[],
  registry: DeploymentRegistry,
): PartitionResult {
  const toSeed: AccountPackageInputView[] = [];
  const staged: AccountPackageInputView[] = [];
  const packageIds = new Set<string>();

  for (const pkg of packages) {
    packageIds.add(pkg.identity.humanId);
    if (registry.deployedHumanIds.has(pkg.identity.humanId)) {
      toSeed.push(pkg);
    } else {
      staged.push(pkg);
    }
  }

  const orphanRegistryEntries: string[] = [];
  for (const id of registry.deployedHumanIds) {
    if (!packageIds.has(id)) {
      orphanRegistryEntries.push(id);
    }
  }

  return { toSeed, staged, orphanRegistryEntries };
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
  relationshipsSkipped: number;
  stewardshipCreated: number;
  collectivesJoined: number;
  errors: string[];
  targetUrl: string;
  attempts: number;
}

// =============================================================================
// Target peer resolution
// =============================================================================

// djb2 string hash — tiny, deterministic, good enough for even distribution.
export function stableHash(s: string): number {
  let h = 5381;
  for (let i = 0; i < s.length; i++) {
    h = (h * 33) ^ s.charCodeAt(i);
  }
  return h >>> 0; // unsigned
}

export function resolveTargetUrl(pkg: AccountPackageInputView, targetPeers: string[]): string {
  if (targetPeers.length === 0) {
    throw new Error('resolveTargetUrl: targetPeers must not be empty');
  }
  const idx = stableHash(pkg.identity.humanId) % targetPeers.length;
  return targetPeers[idx];
}

// =============================================================================
// Import
// =============================================================================

async function importAgainst(targetUrl: string, pkg: AccountPackageInputView): Promise<{ ok: true; result: AccountImportResultView } | { ok: false; error: string }> {
  try {
    const res = await fetch(`${targetUrl}/account/import`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(pkg),
    });
    if (!res.ok) {
      const errorText = await res.text();
      return { ok: false, error: `HTTP ${res.status}: ${errorText}` };
    }
    return { ok: true, result: (await res.json()) as AccountImportResultView };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}

async function importPackage(
  targetPeers: string[],
  pkg: AccountPackageInputView
): Promise<SeedResult> {
  const humanId = pkg.identity.humanId;
  const displayName = pkg.identity.displayName;

  // Preferred target from stable hash; fall back to siblings on failure.
  const primary = resolveTargetUrl(pkg, targetPeers);
  const ordered = [primary, ...targetPeers.filter(p => p !== primary)];

  const errors: string[] = [];
  let attempts = 0;
  for (const targetUrl of ordered) {
    attempts++;
    const outcome = await importAgainst(targetUrl, pkg);
    if (outcome.ok) {
      return {
        humanId,
        displayName,
        outcome: 'imported',
        contentUpdated: outcome.result.contentUpdated,
        relationshipsCreated: outcome.result.relationshipsCreated,
        relationshipsSkipped: outcome.result.relationshipsSkipped ?? 0,
        stewardshipCreated: outcome.result.stewardshipCreated,
        collectivesJoined: outcome.result.collectivesJoined ?? 0,
        errors: outcome.result.errors ?? [],
        targetUrl,
        attempts,
      };
    }
    errors.push(`${targetUrl}: ${outcome.error}`);
  }

  return {
    humanId,
    displayName,
    outcome: 'failed',
    contentUpdated: 0,
    relationshipsCreated: 0,
    relationshipsSkipped: 0,
    stewardshipCreated: 0,
    collectivesJoined: 0,
    errors,
    targetUrl: primary,
    attempts,
  };
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
  const targetPeersArg = args.find(a => a.startsWith('--target-peers='))?.split('=')[1]
    ?? (args.includes('--target-peers') ? args[args.indexOf('--target-peers') + 1] : undefined);

  const registryPathArg = args.find(a => a.startsWith('--registry='))?.split('=')[1]
    ?? (args.includes('--registry') ? args[args.indexOf('--registry') + 1] : undefined);
  const deployedHumansArg = args.find(a => a.startsWith('--deployed-humans='))?.split('=')[1]
    ?? (args.includes('--deployed-humans') ? args[args.indexOf('--deployed-humans') + 1] : undefined);

  const registry = loadDeploymentRegistry({
    registryPath: registryPathArg ?? process.env.SEEDER_REGISTRY,
    deployedHumans: parseDeployedHumansArg(
      deployedHumansArg ?? process.env.SEEDER_DEPLOYED_HUMANS,
    ),
  });

  const stripSlash = (u: string) => u.replace(/\/$/, '');
  const rawTargetPeers = targetPeersArg ?? process.env.SEEDER_TARGET_PEERS;
  const targetPeers: string[] = rawTargetPeers
    ? rawTargetPeers.split(',').map(s => s.trim()).filter(Boolean).map(stripSlash)
    : [stripSlash(doorwayUrlArg ?? process.env.DOORWAY_URL ?? 'https://doorway-alpha.elohim.host')];

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const packagesDir = resolve(__dirname, '../../data/account-packages');

  console.log('=== Seed Accounts ===\n');
  if (targetPeers.length === 1) {
    console.log(`Target:    ${targetPeers[0]}`);
  } else {
    console.log(`Targets:   ${targetPeers.length} peers (hash-mod by humanId)`);
    for (const p of targetPeers) console.log(`             - ${p}`);
  }
  console.log(`Packages:  ${packagesDir}`);
  if (DRY_RUN) console.log('Mode:      DRY RUN');
  if (humanFilter) console.log(`Filter:    ${humanFilter}`);
  console.log('');

  const allPackages = loadPackages(packagesDir, humanFilter);
  const { toSeed: packages, staged, orphanRegistryEntries } = partitionByRegistry(allPackages, registry);

  const registryLabel = registry.path ?? `flag: ${registry.deployedHumanIds.size} humans`;
  console.log(`Registry:  ${registryLabel} (${registry.deployedHumanIds.size} deployed humans)`);
  console.log(
    `Packages:  ${packagesDir} (${allPackages.length} found, ${packages.length} deployed, ${staged.length} staged)\n`,
  );

  for (const pkg of staged) {
    console.log(`  [-] ${pkg.identity.displayName.padEnd(18)} (staged — not in deployment registry)`);
  }
  if (staged.length > 0) console.log('');

  for (const orphanId of orphanRegistryEntries) {
    console.warn(`  WARNING: registry references ${orphanId}, no package found`);
  }
  if (orphanRegistryEntries.length > 0) console.log('');

  if (packages.length === 0) {
    console.log('No packages to import.');
    return;
  }

  if (DRY_RUN) {
    for (const pkg of packages) {
      const target = resolveTargetUrl(pkg, targetPeers);
      console.log(
        `  [DRY] ${pkg.identity.displayName.padEnd(18)} -> ${target} ` +
        `content=${pkg.content.length} rels=${pkg.relationships.length} stew=${pkg.stewardship.length} coll=${pkg.collectives.length}`
      );
    }
    console.log('\nDry run complete. No imports performed.');
    return;
  }

  // Import sequentially to avoid overwhelming a single peer
  const results: SeedResult[] = [];
  let totalContent = 0;
  let totalRels = 0;
  let totalSkipped = 0;
  let totalStew = 0;
  let totalColl = 0;

  for (const pkg of packages) {
    const result = await importPackage(targetPeers, pkg);
    results.push(result);

    const icon = result.outcome === 'imported' ? '+' : 'X';
    const warnings = result.errors.length > 0 ? ` (${result.errors.length} warnings)` : '';
    const retried = result.attempts > 1 ? ` retry=${result.attempts}` : '';
    console.log(
      `  [${icon}] ${result.displayName.padEnd(18)} -> ${result.targetUrl}${retried} ` +
      `content=${result.contentUpdated} rels=${result.relationshipsCreated} skipped=${result.relationshipsSkipped} stew=${result.stewardshipCreated} coll=${result.collectivesJoined}${warnings}`
    );

    totalContent += result.contentUpdated;
    totalRels += result.relationshipsCreated;
    totalSkipped += result.relationshipsSkipped;
    totalStew += result.stewardshipCreated;
    totalColl += result.collectivesJoined;
  }

  // Summary
  const imported = results.filter(r => r.outcome === 'imported').length;
  const failed = results.filter(r => r.outcome === 'failed').length;
  const withErrors = results.filter(r => r.errors.length > 0);

  console.log('');
  console.log(`=== Results: ${imported} imported, ${failed} failed, ${staged.length} staged ===`);
  console.log(`    Content updated: ${totalContent}`);
  console.log(`    Relationships created: ${totalRels}`);
  console.log(`    Relationships skipped: ${totalSkipped}`);
  console.log(`    Stewardship created: ${totalStew}`);
  console.log(`    Collectives joined: ${totalColl}`);

  if (targetPeers.length > 1) {
    const perPeer = new Map<string, { imported: number; failed: number; retries: number }>();
    for (const p of targetPeers) perPeer.set(p, { imported: 0, failed: 0, retries: 0 });
    for (const r of results) {
      const stats = perPeer.get(r.targetUrl) ?? { imported: 0, failed: 0, retries: 0 };
      if (r.outcome === 'imported') stats.imported++;
      else stats.failed++;
      if (r.attempts > 1) stats.retries += r.attempts - 1;
      perPeer.set(r.targetUrl, stats);
    }
    console.log('\n    Per-peer distribution:');
    for (const [peer, stats] of perPeer) {
      const retryNote = stats.retries > 0 ? ` (${stats.retries} retries landed here)` : '';
      console.log(`      ${peer}  imported=${stats.imported} failed=${stats.failed}${retryNote}`);
    }
  }

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

// Only run as a script when invoked directly, so tests can import helpers.
const invokedDirectly = process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1]);
if (invokedDirectly) {
  main();
}
