#!/usr/bin/env node
/**
 * Pillar Import Boundary Auditor
 *
 * Walks all .ts files under each pillar directory and flags cross-pillar
 * imports that violate the defined boundary rules.
 *
 * Rules:
 *   imagodei  → may import: elohim, @elohim/storage-client, @app/generated, @app/testing
 *   account   → may import: imagodei, elohim, @elohim/storage-client, @app/generated, @app/testing
 *   lamad     → may import: elohim, @elohim/storage-client, @app/generated, @app/testing
 *   shefa     → may import: elohim, @elohim/storage-client, @app/generated, @app/testing
 *   qahal     → may import: elohim, @elohim/storage-client, @app/generated, @app/testing
 *   elohim    → may import: @elohim/storage-client, @app/generated, @app/testing
 *
 * Note: lamad, shefa, and qahal may NOT import each other, imagodei, or account.
 *       elohim may NOT import any sibling pillar.
 *
 * @app/generated and @app/testing are shared — not a pillar, always allowed.
 * @elohim/storage-client is the shared SDK — always allowed.
 */

import { readdir, readFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const SRC_ROOT = join(__dirname, '..', 'src', 'app');

// All pillar names (account omitted — does not exist yet, handled gracefully)
const ALL_PILLARS = ['elohim', 'imagodei', 'lamad', 'shefa', 'qahal', 'account'];

// Which pillars each pillar is allowed to import from (excluding itself)
const ALLOWED_IMPORTS = {
  imagodei: new Set(['elohim']),
  account:  new Set(['imagodei', 'elohim']),
  lamad:    new Set(['elohim']),
  shefa:    new Set(['elohim']),
  qahal:    new Set(['elohim']),
  elohim:   new Set([]),
};

// Patterns that are always allowed regardless of pillar:
//   @elohim/storage-client, @app/generated/*, @app/testing/*
const ALWAYS_ALLOWED_PATTERNS = [
  /^@elohim\//,
  /^@app\/generated/,
  /^@app\/testing/,
];

/**
 * Recursively collect all .ts files under a directory.
 */
async function collectTsFiles(dir) {
  const files = [];
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return files; // Directory does not exist — skip gracefully
  }
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectTsFiles(full)));
    } else if (entry.isFile() && entry.name.endsWith('.ts')) {
      files.push(full);
    }
  }
  return files;
}

/**
 * Extract all @app/<pillar> import targets from a file's source text.
 * Returns an array of { line, importedPillar } objects.
 */
function extractPillarImports(source) {
  const results = [];
  const lines = source.split('\n');
  // Match: from '@app/<pillar>' or from "@app/<pillar>"
  // Also catches: from '@app/<pillar>/<subpath>'
  const importRe = /from\s+['"](@app\/[a-z-]+)/g;
  lines.forEach((lineText, idx) => {
    let match;
    while ((match = importRe.exec(lineText)) !== null) {
      results.push({ line: idx + 1, importTarget: match[1] });
    }
    importRe.lastIndex = 0; // Reset for each line
  });
  return results;
}

/**
 * Determine which pillar (if any) an import target refers to.
 * Returns the pillar name or null if it's not a pillar import.
 */
function targetToPillar(importTarget) {
  // @app/<pillar> or @app/<pillar>/<...>
  const match = /^@app\/([a-z-]+)/.exec(importTarget);
  if (!match) return null;
  const candidate = match[1];
  if (ALL_PILLARS.includes(candidate)) return candidate;
  return null;
}

/**
 * Run the audit. Returns { violations, fileCount }.
 */
async function audit() {
  const violations = [];
  let fileCount = 0;

  for (const pillar of ALL_PILLARS) {
    const pillarDir = join(SRC_ROOT, pillar);
    if (!existsSync(pillarDir)) {
      // Gracefully skip pillars that don't exist yet (e.g., account)
      continue;
    }

    const allowed = ALLOWED_IMPORTS[pillar];
    const tsFiles = await collectTsFiles(pillarDir);
    fileCount += tsFiles.length;

    for (const filePath of tsFiles) {
      const source = await readFile(filePath, 'utf8');
      const imports = extractPillarImports(source);

      for (const { line, importTarget } of imports) {
        // Check always-allowed patterns first
        if (ALWAYS_ALLOWED_PATTERNS.some(re => re.test(importTarget))) continue;

        const importedPillar = targetToPillar(importTarget);
        if (importedPillar === null) continue; // Not a pillar import
        if (importedPillar === pillar) continue; // Self-import is fine

        // It's a cross-pillar import — check if allowed
        if (!allowed.has(importedPillar)) {
          violations.push({
            file: relative(join(__dirname, '..'), filePath),
            line,
            pillar,
            importedPillar,
            importTarget,
          });
        }
      }
    }
  }

  return { violations, fileCount };
}

// ---- Main ----

const { violations, fileCount } = await audit();

console.log(`Pillar Import Audit — scanned ${fileCount} .ts files across pillars\n`);

if (violations.length === 0) {
  console.log('✓ No boundary violations found.');
  process.exit(0);
}

// Group by pillar for readability
const byPillar = {};
for (const v of violations) {
  (byPillar[v.pillar] ??= []).push(v);
}

let count = 0;
for (const [pillar, vs] of Object.entries(byPillar)) {
  console.log(`PILLAR: ${pillar} (${vs.length} violation${vs.length > 1 ? 's' : ''})`);
  for (const v of vs) {
    count++;
    console.log(`  [${count}] ${v.file}:${v.line}`);
    console.log(`        imports @app/${v.importedPillar} — NOT allowed from ${v.pillar}`);
  }
  console.log('');
}

console.error(`\nFAIL — ${violations.length} boundary violation${violations.length > 1 ? 's' : ''} found.`);
process.exit(1);
