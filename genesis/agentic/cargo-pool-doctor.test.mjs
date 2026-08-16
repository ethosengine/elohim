import assert from 'node:assert/strict';
import {
  chmodSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
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

  const fixtureId = basename(root);
  const healthyDirSlot = join(pool, 'family/dev/healthy/dev');
  const brokenFlatname = `broken-${fixtureId}`;
  const brokenDirSlot = join(pool, `family/dev/${brokenFlatname}/dev`);
  const genericFailDirSlot = join(pool, 'family/dev/generic-fail/dev');
  mkdirSync(healthyDirSlot, { recursive: true });
  mkdirSync(brokenDirSlot, { recursive: true });
  mkdirSync(genericFailDirSlot, { recursive: true });

  const healthyTarget = join(root, 'targets/healthy');
  const slotTarget = join(root, 'targets/slot');
  const treeTarget = join(root, 'targets/tree');
  const blocker = join(root, 'blocked-parent');
  const blockedTarget = join(blocker, 'nested');
  const outsideTmpTarget = `/var/tmp/${root.split('/').at(-1)}-unsafe`;
  mkdirSync(healthyTarget, { recursive: true });
  writeFileSync(join(healthyTarget, 'artifact'), 'warm\n');
  writeFileSync(join(healthyDirSlot, 'artifact'), 'warm\n');
  writeFileSync(join(brokenDirSlot, 'warm-artifact'), 'preserve me\n');
  writeFileSync(blocker, 'not a directory\n');

  const healthyLink = join(pool, 'family/dev/storage/release');
  const slotLink = join(pool, 'family/dev/storage/dev');
  const blockedLink = join(pool, 'family/dev/storage/test');
  const outsideTmpLink = join(pool, 'family/dev/storage/bench');
  const treeLink = join(repo, 'elohim/target');
  const wasmTargetDir = join(repo, 'elohim/holochain/dna/learning');
  const wasmTargetLink = join(wasmTargetDir, 'target');
  mkdirSync(wasmTargetDir, { recursive: true });
  symlinkSync(healthyTarget, healthyLink, 'dir');
  symlinkSync(slotTarget, slotLink, 'dir');
  symlinkSync(blockedTarget, blockedLink, 'dir');
  symlinkSync(outsideTmpTarget, outsideTmpLink, 'dir');
  symlinkSync(treeTarget, treeLink, 'dir');
  symlinkSync('../../target', wasmTargetLink, 'dir');

  const cargoLog = join(root, 'cargo-invocations.log');
  const fakeCargo = join(root, 'fake-cargo');
  writeFileSync(
    fakeCargo,
    `#!/usr/bin/env bash
printf '%s\\n' "\${CARGO_TARGET_DIR:-unset}" >> "\${FAKE_CARGO_LOG}"
case "\${CARGO_TARGET_DIR:-}" in
  */broken-*/dev/.cargo-pool-doctor-probe)
    echo "error: failed to write \`\${CARGO_TARGET_DIR}/debug/.fingerprint/cargo-pool-doctor-probe/invoked.timestamp\`: No such file or directory (os error 2)" >&2
    exit 101
    ;;
  */generic-fail/dev/.cargo-pool-doctor-probe)
    echo "error: configured probe compiler is unavailable" >&2
    exit 101
    ;;
  *)
    mkdir -p "\${CARGO_TARGET_DIR}/debug/.fingerprint/cargo-pool-doctor-probe"
    : > "\${CARGO_TARGET_DIR}/debug/.fingerprint/cargo-pool-doctor-probe/invoked.timestamp"
    ;;
esac
`,
  );
  chmodSync(fakeCargo, 0o755);

  const env = {
    ...process.env,
    CARGO_TARGET_POOL_ROOT: pool,
    POOL_PARENT_REPO: repo,
    POOL_WORKTREES_DIR: worktrees,
    CARGO: fakeCargo,
    FAKE_CARGO_LOG: cargoLog,
    CARGO_TARGET_POOL_FAMILY: 'active',
  };
  const run = (...args) =>
    spawnSync(cargoPool, args, { env, encoding: 'utf8' });
  const runWithEnv = (overrides, ...args) =>
    spawnSync(cargoPool, args, {
      env: { ...env, ...overrides },
      encoding: 'utf8',
    });
  const fingerprintHealTarget = `/tmp/pool-${brokenFlatname}-target`;
  const cleanup = () => {
    rmSync(root, { recursive: true, force: true });
    rmSync(fingerprintHealTarget, { recursive: true, force: true });
  };
  return {
    root,
    healthyDirSlot,
    brokenDirSlot,
    genericFailDirSlot,
    healthyTarget,
    healthyLink,
    slotLink,
    slotTarget,
    blockedLink,
    blockedTarget,
    outsideTmpLink,
    outsideTmpTarget,
    treeLink,
    treeTarget,
    wasmTargetLink,
    cargoLog,
    fingerprintHealTarget,
    cleanup,
    run,
    runWithEnv,
  };
}

async function holdSlotLock(slot) {
  const lock = join(slot, 'debug/.cargo-lock');
  mkdirSync(dirname(lock), { recursive: true });
  const holder = spawn(
    'bash',
    [
      '-c',
      'exec 9>>"$1"; flock -x 9; echo ready; read -r _',
      'cargo-pool-doctor-lock-holder',
      lock,
    ],
    { stdio: ['pipe', 'pipe', 'pipe'] },
  );
  const [ready] = await once(holder.stdout, 'data');
  assert.match(String(ready), /ready/);
  return async () => {
    holder.stdin.end('\n');
    if (holder.exitCode === null) await once(holder, 'exit');
  };
}

function removeUnrelatedProbeFailures(f) {
  rmSync(f.genericFailDirSlot, { recursive: true, force: true });
  rmSync(f.slotLink, { force: true });
  rmSync(f.blockedLink, { force: true });
  rmSync(f.outsideTmpLink, { force: true });
}

test('doctor reports every dangling slot and in-tree target without touching them', (t) => {
  const f = fixture();
  t.after(() => rmSync(f.root, { recursive: true, force: true }));

  const result = f.run('doctor');
  assert.equal(result.status, 0, result.stderr);
  assert.match(
    result.stdout,
    new RegExp(`pool-slot\\s+dangling\\s+${f.slotLink}`),
  );
  assert.match(
    result.stdout,
    new RegExp(`pool-slot\\s+unpreparable\\s+${f.blockedLink}`),
  );
  assert.match(
    result.stdout,
    new RegExp(`pool-slot\\s+unpreparable\\s+${f.outsideTmpLink}`),
  );
  assert.match(
    result.stdout,
    new RegExp(`in-tree-target\\s+dangling\\s+${f.treeLink}`),
  );
  assert.doesNotMatch(result.stdout, new RegExp(f.healthyLink));
  assert.doesNotMatch(result.stdout, new RegExp(f.wasmTargetLink));
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
  assert.match(result.stdout, /^dev\s+\S+\s+7\s+/m);
  assert.equal(
    existsSync(f.cargoLog),
    false,
    'status must never run write probes',
  );
});

test('--probe-writes uses Cargo semantics, classifies fingerprint ENOENT, and cleans up', (t) => {
  const f = fixture();
  t.after(() => rmSync(f.root, { recursive: true, force: true }));

  const result = f.run('doctor', '--probe-writes');
  assert.equal(result.status, 1);
  assert.match(result.stdout, /Cargo fingerprint write probe/);
  assert.match(result.stdout, new RegExp(`ok\\s+${f.healthyDirSlot}`));
  assert.match(result.stdout, new RegExp(`ok\\s+${f.healthyLink}`));
  assert.match(
    result.stdout,
    new RegExp(`fingerprint-enoent\\s+${f.brokenDirSlot}`),
  );
  assert.match(
    result.stdout,
    new RegExp(`probe-failed\\s+${f.genericFailDirSlot}`),
  );
  assert.match(result.stdout, /probed=4 failed=2 unavailable=3/);
  assert.equal(
    existsSync(join(f.healthyDirSlot, '.cargo-pool-doctor-probe')),
    false,
  );
  assert.equal(
    existsSync(join(f.brokenDirSlot, '.cargo-pool-doctor-probe')),
    false,
  );
  assert.equal(
    existsSync(join(f.healthyLink, '.cargo-pool-doctor-probe')),
    false,
  );
  assert.equal(
    existsSync(join(f.genericFailDirSlot, '.cargo-pool-doctor-probe')),
    false,
  );
});

test('--probe-writes fails closed when a slot is unavailable', (t) => {
  const f = fixture();
  t.after(() => rmSync(f.root, { recursive: true, force: true }));
  rmSync(f.brokenDirSlot, { recursive: true });
  rmSync(f.genericFailDirSlot, { recursive: true });

  const result = f.run('doctor', '--probe-writes');
  assert.equal(result.status, 1);
  assert.match(result.stdout, /probed=2 failed=0 unavailable=3/);
});

test('watermark update includes healthy symlink-backed slots', (t) => {
  const f = fixture();
  t.after(() => rmSync(f.root, { recursive: true, force: true }));

  const result = f.run('watermark', 'update');
  assert.equal(result.status, 0, result.stderr);
  assert.equal(existsSync(join(f.healthyTarget, '.peak-size')), true);
  assert.equal(existsSync(join(f.healthyDirSlot, '.peak-size')), true);
});

test('--heal-fingerprint-enoent dry-runs, then converts only the failing slot', (t) => {
  const f = fixture();
  t.after(f.cleanup);
  removeUnrelatedProbeFailures(f);

  const dryRun = f.run('doctor', '--heal-fingerprint-enoent', '--dry-run');
  assert.equal(dryRun.status, 1);
  assert.match(dryRun.stdout, /would-heal/);
  assert.match(dryRun.stdout, /the next build is cold/);
  assert.match(dryRun.stdout, /converted=0 would-convert=1 refused=0/);
  assert.equal(existsSync(f.fingerprintHealTarget), false);
  assert.equal(existsSync(f.brokenDirSlot), true);
  assert.equal(lstatSync(f.brokenDirSlot).isSymbolicLink(), false);

  const result = f.run('doctor', '--heal-fingerprint-enoent');
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /healed/);
  assert.match(result.stdout, /converted=1 would-convert=0 refused=0/);
  assert.equal(readlinkSync(f.brokenDirSlot), f.fingerprintHealTarget);
  assert.equal(
    existsSync(join(f.fingerprintHealTarget, 'warm-artifact')),
    true,
    'conversion preserves the warm slot at its /tmp target',
  );
});

test('--heal-fingerprint-enoent refuses an active family without --force', (t) => {
  const f = fixture();
  t.after(f.cleanup);
  removeUnrelatedProbeFailures(f);

  const result = f.runWithEnv(
    { CARGO_TARGET_POOL_FAMILY: 'dev' },
    'doctor',
    '--heal-fingerprint-enoent',
  );
  assert.equal(result.status, 1);
  assert.match(result.stdout, /refused.*active family 'dev'/);
  assert.match(result.stdout, /converted=0 would-convert=0 refused=1/);
  assert.equal(existsSync(f.fingerprintHealTarget), false);
  assert.equal(existsSync(f.brokenDirSlot), true);
});

test('--heal-fingerprint-enoent refuses a flock\'d slot without --force', async (t) => {
  const f = fixture();
  t.after(f.cleanup);
  removeUnrelatedProbeFailures(f);
  const release = await holdSlotLock(f.brokenDirSlot);
  t.after(release);

  const result = f.run('doctor', '--heal-fingerprint-enoent');
  assert.equal(result.status, 1);
  assert.match(result.stdout, /refused.*slot is flock'd/);
  assert.match(result.stdout, /converted=0 would-convert=0 refused=1/);
  assert.equal(existsSync(f.fingerprintHealTarget), false);
});

test('--force converts a fingerprint-ENOENT slot despite active-family and flock guards', async (t) => {
  const f = fixture();
  t.after(f.cleanup);
  removeUnrelatedProbeFailures(f);
  const release = await holdSlotLock(f.brokenDirSlot);
  t.after(release);

  const result = f.runWithEnv(
    { CARGO_TARGET_POOL_FAMILY: 'dev' },
    'doctor',
    '--heal-fingerprint-enoent',
    '--force',
  );
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /healed/);
  assert.match(result.stdout, /converted=1 would-convert=0 refused=0/);
  assert.equal(readlinkSync(f.brokenDirSlot), f.fingerprintHealTarget);
});
