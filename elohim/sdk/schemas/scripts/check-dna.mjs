#!/usr/bin/env node
/**
 * Verifies that DNA constants match protocol schema enum definitions.
 * Parses Rust source to extract const arrays and compares against schema enums.
 *
 * Usage: node elohim/sdk/schemas/scripts/check-dna.mjs
 */
import { readFile, readdir } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../');
const SCHEMA_DIR = resolve(__dirname, '../current/enums');

/**
 * Extract a `pub const NAME: [&str; N] = [...]` array from Rust source.
 * Handles multi-line arrays with inline comments.
 */
function extractRustConstArray(source, constName) {
  const pattern = new RegExp(
    `pub\\s+const\\s+${constName}\\s*:\\s*\\[&str;\\s*\\d+\\]\\s*=\\s*\\[([^\\]]+)\\]`,
    's',
  );
  const match = source.match(pattern);
  if (!match) return null;

  // Strip comments first (line by line), then join and split by commas.
  // This prevents commas inside comments from being treated as separators.
  const stripped = match[1]
    .split('\n')
    .map((line) => line.replace(/\/\/.*$/, ''))
    .join(' ');

  return stripped
    .split(',')
    .map((s) => s.trim().replace(/^"/, '').replace(/"$/, ''))
    .filter((s) => s.length > 0);
}

async function main() {
  const enumFiles = (await readdir(SCHEMA_DIR)).filter((f) =>
    f.endsWith('.schema.json'),
  );
  let failures = 0;
  let checks = 0;

  for (const file of enumFiles) {
    const schemaRaw = await readFile(join(SCHEMA_DIR, file), 'utf8');
    const schema = JSON.parse(schemaRaw);

    // Only check schemas that declare a _dna mapping
    if (!schema._dna) continue;

    const { constant: constName, file: dnaFile } = schema._dna;
    const dnaPath = resolve(REPO_ROOT, dnaFile);

    let dnaSource;
    try {
      dnaSource = await readFile(dnaPath, 'utf8');
    } catch {
      console.error(`FAIL: Could not read DNA source: ${dnaPath}`);
      failures++;
      continue;
    }

    const dnaValues = extractRustConstArray(dnaSource, constName);
    if (!dnaValues) {
      console.error(`FAIL: Could not find const ${constName} in ${dnaFile}`);
      failures++;
      continue;
    }

    checks++;
    let enumFailures = 0;

    // Check that every DNA value is in the schema enum
    for (const val of dnaValues) {
      if (!schema.enum.includes(val)) {
        console.error(
          `FAIL: DNA ${constName} has "${val}" but schema ${file} does not`,
        );
        enumFailures++;
      }
    }

    // Check that every schema enum value is in the DNA (unless _storageOnly)
    const storageOnly = schema._storageOnly || [];
    for (const val of schema.enum) {
      if (!dnaValues.includes(val) && !storageOnly.includes(val)) {
        console.error(
          `FAIL: Schema ${file} has "${val}" but DNA ${constName} does not (and not marked _storageOnly)`,
        );
        enumFailures++;
      }
    }

    if (enumFailures === 0) {
      console.log(
        `PASS: ${file} <-> ${constName} (${dnaValues.length} DNA values, ${schema.enum.length} schema values)`,
      );
    } else {
      failures += enumFailures;
    }
  }

  if (checks === 0) {
    console.log('No schemas with _dna mappings found.');
  }

  console.log(
    `\n${failures === 0 ? 'ALL DNA CHECKS PASSED' : `${failures} DNA CHECK(S) FAILED`}`,
  );
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
