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
import { execFileSync } from 'node:child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../');
const ENUM_DIR = resolve(__dirname, '../v1/enums');
const DOMAINS_DIR = resolve(__dirname, '../../domains');
const OUTPUT_DNA = resolve(
  REPO_ROOT,
  'elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_enums.rs',
);
const OUTPUT_STORAGE = resolve(
  REPO_ROOT,
  'elohim/elohim-storage/src/generated_enums.rs',
);
const OUTPUT_ATTESTATION_KINDS = resolve(
  REPO_ROOT,
  'elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs',
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

/**
 * Walk all 4 pillar manifests and collect attestation kinds + governance-action kinds.
 * Emits a Rust constants file with ATTESTATION_KINDS, GOVERNANCE_ACTION_KINDS,
 * manifest_ref_for_attestation_kind(), and manifest_ref_for_governance_action_kind().
 */
async function generateAttestationKinds() {
  const pillars = ['imagodei', 'lamad', 'infrastructure', 'mishpat'];

  // kind -> pillar name (for manifest_ref helpers)
  const attestationKinds = new Map(); // kind -> pillar
  const governanceActionKinds = new Map(); // kind -> pillar

  for (const pillar of pillars) {
    const manifestPath = join(DOMAINS_DIR, pillar, 'manifest.json');
    let manifest;
    try {
      manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
    } catch {
      // Pillar manifest not found — skip
      continue;
    }

    const attestations = manifest['attestations'] || {};
    for (const kind of Object.keys(attestations)) {
      attestationKinds.set(kind, pillar);
    }

    const govActions = manifest['governance-actions'] || {};
    for (const kind of Object.keys(govActions)) {
      governanceActionKinds.set(kind, pillar);
    }
  }

  // Sort alphabetically for deterministic output
  const sortedAttestationKinds = [...attestationKinds.keys()].sort();
  const sortedGovActionKinds = [...governanceActionKinds.keys()].sort();

  const attestationItems = sortedAttestationKinds.map((k) => `    "${k}",`).join('\n');
  const govActionItems = sortedGovActionKinds.map((k) => `    "${k}",`).join('\n');

  const attestationMatchArms = sortedAttestationKinds
    .map((k) => `        "${k}" => Some("${attestationKinds.get(k)}"),`)
    .join('\n');
  const govActionMatchArms = sortedGovActionKinds
    .map((k) => `        "${k}" => Some("${governanceActionKinds.get(k)}"),`)
    .join('\n');

  return `//! AUTO-GENERATED from pillar manifests' attestations + governance-actions sections.
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/domains/{imagodei,lamad,infrastructure,mishpat}/manifest.json

/// Every attestation subtype declared across pillar manifests. Sorted alphabetically.
pub const ATTESTATION_KINDS: &[&str] = &[
${attestationItems}
];

/// Every governance-action kind declared across pillar manifests. Sorted alphabetically.
pub const GOVERNANCE_ACTION_KINDS: &[&str] = &[
${govActionItems}
];

/// Maps an attestation subtype to the pillar manifest that declares it.
pub fn manifest_ref_for_attestation_kind(kind: &str) -> Option<&'static str> {
    match kind {
${attestationMatchArms}
        _ => None,
    }
}

/// Maps a governance-action kind to the pillar manifest that declares it.
pub fn manifest_ref_for_governance_action_kind(kind: &str) -> Option<&'static str> {
    match kind {
${govActionMatchArms}
        _ => None,
    }
}
`;
}

async function main() {
  const generated = await generate();
  const generatedKinds = await generateAttestationKinds();

  const enumTargets = [
    { label: 'DNA', path: OUTPUT_DNA },
    { label: 'Storage', path: OUTPUT_STORAGE },
  ];

  if (VERIFY) {
    // Write to temp file, run rustfmt, then compare against each target
    const tmpDir = await mkdtemp(join(tmpdir(), 'codegen-rs-'));
    let stale = false;

    // Verify enum files
    const tmpEnums = join(tmpDir, 'generated_enums.rs');
    for (const { label, path } of enumTargets) {
      await writeFile(tmpEnums, generated);
      try {
        execFileSync('rustfmt', [tmpEnums], { stdio: 'pipe' });
      } catch {
        // rustfmt not available — compare raw
      }
      const expected = await readFile(tmpEnums, 'utf8');
      let existing;
      try {
        existing = await readFile(path, 'utf8');
      } catch {
        console.error(`FAIL: Generated file does not exist (${label}): ${path}`);
        stale = true;
        continue;
      }
      if (existing !== expected) {
        console.error(`FAIL: Rust codegen is stale (${label}). Run: pnpm run schema:codegen:rs`);
        stale = true;
      }
    }

    // Verify attestation kinds file
    const tmpKinds = join(tmpDir, 'generated_attestation_kinds.rs');
    await writeFile(tmpKinds, generatedKinds);
    try {
      execFileSync('rustfmt', [tmpKinds], { stdio: 'pipe' });
    } catch {
      // rustfmt not available — compare raw
    }
    const expectedKinds = await readFile(tmpKinds, 'utf8');
    let existingKinds;
    try {
      existingKinds = await readFile(OUTPUT_ATTESTATION_KINDS, 'utf8');
    } catch {
      console.error(`FAIL: Generated file does not exist (attestation-kinds): ${OUTPUT_ATTESTATION_KINDS}`);
      stale = true;
    }
    if (existingKinds !== undefined && existingKinds !== expectedKinds) {
      console.error('FAIL: Rust attestation-kinds codegen is stale. Run: pnpm run schema:codegen:rs');
      stale = true;
    }

    await rm(tmpDir, { recursive: true });
    if (stale) process.exit(1);
    console.log('Rust codegen is up to date.');
    process.exit(0);
  }

  // Write enum files (DNA + Storage)
  for (const { label, path } of enumTargets) {
    await writeFile(path, generated);
    // Run rustfmt so the generated file matches each target's formatting config
    try {
      execFileSync('rustfmt', [path], { stdio: 'pipe' });
    } catch {
      // rustfmt not available — file is still valid Rust, just may not match fmt
    }
    console.log(`Generated (${label}): ${path}`);
  }

  // Write attestation kinds file (DNA only — referenced by zome)
  await writeFile(OUTPUT_ATTESTATION_KINDS, generatedKinds);
  try {
    execFileSync('rustfmt', [OUTPUT_ATTESTATION_KINDS], { stdio: 'pipe' });
  } catch {
    // rustfmt not available
  }
  console.log(`Generated (attestation-kinds): ${OUTPUT_ATTESTATION_KINDS}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
