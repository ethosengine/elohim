#!/usr/bin/env node
/**
 * Migrates seed file contentFormat values to resolve divergence between
 * seed files and DNA CONTENT_FORMATS.
 *
 * Phase 1 classification (2026-03-17):
 *   - 4 formats remapped to existing DNA formats
 *   - 0 storage-only, 0 added to DNA
 *
 * Run: node elohim/sdk/schemas/scripts/migrate-content-formats.mjs [--dry-run]
 */
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const SEED_DIR = resolve(import.meta.dirname, '../../../../genesis/data/lamad/content');
const DRY_RUN = process.argv.includes('--dry-run');

const REMAP = {
  'gherkin':   'markdown',
  'plaintext': 'markdown',
  'sophia':    'interactive',
  'html5-app': 'interactive',
};

async function main() {
  const files = (await readdir(SEED_DIR)).filter(f => f.endsWith('.json'));

  const stats = { scanned: 0, remapped: 0, unchanged: 0, errors: 0, byFormat: {} };

  for (const file of files) {
    stats.scanned++;
    const filePath = join(SEED_DIR, file);
    let raw;
    try {
      raw = await readFile(filePath, 'utf8');
    } catch {
      console.error(`READ ERROR: ${file}`);
      stats.errors++;
      continue;
    }

    let data;
    try {
      data = JSON.parse(raw);
    } catch {
      console.error(`PARSE ERROR: ${file}`);
      stats.errors++;
      continue;
    }

    const oldFormat = data.contentFormat;
    if (!oldFormat || !REMAP[oldFormat]) {
      stats.unchanged++;
      continue;
    }

    const newFormat = REMAP[oldFormat];
    stats.byFormat[`${oldFormat} → ${newFormat}`] = (stats.byFormat[`${oldFormat} → ${newFormat}`] || 0) + 1;

    if (!data.metadata) data.metadata = {};
    data.metadata.originalContentFormat = oldFormat;
    data.contentFormat = newFormat;
    stats.remapped++;

    if (!DRY_RUN) {
      await writeFile(filePath, JSON.stringify(data, null, 2) + '\n');
    }
  }

  console.log(`\nContent Format Migration ${DRY_RUN ? '(DRY RUN)' : 'COMPLETE'}`);
  console.log(`  Scanned:    ${stats.scanned}`);
  console.log(`  Remapped:   ${stats.remapped}`);
  console.log(`  Unchanged:  ${stats.unchanged}`);
  console.log(`  Errors:     ${stats.errors}`);
  console.log(`\nRemap breakdown:`);
  for (const [key, count] of Object.entries(stats.byFormat).sort((a, b) => b[1] - a[1])) {
    console.log(`  ${key}: ${count}`);
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
