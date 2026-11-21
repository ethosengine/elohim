// The orchestrator suite is suite-reads.json (E11): these hold the list to the tree it describes,
// and the view (suite-run.mjs) to the key (gate-input-audit.mjs suiteReadSet).
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, readlinkSync, realpathSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, posix, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isReadPattern } from './gate-input-audit.mjs';
import { dirtyInView, loadSuite, missingTools, namedEnv, plan, skipFindings, treeEnv, unexpectedSkips, viewMatcher } from './suite-run.mjs';
import { checkLockVersions, closureDigest, copyClosure, decodeExpectation, encodeExpectation, linksOutside, materialise, packageClosure, readResult, suiteDecision, suiteInventory, writeBlobs, writeResult } from './suite-inventory.mjs';
import { gateChildEnv } from './gate-runner.mjs';
import { createHash } from 'node:crypto';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../..');
const LIST = 'genesis/orchestrator/suite-reads.json';
const text = readFileSync(join(HERE, 'suite-reads.json'), 'utf8');
const suite = loadSuite(text);
const entries = suite.groups.flatMap(group => group.files.map(entry => ({ ...entry, runner: group.runner })));

test('every test file in genesis/orchestrator is in the suite or named as outside it, never both', () => {
  const listed = new Set(entries.map(entry => entry.file));
  assert.equal(listed.size, entries.length, 'no file is listed twice');
  const outside = new Set(Object.keys(suite.notInSuite || {}));
  for (const file of outside) assert.ok(!listed.has(file), `${file}: in the suite and outside it`);
  const tests = readdirSync(HERE).filter(name => /\.test\.(mjs|ts)$/.test(name));
  const missing = tests.filter(name => !listed.has(name) && !outside.has(name));
  assert.deepEqual(missing, [], 'add each to a group of suite-reads.json, or to notInSuite with the reason no gate runs it');
  // A file outside this directory is in the view only when named; suite-run.mjs refuses an absent
  // planned file before it runs either half.
  for (const entry of entries.filter(e => e.reads || !e.file.includes('/'))) assert.ok(existsSync(resolve(HERE, entry.file)), `${entry.file}: listed but absent`);
  for (const name of outside) assert.ok(existsSync(join(HERE, name)), `${name}: notInSuite names an absent file`);
});

test('each entry is named (reads) or tree-keyed (a reason), not both; runners are known', () => {
  for (const entry of entries) {
    assert.ok(['node', 'node-tsx', 'bash'].includes(entry.runner), `${entry.file}: runner ${entry.runner}`);
    const named = Array.isArray(entry.reads);
    const tree = typeof entry.tree === 'string' && entry.tree.length > 0;
    assert.ok(named !== tree, `${entry.file}: exactly one of reads or tree`);
  }
  for (const [dir, names] of Object.entries(suite.modules)) {
    assert.ok(Array.isArray(names) && names.every(name => typeof name === 'string'), `modules.${dir} is a list of package names`);
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
});

test('the view holds what the key hashes and nothing else', () => {
  const view = viewMatcher(text, LIST);
  // The project tree, minus its habit atoms, which the key's dirTree drops.
  assert.ok(view.has('genesis/orchestrator/gate-runner.mjs'));
  assert.ok(!view.has('genesis/orchestrator/.epr-meta/push-gate-pays-once.habit.md'));
  // A named test outside the directory and one of its reads.
  assert.ok(view.has('genesis/agentic/compute/compute.test.mjs'));
  assert.ok(view.has('.husky/pre-push.bash'));
  // A basename walk, at any depth, and inside a declared submodule.
  assert.ok(view.has('app/elohim-app/build-manifest.json'));
  assert.ok(view.inside('build-manifest.json'));
  // A tree-side file and a doc are not in it.
  assert.ok(!view.has('.husky/hooks-path.test.mjs'));
  assert.ok(!view.has('genesis/docs/README.md'));
  assert.ok(!view.has('.claude/data/ci-findings.jsonl'));
  for (const read of view.reads) assert.ok(isReadPattern(read) || view.has(read), read);
});

test('a declared path renamed, edited or added in the working tree is dirt; an undeclared one is not', t => {
  const root = mkdtempSync(join(tmpdir(), 'suite-dirt-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const git = (...args) => spawnSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@example.test', ...args], { cwd: root, encoding: 'utf8' });
  assert.equal(git('init', '-q').status, 0);
  for (const name of ['a.json', 'b.json', 'doc.md']) writeFileSync(join(root, name), '{}\n');
  git('add', '.');
  assert.equal(git('commit', '-q', '-m', 'base').status, 0);
  const matcher = { has: path => path === 'a.json' || path === 'b.json' || path === 'new.json' };
  assert.deepEqual(dirtyInView(matcher, { cwd: root }), []);
  writeFileSync(join(root, 'doc.md'), 'edited\n');
  assert.deepEqual(dirtyInView(matcher, { cwd: root }), [], 'an undeclared edit is not dirt');
  assert.equal(git('mv', 'a.json', 'moved.json').status, 0);
  writeFileSync(join(root, 'b.json'), '{"v":2}\n');
  writeFileSync(join(root, 'new.json'), '{}\n');
  assert.deepEqual(dirtyInView(matcher, { cwd: root }).sort(), ['a.json', 'b.json', 'new.json']);
});

test('the named environment is constructed: nothing ambient reaches a test', () => {
  const env = namedEnv('/s', { execPath: '/opt/node/bin/node' });
  assert.deepEqual(Object.keys(env).sort(), ['GIT_CEILING_DIRECTORIES', 'GIT_CONFIG_NOSYSTEM', 'HOME', 'LANG', 'PATH', 'TMPDIR', 'TZ']);
  assert.equal(env.TZ, 'UTC');
  assert.equal(env.PATH, '/opt/node/bin:/usr/local/bin:/usr/bin:/bin');
  assert.equal(env.GIT_CEILING_DIRECTORIES, '/s');
  assert.ok(env.HOME.startsWith('/s/') && env.TMPDIR.startsWith('/s/'));
});

test('the package closure is the named packages and their links, copied into the view; an unnamed package is absent', t => {
  const root = mkdtempSync(join(tmpdir(), 'suite-modules-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const src = join(root, 'repo');
  const dest = join(root, 'view');
  const pkg = (dir, name, version, deps = {}) => {
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, 'package.json'), JSON.stringify({ name, version, dependencies: deps }));
    writeFileSync(join(dir, 'index.js'), 'module.exports = 1;\n');
  };
  const store = join(src, 'node_modules/.pnpm');
  pkg(join(store, 'a@1.0.0/node_modules/a'), 'a', '1.0.0', { b: '1' });
  pkg(join(store, 'b@1.0.0/node_modules/b'), 'b', '1.0.0');
  pkg(join(store, 'unused@1.0.0/node_modules/unused'), 'unused', '1.0.0');
  symlinkSync('../../b@1.0.0/node_modules/b', join(store, 'a@1.0.0/node_modules/b'));
  symlinkSync('.pnpm/a@1.0.0/node_modules/a', join(src, 'node_modules/a'));
  symlinkSync('.pnpm/unused@1.0.0/node_modules/unused', join(src, 'node_modules/unused'));
  pkg(join(src, 'libs/w'), '@x/w', '0.0.0');
  mkdirSync(join(src, 'node_modules/@x'));
  symlinkSync('../../libs/w', join(src, 'node_modules/@x/w'));
  const closure = packageClosure(src, { '.': ['a', '@x/w'] });
  assert.deepEqual(closure.packages, ['node_modules/.pnpm/a@1.0.0/node_modules/a', 'node_modules/.pnpm/b@1.0.0/node_modules/b']);
  const lock = 'packages:\n\n  a@1.0.0:\n    x\n\n  b@1.0.0:\n    x\n';
  checkLockVersions(closure, lock, { root: src });
  copyClosure(closure, dest, { root: src });
  const a = realpathSync(join(dest, 'node_modules/a'));
  assert.ok(a.startsWith(realpathSync(dest)), a);
  assert.ok(realpathSync(join(a, '../b')).startsWith(realpathSync(dest)), 'a dependency is copied too');
  assert.ok(!existsSync(join(dest, 'node_modules/unused')), 'an unnamed package is absent');
  assert.equal(readlinkSync(join(dest, 'node_modules/@x/w')), '../../libs/w', 'a workspace link points into the view');
  assert.ok(!existsSync(join(dest, 'node_modules/@x/w/package.json')), 'and resolves only when the source is in the view');
  assert.equal(closureDigest(closure, dest), closureDigest(closure, src), 'the copy hashes as the install does');
  writeFileSync(join(store, 'b@1.0.0/node_modules/b/index.js'), 'module.exports = 2;\n');
  assert.notEqual(closureDigest(closure, src), closureDigest(closure, dest), 'an edited package file moves the digest');
  assert.throws(() => checkLockVersions(closure, 'packages:\n\n  a@1.0.0:\n', { root: src }), /b@1\.0\.0 .* is not what pnpm-lock\.yaml records/);
  assert.throws(() => packageClosure(src, { '.': ['missing'] }), /not installed in \./);
});

test('a link that resolves outside the view is named; links inside it are not', t => {
  const root = mkdtempSync(join(tmpdir(), 'suite-links-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'view/a'), { recursive: true });
  writeFileSync(join(root, 'view/a/file'), 'x\n');
  writeFileSync(join(root, 'secret'), 'x\n');
  symlinkSync('file', join(root, 'view/a/in'));
  symlinkSync('../missing-but-inside', join(root, 'view/a/dangling-in'));
  assert.deepEqual(linksOutside(join(root, 'view')), []);
  symlinkSync('../../secret', join(root, 'view/a/out'));
  symlinkSync('/etc', join(root, 'view/abs'));
  assert.deepEqual(linksOutside(join(root, 'view')).sort(), ['a/out -> ../../secret', 'abs -> /etc']);
});

const blobOid = bytes => createHash('sha1').update(Buffer.concat([Buffer.from(`blob ${bytes.length}\0`), bytes])).digest('hex');

test('the inventory is what the view holds, entry by entry: path, mode, object id and pinned submodule files', t => {
  const root = mkdtempSync(join(tmpdir(), 'suite-inventory-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const git = (cwd, ...args) => {
    const r = spawnSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@example.test', ...args], { cwd, encoding: 'utf8' });
    assert.equal(r.status, 0, `${args.join(' ')}: ${r.stderr}`);
    return r.stdout.trim();
  };
  const sub = join(root, 'sub');
  mkdirSync(join(sub, 'x'), { recursive: true });
  git(sub, 'init', '-q');
  writeFileSync(join(sub, 'x/build-manifest.json'), '{"s":1}\n');
  writeFileSync(join(sub, 'other.txt'), 'not declared\n');
  git(sub, 'add', '.');
  git(sub, 'commit', '-q', '-m', 'sub');
  const pin = git(sub, 'rev-parse', 'HEAD');
  const top = join(root, 'top');
  const dir = join(top, 'genesis/orchestrator');
  mkdirSync(join(dir, '.epr-meta'), { recursive: true });
  mkdirSync(join(top, 'docs'));
  git(top, 'init', '-q');
  writeFileSync(join(dir, 'suite-reads.json'), JSON.stringify({ groups: [{ runner: 'node', files: [{ file: 'a.test.mjs', reads: ['sub', '**/build-manifest.json'] }] }] }));
  writeFileSync(join(dir, 'a.test.mjs'), '// a\n');
  writeFileSync(join(dir, 'run.sh'), '#!/bin/sh\n', { mode: 0o755 });
  symlinkSync('a.test.mjs', join(dir, 'link'));
  writeFileSync(join(dir, '.epr-meta/h.habit.md'), 'DELTA\n');
  writeFileSync(join(top, 'docs/x.md'), '# not declared\n');
  git(top, 'add', '.');
  git(top, 'update-index', '--add', '--cacheinfo', `160000,${pin},sub`);
  git(top, 'commit', '-q', '-m', 'top');
  git(root, 'clone', '-q', '--bare', sub, join(top, '.git/modules/sub'));
  const listText = readFileSync(join(dir, 'suite-reads.json'), 'utf8');
  const inventory = suiteInventory({ root: top, commit: 'HEAD', listText });
  assert.deepEqual(inventory.entries.map(e => [e.path, e.mode]), [
    ['genesis/orchestrator/a.test.mjs', '100644'], ['genesis/orchestrator/link', '120000'],
    ['genesis/orchestrator/run.sh', '100755'], ['genesis/orchestrator/suite-reads.json', '100644']]);
  assert.deepEqual(inventory.pins.map(p => [p.path, p.oid, p.entries.map(e => e.path)]), [['sub', pin, ['x/build-manifest.json']]]);
  const view = join(root, 'view');
  assert.equal(materialise(inventory, view, { root: top }), 5);
  const seen = [];
  const walk = (d, prefix = '') => {
    for (const entry of readdirSync(d, { withFileTypes: true })) {
      const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) walk(join(d, entry.name), rel); else seen.push(rel);
    }
  };
  walk(view);
  const records = [...inventory.entries, ...inventory.pins.flatMap(p => p.entries.map(e => ({ ...e, path: `${p.path}/${e.path}` })))];
  assert.deepEqual(seen.sort(), records.map(r => r.path).sort(), 'the view holds every record and nothing else');
  for (const record of records) {
    const file = join(view, record.path);
    const stat = lstatSync(file);
    const bytes = stat.isSymbolicLink() ? Buffer.from(readlinkSync(file)) : readFileSync(file);
    assert.equal(blobOid(bytes), record.oid, record.path);
    const mode = stat.isSymbolicLink() ? '120000' : (stat.mode & 0o111) ? '100755' : '100644';
    assert.equal(mode, record.mode, record.path);
  }
  // A mode-only change moves the digest; an undeclared doc and a habit atom do not.
  writeFileSync(join(top, 'docs/x.md'), '# edited\n');
  writeFileSync(join(dir, '.epr-meta/h.habit.md'), 'DELTA 2\n');
  git(top, 'add', 'docs/x.md', 'genesis/orchestrator/.epr-meta/h.habit.md');
  git(top, 'commit', '-q', '-m', 'doc and habit');
  assert.equal(suiteInventory({ root: top, commit: 'HEAD', listText }).digest, inventory.digest);
  chmodSync(join(dir, 'a.test.mjs'), 0o755);
  git(top, 'add', 'genesis/orchestrator/a.test.mjs');
  git(top, 'commit', '-q', '-m', 'mode');
  assert.notEqual(suiteInventory({ root: top, commit: 'HEAD', listText }).digest, inventory.digest);
  // A missing object is an error naming it, never an empty file.
  const ghost = { path: 'ghost', mode: '100644', type: 'blob', oid: 'f'.repeat(40) };
  assert.throws(() => writeBlobs(top, [ghost], join(root, 'ghost-view')), /object f{40} \(ghost\) is missing/);
});

test('the parent decides from the written result, for every row of the rule', t => {
  const expected = { commit: 'c1', inventory: 'i1', packages: 'p1', node: 'v24' };
  const result = (verdict, extra = {}) => ({ v: 1, ...expected, bytes: 'key', why: [], full: true, groups: [], findings: [], verdict, ...extra });
  const base = { exit: 0, signal: null, expected, gateSha: 'c1', headAfter: 'c1', dirtyAfter: [] };
  const decide = over => suiteDecision({ ...base, ...over });
  // No result, an unparsable one, or a verdict the process status contradicts: red, nothing recorded.
  for (const over of [{ result: null }, { result: result('pass'), exit: 1 }, { result: result('fail'), exit: 0 },
    { result: result('pass'), exit: null, signal: 'SIGKILL' }, { result: { ...result('pass'), verdict: 'maybe' } }]) {
    const d = decide(over);
    assert.equal(d.record, false, JSON.stringify(over));
    assert.notEqual(d.status, 0, JSON.stringify(over));
    assert.match(d.why, /^the named half did not report/);
  }
  const dir = mkdtempSync(join(tmpdir(), 'suite-result-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  writeFileSync(join(dir, 'result.json'), '{ not json');
  assert.equal(readResult(dir), null, 'an unparsable result reads as none');
  writeResult(dir, result('pass'));
  assert.equal(readResult(dir).verdict, 'pass');
  // A pass on the key's bytes, full, digests equal, HEAD and paths unchanged: green recorded.
  assert.deepEqual(decide({ result: result('pass') }), { status: 0, record: true });
  // A pass otherwise: the gate passes, nothing recorded, one reason.
  for (const [over, why] of [
    [{ expected: null }, /no expectation/],
    [{ result: result('pass', { bytes: 'other', why: ['the view holds working-tree bytes'] }) }, /tested other bytes \(the view holds working-tree bytes\)/],
    [{ result: result('pass', { full: false }) }, /not the full named half/],
    [{ result: result('pass', { packages: 'p2' }) }, /its packages p2 is not the key's p1/],
    [{ headAfter: 'c2' }, /HEAD moved to c2/],
    [{ dirtyAfter: ['genesis/orchestrator/x.mjs'] }, /declared paths changed during the run/],
    [{ dirtyAfter: null }, /could not be read after the run/]]) {
    const d = decide({ result: result('pass'), ...over });
    assert.equal(d.status, 0, JSON.stringify(over));
    assert.equal(d.record, false);
    assert.match(d.why, why);
  }
  // A fail on the key's bytes: red recorded; otherwise red, nothing recorded.
  assert.deepEqual(decide({ result: result('fail'), exit: 1 }), { status: 1, record: true });
  const other = decide({ result: result('fail', { packages: 'p2' }), exit: 1 });
  assert.equal(other.status, 1);
  assert.equal(other.record, false);
  assert.match(other.why, /a red, recorded nowhere: its packages/);
});

test('a gate child never inherits a caller\'s JUST_* setting', () => {
  const env = gateChildEnv({ name: 'x', dir: 'x', run: { kind: 'just', recipe: 'gate' } }, { JUST_NO_DEPS: 'true', JUST_DRY_RUN: '1', PATH: '/bin' }, ROOT);
  assert.equal(env.PATH, '/bin');
  assert.deepEqual(Object.keys(env).filter(k => k.startsWith('JUST_')), []);
});

test('the expectation travels without braces and round-trips; a malformed one refuses', () => {
  const expected = { commit: 'a'.repeat(40), inventory: 'b'.repeat(64), packages: 'c'.repeat(64), node: 'v24.18.1' };
  const text = encodeExpectation(expected);
  assert.ok(!/[{}]/.test(text), text);
  assert.deepEqual(decodeExpectation(text), expected);
  assert.equal(decodeExpectation(''), null);
  assert.throws(() => decodeExpectation('a:b'), /malformed/);
  assert.throws(() => encodeExpectation({ ...expected, node: '' }), /not encodable/);
});

test('every node group writes its own skip report; a missing report or an unallowed skip in any group is a finding', t => {
  const dir = mkdtempSync(join(tmpdir(), 'suite-skips-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const steps = plan({ groups: [
    { runner: 'node', files: [{ file: 'a.test.mjs', reads: [] }] },
    { runner: 'bash', files: [{ file: 'b.test.sh', tree: 'shell' }] },
    { runner: 'node', files: [{ file: 'c.test.mjs', reads: [] }] }] }, 'all', { skipsDir: dir });
  const reports = steps.map(step => step.skips).filter(Boolean);
  assert.equal(new Set(reports).size, 2, 'two node groups, two reports');
  const row = (file, name) => `${JSON.stringify({ file: join('/v/genesis/orchestrator', file), name, reason: 'r' })}\n`;
  writeFileSync(reports[0], row('a.test.mjs', 'first-group skip'));
  assert.deepEqual(skipFindings(steps, [], { dir: '/v/genesis/orchestrator' }), [
    'skipped and not in allowedSkips: a.test.mjs › first-group skip (r)', `group 3 wrote no skip report (${reports[1]})`]);
  writeFileSync(reports[1], '');
  assert.deepEqual(skipFindings(steps, [{ file: 'a.test.mjs', test: 'first-group skip', reason: 'allowed' }], { dir: '/v/genesis/orchestrator' }), []);
});

test('a tool the list names that is not on the view PATH is refused with its cure; skips outside the allow-list are named', () => {
  assert.deepEqual(missingTools([['sh', '-c', '']], { PATH: '/usr/bin:/bin' }), []);
  assert.match(missingTools([['no-such-tool-e11', '--version']], { PATH: '/usr/bin:/bin' })[0], /could not run \(ENOENT\): install a working no-such-tool-e11/);
  assert.match(missingTools([['sh', '-c', 'exit 3']], { PATH: '/usr/bin:/bin' })[0], /exited 3: install a working sh/);
  const lines = [{ file: '/v/genesis/orchestrator/a.test.mjs', name: 'x', reason: 'r' }, { file: '/v/genesis/orchestrator/b.test.mjs', name: 'y', reason: 'r' }]
    .map(row => JSON.stringify(row)).join('\n');
  assert.deepEqual(unexpectedSkips(lines, [{ file: 'a.test.mjs', test: 'x', reason: 'allowed' }], { dir: '/v/genesis/orchestrator' }).map(r => r.file), ['b.test.mjs']);
  assert.ok(Array.isArray(suite.allowedSkips) && suite.allowedSkips.every(a => a.file && a.test && a.reason), 'every allowed skip names its file, test and reason');
  assert.ok(suite.tools.some(argv => argv.join(' ') === 'python3 -c '), 'prepush-self needs a python3 that runs: the run refuses without it rather than skipping');
});

test('package.json, the justfile and the manifest run the suite through the list', () => {
  const pkg = JSON.parse(readFileSync(join(HERE, 'package.json'), 'utf8'));
  assert.equal(pkg.scripts.test, 'node suite-run.mjs all');
  assert.equal(pkg.scripts['test:named'], 'node suite-run.mjs named');
  assert.equal(pkg.scripts['test:tree'], 'node suite-run.mjs tree');
  const justfile = readFileSync(join(HERE, 'justfile'), 'utf8');
  assert.match(justfile, /^gate:\n    node suite-run\.mjs named$/m, 'the runner is in the recipe body, so JUST_NO_DEPS cannot skip it');
  assert.match(justfile, /^gate-tree: lint test-suite-tree test-harvest test-berth$/m);
  const projects = JSON.parse(readFileSync(join(HERE, 'build-manifest.json'), 'utf8')).gate.projects;
  const { orchestrator, 'orchestrator-tree': tree } = projects;
  assert.equal(orchestrator.run.recipe, 'gate');
  assert.equal(tree.run.recipe, 'gate-tree');
  // The named half builds nothing: no cargo slot, so no native closure in its key.
  assert.equal(orchestrator.run.cargo, undefined);
  assert.equal(tree.run.cargo.workspace, 'elohim/eprfs');
  // Same dir, steps and inputs: the two halves are selected together, and by name both run.
  for (const key of ['dir', 'steps', 'inputs']) assert.deepEqual(tree[key], orchestrator[key], key);
});

test('`gate-runner.mjs --target orchestrator --print` names both halves outside pre-push, one under it', () => {
  const run = env => spawnSync(process.execPath, [join(HERE, 'gate-runner.mjs'), '--target', 'orchestrator', '--names'],
    { cwd: ROOT, encoding: 'utf8', env: { ...env, GATE_PREFLIGHT: 'off' } });
  const by = { ...process.env };
  delete by.GATE_RUN_KIND;
  assert.deepEqual(run(by).stdout.trim().split('\n'), ['orchestrator', 'orchestrator-tree']);
  assert.deepEqual(run({ ...by, GATE_RUN_KIND: 'pre-push' }).stdout.trim().split('\n'), ['orchestrator']);
});

test('a submodule declared update = none keys its pin and gives the view an empty directory, on any device', t => {
  const root = mkdtempSync(join(tmpdir(), 'suite-none-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const git = (cwd, ...args) => {
    const r = spawnSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@example.test', ...args], { cwd, encoding: 'utf8' });
    assert.equal(r.status, 0, `${args.join(' ')}: ${r.stderr}`);
    return r.stdout.trim();
  };
  const sub = join(root, 'sub');
  mkdirSync(join(sub, 'x'), { recursive: true });
  git(sub, 'init', '-q');
  writeFileSync(join(sub, 'x/build-manifest.json'), '{"s":1}\n');
  git(sub, 'add', '.');
  git(sub, 'commit', '-q', '-m', 'sub');
  const pin = git(sub, 'rev-parse', 'HEAD');
  const listText = JSON.stringify({ groups: [{ runner: 'node', files: [{ file: 'a.test.mjs', reads: ['sub', '**/build-manifest.json'] }] }] });
  const build = (name, { none }) => {
    const top = join(root, name);
    mkdirSync(join(top, 'genesis/orchestrator'), { recursive: true });
    git(top, 'init', '-q');
    writeFileSync(join(top, 'genesis/orchestrator/suite-reads.json'), listText);
    writeFileSync(join(top, '.gitmodules'), `[submodule "sub"]\n\tpath = sub\n\turl = ./sub\n${none ? '\tupdate = none\n' : ''}`);
    git(top, 'add', '.');
    git(top, 'update-index', '--add', '--cacheinfo', `160000,${pin},sub`);
    git(top, 'commit', '-q', '-m', 'top');
    return top;
  };
  const bare = build('bare', { none: true });
  const held = build('held', { none: true });
  git(root, 'clone', '-q', '--bare', sub, join(held, '.git/modules/sub'));
  const a = suiteInventory({ root: bare, commit: 'HEAD', listText });
  assert.deepEqual(a.pins.map(p => [p.path, p.oid, p.store, p.entries.length]), [['sub', pin, null, 0]]);
  const view = join(root, 'view');
  assert.equal(materialise(a, view, { root: bare }), 1, 'only suite-reads.json');
  assert.deepEqual(readdirSync(join(view, 'sub')), [], 'an empty directory');
  assert.equal(suiteInventory({ root: held, commit: 'HEAD', listText }).digest, a.digest, 'a device holding the objects keys and views the same');
  const heldView = join(root, 'held-view');
  materialise(suiteInventory({ root: held, commit: 'HEAD', listText }), heldView, { root: held });
  assert.deepEqual(readdirSync(join(heldView, 'sub')), []);
  // Moving the pin moves the digest.
  writeFileSync(join(sub, 'y.txt'), 'y\n');
  git(sub, 'add', '.');
  git(sub, 'commit', '-q', '-m', 'sub2');
  git(held, 'update-index', '--cacheinfo', `160000,${git(sub, 'rev-parse', 'HEAD')},sub`);
  git(held, 'commit', '-q', '-m', 'move');
  assert.notEqual(suiteInventory({ root: held, commit: 'HEAD', listText }).digest, a.digest);
  // Without the declaration and with no store, the refusal names its cure.
  const plain = build('plain', { none: false });
  assert.throws(() => suiteInventory({ root: plain, commit: 'HEAD', listText }), /git submodule update --init sub/);
});

test('the suite never hands its tests the push\'s live run: treeEnv drops GATE_LIVE_RUN and namedEnv never carried it', () => {
  const env = treeEnv({ PATH: '/bin', GATE_LIVE_RUN: '/run/live.jsonl', NODE_TEST_CONTEXT: 'child', KEEP: '1' });
  assert.equal(Object.hasOwn(env, 'GATE_LIVE_RUN'), false);
  assert.equal(Object.hasOwn(env, 'NODE_TEST_CONTEXT'), false);
  assert.equal(env.KEEP, '1');
  assert.equal(Object.hasOwn(namedEnv('/scratch'), 'GATE_LIVE_RUN'), false);
});
