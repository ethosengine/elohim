import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, mkdirSync, rmSync, existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runSync, validateMappings, MAPPINGS } from './sync-genesis.mjs';

function setupFixtureRepo() {
  const root = mkdtempSync(join(tmpdir(), 'graphos-sync-'));
  const genesis = join(root, 'genesis');
  const out = join(root, 'graphos-imported');
  mkdirSync(genesis, { recursive: true });
  mkdirSync(out, { recursive: true });
  return { root, genesis, out };
}

test('validateMappings returns missing entries when sources do not exist', () => {
  const { root, genesis } = setupFixtureRepo();
  const mappings = [
    { from: 'docs/missing.md', to: 'narrative/why/manifesto.md', title: 'I. Why / Manifesto' },
  ];
  const missing = validateMappings(mappings, genesis);
  assert.equal(missing.length, 1);
  assert.match(missing[0].error, /missing/i);
  rmSync(root, { recursive: true, force: true });
});

test('validateMappings passes when source exists', () => {
  const { root, genesis } = setupFixtureRepo();
  mkdirSync(join(genesis, 'docs/content'), { recursive: true });
  writeFileSync(join(genesis, 'docs/content/manifesto.md'), '# Manifesto\n');
  const mappings = [
    { from: 'docs/content/manifesto.md', to: 'narrative/why/manifesto.md', title: 'I. Why / Manifesto' },
  ];
  const missing = validateMappings(mappings, genesis);
  assert.equal(missing.length, 0);
  rmSync(root, { recursive: true, force: true });
});

test('runSync copies single-file mapping to imported/', () => {
  const { root, genesis, out } = setupFixtureRepo();
  mkdirSync(join(genesis, 'docs/content'), { recursive: true });
  writeFileSync(join(genesis, 'docs/content/manifesto.md'), '# Hello\n');
  const mappings = [
    { from: 'docs/content/manifesto.md', to: 'narrative/why/manifesto.md', title: 'I. Why / Manifesto' },
  ];
  runSync(mappings, genesis, out);
  const expected = join(out, 'narrative/why/manifesto.md');
  assert.equal(existsSync(expected), true);
  assert.equal(readFileSync(expected, 'utf-8'), '# Hello\n');
  rmSync(root, { recursive: true, force: true });
});

test('MAPPINGS constant includes the manifesto entry', () => {
  const entry = MAPPINGS.find(m => m.from === 'docs/content/elohim-protocol/manifesto.md');
  assert.ok(entry, 'manifesto mapping must exist');
  assert.equal(entry.title, 'I. Why / Manifesto');
});
