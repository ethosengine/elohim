#!/usr/bin/env node
/**
 * Tests that protocol schemas accept valid data and reject invalid data.
 */
import { readdir, readFile } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(__dirname, '../current');
let failures = 0;
let passes = 0;

function assert(condition, message) {
  if (!condition) {
    console.error(`FAIL: ${message}`);
    failures++;
  } else {
    console.log(`PASS: ${message}`);
    passes++;
  }
}

async function loadJson(filepath) {
  return JSON.parse(await readFile(filepath, 'utf8'));
}

/**
 * Recursively resolve relative-path $ref values to their $id URIs.
 * Schemas use relative file paths (e.g. "../enums/content-type.schema.json")
 * but AJV resolves $ref against the $id namespace. This function maps
 * file-path refs to the $id of the target schema.
 */
function resolveRefs(schema, schemaDir, idMap) {
  if (typeof schema !== 'object' || schema === null) return schema;
  if (Array.isArray(schema)) return schema.map((item) => resolveRefs(item, schemaDir, idMap));

  const result = {};
  for (const [key, value] of Object.entries(schema)) {
    if (key === '$ref' && typeof value === 'string' && !value.startsWith('#')) {
      // Resolve relative file path to absolute, then look up the $id
      const absPath = resolve(schemaDir, value);
      const id = idMap.get(absPath);
      if (id) {
        result[key] = id;
      } else {
        result[key] = value; // Keep original if not found
      }
    } else {
      result[key] = resolveRefs(value, schemaDir, idMap);
    }
  }
  return result;
}

async function main() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });

  // Load all enum schemas and build a map of absolute-path -> $id
  const idMap = new Map();
  const enumDir = join(SCHEMA_DIR, 'enums');
  const enumFiles = (await readdir(enumDir)).filter((f) => f.endsWith('.schema.json'));
  for (const file of enumFiles) {
    const absPath = join(enumDir, file);
    const schema = await loadJson(absPath);
    if (schema.$id) {
      idMap.set(absPath, schema.$id);
    }
    ajv.addSchema(schema);
  }

  // --- Enum Tests ---

  const ctValidate = ajv.getSchema('epr:schema:enum:content-type');
  assert(ctValidate('epic'), 'content-type accepts "epic"');
  assert(ctValidate('scenario'), 'content-type accepts "scenario"');
  assert(!ctValidate('invalid-type'), 'content-type rejects "invalid-type"');
  assert(!ctValidate(''), 'content-type rejects empty string');
  assert(!ctValidate(42), 'content-type rejects number');

  const reachValidate = ajv.getSchema('epr:schema:enum:reach');
  assert(reachValidate('commons'), 'reach accepts "commons"');
  assert(reachValidate('private'), 'reach accepts "private"');
  assert(!reachValidate('public-all'), 'reach rejects "public-all"');
  assert(!reachValidate('invited'), 'reach rejects "invited" (not a DNA value)');

  const masteryValidate = ajv.getSchema('epr:schema:enum:mastery-level');
  assert(masteryValidate('create'), 'mastery-level accepts "create"');
  assert(!masteryValidate('expert'), 'mastery-level rejects "expert"');

  const vsValidate = ajv.getSchema('epr:schema:enum:validation-status');
  assert(vsValidate('valid'), 'validation-status accepts "valid"');
  assert(vsValidate('healing'), 'validation-status accepts "healing"');
  assert(!vsValidate('broken'), 'validation-status rejects "broken"');

  // --- CreateContentInput Tests ---

  const inputSchemaRaw = await loadJson(
    join(SCHEMA_DIR, 'inputs', 'create-content-input.schema.json'),
  );
  const inputSchema = resolveRefs(
    inputSchemaRaw,
    join(SCHEMA_DIR, 'inputs'),
    idMap,
  );
  const inputValidate = ajv.compile(inputSchema);

  // Valid minimal
  assert(
    inputValidate({ id: 'test-1', title: 'Test' }),
    'CreateContentInput accepts minimal valid input',
  );

  // Valid full
  assert(
    inputValidate({
      id: 'test-2',
      title: 'Full Test',
      contentType: 'concept',
      contentFormat: 'markdown',
      reach: 'commons',
      tags: ['test'],
      description: 'A test',
      contentBody: '# Hello',
      metadata: { key: 'val' },
    }),
    'CreateContentInput accepts full valid input',
  );

  // Missing required: id
  assert(
    !inputValidate({ title: 'Test' }),
    'CreateContentInput rejects missing id',
  );

  // Missing required: title
  assert(
    !inputValidate({ id: 'test' }),
    'CreateContentInput rejects missing title',
  );

  // Invalid content type
  assert(
    !inputValidate({ id: 'test', title: 'Test', contentType: 'bible-verse' }),
    'CreateContentInput rejects non-DNA content type "bible-verse"',
  );

  // Invalid reach
  assert(
    !inputValidate({ id: 'test', title: 'Test', reach: 'invited' }),
    'CreateContentInput rejects invalid reach "invited"',
  );

  // Additional properties rejected
  assert(
    !inputValidate({ id: 'test', title: 'Test', unknownField: true }),
    'CreateContentInput rejects unknown properties',
  );

  // --- ContentView Tests ---

  const viewSchemaRaw = await loadJson(
    join(SCHEMA_DIR, 'views', 'content-view.schema.json'),
  );
  const viewSchema = resolveRefs(
    viewSchemaRaw,
    join(SCHEMA_DIR, 'views'),
    idMap,
  );
  const viewValidate = ajv.compile(viewSchema);

  // Valid minimal
  assert(
    viewValidate({
      id: 'test',
      appId: 'app1',
      title: 'Test',
      contentType: 'concept',
      contentFormat: 'markdown',
      reach: 'commons',
      validationStatus: 'valid',
      createdAt: '2026-03-17T00:00:00Z',
      updatedAt: '2026-03-17T00:00:00Z',
    }),
    'ContentView accepts valid record',
  );

  // Missing required field
  assert(
    !viewValidate({
      id: 'test',
      title: 'Test',
    }),
    'ContentView rejects missing required fields',
  );

  // Nullable fields accept null
  assert(
    viewValidate({
      id: 'test',
      appId: 'app1',
      title: 'Test',
      contentType: 'concept',
      contentFormat: 'markdown',
      reach: 'commons',
      validationStatus: 'valid',
      createdAt: '2026-03-17',
      updatedAt: '2026-03-17',
      description: null,
      blobHash: null,
      dhtAnchorHash: null,
    }),
    'ContentView accepts null for nullable fields',
  );

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
