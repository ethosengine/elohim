#!/usr/bin/env node
/**
 * Validates genesis seed JSON files against the protocol schema.
 * Exit code 0 = all valid, 1 = validation errors found.
 */
import { readdir, readFile } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(__dirname, '../current');
const SEED_DIR = resolve(__dirname, '../../../../genesis/data/lamad/content');

async function loadJson(filepath) {
  return JSON.parse(await readFile(filepath, 'utf8'));
}

async function loadEnumSchemas(enumDir) {
  const files = (await readdir(enumDir)).filter((f) => f.endsWith('.schema.json'));
  const schemas = [];
  for (const file of files) {
    schemas.push({ schema: await loadJson(join(enumDir, file)), absPath: join(enumDir, file) });
  }
  return schemas;
}

/**
 * Recursively resolve relative-path $ref values to their $id URIs.
 * Schemas use relative file paths (e.g. "../enums/content-type.schema.json")
 * but AJV resolves $ref against the $id namespace.
 */
function resolveRefs(schema, schemaDir, idMap) {
  if (typeof schema !== 'object' || schema === null) return schema;
  if (Array.isArray(schema)) return schema.map((item) => resolveRefs(item, schemaDir, idMap));

  const result = {};
  for (const [key, value] of Object.entries(schema)) {
    if (key === '$ref' && typeof value === 'string' && !value.startsWith('#')) {
      const absPath = resolve(schemaDir, value);
      const id = idMap.get(absPath);
      if (id) {
        result[key] = id;
      } else {
        result[key] = value;
      }
    } else {
      result[key] = resolveRefs(value, schemaDir, idMap);
    }
  }
  return result;
}

async function main() {
  const enumDir = join(SCHEMA_DIR, 'enums');
  const enumEntries = await loadEnumSchemas(enumDir);
  const inputSchemaRaw = await loadJson(
    join(SCHEMA_DIR, 'inputs', 'create-content-input.schema.json'),
  );

  const ajv = new Ajv2020({ allErrors: true, strict: false });

  // Build a map of absolute-path -> $id for $ref resolution
  const idMap = new Map();
  for (const { schema, absPath } of enumEntries) {
    if (schema.$id) {
      idMap.set(absPath, schema.$id);
    }
    ajv.addSchema(schema);
  }

  // Resolve relative-path $ref to $id URIs before compiling
  const inputSchema = resolveRefs(inputSchemaRaw, join(SCHEMA_DIR, 'inputs'), idMap);
  const validate = ajv.compile(inputSchema);

  let files;
  try {
    files = (await readdir(SEED_DIR)).filter((f) => f.endsWith('.json'));
  } catch {
    console.error(`Seed directory not found: ${SEED_DIR}`);
    process.exit(1);
  }

  let errors = 0;
  let valid = 0;
  const errorSummary = new Map(); // Track error types for summary

  for (const file of files) {
    const raw = await readFile(join(SEED_DIR, file), 'utf8');
    let data;
    try {
      data = JSON.parse(raw);
    } catch {
      console.error(`PARSE ERROR: ${file}`);
      errors++;
      continue;
    }

    if (!validate(data)) {
      // Filter to meaningful errors (enum mismatches, type errors, missing required)
      const meaningful = validate.errors.filter(
        (e) => e.keyword === 'enum' || e.keyword === 'type' || e.keyword === 'required',
      );
      if (meaningful.length > 0) {
        // Only print first 50 errors to avoid flooding
        if (errors < 50) {
          console.error(`INVALID: ${file}`);
          for (const err of meaningful) {
            const key = `${err.instancePath}:${err.keyword}`;
            errorSummary.set(key, (errorSummary.get(key) || 0) + 1);
            console.error(`  ${err.instancePath} ${err.message}`);
          }
        }
        errors++;
      } else {
        valid++;
      }
    } else {
      valid++;
    }
  }

  console.log(`\nSchema validation: ${valid} valid, ${errors} errors, ${files.length} total`);

  if (errorSummary.size > 0 && errors >= 50) {
    console.log(`\n(Showing first 50 errors. ${errors} total files with errors.)`);
  }

  process.exit(errors > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
