import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildPreload, checkLog, entryFor, judgeRead, judgeSpawn } from './read-fence.mjs';

const FENCE = fileURLToPath(new URL('./read-fence.mjs', import.meta.url));
const testFile = 'genesis/orchestrator/x.test.mjs';
const entry = { file: 'x.test.mjs', reads: ['elohim/sdk/schemas/v1/a.json', 'scripts/ci', '**/build-manifest.json'] };

test('a named file reads its own tree, its declared paths and prefixes, and nothing else', () => {
  for (const path of ['genesis/orchestrator/gate-runner.mjs', testFile, 'elohim/sdk/schemas/v1/a.json', 'scripts/ci/x.sh',
    'app/elohim-app/build-manifest.json']) {
    assert.equal(judgeRead({ op: 'readFileSync', path }, entry, testFile), null, path);
  }
  for (const path of ['elohim/sdk/schemas/v1/b.json', 'scripts/cix', 'CLAUDE.md', 'genesis/orchestratorx/y.mjs']) {
    assert.equal(judgeRead({ op: 'readFileSync', path }, entry, testFile), 'undeclared read', path);
  }
  assert.equal(judgeRead({ op: 'module', path: 'genesis/agentic/compute/common.mjs' }, entry, testFile), 'undeclared read');
});

test('probes may climb to a declared read; a probe of an undeclared file is a read', () => {
  for (const path of ['.', 'elohim', 'elohim/sdk/schemas', 'genesis']) {
    assert.equal(judgeRead({ op: 'statSync', path }, entry, testFile), null, path);
  }
  assert.equal(judgeRead({ op: 'existsSync', path: 'devfile.yaml' }, entry, testFile), 'undeclared read');
  // Under a declared walk, a probe of another tracked file of the walked name is the walk.
  assert.equal(judgeRead({ op: 'existsSync', path: 'x/build-manifest.json' }, entry, testFile), null);
});

test('a listing needs a declared prefix or a declared walk', () => {
  const plain = { file: 'x.test.mjs', reads: ['scripts/ci'] };
  assert.equal(judgeRead({ op: 'readdirSync', path: 'scripts/ci' }, plain, testFile), null);
  assert.equal(judgeRead({ op: 'readdirSync', path: 'elohim' }, plain, testFile), 'undeclared listing');
  assert.equal(judgeRead({ op: 'readdirSync', path: 'elohim' }, entry, testFile), null);
});

test('children: node must carry the whole fence; any other child must be bound to a fixture', () => {
  const root = '/repo';
  const baseEnv = { NODE_OPTIONS: `--import=${FENCE}`, GATE_READ_FENCE_LOG: '/l', GATE_READ_FENCE_ROOT: root,
    GATE_READ_FENCE_FILE: testFile, PWD: '/repo/genesis/orchestrator' };
  const ctx = { root, fence: FENCE, baseEnv };
  assert.equal(judgeSpawn({ cmd: 'node', args: ['x.mjs'], cwd: '/repo', env: null }, ctx), null);
  assert.equal(judgeSpawn({ cmd: 'node', args: ['x.mjs'], cwd: '/repo', env: { PATH: '/bin' } }, ctx), 'node child without the read fence');
  assert.equal(judgeSpawn({ cmd: 'node', args: ['x.mjs'], cwd: '/repo', env: { ...baseEnv, GATE_READ_FENCE_LOG: undefined } }, ctx),
    'node child without the read fence');
  assert.equal(judgeSpawn({ cmd: 'git', args: ['init', '-q', '/tmp/f'], cwd: '/tmp/f', env: null }, ctx), null);
  assert.equal(judgeSpawn({ cmd: 'bash', args: ['-c', 'true'], cwd: '/repo/genesis/orchestrator', env: null }, ctx),
    'unfenced child in the repository');
  assert.equal(judgeSpawn({ cmd: 'git', args: ['-C', '/repo', 'ls-files'], cwd: '/tmp/f', env: null }, ctx),
    'unfenced child in the repository');
  assert.equal(judgeSpawn({ cmd: 'cat', args: ['/repo/x'], cwd: '/tmp/f', env: null }, ctx), 'unfenced child names a repository path');
  // A sibling whose name merely starts with the root is not the repository.
  assert.equal(judgeSpawn({ cmd: 'cat', args: ['/repo-other/x', '/repox'], cwd: '/tmp/f', env: null }, ctx), null);
  assert.equal(judgeSpawn({ cmd: 'bash', args: ['/tmp/f/x.sh'], cwd: '/tmp/f', env: { ...baseEnv, HOOK_ROOT: '/repo' } }, ctx),
    'unfenced child given a repository path in HOOK_ROOT');
  // Inherited ambient values that only say where the shell was are not reads.
  assert.equal(judgeSpawn({ cmd: 'bash', args: ['/tmp/f/x.sh'], cwd: '/tmp/f', env: { ...baseEnv, PWD: '/repo/elsewhere' } }, ctx), null);
});

test('git: -C names where it works; the object store and an inherited GIT_DIR are the repository', () => {
  const root = '/repo/.worktrees/w';
  const gitDir = '/repo/.git';
  const baseEnv = { PATH: '/bin' };
  const ctx = { root, gitDir, fence: FENCE, baseEnv };
  assert.equal(judgeSpawn({ cmd: 'git', args: ['-C', '/tmp/f', 'status'], cwd: `${root}/genesis/orchestrator`, env: null }, ctx), null);
  assert.equal(judgeSpawn({ cmd: 'git', args: ['status'], cwd: `${root}/genesis/orchestrator`, env: null }, ctx),
    'unfenced child in the repository');
  assert.equal(judgeSpawn({ cmd: 'git', args: ['-C', '.', 'status'], cwd: `${root}/genesis/orchestrator`, env: null }, ctx),
    'unfenced child in the repository');
  assert.equal(judgeSpawn({ cmd: 'git', args: ['--git-dir=/repo/.git', 'log'], cwd: '/tmp/f', env: null }, ctx),
    'unfenced child names a repository path');
  const hooked = { ...ctx, baseEnv: { PATH: '/bin', GIT_DIR: '/repo/.git/worktrees/w' } };
  assert.equal(judgeSpawn({ cmd: 'git', args: ['-C', '/tmp/f', 'status'], cwd: '/tmp/f', env: null }, hooked),
    'unfenced child given a repository path in GIT_DIR');
  // A non-git child's -C is just an argument: its process cwd decides.
  assert.equal(judgeSpawn({ cmd: 'tar', args: ['-C', '/tmp/f', '-czf', '/tmp/x.tgz', '.'], cwd: `${root}/genesis/orchestrator`, env: null }, ctx),
    'unfenced child in the repository');
});

test('entryFor resolves list paths relative to genesis/orchestrator', () => {
  const list = { groups: [{ runner: 'node', files: [{ file: '../agentic/compute/a.test.mjs', reads: [] }, { file: 'b.test.mjs', tree: 'why' }] }] };
  assert.equal(entryFor(list, 'genesis/agentic/compute/a.test.mjs').file, '../agentic/compute/a.test.mjs');
  assert.equal(entryFor(list, 'genesis/orchestrator/b.test.mjs').tree, 'why');
  assert.equal(entryFor(list, 'genesis/orchestrator/c.test.mjs'), null);
});

test('REAL fence: a test process and its node child are judged, the outcome is left alone, check fails naming each read', () => {
  const root = mkdtempSync(join(tmpdir(), 'read-fence-'));
  try {
    const dir = join(root, 'genesis/orchestrator');
    mkdirSync(dir, { recursive: true });
    mkdirSync(join(root, 'declared'), { recursive: true });
    mkdirSync(join(root, 'secret'), { recursive: true });
    writeFileSync(join(root, 'declared/ok.txt'), 'ok');
    writeFileSync(join(root, 'secret/no.txt'), 'no');
    mkdirSync(join(root, '.gitdir'), { recursive: true });
    writeFileSync(join(root, '.gitdir/HEAD'), 'ref: refs/heads/dev');
    writeFileSync(join(dir, 'suite-reads.json'), JSON.stringify({ groups: [{ runner: 'node', files: [
      { file: 'x.test.mjs', reads: ['declared'] }, { file: 'tree.test.mjs', tree: 'reads the tree' }] }] }));
    writeFileSync(join(dir, 'child.mjs'), `import { readFileSync } from 'node:fs';\nreadFileSync(${JSON.stringify(join(root, 'secret/no.txt'))});\n`);
    writeFileSync(join(dir, 'x.test.mjs'), [
      "import test from 'node:test';",
      "import { readFileSync } from 'node:fs';",
      "import { spawnSync } from 'node:child_process';",
      `const root = ${JSON.stringify(root)};`,
      "test('reads', () => {",
      "  readFileSync(root + '/declared/ok.txt');",
      "  readFileSync(root + '/secret/no.txt');",
      "  readFileSync(root + '/.gitdir/HEAD');",
      "  spawnSync('true', [], { cwd: root });",
      "  spawnSync('true', [], { cwd: '/' });",
      "  const child = spawnSync(process.execPath, [root + '/genesis/orchestrator/child.mjs']);",
      "  if (child.status !== 0) throw new Error(String(child.stderr));",
      "});",
    ].join('\n'));
    writeFileSync(join(dir, 'tree.test.mjs'), "import test from 'node:test'; test('t', () => {});\n");
    const log = join(root, 'fence.jsonl');
    const env = { ...process.env, NODE_OPTIONS: `--import=${FENCE}`, GATE_READ_FENCE_ROOT: root, GATE_READ_FENCE_LOG: log,
      GATE_READ_FENCE_GITDIR: join(root, '.gitdir') };
    delete env.NODE_TEST_CONTEXT; // this test runs inside a runner; the fixture runner must isolate its files
    delete env.GATE_READ_FENCE_FILE;
    const run = spawnSync(process.execPath, ['--test', '--test-isolation=process', 'x.test.mjs', 'tree.test.mjs'], {
      cwd: dir, encoding: 'utf8', env,
    });
    assert.equal(run.status, 0, run.stdout + run.stderr);
    const lines = [];
    const status = checkLog(log, { write: line => lines.push(line) });
    assert.equal(status, 1);
    const text = lines.join('');
    assert.match(text, /genesis\/orchestrator\/x\.test\.mjs: undeclared read: secret\/no\.txt \(readFileSync\)/);
    assert.match(text, /x\.test\.mjs: unfenced child in the repository: true/);
    assert.match(text, /x\.test\.mjs: repository object store read: <git-dir>\/HEAD/);
    assert.match(text, /genesis\/orchestrator\/tree\.test\.mjs: tree file run under the fence/);
    assert.doesNotMatch(text, /declared\/ok\.txt/);
    assert.equal(lines.filter(line => /secret\/no\.txt/.test(line)).length, 1, 'the child read joins its parent test file once');
    assert.equal(readFileSync(log, 'utf8').split('\n').filter(Boolean).length >= 3, true);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('REAL preload: a dynamic child is seen and judged; a cleared fence and a static program are named; a fixture-bound exec is not', () => {
  const so = buildPreload();
  const root = mkdtempSync(join(tmpdir(), 'read-fence-preload-'));
  try {
    for (const dir of ['genesis/orchestrator', 'declared', 'secret', '.gitdir']) mkdirSync(join(root, dir), { recursive: true });
    writeFileSync(join(root, 'declared/ok.txt'), 'ok');
    writeFileSync(join(root, 'secret/no.txt'), 'no');
    writeFileSync(join(root, '.gitdir/HEAD'), 'ref');
    const file = 'genesis/orchestrator/x.test.mjs';
    const list = { groups: [{ runner: 'node', files: [{ file: 'x.test.mjs', reads: ['declared'] }] }] };
    const log = join(root, 'fence.jsonl');
    const staticProgram = ['/usr/local/bin/just', '/usr/bin/just'].find(p => existsSync(p));
    const script = [
      'cat ../../declared/ok.txt ../../secret/no.txt >/dev/null',
      'ls ../../secret >/dev/null',
      'test -e ../../.gitdir/HEAD',
      'env -u LD_PRELOAD cat ../../declared/ok.txt >/dev/null',
      '(cd / && env -u LD_PRELOAD true)',
      staticProgram ? `${staticProgram} --version >/dev/null` : 'true',
    ].join('\n');
    const run = spawnSync('bash', ['-c', script], {
      cwd: join(root, 'genesis/orchestrator'), encoding: 'utf8',
      env: { PATH: process.env.PATH, LD_PRELOAD: so, GATE_READ_FENCE_SO: so, GATE_READ_FENCE_ROOT: root,
        GATE_READ_FENCE_LOG: log, GATE_READ_FENCE_GITDIR: join(root, '.gitdir'), GATE_READ_FENCE_FILE: file },
    });
    assert.equal(run.status, 0, run.stderr);
    const lines = [];
    assert.equal(checkLog(log, { write: line => lines.push(line) }, list), 1);
    const text = lines.join('');
    assert.match(text, /x\.test\.mjs: undeclared read: secret\/no\.txt \(open, /);
    assert.match(text, /x\.test\.mjs: undeclared listing: secret \(opendir, /);
    assert.match(text, /x\.test\.mjs: repository object store read: <git-dir>\/HEAD/);
    assert.match(text, /x\.test\.mjs: child cleared the read fence: \S*\/cat \(exec/, 'env cleared the fence for cat, inside the repository');
    assert.doesNotMatch(text, /declared\/ok\.txt/);
    assert.doesNotMatch(text, /true/, 'a cleared exec from outside the repository naming nothing in it is fixture-bound');
    if (staticProgram) assert.match(text, /static program, unseen by the read fence: \S*just/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('check passes on an absent or empty log', () => {
  const root = mkdtempSync(join(tmpdir(), 'read-fence-'));
  try {
    assert.equal(checkLog(join(root, 'none.jsonl'), { write() {} }), 0);
    writeFileSync(join(root, 'empty.jsonl'), '');
    assert.equal(checkLog(join(root, 'empty.jsonl'), { write() {} }), 0);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('the fence lives beside the suite it judges', () => {
  assert.equal(dirname(FENCE).endsWith('genesis/orchestrator'), true);
});
