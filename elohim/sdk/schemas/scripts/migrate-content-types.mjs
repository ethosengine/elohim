#!/usr/bin/env node
/**
 * Migrates seed file contentType values to resolve divergence between
 * seed files and DNA CONTENT_TYPES.
 *
 * Phase 1 classification (2026-03-17):
 *   - 13 types remapped to existing DNA types
 *   - 2 types kept as storage-only (human, role)
 *   - 0 types added to DNA
 *
 * Run: node elohim/sdk/schemas/scripts/migrate-content-types.mjs [--dry-run]
 */
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const SEED_DIR = resolve(import.meta.dirname, '../../../../genesis/data/lamad/content');
const DRY_RUN = process.argv.includes('--dry-run');

// Remap table: seed contentType → DNA contentType
// Types not listed here are either already valid DNA types or storage-only (kept as-is).
const REMAP = {
  'bible-verse':   'reference',
  'organization':  'resource',
  'book':          'resource',
  'contributor':   'resource',
  'course-module': 'lesson',
  'practice':      'exercise',
  'narrative':     'example',
  'activity':      'exercise',
  'video':         'resource',
  'documentary':   'resource',
  'book-chapter':  'reference',
  'podcast':       'resource',
  'simulation':    'resource',
  'feature':       'scenario',
};

// Storage-only types — kept as-is, no remap needed
const STORAGE_ONLY = new Set(['human', 'role']);

async function main() {
  const files = (await readdir(SEED_DIR)).filter(f => f.endsWith('.json'));

  const stats = {
    scanned: 0,
    remapped: 0,
    storageOnly: 0,
    alreadyValid: 0,
    errors: 0,
    byType: {},
  };

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

    const oldType = data.contentType;
    if (!oldType) continue;

    if (REMAP[oldType]) {
      const newType = REMAP[oldType];
      stats.byType[`${oldType} → ${newType}`] = (stats.byType[`${oldType} → ${newType}`] || 0) + 1;

      // Preserve original type in metadata for traceability
      if (!data.metadata) data.metadata = {};
      data.metadata.originalContentType = oldType;

      data.contentType = newType;
      stats.remapped++;

      if (!DRY_RUN) {
        await writeFile(filePath, JSON.stringify(data, null, 2) + '\n');
      }
    } else if (STORAGE_ONLY.has(oldType)) {
      stats.storageOnly++;
    } else {
      stats.alreadyValid++;
    }
  }

  console.log(`\nContent Type Migration ${DRY_RUN ? '(DRY RUN)' : 'COMPLETE'}`);
  console.log(`  Scanned:       ${stats.scanned}`);
  console.log(`  Remapped:      ${stats.remapped}`);
  console.log(`  Storage-only:  ${stats.storageOnly} (kept as-is)`);
  console.log(`  Already valid: ${stats.alreadyValid}`);
  console.log(`  Errors:        ${stats.errors}`);
  console.log(`\nRemap breakdown:`);
  for (const [key, count] of Object.entries(stats.byType).sort((a, b) => b[1] - a[1])) {
    console.log(`  ${key}: ${count}`);
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
