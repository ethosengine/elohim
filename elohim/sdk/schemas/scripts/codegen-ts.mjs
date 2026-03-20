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
import { compile } from 'json-schema-to-typescript';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../');
const SCHEMA_DIR = resolve(__dirname, '../current');
const OUTPUT_DIR = resolve(__dirname, '../generated-ts');
const ENUM_DIR = resolve(__dirname, '../v1/enums');

const VERIFY = process.argv.includes('--verify');

// Both locations get identical schema-enums.ts
const ENUM_OUTPUT_PATHS = [
  resolve(REPO_ROOT, 'genesis/seeder/src/generated/schema-enums.ts'),
  resolve(REPO_ROOT, 'app/elohim-app/src/app/generated/schema-enums.ts'),
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
        // Inline the referenced schema (strip meta-schema fields)
        const { $id, $schema, _dna, _source, ...rest } = referenced;
        Object.assign(result, rest);
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
      const ts = await compile(schema, schema.title || name, {
        bannerComment: `/* Generated from protocol schema: ${subdir}/${file} -- DO NOT EDIT */`,
        additionalProperties: false,
        style: { singleQuote: true, trailingComma: 'all' },
      });

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

  return header + '\n' + blocks.join('\n') + '\n';
}

function formatTsConst(name, values) {
  const items = values.map((v) => `  '${v}',`).join('\n');
  return `export const ${name} = [\n${items}\n] as const;`;
}

async function main() {
  // --- Part 1: Interface generation (existing) ---
  if (!VERIFY) {
    await mkdir(OUTPUT_DIR, { recursive: true });

    const refMap = await loadRefMap(SCHEMA_DIR);

    const allGenerated = [];
    for (const subdir of ['enums', 'inputs', 'views']) {
      const results = await generateFromDir(subdir, refMap);
      allGenerated.push(...results);
    }

    // Generate barrel export
    const exports = allGenerated.map(
      ({ dir, file }) => `export * from './${dir}/${basename(file, '.ts')}';`,
    );
    await writeFile(join(OUTPUT_DIR, 'index.ts'), exports.join('\n') + '\n');

    console.log(`\nTypeScript interface generation complete: ${allGenerated.length} files`);
  }

  // --- Part 2: Enum constant generation ---
  const enumContent = await generateEnumConstants();

  if (VERIFY) {
    let hasFailure = false;
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
    if (hasFailure) {
      process.exit(1);
    }
    console.log('TypeScript enum codegen is up to date.');
    return;
  }

  for (const outPath of ENUM_OUTPUT_PATHS) {
    await mkdir(dirname(outPath), { recursive: true });
    await writeFile(outPath, enumContent);
    console.log(`Generated: ${outPath}`);
  }

  console.log('TypeScript enum generation complete.');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
