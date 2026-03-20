#!/usr/bin/env node
/**
 * Verifies that generated Rust constants match protocol schema enum definitions.
 * Parses generated_enums.rs to extract CORE_* and ALL_* arrays, then compares
 * against schema _tiers.core and full enum values respectively.
 *
 * Usage: node elohim/sdk/schemas/scripts/check-dna.mjs
 */
import { readFile, readdir } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../');
const SCHEMA_DIR = resolve(__dirname, '../v1/enums');
const GENERATED_RS = resolve(
  REPO_ROOT,
  'elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_enums.rs',
);

/**
 * Extract a `pub const NAME: &[&str] = &[...]` array from generated Rust source.
 * Also handles legacy `[&str; N]` fixed-size arrays for backward compat.
 */
function extractRustConstArray(source, constName) {
  // Match both &[&str] slices and [&str; N] fixed arrays
  const pattern = new RegExp(
    `pub\\s+const\\s+${constName}\\s*:\\s*(?:&\\[&str\\]|\\[&str;\\s*\\d+\\])\\s*=\\s*&?\\[([\\s\\S]*?)\\];`,
  );
  const match = source.match(pattern);
  if (!match) return null;

  // Strip comments first (line by line), then join and split by commas.
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
  let generatedSource;
  try {
    generatedSource = await readFile(GENERATED_RS, 'utf8');
  } catch {
    console.error(`FAIL: Could not read generated enums: ${GENERATED_RS}`);
    console.error('Run: pnpm run schema:codegen:rs');
    process.exit(1);
  }

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

    const { constant: constName } = schema._dna;
    const coreValues = schema._tiers?.core?.values || schema.enum;

    // Check CORE_* constant matches _tiers.core values
    const coreRust = extractRustConstArray(generatedSource, `CORE_${constName}`);
    if (!coreRust) {
      console.error(`FAIL: Could not find CORE_${constName} in generated_enums.rs`);
      failures++;
      continue;
    }

    checks++;
    let enumFailures = 0;

    // CORE_* should exactly match _tiers.core.values
    for (const val of coreRust) {
      if (!coreValues.includes(val)) {
        console.error(
          `FAIL: CORE_${constName} has "${val}" but schema ${file} _tiers.core does not`,
        );
        enumFailures++;
      }
    }
    for (const val of coreValues) {
      if (!coreRust.includes(val)) {
        console.error(
          `FAIL: Schema ${file} _tiers.core has "${val}" but CORE_${constName} does not`,
        );
        enumFailures++;
      }
    }

    // Check ALL_* constant matches full enum
    const allRust = extractRustConstArray(generatedSource, `ALL_${constName}`);
    if (!allRust) {
      console.error(`FAIL: Could not find ALL_${constName} in generated_enums.rs`);
      failures++;
      continue;
    }

    for (const val of allRust) {
      if (!schema.enum.includes(val)) {
        console.error(
          `FAIL: ALL_${constName} has "${val}" but schema ${file} enum does not`,
        );
        enumFailures++;
      }
    }
    for (const val of schema.enum) {
      if (!allRust.includes(val)) {
        console.error(
          `FAIL: Schema ${file} enum has "${val}" but ALL_${constName} does not`,
        );
        enumFailures++;
      }
    }

    if (enumFailures === 0) {
      console.log(
        `PASS: ${file} <-> CORE_${constName} (${coreRust.length}), ALL_${constName} (${allRust.length})`,
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
