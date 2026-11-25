import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const repoRoot = fileURLToPath(new URL('../..', import.meta.url));
const cargoPool = join(repoRoot, 'genesis/agentic/bin/cargo-pool');
const gateRunner = join(repoRoot, 'genesis/orchestrator/gate-runner.mjs');

function gateWorktreeTargetDir() {
  const source = readFileSync(gateRunner, 'utf8');
  const match = source.match(
    /export function worktreeTargetDir[\s\S]*?\n}\n\nfunction gitDirs/,
  );
  assert.ok(match, 'gate-runner must still export worktreeTargetDir');
  const declaration = match[0]
    .replace(/^export /, '')
    .replace(/\n\nfunction gitDirs$/, '');
  return Function('basename', `${declaration}; return worktreeTargetDir;`)(basename);
}

function git(cwd, ...args) {
  const result = spawnSync('git', ['-C', cwd, ...args], {
    encoding: 'utf8',
    env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' },
  });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'cargo-pool-worktree-slots-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repo = join(root, 'repo');
  const worktrees = join(root, 'worktrees');
  const pool = join(root, 'pool');
  mkdirSync(join(repo, 'elohim/eprfs'), { recursive: true });
  mkdirSync(worktrees, { recursive: true });
  writeFileSync(join(repo, 'elohim/eprfs/Cargo.toml'), '[workspace]\nmembers = []\n');
  git(repo, 'init', '-q', '-b', 'dev');
  git(repo, 'add', 'elohim/eprfs/Cargo.toml');
  git(repo, '-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid',
    'commit', '-q', '-m', 'fixture');
  const a = join(worktrees, 'wt-a');
  const b = join(worktrees, 'wt-b');
  git(repo, 'worktree', 'add', '-q', '-b', 'cd/a', a);
  git(repo, 'worktree', 'add', '-q', '-b', 'cd/b', b);
  const env = {
    ...process.env,
    CARGO_TARGET_POOL_ROOT: pool,
    POOL_PARENT_REPO: repo,
    POOL_WORKTREES_DIR: worktrees,
    GIT_CONFIG_GLOBAL: '/dev/null',
    GIT_CONFIG_NOSYSTEM: '1',
    EPR_BIN: '/nonexistent',
  };
  delete env.CARGO_TARGET_POOL_FAMILY;

  const run = (cwd, ...args) => spawnSync(cargoPool, args, { cwd, env, encoding: 'utf8' });
  const key = (checkout) => {
    const result = run(join(checkout, 'elohim/eprfs'), 'key');
    assert.equal(result.status, 0, result.stderr);
    return Object.fromEntries(result.stdout.split('\n').filter(line => line.includes('='))
      .map((line) => {
        const at = line.indexOf('=');
        return [line.slice(0, at), line.slice(at + 1)];
      }));
  };
  const gateTarget = (base, checkout) => {
    const dirs = () => ({
      gitDir: git(checkout, 'rev-parse', '--path-format=absolute', '--git-dir'),
      commonDir: git(checkout, 'rev-parse', '--path-format=absolute', '--git-common-dir'),
    });
    return gateWorktreeTargetDir()(base, checkout, dirs);
  };
  return { repo, worktrees, pool, a, b, run, key, gateTarget };
}

test('two cd linked worktrees get distinct key slots and the main checkout stays unsuffixed', (t) => {
  const f = fixture(t);
  const main = f.key(f.repo);
  const a = f.key(f.a);
  const b = f.key(f.b);

  assert.equal(main.slot_dev, join(f.pool, 'family/dev/elohim__eprfs/dev'));
  assert.equal(main.rule, 'main-checkout suffix=none');
  assert.equal(a.rule, `linked-worktree suffix=-wt-${basename(f.a)}`);
  assert.equal(b.rule, `linked-worktree suffix=-wt-${basename(f.b)}`);
  assert.notEqual(a.slot_dev, b.slot_dev);

  const status = f.run(f.a, 'status');
  assert.equal(status.status, 0, status.stderr);
  assert.match(status.stdout, /Slot rule:\s+linked-worktree suffix=-wt-wt-a/);
});

test('cargo-pool key and gate-runner compute the same linked-worktree slot', (t) => {
  const f = fixture(t);
  const key = f.key(f.a);
  const base = join(f.pool, 'family/cd/elohim__eprfs/dev');
  assert.equal(key.slot_dev, f.gateTarget(base, f.a));
  assert.equal(key.slot_release,
    f.gateTarget(join(f.pool, 'family/cd/elohim__eprfs/release'), f.a));
});

test('steward dry-run marks a gone worktree slot reclaimable without deleting it', (t) => {
  const f = fixture(t);
  const live = f.key(f.a).slot_dev;
  const gone = f.key(f.b).slot_dev;
  mkdirSync(live, { recursive: true });
  mkdirSync(gone, { recursive: true });
  git(f.repo, 'worktree', 'remove', f.b);

  const result = f.run(f.repo, 'steward', '--dry-run');
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /stale-worktree-slot\s+reclaimable/);
  assert.match(result.stdout, new RegExp(gone.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  assert.doesNotMatch(result.stdout, new RegExp(`${live.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}.*reclaimable`));
  assert.equal(spawnSync('test', ['-d', gone]).status, 0, 'dry-run must preserve the slot');
});
