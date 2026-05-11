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
const GENERATED_RS_ATTESTATION_KINDS = resolve(
  REPO_ROOT,
  'elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs',
);
const DOMAINS_DIR = resolve(REPO_ROOT, 'elohim/sdk/domains');

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

/**
 * Check that ATTESTATION_KINDS and GOVERNANCE_ACTION_KINDS in generated_attestation_kinds.rs
 * exactly match the union of keys declared in pillar manifests' attestations / governance-actions
 * sections, and vice versa.
 *
 * @returns {Promise<{failures: number, checks: number}>}
 */
async function checkAttestationKindsParity() {
  let failures = 0;
  let checks = 0;

  // --- Read generated Rust constants ---
  let attestationSource;
  try {
    attestationSource = await readFile(GENERATED_RS_ATTESTATION_KINDS, 'utf8');
  } catch {
    console.error(
      `FAIL: Could not read generated attestation kinds: ${GENERATED_RS_ATTESTATION_KINDS}`,
    );
    console.error('Run: pnpm run schema:codegen:rs');
    return { failures: 1, checks: 0 };
  }

  const rustAttestationKinds = extractRustConstArray(attestationSource, 'ATTESTATION_KINDS');
  const rustGovernanceActionKinds = extractRustConstArray(
    attestationSource,
    'GOVERNANCE_ACTION_KINDS',
  );

  if (!rustAttestationKinds) {
    console.error('FAIL: Could not find ATTESTATION_KINDS in generated_attestation_kinds.rs');
    return { failures: 1, checks: 0 };
  }
  if (!rustGovernanceActionKinds) {
    console.error(
      'FAIL: Could not find GOVERNANCE_ACTION_KINDS in generated_attestation_kinds.rs',
    );
    return { failures: 1, checks: 0 };
  }

  // --- Read manifest keys across all pillar domains ---
  const PILLAR_DOMAINS = ['imagodei', 'lamad', 'infrastructure', 'mishpat'];
  const manifestAttestationKinds = new Set();
  const manifestGovernanceActionKinds = new Set();

  for (const domain of PILLAR_DOMAINS) {
    const manifestPath = join(DOMAINS_DIR, domain, 'manifest.json');
    let manifest;
    try {
      manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
    } catch {
      console.error(`FAIL: Could not read manifest for domain "${domain}": ${manifestPath}`);
      failures++;
      continue;
    }
    for (const key of Object.keys(manifest.attestations ?? {})) {
      manifestAttestationKinds.add(key);
    }
    for (const key of Object.keys(manifest['governance-actions'] ?? {})) {
      manifestGovernanceActionKinds.add(key);
    }
  }

  console.log('\nAttestation-kinds parity check:');

  // --- Compare ATTESTATION_KINDS ---
  checks++;
  {
    const rustSet = new Set(rustAttestationKinds);
    const inRustNotManifest = rustAttestationKinds.filter((k) => !manifestAttestationKinds.has(k));
    const inManifestNotRust = [...manifestAttestationKinds].filter((k) => !rustSet.has(k));

    if (inRustNotManifest.length === 0 && inManifestNotRust.length === 0) {
      console.log(
        `  ATTESTATION_KINDS: ${rustAttestationKinds.length} in Rust, ${manifestAttestationKinds.size} in manifests — ✓ matched`,
      );
    } else {
      console.error('  ATTESTATION_KINDS:');
      if (inRustNotManifest.length > 0) {
        console.error(
          `    in Rust but missing from manifests: ${JSON.stringify(inRustNotManifest)}`,
        );
      }
      if (inManifestNotRust.length > 0) {
        console.error(
          `    in manifests but missing from Rust: ${JSON.stringify(inManifestNotRust)}`,
        );
      }
      console.error('  FAIL');
      failures += inRustNotManifest.length + inManifestNotRust.length;
    }
  }

  // --- Compare GOVERNANCE_ACTION_KINDS ---
  checks++;
  {
    const rustSet = new Set(rustGovernanceActionKinds);
    const inRustNotManifest = rustGovernanceActionKinds.filter(
      (k) => !manifestGovernanceActionKinds.has(k),
    );
    const inManifestNotRust = [...manifestGovernanceActionKinds].filter((k) => !rustSet.has(k));

    if (inRustNotManifest.length === 0 && inManifestNotRust.length === 0) {
      console.log(
        `  GOVERNANCE_ACTION_KINDS: ${rustGovernanceActionKinds.length} in Rust, ${manifestGovernanceActionKinds.size} in manifests — ✓ matched`,
      );
    } else {
      console.error('  GOVERNANCE_ACTION_KINDS:');
      if (inRustNotManifest.length > 0) {
        console.error(
          `    in Rust but missing from manifests: ${JSON.stringify(inRustNotManifest)}`,
        );
      }
      if (inManifestNotRust.length > 0) {
        console.error(
          `    in manifests but missing from Rust: ${JSON.stringify(inManifestNotRust)}`,
        );
      }
      console.error('  FAIL');
      failures += inRustNotManifest.length + inManifestNotRust.length;
    }
  }

  return { failures, checks };
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

  // Attestation-kinds parity check (DNA-vs-manifest)
  const parityResult = await checkAttestationKindsParity();
  failures += parityResult.failures;
  checks += parityResult.checks;

  console.log(
    `\n${failures === 0 ? 'ALL DNA CHECKS PASSED' : `${failures} DNA CHECK(S) FAILED`}`,
  );
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
