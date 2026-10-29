// brit-helper verify/plan/build-ref remain advisory; attestation belongs to the device.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';

function scratch(prefix) {
  const parent = realpathSync(tmpdir());
  const root = realpathSync(resolve(import.meta.dirname, '../..'));
  const rel = relative(root, parent);
  if (!rel || (rel !== '..' && !rel.startsWith(`..${sep}`))) {
    throw new Error('REFUSED: set TMPDIR outside the tree');
  }
  const dir = mkdtempSync(join(parent, prefix));
  scratchDirs.push(dir);
  return dir;
}
const scratchDirs = [];
process.on('exit', () => { for (const dir of scratchDirs) rmSync(dir, { recursive: true, force: true }); });
import { join, relative, resolve, sep } from 'node:path';


const HELPER = join(import.meta.dirname, 'scripts', 'brit-helper.sh');
const MANIFEST = Buffer.from('{"pipeline":"elohim"}\n');

const graph = {
  commitSha: 'c0ffee',
  executionOrder: ['elohim-edge', 'elohim', 'elohim-genesis', 'elohim-sophia', 'elohim-steward'],
  results: {
    'elohim-edge': { result: 'SUCCESS', duration: 1_800_000, buildNumber: 90, url: 'u/edge/90' },
    elohim: { result: 'FAILURE', duration: 7_200_000.4, buildNumber: 1725, url: 'u/app/1725' },
    'elohim-genesis': { result: 'DISPATCHED', duration: 0 },
    'elohim-sophia': { result: 'UNSTABLE', duration: 60_000 },
  },
};
function fixtureGitEnv(home, inherited = process.env) {
  return {
    ...Object.fromEntries(Object.entries(inherited).filter(([key]) => !key.startsWith('GIT_'))),
    HOME: home,
    GIT_CONFIG_GLOBAL: join(home, '.gitconfig-fixture-disabled'),
    GIT_CONFIG_NOSYSTEM: '1',
  };
}

function fixtureGit(cwd, args, inherited = process.env) {
  return execFileSync('git', args, { cwd, env: fixtureGitEnv(cwd, inherited) }).toString().trim();
}

/** A git repo with one commit and one pipeline manifest, as a CI workspace would hold. */
function workspace(inherited = process.env) {
  const dir = scratch('brit-helper-');
  const git = (...args) => fixtureGit(dir, args, inherited);
  git('init', '-q', '.');
  mkdirSync(join(dir, 'app'));
  writeFileSync(join(dir, 'app', 'build-manifest.json'), MANIFEST);
  mkdirSync(join(dir, 'edge'));
  writeFileSync(join(dir, 'edge', 'build-manifest.json'), '{"pipeline":"elohim-edge"}\n');
  git('add', '.');
  git('-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '-m', 'c');
  const sha = git('rev-parse', 'HEAD');
  const graphPath = join(dir, 'actual-build-graph.json');
  writeFileSync(graphPath, JSON.stringify({ ...graph, commitSha: sha }));
  return { dir, sha, graphPath, git };
}

test('fixture Git ignores hostile inherited repository and config paths', () => {
  const decoy = scratch('brit-helper-decoy-');
  fixtureGit(decoy, ['init', '-q', '.']);
  const hostile = {
    ...process.env,
    GIT_DIR: join(decoy, '.git'),
    GIT_WORK_TREE: decoy,
    GIT_INDEX_FILE: join(decoy, '.git', 'index'),
    GIT_CONFIG_GLOBAL: join(decoy, 'hostile-global-config'),
    GIT_CONFIG_NOSYSTEM: '0',
  };
  const { dir, sha, git } = workspace(hostile);
  assert.equal(git('rev-parse', '--show-toplevel'), dir);
  assert.equal(git('rev-parse', 'HEAD'), sha);
  const decoyHead = spawnSync('git', ['rev-parse', '--verify', 'HEAD'], {
    cwd: decoy,
    encoding: 'utf8',
    env: fixtureGitEnv(decoy),
  });
  assert.notEqual(decoyHead.status, 0, 'hostile inherited Git state must not receive the fixture commit');
});

function helper(args, env) {
  return spawnSync('sh', [HELPER, ...args], {
    encoding: 'utf8',
    env: {
      PATH: process.env.PATH,
      HOME: env.REPO_ROOT,
      GIT_CONFIG_GLOBAL: join(env.REPO_ROOT, '.gitconfig-fixture-disabled'),
      GIT_CONFIG_NOSYSTEM: '1',
      ...env,
    },
  });
}

function stub(dir, name, body) {
  const path = join(dir, name);
  writeFileSync(path, `#!/bin/sh\n${body}\n`);
  chmodSync(path, 0o755);
  return path;
}

test('explicit BRIT_BIN selects unified build plan, never older PATH brit or rakia', () => {
  const dir = scratch('brit-helper-plan-');
  const log = join(dir, 'argv.log');
  stub(dir, 'brit', `echo old-brit >> '${log}'`);
  stub(dir, 'rakia', `echo old-rakia >> '${log}'`);
  const unified = stub(dir, 'unified-brit', `printf 'unified:%s\\n' "$*" >> '${log}'`);
  const r = helper(['plan', '--since', 'origin/dev'], {
    BRIT_BIN: unified,
    PATH: `${dir}:/usr/bin:/bin`,
    REPO_ROOT: dir,
  });
  assert.equal(r.status, 0, r.stderr);
  assert.deepEqual(readFileSync(log, 'utf8').trim().split('\n'), [
    'unified:build plan --help',
    'unified:build plan --since origin/dev',
  ]);
});

test('brit verify is bounded by BRIT_ADVISORY_TIMEOUT_S and a timeout stays a warn, exit 0', () => {
  const dir = mkdtempSync(join(tmpdir(), 'brit-helper-timeout-'));
  const slow = stub(dir, 'brit', 'sleep 5; echo should-not-finish');
  const env = { BRIT_BIN: slow, BRIT_ADVISORY_TIMEOUT_S: '1', PATH: `${dir}:/usr/bin:/bin`, REPO_ROOT: dir };
  const started = Date.now();
  const verify = helper(['verify'], env);
  assert.equal(verify.status, 0, verify.stderr);
  assert.match(verify.stderr, /brit verify timed out after 1s — advisory only/);
  assert.ok(Date.now() - started < 4000, 'the helper returned at the timeout, not after the slow verify');
});

test('invalid explicit BRIT_BIN never falls back to old tools or blocks build-ref', () => {
  const dir = scratch('brit-helper-invalid-');
  const log = join(dir, 'argv.log');
  stub(dir, 'brit', `echo old-brit >> '${log}'`);
  stub(dir, 'rakia', `echo old-rakia >> '${log}'`);
  const buildRef = stub(dir, 'build-ref', `echo build-ref >> '${log}'`);
  const env = {
    BRIT_BIN: join(dir, 'missing-brit'),
    BRIT_BUILD_REF_BIN: buildRef,
    PATH: `${dir}:/usr/bin:/bin`,
    REPO_ROOT: dir,
  };
  const plan = helper(['plan', '--since', 'origin/dev'], env);
  assert.equal(plan.status, 0);
  assert.match(plan.stderr, /explicit BRIT_BIN is not executable; plan advisory skipped/);
  const verify = helper(['verify'], env);
  assert.equal(verify.status, 0);
  assert.match(verify.stderr, /explicit BRIT_BIN is not executable; repository-integrity advisory skipped/);
  const independent = helper(['build-ref', 'build', 'put'], env);
  assert.equal(independent.status, 0);
  assert.equal(readFileSync(log, 'utf8').trim(), 'build-ref');
});

test('a brit without `build plan` stays advisory and never falls back to a rakia binary', () => {
  // `rakia` on PATH is now the bin-store launcher alias for the same pinned brit, so there is no
  // older plan engine to fall back to: an old brit is reported, and nothing else is executed.
  const dir = scratch('brit-helper-no-plan-');
  const log = join(dir, 'argv.log');
  stub(dir, 'brit', 'exit 2'); // An old brit with no `build plan` namespace.
  stub(dir, 'rakia', `printf 'rakia:%s\\n' "$*" >> '${log}'`);
  const env = { PATH: `${dir}:/usr/bin:/bin`, REPO_ROOT: dir };
  const plan = helper(['plan', '--since', 'origin/dev'], env);
  assert.equal(plan.status, 0, plan.stderr);
  assert.equal(existsSync(log), false, 'rakia must never be executed as a plan fallback');
});

test('a failing unified plan remains warn-only, and pre-push supplies --since', () => {
  const dir = scratch('brit-helper-failure-');
  const unified = stub(dir, 'unified-brit', '[ "$3" = "--help" ] && exit 0\nexit 17');
  const r = helper(['plan', '--since', 'origin/dev'], {
    BRIT_BIN: unified,
    PATH: `${dir}:/usr/bin:/bin`,
    REPO_ROOT: dir,
  });
  assert.equal(r.status, 0);
  assert.match(r.stderr, /brit build plan exited 17 — advisory only/);
  const hook = readFileSync(join(import.meta.dirname, '../../.husky/pre-push.bash'), 'utf8');
  assert.match(hook, /"\$HELPER" plan --since origin\/dev \|\| true/);
  assert.doesNotMatch(hook, /"\$HELPER" plan --target origin\/dev/);
});

