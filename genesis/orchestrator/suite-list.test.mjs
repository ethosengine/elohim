// The orchestrator suite is suite-reads.json (E11): these hold the list to the tree it describes.
import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, posix, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isReadPattern } from './gate-input-audit.mjs';
import { loadSuite, plan } from './suite-run.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../..');
const DIR = 'genesis/orchestrator';
const suite = loadSuite();
const entries = suite.groups.flatMap(group => group.files.map(entry => ({ ...entry, runner: group.runner })));

test('every test file in genesis/orchestrator is in the suite or named as outside it, never both', () => {
  const listed = new Set(entries.map(entry => entry.file));
  assert.equal(listed.size, entries.length, 'no file is listed twice');
  const outside = new Set(Object.keys(suite.notInSuite || {}));
  for (const file of outside) assert.ok(!listed.has(file), `${file}: in the suite and outside it`);
  const tests = readdirSync(HERE).filter(name => /\.test\.(mjs|ts)$/.test(name));
  const missing = tests.filter(name => !listed.has(name) && !outside.has(name));
  assert.deepEqual(missing, [], 'add each to a group of suite-reads.json, or to notInSuite with the reason no gate runs it');
  for (const name of [...listed].filter(file => !file.includes('/'))) assert.ok(existsSync(join(HERE, name)), `${name}: listed but absent`);
  for (const name of outside) assert.ok(existsSync(join(HERE, name)), `${name}: notInSuite names an absent file`);
});

test('each entry is named (reads) or tree-keyed (a reason), not both; runners are known', () => {
  for (const entry of entries) {
    assert.ok(['node', 'node-tsx', 'bash'].includes(entry.runner), `${entry.file}: runner ${entry.runner}`);
    const named = Array.isArray(entry.reads);
    const tree = typeof entry.tree === 'string' && entry.tree.length > 0;
    assert.ok(named !== tree, `${entry.file}: exactly one of reads or tree`);
    if (entry.runner === 'bash') assert.ok(tree, `${entry.file}: a shell suite is unseen by the fence`);
  }
});

test('a declared read is a repository path, a prefix or a basename walk, and never a path inside a submodule', () => {
  const submodules = [...readFileSync(join(ROOT, '.gitmodules'), 'utf8').matchAll(/^\s*path\s*=\s*(\S+)\s*$/gm)].map(m => m[1]);
  assert.ok(submodules.length > 0);
  for (const entry of entries.filter(e => e.reads)) {
    for (const read of entry.reads) {
      assert.ok(isReadPattern(read) || !/[*?[\]]/.test(read), `${entry.file}: ${read} is not a read the key can hash`);
      assert.ok(!read.startsWith('/') && posix.normalize(read) === read && !read.startsWith('..'), `${entry.file}: ${read}`);
      // Inside a gitlink no blob exists at the commit: the key would hash "absent" whatever the module holds.
      const inside = submodules.find(module => read.startsWith(`${module}/`));
      assert.equal(inside, undefined, `${entry.file}: ${read} is inside submodule ${inside}; declare ${inside} itself`);
    }
  }
});

test('the two sides run every listed file exactly once, groups in order', () => {
  const ran = side => plan(suite, side).flatMap(step => step.args.filter(arg => /\.test\.(mjs|ts|sh)$/.test(arg)));
  const named = ran('named');
  const tree = ran('tree');
  assert.deepEqual([...named, ...tree].sort(), entries.map(entry => entry.file).sort());
  assert.deepEqual(named, entries.filter(entry => entry.reads).map(entry => entry.file));
  for (const step of plan(suite, 'named')) {
    if (step.runner !== 'bash') assert.ok(step.args.includes('--test-isolation=process'), 'the fence attributes reads per test process');
  }
});

test('package.json, the justfile and the manifest run the suite through the list', () => {
  const pkg = JSON.parse(readFileSync(join(HERE, 'package.json'), 'utf8'));
  assert.equal(pkg.scripts.test, 'node suite-run.mjs all');
  const justfile = readFileSync(join(HERE, 'justfile'), 'utf8');
  assert.match(justfile, /^gate: test-suite$/m);
  assert.match(justfile, /^gate-tree: lint test-suite-tree test-harvest test-berth$/m);
  assert.match(justfile, /^test-suite: \(_suite "named"\)$/m);
  assert.match(justfile, /^test-suite-tree: \(_suite "tree"\)$/m);
  const projects = JSON.parse(readFileSync(join(HERE, 'build-manifest.json'), 'utf8')).gate.projects;
  const { orchestrator, 'orchestrator-tree': tree } = projects;
  assert.equal(orchestrator.run.recipe, 'gate');
  assert.equal(tree.run.recipe, 'gate-tree');
  // Same dir, steps, inputs and cargo slot: the two halves are always selected together.
  for (const key of ['dir', 'steps', 'inputs']) assert.deepEqual(tree[key], orchestrator[key], key);
  assert.deepEqual(tree.run.cargo, orchestrator.run.cargo);
});
