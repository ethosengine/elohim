import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, mkdirSync, rmSync, existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runSync, validateMappings, MAPPINGS, gherkinToMarkdown, expandGlob, runSyncWithGlobs } from './sync-genesis.mjs';

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

test('gherkinToMarkdown wraps feature content in a code fence with gherkin language', () => {
  const featureContent = `Feature: Auth\n  Scenario: Login\n    When user logs in\n    Then they are authenticated\n`;
  const md = gherkinToMarkdown(featureContent, 'auth.feature');
  assert.match(md, /```gherkin/);
  assert.match(md, /```\n*$/);
  assert.match(md, /Feature: Auth/);
});

test('expandGlob finds .feature files matching the pattern', () => {
  const { root, genesis } = setupFixtureRepo();
  mkdirSync(join(genesis, 'a2o/features/auth'), { recursive: true });
  writeFileSync(join(genesis, 'a2o/features/auth/login.feature'), 'Feature: Login\n');
  writeFileSync(join(genesis, 'a2o/features/auth/recovery.feature'), 'Feature: Recovery\n');
  writeFileSync(join(genesis, 'a2o/features/auth/notes.txt'), 'ignored\n');
  const matches = expandGlob('a2o/features/auth/*.feature', genesis);
  assert.equal(matches.length, 2);
  assert.ok(matches.some(p => p.endsWith('login.feature')));
  assert.ok(matches.some(p => p.endsWith('recovery.feature')));
  rmSync(root, { recursive: true, force: true });
});

test('runSyncWithGlobs writes both an imported .md AND a generated .mdx wrapper per match', () => {
  const { root, genesis, out } = setupFixtureRepo();
  mkdirSync(join(genesis, 'a2o/features/auth'), { recursive: true });
  writeFileSync(join(genesis, 'a2o/features/auth/login.feature'), 'Feature: Login\n  Scenario: x\n');
  const wrappersDir = join(out, '..', 'graphos-wrappers');
  mkdirSync(wrappersDir, { recursive: true });
  const mappings = [
    { fromGlob: 'a2o/features/auth/*.feature',
      toDir: 'domains/identity/stories/',
      titleFn: (name) => `III. Domains / Identity (Imagodei) / Stories / ${name}` },
  ];
  runSyncWithGlobs(mappings, genesis, out, wrappersDir);
  const importedMd = join(out, 'domains/identity/stories/login.md');
  const generatedMdx = join(wrappersDir, 'domains/identity/__docs__/_generated/login.mdx');
  assert.equal(existsSync(importedMd), true, 'imported markdown should exist');
  assert.equal(existsSync(generatedMdx), true, 'generated MDX wrapper should exist');
  const mdxContent = readFileSync(generatedMdx, 'utf-8');
  assert.match(mdxContent, /III\. Domains \/ Identity \(Imagodei\) \/ Stories \/ Login/);
  assert.match(mdxContent, /<Markdown>\{content\}<\/Markdown>/);
  rmSync(root, { recursive: true, force: true });
});

test('runSyncWithGlobs uses wrapperDir override for reference subcategory mappings', () => {
  const { root, genesis, out } = setupFixtureRepo();
  mkdirSync(join(genesis, 'a2o/features/federation'), { recursive: true });
  writeFileSync(join(genesis, 'a2o/features/federation/peer-discovery.feature'), 'Feature: Peer Discovery\n  Scenario: x\n');
  const wrappersDir = join(out, '..', 'graphos-wrappers');
  mkdirSync(wrappersDir, { recursive: true });
  const mappings = [
    { fromGlob: 'a2o/features/federation/*.feature',
      toDir: 'reference/federation/',
      wrapperDir: 'reference/federation/__docs__/_generated/',
      titleFn: (name) => `IV. Reference / Federation / ${name}` },
  ];
  runSyncWithGlobs(mappings, genesis, out, wrappersDir);
  // Wrapper must land in per-category dir, NOT the collapsed reference/__docs__/_generated/
  const correctMdx = join(wrappersDir, 'reference/federation/__docs__/_generated/peer-discovery.mdx');
  const collapsedMdx = join(wrappersDir, 'reference/__docs__/_generated/peer-discovery.mdx');
  assert.equal(existsSync(correctMdx), true, 'wrapper should exist in per-category wrapperDir');
  assert.equal(existsSync(collapsedMdx), false, 'wrapper must NOT exist in collapsed reference/__docs__/_generated/');
  rmSync(root, { recursive: true, force: true });
});

test('gherkinToMarkdown falls back to fileName when no Feature: line', () => {
  const md = gherkinToMarkdown('# some comment\nScenario: x\n', 'orphan.feature');
  assert.match(md, /# orphan\.feature/);
});
