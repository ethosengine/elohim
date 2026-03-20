#!/usr/bin/env node
/**
 * Generates Rust enum constants from protocol JSON schemas.
 *
 * Usage:
 *   node codegen-rs.mjs           # Generate
 *   node codegen-rs.mjs --verify  # Check if generated file is stale
 *
 * Source: elohim/sdk/schemas/v1/enums/*.schema.json (schemas with _dna metadata)
 * Output: elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_enums.rs
 */
import { readdir, readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../');
const ENUM_DIR = resolve(__dirname, '../v1/enums');
const OUTPUT_FILE = resolve(
  REPO_ROOT,
  'elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_enums.rs',
);

const VERIFY = process.argv.includes('--verify');

/**
 * Convert kebab-case schema name to SCREAMING_SNAKE_CASE Rust constant name.
 * e.g. "CONTENT_TYPES" stays as-is (comes from _dna.constant).
 */
function toRustDoc(schema) {
  const tier = schema._dna?.tier || 'core';
  const title = schema.title || schema.$id || 'Unknown';
  return { title, tier };
}

/**
 * Generate Rust source from all enum schemas with _dna metadata.
 */
async function generate() {
  const files = (await readdir(ENUM_DIR))
    .filter((f) => f.endsWith('.schema.json'))
    .sort();

  const blocks = [];

  for (const file of files) {
    const raw = await readFile(join(ENUM_DIR, file), 'utf8');
    const schema = JSON.parse(raw);

    if (!schema._dna) continue;

    const { constant } = schema._dna;
    const { title } = toRustDoc(schema);
    const allValues = schema.enum;
    const coreValues = schema._tiers?.core?.values || allValues;
    const coreRationale = schema._tiers?.core?.rationale || '';

    // CORE_* constant (core tier values only)
    blocks.push(formatConst(
      `Core ${title.toLowerCase()} \u2014 ${coreRationale || 'DNA-notarized.'}`,
      `CORE_${constant}`,
      coreValues,
    ));

    // ALL_* constant (full enum)
    blocks.push(formatConst(
      `All ${title.toLowerCase()} \u2014 includes storage-only and extensible.`,
      `ALL_${constant}`,
      allValues,
    ));
  }

  const header = `//! AUTO-GENERATED from protocol JSON schemas.
//! DO NOT EDIT \u2014 regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json
`;

  return header + '\n' + blocks.join('\n');
}

function formatConst(doc, name, values) {
  const items = values.map((v) => `    "${v}",`).join('\n');
  return `/// ${doc}
pub const ${name}: &[&str] = &[
${items}
];
`;
}

async function main() {
  const generated = await generate();

  if (VERIFY) {
    let existing;
    try {
      existing = await readFile(OUTPUT_FILE, 'utf8');
    } catch {
      console.error(`FAIL: Generated file does not exist: ${OUTPUT_FILE}`);
      process.exit(1);
    }

    if (existing === generated) {
      console.log('Rust codegen is up to date.');
      process.exit(0);
    } else {
      console.error('FAIL: Rust codegen is stale. Run: pnpm run schema:codegen:rs');
      process.exit(1);
    }
  }

  await writeFile(OUTPUT_FILE, generated);
  console.log(`Generated: ${OUTPUT_FILE}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
