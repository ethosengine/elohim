#!/usr/bin/env node
/**
 * Generates TypeScript interfaces from protocol schemas.
 * Phase 1: verification mode — outputs to schemas/generated-ts/ for comparison.
 * Phase 2: will replace ts-rs output in storage-client-ts/src/generated/.
 */
import { readdir, readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, resolve, basename, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { compile } from 'json-schema-to-typescript';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(__dirname, '../current');
const OUTPUT_DIR = resolve(__dirname, '../generated-ts');

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

async function main() {
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

  console.log(`\nTypeScript generation complete: ${allGenerated.length} files`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
