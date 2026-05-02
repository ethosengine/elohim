#!/usr/bin/env node
/**
 * Generates TypeScript from protocol schemas:
 * 1. Interfaces from input/view schemas (json-schema-to-typescript)
 * 2. Enum constants from _dna schemas (tier-aware CORE_* and ALL_*)
 *
 * Usage:
 *   node codegen-ts.mjs           # Generate all
 *   node codegen-ts.mjs --verify  # Check if generated files are stale
 */
import { readdir, readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, resolve, basename, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';
import { compile } from 'json-schema-to-typescript';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../');
const SCHEMA_DIR = resolve(__dirname, '../current');
const OUTPUT_DIR = resolve(__dirname, '../generated-ts');
const ENUM_DIR = resolve(__dirname, '../v1/enums');

const VERIFY = process.argv.includes('--verify');

// All locations get identical copies of generated files
const GENERATED_OUTPUT_DIRS = [
  resolve(REPO_ROOT, 'genesis/seeder/src/generated'),
  resolve(REPO_ROOT, 'app/elohim-app/src/app/generated'),
  resolve(REPO_ROOT, 'app/elohim-library/projects/elohim-service/src/generated'),
];

const ENUM_OUTPUT_PATHS = GENERATED_OUTPUT_DIRS.map((d) => join(d, 'schema-enums.ts'));

// Interface files to distribute alongside schema-enums.ts
const INTERFACE_FILES = [
  { src: 'inputs/create-content-input.ts', dest: 'create-content-input.ts' },
  { src: 'inputs/create-economic-event-input.ts', dest: 'create-economic-event-input.ts' },
  { src: 'inputs/create-attestation-input.ts', dest: 'create-attestation-input.ts' },
  { src: 'views/content-view.ts', dest: 'content-view.ts' },
  { src: 'views/economic-event-view.ts', dest: 'economic-event-view.ts' },
  { src: 'views/p2p-status-view.ts', dest: 'p2p-status-view.ts' },
  { src: 'views/drain-status-view.ts', dest: 'drain-status-view.ts' },
  { src: 'views/replication-status-view.ts', dest: 'replication-status-view.ts' },
  { src: 'views/peer-info-view.ts', dest: 'peer-info-view.ts' },
  { src: 'views/peer-list-view.ts', dest: 'peer-list-view.ts' },
  { src: 'views/gate-decision-attestation-view.ts', dest: 'gate-decision-attestation-view.ts' },
  { src: 'views/gate-decision-challenge-view.ts', dest: 'gate-decision-challenge-view.ts' },
  { src: 'views/challenge-outcome-view.ts', dest: 'challenge-outcome-view.ts' },
  { src: 'inputs/wisdom-invocation-input.ts', dest: 'wisdom-invocation-input.ts' },
  { src: 'views/wisdom-invocation-response.ts', dest: 'wisdom-invocation-response.ts' },
  { src: 'views/elohim-capability-profile.ts', dest: 'elohim-capability-profile.ts' },
  { src: 'views/peer-status-view.ts', dest: 'peer-status-view.ts' },
  { src: 'views/elohim-reputation-profile-view.ts', dest: 'elohim-reputation-profile-view.ts' },
  { src: 'views/node-shape-view.ts', dest: 'node-shape-view.ts' },
  { src: 'views/household-devices-view.ts', dest: 'household-devices-view.ts' },
  { src: 'views/network-posture-view.ts', dest: 'network-posture-view.ts' },
  { src: 'views/placement-gap-view.ts', dest: 'placement-gap-view.ts' },
  { src: 'views/resilience-snapshot-view.ts', dest: 'resilience-snapshot-view.ts' },
  { src: 'views/recovery-request.ts', dest: 'recovery-request.ts' },
  { src: 'views/key-rotation.ts', dest: 'key-rotation.ts' },
  { src: 'views/recovery-witness.ts', dest: 'recovery-witness.ts' },
  // P2P protocol wire contracts (Category C operational — internal to libp2p protocols)
  { src: 'p2p/identity-handshake.ts', dest: 'identity-handshake.ts' },
  // Phase 3.5 Trust-Compute Gradient substrate — B2 agent-scoped with attestation
  { src: 'p2p/feedback-signal.ts', dest: 'feedback-signal.ts' },
  // Phase 3.5 Trust-Compute Gradient tending subsystem — B peer-private discernment
  { src: 'p2p/attention-tending.ts', dest: 'attention-tending.ts' },
  { src: 'views/key-revocation.ts', dest: 'key-revocation.ts' },
  { src: 'views/revocation-vote.ts', dest: 'revocation-vote.ts' },
  // M5: Auth Portal Convergence + Revocation UX + Stub Defender
  { src: 'views/human.ts', dest: 'human.ts' },
  { src: 'views/human-relationship.ts', dest: 'human-relationship.ts' },
  { src: 'views/portal-host-view.ts', dest: 'portal-host-view.ts' },
  { src: 'views/agent-peer-binding-view.ts', dest: 'agent-peer-binding-view.ts' },
  { src: 'views/account-view.ts', dest: 'account-view.ts' },
  // Light-Up-Topology Phase 1 — operational distribution + cluster + reciprocity (Category C)
  { src: 'views/distribution-summary.ts', dest: 'distribution-summary.ts' },
  { src: 'views/diversity-hint.ts', dest: 'diversity-hint.ts' },
];

/**
 * Load all enum schemas and build a map of relative-path -> schema object.
 * This allows us to inline $ref targets that use relative file paths.
 */
async function loadRefMap(baseDir) {
  const refMap = new Map();
  const enumDir = join(baseDir, 'enums');
  let files;
  try {
    files = (await readdir(enumDir)).filter((f) => f.endsWith('.schema.json'));
  } catch {
    return refMap;
  }
  for (const file of files) {
    const schema = JSON.parse(await readFile(join(enumDir, file), 'utf8'));
    // Map the relative path as used from inputs/ and views/ directories
    refMap.set(`../enums/${file}`, schema);
    // Also map from same-level reference
    refMap.set(`enums/${file}`, schema);
  }

  // Load view schemas for cross-view $ref (e.g., p2p-status-view → drain-status-view)
  const viewDir = join(baseDir, 'views');
  let viewFiles;
  try {
    viewFiles = (await readdir(viewDir)).filter((f) => f.endsWith('.schema.json'));
  } catch {
    viewFiles = [];
  }
  for (const file of viewFiles) {
    const schema = JSON.parse(await readFile(join(viewDir, file), 'utf8'));
    // Same-directory ref from views/: just the filename
    refMap.set(file, schema);
    // Cross-dir ref from other directories
    refMap.set(`../views/${file}`, schema);
    // URI-style $id ref (e.g. "epr:schema:view:human") — resolves $ref in
    // account-view.schema.json and similar schemas that use canonical $id refs.
    if (schema.$id) {
      refMap.set(schema.$id, schema);
    }
  }

  return refMap;
}

/**
 * Recursively inline $ref values that point to relative file paths,
 * replacing them with the referenced schema content. This is necessary
 * because json-schema-to-typescript does not resolve file-path $ref
 * against our custom $id namespace.
 */
function inlineRefs(schema, refMap) {
  if (typeof schema !== 'object' || schema === null) return schema;
  if (Array.isArray(schema)) return schema.map((item) => inlineRefs(item, refMap));

  const result = {};
  for (const [key, value] of Object.entries(schema)) {
    if (key === '$ref' && typeof value === 'string' && !value.startsWith('#')) {
      const referenced = refMap.get(value);
      if (referenced) {
        // Inline the referenced schema (strip meta-schema fields),
        // then recursively inline any nested $ref in the result
        const { $id, $schema, _dna, _source, ...rest } = referenced;
        Object.assign(result, inlineRefs(rest, refMap));
      } else {
        result[key] = value;
      }
    } else {
      result[key] = inlineRefs(value, refMap);
    }
  }
  return result;
}

async function generateFromDir(subdir, refMap) {
  const dir = join(SCHEMA_DIR, subdir);
  let files;
  try {
    files = (await readdir(dir)).filter((f) => f.endsWith('.schema.json'));
  } catch {
    return [];
  }

  const outDir = join(OUTPUT_DIR, subdir);
  await mkdir(outDir, { recursive: true });

  const generated = [];
  for (const file of files) {
    const raw = await readFile(join(dir, file), 'utf8');
    const schemaRaw = JSON.parse(raw);
    const name = basename(file, '.schema.json');

    // Inline $ref before passing to compiler
    const schema = inlineRefs(schemaRaw, refMap);

    try {
      let ts = await compile(schema, schema.title || name, {
        bannerComment: `/* eslint-disable @typescript-eslint/consistent-indexed-object-style */\n/* Generated from protocol schema: ${subdir}/${file} -- DO NOT EDIT */`,
        additionalProperties: false,
        style: { singleQuote: true, trailingComma: 'all' },
      });

      // Post-process: replace {} empty object type with Record<string, unknown>
      // to satisfy @typescript-eslint/no-empty-object-type rule
      ts = ts.replace(/\?: \{\}/g, '?: Record<string, unknown>');
      ts = ts.replace(/\| \{\}/g, '| Record<string, unknown>');

      const outFile = `${name}.ts`;
      await writeFile(join(outDir, outFile), ts);
      console.log(`Generated: ${subdir}/${outFile}`);
      generated.push({ dir: subdir, file: outFile });
    } catch (err) {
      console.error(`ERROR generating ${subdir}/${file}: ${err.message}`);
    }
  }

  return generated;
}

/**
 * Generate tier-aware enum constants from schemas with _dna metadata.
 * Uses _dna.constant as the base name (e.g. "CONTENT_TYPES", "REACH_LEVELS").
 */
async function generateEnumConstants() {
  const files = (await readdir(ENUM_DIR))
    .filter((f) => f.endsWith('.schema.json'))
    .sort();

  const blocks = [];

  for (const file of files) {
    const raw = await readFile(join(ENUM_DIR, file), 'utf8');
    const schema = JSON.parse(raw);

    if (!schema._dna) continue;

    const title = schema.title;
    const baseName = schema._dna.constant; // e.g. "CONTENT_TYPES", "REACH_LEVELS"
    const allValues = schema.enum;
    const coreValues = schema._tiers?.core?.values || allValues;

    // CORE_* constant
    blocks.push(formatTsConst(`CORE_${baseName}`, coreValues));

    // ALL_* constant
    blocks.push(formatTsConst(`ALL_${baseName}`, allValues));

    // Backward-compat alias: NAME = ALL_NAME
    blocks.push(`export const ${baseName} = ALL_${baseName};`);

    // Type alias from the ALL_* constant
    blocks.push(`export type ${title} = (typeof ALL_${baseName})[number];`);

    blocks.push('');
  }

  const header = `// AUTO-GENERATED from protocol JSON schemas.
// DO NOT EDIT \u2014 regenerate with: pnpm run schema:codegen:ts
//
// Source: elohim/sdk/schemas/v1/enums/*.schema.json
`;

  // Remove trailing empty block to avoid double-newline at EOF
  while (blocks.length > 0 && blocks[blocks.length - 1] === '') blocks.pop();
  return header + '\n' + blocks.join('\n') + '\n';
}

function formatTsConst(name, values) {
  // Try single-line format first (prettier collapses short arrays)
  const singleLine = `export const ${name} = [${values.map((v) => `'${v}'`).join(', ')}] as const;`;
  if (singleLine.length <= 100) return singleLine;
  // Fall back to multi-line for long arrays
  const items = values.map((v) => `  '${v}',`).join('\n');
  return `export const ${name} = [\n${items}\n] as const;`;
}

async function main() {
  // --- Part 1: Interface generation (existing) ---
  if (!VERIFY) {
    await mkdir(OUTPUT_DIR, { recursive: true });

    const refMap = await loadRefMap(SCHEMA_DIR);

    const allGenerated = [];
    for (const subdir of ['enums', 'inputs', 'views', 'p2p']) {
      const results = await generateFromDir(subdir, refMap);
      allGenerated.push(...results);
    }

    // Generate barrel export
    const exports = allGenerated.map(
      ({ dir, file }) => `export * from './${dir}/${basename(file, '.ts')}';`,
    );
    await writeFile(join(OUTPUT_DIR, 'index.ts'), exports.join('\n') + '\n');

    // Run prettier on generated files using elohim-app config (single quotes, 100-char width)
    const prettierConfig = join(REPO_ROOT, 'app/elohim-app/.prettierrc.js');
    try {
      execSync(
        `pnpm exec prettier --write --config "${prettierConfig}" "${OUTPUT_DIR}/**/*.ts"`,
        { cwd: REPO_ROOT, stdio: 'pipe' },
      );
    } catch {
      // prettier not available — generated files remain as-is
    }

    console.log(`\nTypeScript interface generation complete: ${allGenerated.length} files`);
  }

  // --- Part 2: Enum constant generation ---
  const enumContent = await generateEnumConstants();

  // --- Part 3: Interface file distribution ---
  // Read canonical interface files from generated-ts/
  const interfaceContents = new Map();
  for (const { src, dest } of INTERFACE_FILES) {
    const content = await readFile(join(OUTPUT_DIR, src), 'utf8');
    interfaceContents.set(dest, content);
  }

  if (VERIFY) {
    let hasFailure = false;

    // Verify enum files
    for (const outPath of ENUM_OUTPUT_PATHS) {
      let existing;
      try {
        existing = await readFile(outPath, 'utf8');
      } catch {
        console.error(`FAIL: Generated file does not exist: ${outPath}`);
        hasFailure = true;
        continue;
      }
      if (existing !== enumContent) {
        console.error(`FAIL: ${outPath} is stale. Run: pnpm run schema:codegen:ts`);
        hasFailure = true;
      }
    }

    // Verify interface files
    for (const dir of GENERATED_OUTPUT_DIRS) {
      for (const [dest, content] of interfaceContents) {
        const outPath = join(dir, dest);
        let existing;
        try {
          existing = await readFile(outPath, 'utf8');
        } catch {
          console.error(`FAIL: Generated file does not exist: ${outPath}`);
          hasFailure = true;
          continue;
        }
        if (existing !== content) {
          console.error(`FAIL: ${outPath} is stale. Run: pnpm run schema:codegen:ts`);
          hasFailure = true;
        }
      }
    }

    if (hasFailure) {
      process.exit(1);
    }
    console.log('TypeScript codegen is up to date.');
    return;
  }

  // Distribute enum files
  for (const outPath of ENUM_OUTPUT_PATHS) {
    await mkdir(dirname(outPath), { recursive: true });
    await writeFile(outPath, enumContent);
    console.log(`Distributed: ${outPath}`);
  }

  // Distribute interface files
  for (const dir of GENERATED_OUTPUT_DIRS) {
    await mkdir(dir, { recursive: true });
    for (const [dest, content] of interfaceContents) {
      const outPath = join(dir, dest);
      await writeFile(outPath, content);
      console.log(`Distributed: ${outPath}`);
    }
  }

  console.log('TypeScript generation and distribution complete.');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
