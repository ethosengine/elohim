#!/usr/bin/env npx tsx
/**
 * Validate every corpus trust declaration under genesis/data/.
 *
 * The repo-wide reader for the `corpus.json` convention (see corpus-trust.ts).
 * `seed.ts` validates only the corpus it is about to seed; this script validates
 * ALL of them so a malformed or vocabulary-drifting declaration in a corpus that
 * this run does not touch still fails a gate.
 *
 * Usage:
 *   npx tsx src/validate-corpora.ts            # validate all corpora under data/
 *   npx tsx src/validate-corpora.ts --json     # machine-readable summary
 *
 * Exit codes:
 *   0 — every declaration found is valid (zero declarations is also 0: absence is
 *       the fail-closed path, not an error)
 *   1 — at least one declaration is invalid
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  CORPUS_DECLARATION_FILE,
  CorpusDeclarationError,
  loadCorpusDeclaration,
  type CorpusDeclaration,
} from './corpus-trust.js';

const __filename = fileURLToPath(import.meta.url);
const SEEDER_DIR = path.dirname(path.dirname(__filename));
const GENESIS_DIR = path.resolve(SEEDER_DIR, '..');
const DATA_ROOT = process.env.CORPORA_ROOT || path.join(GENESIS_DIR, 'data');

export interface CorpusValidationRow {
  dir: string;
  ok: boolean;
  declaration?: CorpusDeclaration;
  error?: string;
}

/** Directories directly under `root` that carry a corpus declaration. */
export function findCorpusDirs(root: string): string[] {
  if (!fs.existsSync(root)) return [];
  return fs
    .readdirSync(root, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => path.join(root, e.name))
    .filter((dir) => fs.existsSync(path.join(dir, CORPUS_DECLARATION_FILE)))
    .sort();
}

/** Validate every declaration under `root`. Pure over the filesystem. */
export function validateCorpora(root: string): CorpusValidationRow[] {
  return findCorpusDirs(root).map((dir) => {
    try {
      const loaded = loadCorpusDeclaration(dir);
      return { dir, ok: true, declaration: loaded!.declaration };
    } catch (e) {
      return {
        dir,
        ok: false,
        error: e instanceof CorpusDeclarationError ? e.message : String(e),
      };
    }
  });
}

function main(): void {
  const asJson = process.argv.includes('--json');
  const rows = validateCorpora(DATA_ROOT);
  const failures = rows.filter((r) => !r.ok);

  if (asJson) {
    console.log(JSON.stringify({ root: DATA_ROOT, rows, failures: failures.length }, null, 2));
  } else {
    console.log(`🔎 Corpus trust declarations under ${DATA_ROOT}`);
    if (rows.length === 0) {
      console.log('   (none found — absence is the fail-closed path, not an error)');
    }
    for (const row of rows) {
      if (row.ok && row.declaration) {
        const g = row.declaration.trustBootstrap;
        const grant = g ? `${g.stage} by ${g.grantor}` : 'no grant declared (nothing minted)';
        console.log(
          `   ✅ ${path.basename(row.dir)}: ${row.declaration.id} ` +
            `[${row.declaration.environment}, rung ${row.declaration.realism.rung}] — ${grant}`,
        );
      } else {
        console.log(`   ❌ ${path.basename(row.dir)}: ${row.error}`);
      }
    }
    console.log(`\n${rows.length - failures.length}/${rows.length} valid`);
  }

  process.exit(failures.length === 0 ? 0 : 1);
}

// Only run as a CLI, never on import (the tests import findCorpusDirs/validateCorpora).
if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(__filename)) {
  main();
}
