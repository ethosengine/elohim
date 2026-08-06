import assert from 'node:assert/strict';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const repoRoot = fileURLToPath(new URL('../..', import.meta.url));
const cargoPool = join(repoRoot, 'genesis/agentic/bin/cargo-pool');

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'cargo-pool-doctor-'));
  const repo = join(root, 'repo');
  const worktrees = join(root, 'worktrees');
  const pool = join(root, 'pool');
  mkdirSync(join(repo, 'elohim'), { recursive: true });
  mkdirSync(worktrees, { recursive: true });
  mkdirSync(join(pool, 'family/dev/storage'), { recursive: true });

  const healthyTarget = join(root, 'targets/healthy');
  const slotTarget = join(root, 'targets/slot');
  const treeTarget = join(root, 'targets/tree');
  const blocker = join(root, 'blocked-parent');
  const blockedTarget = join(blocker, 'nested');
  const outsideTmpTarget = `/var/tmp/${root.split('/').at(-1)}-unsafe`;
  mkdirSync(healthyTarget, { recursive: true });
  writeFileSync(blocker, 'not a directory\n');

  const healthyLink = join(pool, 'family/dev/storage/release');
  const slotLink = join(pool, 'family/dev/storage/dev');
  const blockedLink = join(pool, 'family/dev/storage/test');
  const outsideTmpLink = join(pool, 'family/dev/storage/bench');
  const treeLink = join(repo, 'elohim/target');
  symlinkSync(healthyTarget, healthyLink, 'dir');
  symlinkSync(slotTarget, slotLink, 'dir');
  symlinkSync(blockedTarget, blockedLink, 'dir');
  symlinkSync(outsideTmpTarget, outsideTmpLink, 'dir');
  symlinkSync(treeTarget, treeLink, 'dir');

  const env = {
    ...process.env,
    CARGO_TARGET_POOL_ROOT: pool,
    POOL_PARENT_REPO: repo,
    POOL_WORKTREES_DIR: worktrees,
  };
  const run = (...args) => spawnSync(cargoPool, args, { env, encoding: 'utf8' });
  return {
    root,
    healthyLink,
    slotLink,
    slotTarget,
    blockedLink,
    blockedTarget,
    outsideTmpLink,
    outsideTmpTarget,
    treeLink,
    treeTarget,
    run,
  };
}

test('doctor reports every dangling slot and in-tree target without touching them', (t) => {
  const f = fixture();
  t.after(() => rmSync(f.root, { recursive: true, force: true }));

  const result = f.run('doctor');
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, new RegExp(`pool-slot\\s+dangling\\s+${f.slotLink}`));
  assert.match(result.stdout, new RegExp(`pool-slot\\s+unpreparable\\s+${f.blockedLink}`));
  assert.match(result.stdout, new RegExp(`pool-slot\\s+unpreparable\\s+${f.outsideTmpLink}`));
  assert.match(result.stdout, new RegExp(`in-tree-target\\s+dangling\\s+${f.treeLink}`));
  assert.doesNotMatch(result.stdout, new RegExp(f.healthyLink));
  assert.equal(existsSync(f.slotTarget), false);
  assert.equal(existsSync(f.treeTarget), false);
});

test('--heal recreates only preparable /tmp targets and fails closed on blockers', (t) => {
  const f = fixture();
  t.after(() => rmSync(f.root, { recursive: true, force: true }));

  const result = f.run('doctor', '--heal');
  assert.equal(result.status, 1);
  assert.match(result.stdout, /HEALED/);
  assert.match(result.stdout, /UNPREPARABLE/);
  assert.match(result.stdout, /healed=2 unpreparable=2/);
  assert.equal(existsSync(f.slotTarget), true);
  assert.equal(existsSync(f.treeTarget), true);
  assert.equal(existsSync(f.blockedTarget), false);
  assert.equal(existsSync(f.outsideTmpTarget), false);
  assert.equal(readlinkSync(f.slotLink), f.slotTarget);
  assert.equal(readlinkSync(f.treeLink), f.treeTarget);
});

test('status folds in the link doctor findings', (t) => {
  const f = fixture();
  t.after(() => rmSync(f.root, { recursive: true, force: true }));

  const result = f.run('status');
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /Cargo target link doctor:/);
  assert.match(result.stdout, new RegExp(f.treeLink));
});
