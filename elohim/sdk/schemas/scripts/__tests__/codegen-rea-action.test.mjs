import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// Node 3 of the reliance-as-cite spec: one REA action vocabulary, generated from
// v1/enums/rea-action.schema.json into TypeScript and into the epr crate, and NEVER into
// an integrity zome (a new const in content_store_integrity/src/generated_enums.rs is a
// DNA-hash move). The DNA's hand-written REA_ACTIONS is checked, not written: check-dna.mjs.

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../../');
const SCHEMA = resolve(REPO_ROOT, 'elohim/sdk/schemas/v1/enums/rea-action.schema.json');
const TS_ENUMS = resolve(REPO_ROOT, 'genesis/seeder/src/generated/schema-enums.ts');
const EPR_ENUMS = resolve(REPO_ROOT, 'elohim/epr/src/generated/enums.rs');
const DNA_GENERATED = resolve(
  REPO_ROOT,
  'elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_enums.rs',
);

const schema = JSON.parse(await readFile(SCHEMA, 'utf8'));

test('rea-action is routed by _codegen, never by _dna', () => {
  assert.equal(schema._dna, undefined, 'a _dna key would generate into the integrity zome');
  assert.equal(schema._codegen.constant, 'REA_ACTIONS');
  assert.deepEqual(schema._codegen.rust, ['epr']);
  assert.equal(schema._dnaMirror.constant, 'REA_ACTIONS');
});

test('the vocabulary carries what the grocer and the garden need, and every ReaVerb', () => {
  for (const word of ['cite', 'use', 'consume', 'produce', 'transfer', 'transfer-custody', 'modify', 'pickup', 'dropoff']) {
    assert.ok(schema.enum.includes(word), `${word} missing from rea-action`);
  }
  for (const verb of ['use', 'consume', 'produce', 'cite', 'affirm', 'dismiss']) {
    assert.ok(schema.enum.includes(verb), `ReaVerb ${verb} missing from rea-action`);
  }
});

test('generated TypeScript carries CORE_/ALL_REA_ACTIONS and the ReaAction type', async () => {
  const ts = await readFile(TS_ENUMS, 'utf8');
  assert.match(ts, /export const CORE_REA_ACTIONS = \[/);
  assert.match(ts, /export const ALL_REA_ACTIONS = \[/);
  assert.match(ts, /export type ReaAction = \(typeof ALL_REA_ACTIONS\)\[number\];/);
});

test('generated Rust lands in epr and not in the integrity zome', async () => {
  const epr = await readFile(EPR_ENUMS, 'utf8');
  assert.match(epr, /pub const ALL_REA_ACTIONS: &\[&str\] = &\[/);
  assert.match(epr, /pub const CORE_REA_ACTIONS: &\[&str\] = &\[/);
  const dna = await readFile(DNA_GENERATED, 'utf8');
  assert.ok(!dna.includes('REA_ACTIONS'), 'rea-action leaked into the integrity zome generated file');
});
