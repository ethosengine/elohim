/**
 * Matthew's workspace follows the epr head it lands.
 *
 * Every scenario owns a temporary device store, rendered launcher, fake process table and git
 * history. The candidates are padded copies of the real epr evaluator selected by EPR_BIN (or the
 * gate's documented build path), while every store transition is driven through the checked-in
 * bin-store CLI. Nothing reaches the persistent device store.
 */
import assert from 'node:assert/strict';
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  appendFileSync,
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

import { After, Given, Then, When } from '@cucumber/cucumber';

import { loadRegistry } from '../../src/framework/session.js';

type JsonObject = Record<string, unknown>;

interface Invocation {
  status: number;
  stdout: string;
  stderr: string;
}

interface FollowFixture {
  base: string;
  repo: string;
  store: string;
  procRoot: string;
  launcher: string;
  wrapper: string;
  commitA: string;
  commitB?: string;
  cidA?: string;
  cidB?: string;
  candidateA?: string;
  candidateB?: string;
  sessionEnv?: NodeJS.ProcessEnv;
  launchedCid?: string;
  last?: Invocation;
  daemon?: ChildProcess;
  daemonEnv?: NodeJS.ProcessEnv;
  bootstrapEnv?: NodeJS.ProcessEnv;
}

const fixtures = new WeakMap<object, FollowFixture>();
const repository = resolve('../..');
const registrySource = join(
  repository,
  'genesis/orchestrator/manifests/runtime/workspace-device.manifest.json'
);
// This reviewed gate path is only a read source; each scenario copies it into a private mkdtemp.
// eslint-disable-next-line sonarjs/publicly-writable-directories
const documentedGateBuild = '/tmp/eprfs-gate-target/debug/epr';
const candidateSource = process.env.EPR_BIN ?? documentedGateBuild;
const storeCli = candidateSource;
const launcherSource = process.env.BIN_LAUNCH_BIN ?? join(dirname(candidateSource), 'bin-launch');
const scratchBase = process.env.A2O_TMPDIR ?? tmpdir();

function fixture(world: object): FollowFixture {
  const found = fixtures.get(world);
  assert.ok(found, 'the isolated epr fixture must exist — the Background creates it');
  return found;
}

function object(value: unknown): JsonObject {
  assert.ok(value && typeof value === 'object' && !Array.isArray(value), 'expected a JSON object');
  return value as JsonObject;
}

function run(
  state: FollowFixture,
  argv: string[],
  env: NodeJS.ProcessEnv = {},
  cwd = state.repo
): Invocation {
  const result = spawnSync(state.wrapper, argv, {
    cwd,
    env: {
      ...process.env,
      ...env,
      EPR_PROC_ROOT: state.procRoot,
      EPR_FIXTURE_OWNER_PID: String(process.pid),
    },
    encoding: 'utf8',
    timeout: 75_000,
    maxBuffer: 8 * 1024 * 1024,
  });
  return {
    status: result.status ?? -1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? String(result.error ?? ''),
  };
}

function ok(
  state: FollowFixture,
  argv: string[],
  env: NodeJS.ProcessEnv = {},
  cwd = state.repo
): Invocation {
  const result = run(state, argv, env, cwd);
  assert.equal(result.status, 0, `${argv.join(' ')}\n${result.stdout}\n${result.stderr}`);
  return result;
}

function git(state: FollowFixture, args: string[]): string {
  return ok(state, ['git', '-c', 'core.hooksPath=/dev/null', ...args], {
    HOME: join(state.base, 'home'),
  }).stdout.trim();
}

function commit(state: FollowFixture, message: string): string {
  git(state, [
    'add',
    'source.txt',
    ...(existsSync(join(state.repo, '.epr-meta')) ? ['.epr-meta'] : []),
    ...(existsSync(join(state.repo, 'genesis')) ? ['genesis'] : []),
  ]);
  git(state, [
    '-c',
    'user.name=Epr Follow Fixture',
    '-c',
    'user.email=epr-follow@example.invalid',
    'commit',
    '-q',
    '-m',
    message,
  ]);
  return git(state, ['rev-parse', 'HEAD']);
}

function storeEnv(state: FollowFixture, extra: NodeJS.ProcessEnv = {}): NodeJS.ProcessEnv {
  return {
    EPR_STORE: state.store,
    BIN_STORE: dirname(state.store),
    ELOHIM_DEVICE_KEY_FILE: join(state.base, 'keys/device.seed'),
    EPR_SESSION_LAZY: undefined,
    ELOHIM_RUNTIME_CONFIG_PATH: undefined,
    ELOHIM_RUNTIME_MANIFEST_PATH: undefined,
    ELOHIM_SESSION_ID: 'head-fixture',
    ELOHIM_SESSION_OWNER_PID: String(process.pid),
    EPR_PROC_ROOT: state.procRoot,
    HOME: join(state.base, 'home'),
    ...extra,
  };
}

function launcherEnv(state: FollowFixture, extra: NodeJS.ProcessEnv = {}): NodeJS.ProcessEnv {
  const env = { ...process.env, ...storeEnv(state), ...extra };
  env.EPR_BIN = '';
  env.EPR_PIN_CID = '';
  env.ELOHIM_SESSION_ID = '';
  for (const runtime of loadRegistry(env).runtimes) {
    for (const key of runtime.sessionEnv) env[key] = '';
  }
  return { ...env, ...extra };
}

function store(state: FollowFixture, args: string[], env: NodeJS.ProcessEnv = {}): Invocation {
  return run(
    state,
    [
      storeCli,
      'bin',
      '--registry',
      registrySource,
      '--tool',
      'epr',
      '--store',
      state.store,
      ...args,
    ],
    storeEnv(state, env)
  );
}

function storeOk(state: FollowFixture, args: string[], env: NodeJS.ProcessEnv = {}): Invocation {
  const result = store(state, args, env);
  assert.equal(result.status, 0, `${args.join(' ')}\n${result.stdout}\n${result.stderr}`);
  return result;
}

function makeCandidate(state: FollowFixture, label: 'A' | 'B'): string {
  assert.ok(existsSync(candidateSource), `real epr candidate is absent: ${candidateSource}`);
  const destination = join(state.base, 'candidates', `epr-${label.toLowerCase()}`);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(candidateSource, destination);
  const candidateLauncher = join(dirname(destination), 'bin-launch');
  copyFileSync(launcherSource, candidateLauncher);
  chmodSync(candidateLauncher, 0o755);
  appendFileSync(candidateLauncher, `\nEPR-A2O-LAUNCHER-${label}\n`);
  appendFileSync(destination, `\nEPR-A2O-BUILD-${label}\n`);
  chmodSync(destination, 0o755);
  const identity = object(JSON.parse(ok(state, [destination, '--version']).stdout));
  assert.equal(identity.id, 'elohim-epr-cli');
  assert.match(String(identity.cid), /^bafkrei[a-z2-7]+$/);
  return destination;
}

function makeWrongCidCandidate(state: FollowFixture): string {
  const destination = join(state.base, 'candidates', 'epr-wrong-cid');
  const counter = join(state.base, 'wrong-cid-version-count');
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(
    destination,
    `#!/usr/bin/env python3
import subprocess
import json
from pathlib import Path
import sys

here = Path(__file__)
counter = Path(${JSON.stringify(counter)})
if sys.argv[1:] == ['--version']:
    count = int(counter.read_text() or '0') + 1 if counter.exists() else 1
    counter.write_text(str(count))
    result = subprocess.run([${JSON.stringify(candidateSource)}, 'bin', 'resolve', '--tool', 'epr', '--registry', ${JSON.stringify(registrySource)}, '--json'], env={**__import__('os').environ, 'EPR_BIN': str(here)}, capture_output=True, text=True, check=True)
    cid = json.loads(result.stdout)['cid']
    if count > 1:
        cid = 'bafkreiaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
    print(json.dumps({'id': 'elohim-epr-cli', 'version': 'wrong-cid-fixture', 'cid': cid}))
elif sys.argv[1:] == ['flow', 'report', 'scope']:
    print('scope: aligned')
else:
    raise SystemExit(2)
`
  );
  chmodSync(destination, 0o755);
  return destination;
}

function ingest(
  state: FollowFixture,
  label: 'A' | 'B',
  sourceCommit: string,
  treeClean: boolean
): string {
  const candidate = makeCandidate(state, label);
  const result = storeOk(state, [
    'ingest',
    candidate,
    '--source-commit',
    sourceCommit,
    '--gate-run',
    `a2o-build-${label.toLowerCase()}`,
    '--tree-clean',
    String(treeClean),
    '--repo',
    state.repo,
  ]);
  const cid = String(object(JSON.parse(result.stdout)).cid);
  if (label === 'A') {
    state.candidateA = candidate;
    state.cidA = cid;
  } else {
    state.candidateB = candidate;
    state.cidB = cid;
  }
  return cid;
}

function land(state: FollowFixture, cid: string, sourceCommit: string, revert = false): Invocation {
  return store(state, [
    'land',
    cid,
    '--commit',
    sourceCommit,
    '--repo',
    state.repo,
    ...(revert ? ['--revert'] : []),
  ]);
}

function landedCid(state: FollowFixture): string {
  const resolved = storeOk(state, ['resolve', '--current']).stdout.trim();
  return basename(dirname(resolved));
}

function nextCleanCommit(state: FollowFixture): string {
  writeFileSync(join(state.repo, 'source.txt'), 'build B\n');
  state.commitB = commit(state, 'source for build B');
  return state.commitB;
}

function versionThroughLauncher(state: FollowFixture, env: NodeJS.ProcessEnv): JsonObject {
  const identity = object(JSON.parse(ok(state, [state.launcher, '--version'], env).stdout));
  const resolved = object(JSON.parse(storeOk(state, ['resolve', '--json'], env).stdout));
  if (resolved.source === 'candidate') return identity;
  const release = object(
    JSON.parse(readFileSync(join(state.store, String(resolved.cid), 'record.json'), 'utf8'))
  );
  assert.equal(
    identity.cid,
    object(release.bins).epr,
    'the executing binary matches the verified release member'
  );
  return { ...identity, cid: resolved.cid };
}

Given("Matthew's workspace has an isolated device directory and source history", function () {
  const base = mkdtempSync(join(scratchBase, 'epr-follows-head-'));
  const repo = join(base, 'repo');
  const storeRoot = join(base, 'device/store/epr');
  const procRoot = join(base, 'proc');
  const bin = join(base, 'device/bin');
  mkdirSync(repo, { recursive: true });
  mkdirSync(procRoot, { recursive: true });
  mkdirSync(join(procRoot, 'self/ns'), { recursive: true });
  mkdirSync(join(procRoot, 'sys/kernel/random'), { recursive: true });
  writeFileSync(join(procRoot, 'self/ns/pid'), 'fixture namespace');
  writeFileSync(join(procRoot, 'sys/kernel/random/boot_id'), 'head-fixture-boot');
  mkdirSync(bin, { recursive: true });
  const state: FollowFixture = {
    base,
    repo,
    store: storeRoot,
    procRoot,
    launcher: join(bin, 'epr'),
    wrapper: join(base, 'fixture-process'),
    commitA: '',
  };
  fixtures.set(this, state);
  // Expose only the fixture owner and its own children. exec retains the child's
  // PID/start time; no session resolver can walk into the host's agent ancestry.
  writeFileSync(
    state.wrapper,
    '#!/bin/sh\nset -eu\nmkdir -p "$EPR_PROC_ROOT/$$" "$EPR_PROC_ROOT/$EPR_FIXTURE_OWNER_PID"\ncp --remove-destination "/proc/$$/stat" "/proc/$$/cmdline" "$EPR_PROC_ROOT/$$/"\ncp --remove-destination "/proc/$EPR_FIXTURE_OWNER_PID/stat" "/proc/$EPR_FIXTURE_OWNER_PID/cmdline" "$EPR_PROC_ROOT/$EPR_FIXTURE_OWNER_PID/"\nexec "$@"\n'
  );
  chmodSync(state.wrapper, 0o755);

  git(state, ['init', '-q', '-b', 'dev']);
  const algorithms = join(repo, '.epr-meta/elohim/algorithms');
  mkdirSync(algorithms, { recursive: true });
  copyFileSync(
    join(repository, '.epr-meta/elohim/algorithms/recall-contract.json'),
    join(algorithms, 'recall-contract.json')
  );
  writeFileSync(join(repo, 'source.txt'), 'build A\n');
  state.commitA = commit(state, 'source for build A');

  copyFileSync(registrySource, join(bin, 'bin-tools.json'));
  copyFileSync(
    join(repository, 'genesis/agentic/agent-runtimes.json'),
    join(bin, 'agent-runtimes.json')
  );
  copyFileSync(launcherSource, state.launcher);
  chmodSync(state.launcher, 0o755);
});

After(async function () {
  const state = fixtures.get(this);
  if (state?.daemon?.exitCode === null) {
    const exited = new Promise<void>(done => state.daemon?.once('exit', () => done()));
    state.daemon.kill('SIGTERM');
    await exited;
  }
  if (state) rmSync(state.base, { recursive: true, force: true });
});

Given('clean build A is the landed epr head', function () {
  const state = fixture(this);
  const cid = ingest(state, 'A', state.commitA, true);
  const result = land(state, cid, state.commitA);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  assert.equal(landedCid(state), cid);
});

When('clean build B lands from the next source commit', function () {
  const state = fixture(this);
  const sourceCommit = nextCleanCommit(state);
  const cid = ingest(state, 'B', sourceCommit, true);
  const result = land(state, cid, sourceCommit);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

Given('Matthew opens a fresh session with no agent runtime above it', function () {
  const state = fixture(this);
  state.sessionEnv = launcherEnv(state, { ELOHIM_SESSION_ID: 'fresh-session-b' });
  const identity = object(
    JSON.parse(ok(state, [storeCli, 'session', 'id', '--json'], state.sessionEnv).stdout)
  );
  assert.equal(identity.source, 'declared');
  assert.equal(identity.session, 'fresh-session-b');
});

Then("epr through the launcher reports build B's CID", function () {
  const state = fixture(this);
  assert.ok(state.sessionEnv);
  assert.equal(versionThroughLauncher(state, state.sessionEnv).cid, state.cidB);
});

Then("the store resolves build B's CID for that session", function () {
  const state = fixture(this);
  assert.ok(state.sessionEnv);
  const resolved = storeOk(state, ['resolve'], state.sessionEnv).stdout.trim();
  assert.equal(basename(dirname(resolved)), state.cidB);
});

Given("Matthew's open session has run epr through the launcher", function () {
  const state = fixture(this);
  state.sessionEnv = launcherEnv(state, { ELOHIM_SESSION_ID: 'open-session-a' });
  assert.equal(versionThroughLauncher(state, state.sessionEnv).cid, state.cidA);
});

Then("the open session still runs build A's CID", function () {
  const state = fixture(this);
  assert.ok(state.sessionEnv);
  assert.equal(versionThroughLauncher(state, state.sessionEnv).cid, state.cidA);
});

Then('the store reports that session behind the head', function () {
  const state = fixture(this);
  assert.ok(state.sessionEnv);
  const status = object(JSON.parse(storeOk(state, ['status', '--json'], state.sessionEnv).stdout));
  assert.equal(status.sessionCid, state.cidA);
  assert.equal(status.head, state.cidB);
  assert.equal(status.behind, true);
});

Then('the open session receives one notice naming the landed release', function () {
  const state = fixture(this);
  assert.ok(state.sessionEnv);
  const settings = JSON.parse(readFileSync(resolve('../../.claude/settings.json'), 'utf8'));
  const hook = settings.hooks.UserPromptSubmit.flatMap(
    (entry: { hooks: { command: string }[] }) => entry.hooks
  ).find((entry: { command: string }) => entry.command.includes('device-head-notice.py')).command;
  const hookEnv = { ...state.sessionEnv, CLAUDE_PROJECT_DIR: resolve('../..') };
  const first = ok(state, ['sh', '-c', hook], hookEnv).stdout;
  assert.ok(first.includes(state.cidA ?? '') && first.includes(state.cidB ?? ''), first);
  assert.match(first, /Switch: \/clear or epr bin pin/);
  storeOk(state, ['gc'], state.sessionEnv);
  assert.equal(ok(state, ['sh', '-c', hook], hookEnv).stdout, '');
  assert.equal(versionThroughLauncher(state, state.sessionEnv).cid, state.cidA);
});

When('Matthew explicitly pins build B for that session', function () {
  const state = fixture(this);
  assert.ok(state.sessionEnv && state.cidB);
  storeOk(state, ['pin', '--cid', state.cidB], state.sessionEnv);
});

When('build B is ingested from a clean commit whose health check fails', function () {
  const state = fixture(this);
  const healthRoot = join(state.repo, 'genesis/docs/superpowers/specs');
  mkdirSync(healthRoot, { recursive: true });
  ok(state, ['mkfifo', join(state.repo, '.git/health-check-blocked')]);
  symlinkSync('../../../../.git/health-check-blocked', join(healthRoot, 'health-check.md'));
  const sourceCommit = nextCleanCommit(state);
  ingest(state, 'B', sourceCommit, true);
});

When(
  'build B is ingested from a clean commit but reports the wrong CID during health',
  function () {
    const state = fixture(this);
    const sourceCommit = nextCleanCommit(state);
    const candidate = makeWrongCidCandidate(state);
    const result = storeOk(state, [
      'ingest',
      candidate,
      '--source-commit',
      sourceCommit,
      '--gate-run',
      'a2o-build-wrong-cid',
      '--tree-clean',
      'true',
      '--repo',
      state.repo,
    ]);
    state.candidateB = candidate;
    state.cidB = String(object(JSON.parse(result.stdout)).cid);
  }
);

When('Matthew tries to land build B', { timeout: 80_000 }, function () {
  const state = fixture(this);
  assert.ok(state.cidB && state.commitB);
  state.last = land(state, state.cidB, state.commitB);
});

Then('the landing is refused as {string}', function (kind: string) {
  const result = fixture(this).last;
  assert.ok(result, 'a landing attempt must have run');
  assert.equal(result.status, 2, `${result.stdout}\n${result.stderr}`);
  assert.match(result.stderr, new RegExp(`REFUSED ${kind}:`));
});

Then('build A remains the landed epr head', function () {
  const state = fixture(this);
  assert.equal(landedCid(state), state.cidA);
});

When("build B is ingested from Matthew's dirty working tree", function () {
  const state = fixture(this);
  const sourceCommit = nextCleanCommit(state);
  writeFileSync(join(state.repo, 'source.txt'), 'unfinished build B work\n');
  ingest(state, 'B', sourceCommit, false);
});

Then("a fresh unpinned session runs build A's CID", function () {
  const state = fixture(this);
  state.sessionEnv = launcherEnv(state, { ELOHIM_SESSION_ID: 'dirty-build-unpinned' });
  assert.equal(versionThroughLauncher(state, state.sessionEnv).cid, state.cidA);
});

When('Matthew launches build B with {string}', function (variable: string) {
  const state = fixture(this);
  assert.equal(variable, 'EPR_PIN_CID');
  assert.ok(state.cidB);
  const env = launcherEnv(state, { EPR_PIN_CID: state.cidB });
  state.launchedCid = String(versionThroughLauncher(state, env).cid);
});

Then("the launcher reports build B's CID", function () {
  const state = fixture(this);
  assert.equal(state.launchedCid, state.cidB);
});

Then("that session's own launcher resolution is still build A's CID", function () {
  const state = fixture(this);
  assert.ok(state.sessionEnv);
  assert.equal(versionThroughLauncher(state, state.sessionEnv).cid, state.cidA);
});

When("Matthew checks out build A's older source commit", function () {
  const state = fixture(this);
  git(state, ['checkout', '-q', '--detach', state.commitA]);
});

When('Matthew tries to land the stored build A', function () {
  const state = fixture(this);
  assert.ok(state.cidA);
  state.last = land(state, state.cidA, state.commitA);
});

When('Matthew promotes the stored build A as head with --revert', function () {
  const state = fixture(this);
  assert.ok(state.cidA);
  state.last = land(state, state.cidA, state.commitA, true);
});

Then('the revert succeeds and build A is the landed epr head again', function () {
  const state = fixture(this);
  assert.ok(state.last);
  assert.equal(state.last.status, 0, `${state.last.stdout}\n${state.last.stderr}`);
  assert.equal(object(JSON.parse(state.last.stdout)).ring, 'reverted');
  assert.equal(landedCid(state), state.cidA);
});

Given(
  "instead Matthew's device has a legacy sha256 store and a launcher without the bin verb",
  function () {
    const state = fixture(this);
    const target = join(state.base, 'cargo-target');
    const mockBin = join(state.base, 'bootstrap-bin');
    mkdirSync(mockBin, { recursive: true });
    mkdirSync(join(state.repo, 'elohim/eprfs'), { recursive: true });
    const manifest = object(
      JSON.parse(readFileSync(join(repository, 'elohim/eprfs/build-manifest.json'), 'utf8'))
    );
    const cargo = object(object(object(object(object(manifest.gate).projects).eprfs).run).cargo);
    cargo.targetDir = target;
    cargo.env = { RUSTUP_TOOLCHAIN: '1.96.1' };
    writeFileSync(join(state.repo, 'elohim/eprfs/build-manifest.json'), JSON.stringify(manifest));
    writeFileSync(join(state.repo, '.gitignore'), 'genesis/local-dev/\n.eprfs/\n');
    git(state, ['add', '.gitignore', 'elohim/eprfs/build-manifest.json']);
    state.commitA = commit(state, 'bootstrap fixture inputs');
    // Compile is isolated at the cargo boundary; both installed binaries are real offline gate builds.
    writeFileSync(
      join(mockBin, 'cargo'),
      `#!/usr/bin/env python3
import os, pathlib, shutil, sys
assert sys.argv[1:] == ['build', '--manifest-path', 'elohim/eprfs/Cargo.toml', '-p', 'elohim-epr-cli', '-p', 'elohim-device-runtime', '--bins']
assert os.environ['RUSTUP_TOOLCHAIN'] == '1.96.1'
assert os.environ['RUSTFLAGS'] == ''
out = pathlib.Path(os.environ['CARGO_TARGET_DIR']) / 'debug'
out.mkdir(parents=True, exist_ok=True)
shutil.copy2(${JSON.stringify(candidateSource)}, out / 'epr')
shutil.copy2(${JSON.stringify(launcherSource)}, out / 'bin-launch')
`
    );
    chmodSync(join(mockBin, 'cargo'), 0o755);
    const legacy = '#!/bin/sh\necho "unknown command bin" >&2\nexit 2\n';
    const sha = 'sha256:' + createHash('sha256').update(legacy).digest('hex');
    const old = join(state.store, sha);
    mkdirSync(old, { recursive: true });
    writeFileSync(join(old, 'epr'), legacy);
    chmodSync(join(old, 'epr'), 0o755);
    writeFileSync(
      join(old, 'record.json'),
      JSON.stringify({
        cid: sha,
        rcid: sha,
        tool: 'epr',
        bins: { epr: sha },
        version: 'legacy',
        source_commit: state.commitA,
        tree_clean: true,
        gate_run: 'fixture',
        ingested_at: '2026-10-23T00:00:00Z',
      })
    );
    symlinkSync(sha, join(state.store, 'current'));
    writeFileSync(
      join(state.store, 'current.json'),
      JSON.stringify({ cid: sha, commit: state.commitA })
    );
    const managed = join(dirname(state.store), 'bin');
    mkdirSync(managed, { recursive: true });
    writeFileSync(join(managed, 'epr-launch'), legacy);
    chmodSync(join(managed, 'epr-launch'), 0o755);
    symlinkSync(join(managed, 'epr-launch'), join(managed, 'epr'));
    rmSync(state.launcher);
    symlinkSync(join(managed, 'epr-launch'), state.launcher);
    state.bootstrapEnv = launcherEnv(state, {
      PATH: `${mockBin}:${dirname(state.launcher)}:${process.env.PATH}`,
      CARGO_HOME: dirname(dirname(state.launcher)),
      EPR_BIN_TOOLS: registrySource,
      CARGO_NET_OFFLINE: 'true',
      GIT_CEILING_DIRECTORIES: '/tmp',
    });
    // Use the projected hook bytes with the fixture checkout as its root.
    mkdirSync(join(state.repo, '.claude/hooks'), { recursive: true });
    copyFileSync(
      join(repository, '.claude/hooks/device-arrive.sh'),
      join(state.repo, '.claude/hooks/device-arrive.sh')
    );
    mkdirSync(join(state.repo, 'genesis/agentic'), { recursive: true });
    writeFileSync(
      join(state.repo, 'genesis/agentic/native-epr-bootstrap.mjs'),
      `import ${JSON.stringify(join(repository, 'genesis/agentic/native-epr-bootstrap.mjs'))};\n`
    );
    git(state, [
      'add',
      '.claude/hooks/device-arrive.sh',
      'genesis/agentic/native-epr-bootstrap.mjs',
    ]);
    commit(state, 'projected arrival hook');
  }
);

When("the device's start hook queues a native bootstrap", function () {
  const state = fixture(this);
  const start = Date.now();
  ok(state, ['sh', join(state.repo, '.claude/hooks/device-arrive.sh')], state.bootstrapEnv);
  assert.ok(Date.now() - start < 3000, 'hook returns without waiting for a build');
});

Then(
  'the detached bootstrap lands a native release CID and replaces the legacy launcher',
  { timeout: 30000 },
  async function () {
    const state = fixture(this);
    const log = join(state.repo, 'genesis/local-dev/native-epr-bootstrap/bootstrap.log');
    let body = '';
    for (let n = 0; n < 100; n++) {
      body = existsSync(log) ? readFileSync(log, 'utf8') : '';
      if (body.includes('native evaluator ready:')) break;
      if (body.includes('native evaluator bootstrap:')) assert.fail(body);
      await delay(100);
    }
    assert.match(body, /native evaluator ready: bafyrei/, body);
    assert.match(landedCid(state), /^bafyrei/);

    assert.equal(
      readFileSync(join(dirname(state.store), 'bin/bin-launch')).equals(
        readFileSync(launcherSource)
      ),
      true
    );
  }
);

Then('the replacement launcher runs the landed native evaluator', function () {
  const state = fixture(this);
  assert.equal(versionThroughLauncher(state, state.bootstrapEnv ?? {}).cid, landedCid(state));
});

Given(
  'the gate stored paired build B before an older arrival landed single-file build A from that same commit',
  function () {
    const state = fixture(this);
    ingest(state, 'B', state.commitA, true);
    const legacy = JSON.parse(readFileSync(registrySource, 'utf8')) as {
      tools: { epr: { bins: string[] } };
    };
    legacy.tools.epr.bins = ['epr'];
    const legacyPath = join(state.base, 'old-manifest.json');
    writeFileSync(legacyPath, JSON.stringify(legacy));
    const call = (args: string[]) =>
      ok(
        state,
        [
          storeCli,
          'bin',
          '--registry',
          legacyPath,
          '--tool',
          'epr',
          '--store',
          state.store,
          ...args,
        ],
        storeEnv(state)
      );
    const candidate = makeCandidate(state, 'A');
    state.cidA = String(
      object(
        JSON.parse(
          call(['ingest', candidate, '--source-commit', state.commitA, '--tree-clean', 'true'])
            .stdout
        )
      ).cid
    );
    call(['land', state.cidA, '--commit', state.commitA, '--repo', state.repo]);
    // Model an installed launcher link, rather than the standalone launcher copy
    // used by scenarios that never exercise installation's plain-binary adoption.
    const managedLauncher = join(dirname(state.store), 'bin/bin-launch');
    mkdirSync(dirname(managedLauncher), { recursive: true });
    copyFileSync(launcherSource, managedLauncher);
    chmodSync(managedLauncher, 0o755);
    appendFileSync(managedLauncher, '\nold launcher bytes\n');
    rmSync(state.launcher);
    symlinkSync(managedLauncher, state.launcher);
    // A historical current remains executable but cannot supply an unrelated launcher.
    const refused = store(state, [
      'install-launcher',
      '--destination',
      dirname(state.launcher),
      '--repo',
      state.repo,
    ]);
    assert.equal(refused.status, 2);
    assert.match(refused.stderr, /launcher-absent/);
    assert.ok(readFileSync(managedLauncher).includes(Buffer.from('old launcher bytes')));
    // Separate the two fixture arrival times even on filesystems with coarse timestamps.
    const recordPath = join(state.store, state.cidA, 'record.json');
    const record = object(JSON.parse(readFileSync(recordPath, 'utf8')));
    record.ingestedAt = '2099-01-01T00:00:00Z';
    writeFileSync(recordPath, JSON.stringify(record));
  }
);

When('the device applies and follows its declared epr inventory', function () {
  const state = fixture(this);
  ok(state, [storeCli, 'device', 'follow', 'bin:private', '--mode', 'observe'], storeEnv(state));
  storeOk(state, ['follow', '--repo', state.repo]);
  const registry = join(dirname(state.store), 'bin/bin-tools.json');
  const before = readFileSync(registry, 'utf8');
  ok(
    state,
    [
      storeCli,
      'bin',
      '--tool',
      'epr',
      '--store',
      state.store,
      'install-launcher',
      '--destination',
      dirname(state.launcher),
      '--repo',
      state.repo,
    ],
    storeEnv(state)
  );
  assert.equal(readFileSync(registry, 'utf8'), before);
  assert.equal(readFileSync(`${registry}.owner`, 'utf8'), join(state.base, 'keys/manifest'));
  const declared = object(
    JSON.parse(ok(state, [storeCli, 'device', 'show', '--json'], storeEnv(state)).stdout)
  );
  assert.ok(JSON.stringify(declared).includes('bin:private'));
});

Then('the device lands the paired release and installs its launcher', function () {
  const state = fixture(this);
  assert.equal(landedCid(state), state.cidB);
  assert.ok(state.cidB);
  assert.notDeepEqual(
    readFileSync(join(state.store, state.cidB, 'bin-launch')),
    readFileSync(launcherSource)
  );
  assert.deepEqual(
    readFileSync(join(dirname(state.store), 'bin/bin-launch')),
    readFileSync(join(state.store, state.cidB, 'bin-launch'))
  );
});

When('that pinned session starts the peer daemon', { timeout: 30000 }, async function () {
  const state = fixture(this);
  assert.ok(state.sessionEnv);
  appendFileSync(join(state.repo, '.git/info/exclude'), '\n.eprfs/\n');
  state.daemonEnv = {
    ...state.sessionEnv,
    EPR_PROC_ROOT: state.procRoot,
    EPR_BIN_TOOLS: registrySource,
    ELOHIM_DEVICE_KEY_FILE: join(state.base, 'keys/seed'),
    XDG_CONFIG_HOME: join(state.base, 'config'),
    GIT_CEILING_DIRECTORIES: '/tmp',
    EPR_PEER_RELAY_URL: '',
    EPR_PIN_CID: state.cidA,
    EPR_FIXTURE_OWNER_PID: String(process.pid),
  };
  const native = (args: string[]) =>
    ok(state, [storeCli, ...args, '--root', state.repo], state.daemonEnv);
  native([
    'actor',
    'witness',
    '--subject',
    'human:matthew',
    '--as',
    'agent:fixture@a2o',
    '--session',
    'fixture',
    '--basis',
    'fixture operator present',
  ]);
  const id = ok(state, [storeCli, 'peer', 'id'], state.daemonEnv).stdout.trim();
  native(['actor', 'device', 'endpoint', '--node-id', id]);
  const peerPolicy = join(state.base, 'peer-policy.json');
  writeFileSync(peerPolicy, JSON.stringify({ berth: { channel: { crosses: {} } } }));
  state.daemon = spawn(
    state.wrapper,
    [
      state.launcher,
      'peer',
      'serve',
      '--root',
      state.repo,
      '--listen',
      '127.0.0.1:0',
      '--policy',
      peerPolicy,
    ],
    {
      cwd: state.repo,
      env: state.daemonEnv,
      stdio: ['ignore', 'ignore', 'pipe'],
    }
  );
  let errors = '';
  state.daemon.stderr?.on('data', bytes => {
    errors += String(bytes);
  });
  for (let n = 0; n < 100; n++) {
    assert.equal(state.daemon.exitCode, null, errors);
    const leases = readdirSync(join(state.store, 'sessions')).filter(name =>
      name.startsWith('daemon-')
    );
    if (
      leases.some(
        name =>
          object(JSON.parse(readFileSync(join(state.store, 'sessions', name), 'utf8')))
            .runningCid === state.cidB
      )
    )
      return;
    await delay(100);
  }
  assert.fail('daemon did not record its running CID: ' + errors);
});

Then("the peer daemon's lease and status name build B as its running build", function () {
  const state = fixture(this);
  const leases = readdirSync(join(state.store, 'sessions')).filter(name =>
    name.startsWith('daemon-')
  );
  const lease = object(JSON.parse(readFileSync(join(state.store, 'sessions', leases[0]), 'utf8')));
  assert.equal(lease.cid, state.cidB);
  assert.equal(lease.runningCid, state.cidB);
  assert.equal(lease.ownerPid, state.daemon?.pid);
  const status = object(
    JSON.parse(
      ok(state, [state.launcher, 'peer', 'status', '--root', state.repo, '--json'], {
        ...state.daemonEnv,
        EPR_PIN_CID: '',
      }).stdout
    )
  );
  assert.equal(status.cliCid, state.cidA);
  assert.deepEqual(status.daemonCids, [state.cidB]);
  assert.equal(status.daemonDiffers, true);
});

When(
  'the head is reverted to build A and build B loses previous-head and 14-day history protection',
  function () {
    const state = fixture(this);
    // Make B rely solely on its live lease: no head, previous head, or grace-period retention.
    git(state, ['checkout', '-q', '--detach', state.commitA]);
    assert.ok(state.cidA);
    const reverted = land(state, state.cidA, state.commitA, true);
    assert.equal(reverted.status, 0, reverted.stderr);
    rmSync(join(state.store, 'previous'), { force: true });
    const currentPath = join(state.store, 'current.json');
    const current = object(JSON.parse(readFileSync(currentPath, 'utf8')));
    delete current.previous;
    writeFileSync(currentPath, JSON.stringify(current));
    for (const cid of [state.cidA, state.cidB]) {
      assert.ok(cid);
      const path = join(state.store, cid, 'record.json');
      const record = object(JSON.parse(readFileSync(path, 'utf8')));
      record.ingestedAt = '2000-01-01T00:00:00Z';
      writeFileSync(path, JSON.stringify(record));
      rmSync(join(state.store, cid, '.used'), { force: true });
    }
  }
);

When('garbage collection runs while the peer daemon lives', function () {
  const state = fixture(this);
  state.last = storeOk(state, ['gc']);
});

Then("build B remains in the store, held by the daemon's lease", function () {
  const state = fixture(this);
  const result = object(JSON.parse(state.last?.stdout ?? '{}'));
  assert.ok(Array.isArray(result.kept));
  assert.ok(result.kept.includes(state.cidB));
  assert.ok(existsSync(join(state.store, String(state.cidB), 'epr')));
  // The lease is the reason it was kept: a live daemon's record still names build B.
  const leases = readdirSync(join(state.store, 'sessions')).filter(name =>
    name.startsWith('daemon-')
  );
  assert.ok(
    leases.some(name => {
      const lease = object(JSON.parse(readFileSync(join(state.store, 'sessions', name), 'utf8')));
      return lease.cid === state.cidB && lease.ownerPid === state.daemon?.pid;
    }),
    'no live daemon lease names build B'
  );
});

When('build B is ingested from a clean unpublished feature branch', function () {
  const state = fixture(this);
  git(state, ['checkout', '-qb', 'unpublished-feature']);
  const sourceCommit = nextCleanCommit(state);
  ingest(state, 'B', sourceCommit, true);
});

When('Matthew tries to revert to stored build B', function () {
  const state = fixture(this);
  assert.ok(state.cidB && state.commitB);
  state.last = land(state, state.cidB, state.commitB, true);
});
