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
 *         (+ elohim-storage copy) for `_dna` enums; elohim/epr/src/generated/enums.rs for
 *         `_codegen.rust: ["epr"]` enums; elohim/epr/src/generated/ordinals.rs for `_ordinal`.
 */
import { readdir, readFile, writeFile, mkdtemp, mkdir, rm } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { execFileSync } from 'node:child_process';
import { tableRows } from './lib/table-enum.mjs';

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
// epr is a foundational crate upstream of elohim-storage, so it cannot import
// storage's generated_enums.rs. Ordinal slices (e.g. REACH_OPENNESS) for
// `_ordinal` enums are emitted into epr's own generated module instead.
const OUTPUT_EPR_ORDINALS = resolve(
  REPO_ROOT,
  'elohim/epr/src/generated/ordinals.rs',
);
// Enums whose schema declares `_codegen.rust: ["epr", ...]` — vocabularies every Rust
// consumer reads from epr, and which must NEVER be written into an integrity zome's
// generated_enums.rs (that file is compiled into the DNA, so a new const there is a
// DNA-hash move). rea-action is the first: its DNA const is hand-written and only
// checked (check-dna.mjs `_dnaMirror`).
const OUTPUT_EPR_ENUMS = resolve(
  REPO_ROOT,
  'elohim/epr/src/generated/enums.rs',
);

// Non-DNA vocabulary enums carrying a `_table` marker (frame-capability,
// page-info-section) project into lamad-types, the domain crate the html5-app body
// parser lives in and the doorway / chrome / storage producers already share. NEVER
// into the DNA's generated_enums.rs: these are presentation-edge vocabulary, and an
// integrity-zome edit would move the DNA hash.
const OUTPUT_LAMAD_VOCAB = resolve(
  REPO_ROOT,
  'elohim/sdk/domains/lamad/types/src/generated_vocab.rs',
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
 * CORE_* / ALL_* slice blocks for every enum schema whose `specOf(schema)` returns a
 * `{ constant }` spec. Shared by the DNA/storage file (`_dna`) and the epr file
 * (`_codegen.rust` includes "epr") so the two cannot format differently.
 */
async function generateConstBlocks(specOf) {
  const files = (await readdir(ENUM_DIR))
    .filter((f) => f.endsWith('.schema.json'))
    .sort();

  const blocks = [];

  for (const file of files) {
    const raw = await readFile(join(ENUM_DIR, file), 'utf8');
    const schema = JSON.parse(raw);

    const spec = specOf(schema);
    if (!spec) continue;

    const { constant } = spec;
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

  return blocks;
}

/**
 * Generate Rust source from all enum schemas with _dna metadata (DNA + storage copies).
 */
async function generateDnaEnums() {
  const blocks = await generateConstBlocks((schema) => schema._dna);
  const header = `//! AUTO-GENERATED from protocol JSON schemas.
//! DO NOT EDIT \u2014 regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json
`;

  return header + '\n' + blocks.join('\n');
}

/**
 * Generate Rust source for enums routed to the epr crate (`_codegen.rust` includes "epr").
 * Returns null when no schema asks for it (nothing to write).
 */
async function generateEprEnums() {
  const blocks = await generateConstBlocks((schema) =>
    Array.isArray(schema._codegen?.rust) && schema._codegen.rust.includes('epr')
      ? schema._codegen
      : null,
  );
  if (blocks.length === 0) return null;

  const header = `//! AUTO-GENERATED from protocol JSON schemas whose \`_codegen.rust\` names "epr".
//! DO NOT EDIT \u2014 regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json
//!
//! Lives in epr because epr is upstream of epr-rea and elohim-storage, and because these
//! vocabularies must never be generated into an integrity zome (that moves the DNA hash).
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
 * Format an ordinal slice for an `_ordinal` enum: (value, openness score) pairs
 * where the score is the 1-based declaration index (1 = most restrictive,
 * N = most open). Derives the const name from the enum's _dna.constant
 * (e.g. REACH_LEVELS -> REACH_OPENNESS).
 */
function formatOrdinalSlice(constant, allValues) {
  const name = constant.replace(/_LEVELS$/, '') + '_OPENNESS';
  const rows = allValues.map((v, i) => `    ("${v}", ${i + 1}),`).join('\n');
  return [
    `/// (value, ordinal score) — 1 = lowest, ${allValues.length} = highest.`,
    `pub const ${name}: &[(&str, u8)] = &[`,
    rows,
    `];`,
    ``,
  ].join('\n');
}

/**
 * Generate Rust source for epr-bound ordinal slices from `_ordinal` enum schemas.
 * Returns null if no `_ordinal` enums exist (nothing to write).
 */
async function generateEprOrdinals() {
  const files = (await readdir(ENUM_DIR))
    .filter((f) => f.endsWith('.schema.json'))
    .sort();

  const blocks = [];

  for (const file of files) {
    const raw = await readFile(join(ENUM_DIR, file), 'utf8');
    const schema = JSON.parse(raw);

    // Gate on the explicit _ordinal marker (mirrors the TS half); never a name heuristic.
    if (schema._ordinal !== true) continue;
    if (!schema._dna) continue;

    const { constant } = schema._dna;
    blocks.push(formatOrdinalSlice(constant, schema.enum));
  }

  if (blocks.length === 0) return null;

  const header = `//! AUTO-GENERATED from protocol JSON schemas with the \`_ordinal\` marker.
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json (\`_ordinal: true\`)
//!
//! Lives in epr (not elohim-storage) because epr is upstream of storage and
//! cannot import storage's generated enums without a circular dependency.
`;

  return header + '\n' + blocks.join('\n');
}

/** kebab-case / snake value → PascalCase Rust variant name. */
function rustVariant(value) {
  return value
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join('');
}

function rustStr(v) {
  return JSON.stringify(String(v));
}

/** The first sentence of a schema description, for a one-line doc comment. */
function firstSentence(text) {
  if (!text) return '';
  const m = text.match(/^(.*?[.!?])(\s|$)/s);
  return (m ? m[1] : text).replace(/\s+/g, ' ').trim();
}

/**
 * Rust enum + accessor methods for one `_table` vocabulary enum. Annotation values
 * are typed by what the rows carry: all-boolean → bool, strings with any null →
 * Option<&'static str>, all strings → &'static str.
 */
function formatRustTable(schema) {
  const type = schema._table.rustType;
  const constant = schema._table.constant;
  if (!type) throw new Error(`${schema.title}: _table.rustType is required for Rust codegen`);
  const rows = tableRows(schema);
  const mapping = schema._table.annotations || {};
  const lines = [];
  const doc = firstSentence(schema.description);
  lines.push(`/// ${schema.title} — ${doc}`);
  lines.push('///');
  lines.push(`/// Generated from \`${schema.$id}\`; wire values are the schema's kebab-case strings.`);
  lines.push('#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]');
  lines.push(`pub enum ${type} {`);
  for (const r of rows) {
    lines.push(`    #[serde(rename = ${rustStr(r.value)})]`);
    lines.push(`    ${rustVariant(r.value)},`);
  }
  lines.push('}');
  lines.push('');
  lines.push(`impl ${type} {`);
  lines.push('    /// Every value, in schema order.');
  lines.push(`    pub const ALL: &'static [${type}] = &[`);
  for (const r of rows) lines.push(`        ${type}::${rustVariant(r.value)},`);
  lines.push('    ];');
  lines.push('');
  lines.push('    /// The wire value.');
  lines.push('    pub const fn as_str(self) -> &\'static str {');
  lines.push('        match self {');
  for (const r of rows) lines.push(`            ${type}::${rustVariant(r.value)} => ${rustStr(r.value)},`);
  lines.push('        }');
  lines.push('    }');
  lines.push('');
  lines.push('    /// Parse a wire value; `None` for anything outside the vocabulary.');
  lines.push('    pub fn parse(value: &str) -> Option<Self> {');
  lines.push('        match value {');
  for (const r of rows) lines.push(`            ${rustStr(r.value)} => Some(${type}::${rustVariant(r.value)}),`);
  lines.push('            _ => None,');
  lines.push('        }');
  lines.push('    }');
  for (const [key, names] of Object.entries(mapping)) {
    const kinds = new Set(rows.map((r) => (r[key] === null ? 'null' : typeof r[key])));
    let ret;
    let lit;
    if (kinds.has('boolean')) {
      if (kinds.size !== 1) throw new Error(`${schema.title}: ${key} mixes booleans with other values`);
      ret = 'bool';
      lit = (v) => String(v);
    } else if (kinds.has('null')) {
      ret = "Option<&'static str>";
      lit = (v) => (v === null ? 'None' : `Some(${rustStr(v)})`);
    } else {
      ret = "&'static str";
      lit = (v) => rustStr(v);
    }
    lines.push('');
    lines.push(`    /// The value's \`${key}\` annotation.`);
    lines.push(`    pub const fn ${names.rs}(self) -> ${ret} {`);
    lines.push('        match self {');
    for (const r of rows) lines.push(`            ${type}::${rustVariant(r.value)} => ${lit(r[key])},`);
    lines.push('        }');
    lines.push('    }');
  }
  lines.push('}');
  lines.push('');
  lines.push(`impl std::fmt::Display for ${type} {`);
  lines.push('    fn fmt(&self, f: &mut std::fmt::Formatter<\'_>) -> std::fmt::Result {');
  lines.push('        f.write_str(self.as_str())');
  lines.push('    }');
  lines.push('}');
  if (Object.keys(mapping).length > 0) {
    const json = JSON.stringify(
      rows.map((r) => {
        const o = { value: r.value };
        for (const [key, names] of Object.entries(mapping)) o[names.ts] = r[key];
        return o;
      }),
    );
    lines.push('');
    lines.push(`/// The ${schema.title} table as compact JSON, one object per value in schema order,`);
    lines.push('/// keyed exactly like the TS `' + constant + '_TABLE` rows — for a script that cannot');
    lines.push('/// import TS (e.g. the chrome element, which splices it in at serve time).');
    lines.push(`pub const ${constant}_TABLE_JSON: &str = r#"${json}"#;`);
  }
  return lines.join('\n');
}

/**
 * lamad-types/src/generated_vocab.rs from every `_table` enum without `_dna`.
 * Returns null when there are none.
 */
async function generateLamadVocab() {
  const files = (await readdir(ENUM_DIR)).filter((f) => f.endsWith('.schema.json')).sort();
  const blocks = [];
  for (const file of files) {
    const schema = JSON.parse(await readFile(join(ENUM_DIR, file), 'utf8'));
    if (schema._dna || !schema._table) continue;
    blocks.push(formatRustTable(schema));
  }
  if (blocks.length === 0) return null;
  const header = `//! AUTO-GENERATED from protocol vocabulary schemas carrying a \`_table\` marker.
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json (\`_table\`, no \`_dna\`)

use serde::{Deserialize, Serialize};
`;
  return header + '\n' + blocks.join('\n\n') + '\n';
}

/**
 * Resolves a manifest block that may be an inline object or a JSON $ref pointer.
 * When the block has a "$ref" key, reads and parses the referenced JSON relative
 * to the manifest file's directory. Returns the resolved object (or {} if absent).
 */
async function resolveRefBlock(block, manifestPath) {
  if (block && typeof block === 'object' && block['$ref']) {
    const refPath = resolve(dirname(manifestPath), block['$ref']);
    return JSON.parse(await readFile(refPath, 'utf8'));
  }
  return block || {};
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

    const attestations = await resolveRefBlock(manifest['attestations'], manifestPath);
    for (const kind of Object.keys(attestations)) {
      attestationKinds.set(kind, pillar);
    }

    const govActions = await resolveRefBlock(manifest['governance-actions'], manifestPath);
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
  const generated = await generateDnaEnums();
  const generatedKinds = await generateAttestationKinds();
  const generatedEprOrdinals = await generateEprOrdinals();
  const generatedEprEnums = await generateEprEnums();
  const generatedLamadVocab = await generateLamadVocab();

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

    // Verify epr ordinal slice file (REACH_OPENNESS, etc.)
    if (generatedEprOrdinals !== null) {
      const tmpEprOrdinals = join(tmpDir, 'ordinals.rs');
      await writeFile(tmpEprOrdinals, generatedEprOrdinals);
      try {
        execFileSync('rustfmt', [tmpEprOrdinals], { stdio: 'pipe' });
      } catch {
        // rustfmt not available — compare raw
      }
      const expectedEprOrdinals = await readFile(tmpEprOrdinals, 'utf8');
      let existingEprOrdinals;
      try {
        existingEprOrdinals = await readFile(OUTPUT_EPR_ORDINALS, 'utf8');
      } catch {
        console.error(`FAIL: Generated file does not exist (epr-ordinal): ${OUTPUT_EPR_ORDINALS}`);
        stale = true;
      }
      if (existingEprOrdinals !== undefined && existingEprOrdinals !== expectedEprOrdinals) {
        console.error('FAIL: Rust epr-ordinal codegen is stale. Run: pnpm run schema:codegen:rs');
        stale = true;
      }
    }

    // Verify epr enum file (REA_ACTIONS, etc.)
    if (generatedEprEnums !== null) {
      const tmpEprEnums = join(tmpDir, 'enums.rs');
      await writeFile(tmpEprEnums, generatedEprEnums);
      try {
        execFileSync('rustfmt', [tmpEprEnums], { stdio: 'pipe' });
      } catch {
        // rustfmt not available — compare raw
      }
      const expectedEprEnums = await readFile(tmpEprEnums, 'utf8');
      let existingEprEnums;
      try {
        existingEprEnums = await readFile(OUTPUT_EPR_ENUMS, 'utf8');
      } catch {
        console.error(`FAIL: Generated file does not exist (epr-enums): ${OUTPUT_EPR_ENUMS}`);
        stale = true;
      }
      if (existingEprEnums !== undefined && existingEprEnums !== expectedEprEnums) {
        console.error('FAIL: Rust epr-enums codegen is stale. Run: pnpm run schema:codegen:rs');
        stale = true;
      }
    }

    // Verify lamad-types vocabulary file (FrameCapability, PageInfoSection, ...)
    if (generatedLamadVocab !== null) {
      const tmpVocab = join(tmpDir, 'generated_vocab.rs');
      await writeFile(tmpVocab, generatedLamadVocab);
      try {
        execFileSync('rustfmt', ['--edition', '2021', tmpVocab], { stdio: 'pipe' });
      } catch {
        // rustfmt not available — compare raw
      }
      const expectedVocab = await readFile(tmpVocab, 'utf8');
      let existingVocab;
      try {
        existingVocab = await readFile(OUTPUT_LAMAD_VOCAB, 'utf8');
      } catch {
        console.error(`FAIL: Generated file does not exist (lamad-vocab): ${OUTPUT_LAMAD_VOCAB}`);
        stale = true;
      }
      if (existingVocab !== undefined && existingVocab !== expectedVocab) {
        console.error('FAIL: Rust lamad-vocab codegen is stale. Run: pnpm run schema:codegen:rs');
        stale = true;
      }
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

  // Write epr ordinal slice file (REACH_OPENNESS, etc. — epr is upstream of storage)
  if (generatedEprOrdinals !== null) {
    await mkdir(dirname(OUTPUT_EPR_ORDINALS), { recursive: true });
    await writeFile(OUTPUT_EPR_ORDINALS, generatedEprOrdinals);
    try {
      execFileSync('rustfmt', [OUTPUT_EPR_ORDINALS], { stdio: 'pipe' });
    } catch {
      // rustfmt not available
    }
    console.log(`Generated (epr-ordinal): ${OUTPUT_EPR_ORDINALS}`);
  }

  // Write epr enum file (REA_ACTIONS, etc. — never into an integrity zome)
  if (generatedEprEnums !== null) {
    await mkdir(dirname(OUTPUT_EPR_ENUMS), { recursive: true });
    await writeFile(OUTPUT_EPR_ENUMS, generatedEprEnums);
    try {
      execFileSync('rustfmt', [OUTPUT_EPR_ENUMS], { stdio: 'pipe' });
    } catch {
      // rustfmt not available
    }
    console.log(`Generated (epr-enums): ${OUTPUT_EPR_ENUMS}`);
  }

  // Write lamad-types vocabulary file (non-DNA `_table` enums)
  if (generatedLamadVocab !== null) {
    await writeFile(OUTPUT_LAMAD_VOCAB, generatedLamadVocab);
    try {
      execFileSync('rustfmt', ['--edition', '2021', OUTPUT_LAMAD_VOCAB], { stdio: 'pipe' });
    } catch {
      // rustfmt not available
    }
    console.log(`Generated (lamad-vocab): ${OUTPUT_LAMAD_VOCAB}`);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
