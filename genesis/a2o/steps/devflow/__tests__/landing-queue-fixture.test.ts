import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  accessSync,
  constants,
  existsSync,
  mkdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';
import { afterEach, test } from 'node:test';
import { pathToFileURL } from 'node:url';

import {
  type Device,
  git,
  must,
  repository,
  runSync,
  seedRepository,
} from '../devices-share-one-berth.fixture.js';
import { ENTRIES, orchestratorClosure, seedGateClosure, stubTools } from '../gate-closure.seed.js';
import {
  installLandingHooks,
  landingHousehold,
  pathWithout,
  seedLanding,
} from '../landing-queue.fixture.js';

const bases: string[] = [];

interface Marker {
  re: RegExp;
  format: (...args: string[]) => string;
}

afterEach(() => {
  while (bases.length > 0) rmSync(bases.pop() as string, { recursive: true, force: true });
});

function seeded(options: Parameters<typeof seedRepository>[0] = {}) {
  const value = seedRepository(options);
  bases.push(value.base);
  return value;
}

function fakeDevice(repo: string): Device {
  return {
    label: 'A',
    dir: repo,
    config: join(repo, 'config'),
    berthDir: join(repo, 'berth'),
    repo,
    log: join(repo, 'serve.log'),
    did: 'did:key:fixture',
    node: 'fixture',
    port: 1,
    daemonStarts: 0,
  };
}

void test('seedRepository with no options seeds exactly the berth files', () => {
  const f = seeded();
  assert.match(f.base, /\/devices-share-one-berth-[^/]+$/);
  const files = must(git(f.seed, ['ls-tree', '-r', '--name-only', 'dev']), 'list seed tree')
    .stdout.trim()
    .split('\n');
  assert.deepEqual(files, [
    '.ci-ignore',
    '.epr-meta/elohim/algorithms/recall-contract.json',
    '.epr-meta/repository.yaml',
    '.gitignore',
    'README.md',
    'genesis/agentic/bin/berth',
    'genesis/orchestrator/ci-ignore.mjs',
    'genesis/orchestrator/manifests/runtime/workspace-device.manifest.json',
    'genesis/orchestrator/rust-style.sh',
  ]);
});

void test("orchestratorClosure covers gate-runner's runtime imports", () => {
  const closure = orchestratorClosure(['genesis/orchestrator/gate-runner.mjs']);
  assert.equal(closure.length, 16);
  for (const path of [
    'genesis/orchestrator/gate-runner.mjs',
    'genesis/orchestrator/manifest-utils.mjs',
    'genesis/orchestrator/pipeline-registry.mjs',
    'genesis/orchestrator/scripts/brit-attest.mjs',
  ]) {
    assert.ok(closure.includes(path), path);
  }
  assert.ok(!closure.some(path => path.includes('gate-source-state.types')));
});

void test('every ENTRIES file exists on this tree', () => {
  for (const path of ENTRIES) {
    assert.ok(existsSync(join(repository, path)), `${path} is absent — S2 must land it first`);
  }
});

void test('the Change-Id shim is installed when present, and its absence names S4', async () => {
  await assert.rejects(
    landingHousehold(),
    /no \.husky\/commit-msg on this tree — the Change-Id shim lands in S4 \(R-G\)/
  );
  const f = seeded({
    seed(seed) {
      seedGateClosure(seed, { reuse: [] });
      writeFileSync(join(seed, '.husky/commit-msg'), '#!/usr/bin/env bash\nexit 0\n');
    },
  });
  installLandingHooks(fakeDevice(f.seed));
  assert.equal(
    must(git(f.seed, ['config', '--get', 'core.hooksPath']), 'read hooks path').stdout.trim(),
    join(f.seed, '.husky')
  );
});

void test("the seeded manifest's only gate project is app", () => {
  const f = seeded({ seed: seed => seedGateClosure(seed, { reuse: ['app'] }) });
  symlinkSync(join(repository, 'node_modules'), join(f.base, 'node_modules'));
  const result = runSync(['node', 'genesis/orchestrator/gate-runner.mjs', '--list'], {
    cwd: f.seed,
    env: { ...process.env, GATE_ROOT: f.seed },
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^app\tapp\tjust:gate\s*$/);
});

void test("the seeded pool-policy keeps gate_reuse's shape", () => {
  const f = seeded({
    seed: seed =>
      seedGateClosure(seed, {
        reuse: ['app'],
        channel: { max_skew_s: 5 },
      }),
  });
  const policy = JSON.parse(
    must(
      runSync([
        'node',
        '-e',
        "process.stdout.write(require('fs').readFileSync(process.argv[1]))",
        join(f.seed, 'genesis/agentic/pool-policy.json'),
      ]),
      'read policy'
    ).stdout
  );
  assert.equal(typeof policy.gate_reuse._comment, 'string');
  assert.deepEqual(policy.gate_reuse.projects, ['app']);
  assert.equal(policy.berth.channel.max_skew_s, 5);
});

void test('a clone of the seed with a roster, a gate-live log and a node_modules link is clean', () => {
  const f = seeded({ seed: seedLanding });
  const clone = join(f.base, 'clone');
  must(git(f.base, ['clone', '-q', '-b', 'dev', f.origin, clone]), 'clone landing seed');
  mkdirSync(join(clone, '.eprfs/status/participants'), { recursive: true });
  mkdirSync(join(clone, '.eprfs/status/gate-live'), { recursive: true });
  writeFileSync(join(clone, '.eprfs/status/participants/matthew.jsonl'), '{}\n');
  writeFileSync(join(clone, '.eprfs/status/gate-live/r.jsonl'), '{}\n');
  symlinkSync(join(repository, 'node_modules'), join(clone, 'node_modules'));
  assert.equal(git(clone, ['status', '--porcelain']).stdout, '');
});

void test('the stub fingerprint is the app tree oid', () => {
  const f = seeded({ seed: seed => seedGateClosure(seed, { reuse: ['app'] }) });
  const tools = stubTools(f.base);
  const first = must(git(f.seed, ['rev-parse', 'HEAD']), 'first commit').stdout.trim();
  must(git(f.seed, ['commit', '-q', '--allow-empty', '-m', 'same tree']), 'empty commit');
  const second = must(git(f.seed, ['rev-parse', 'HEAD']), 'second commit').stdout.trim();
  const fingerprint = (commit: string) => {
    const result = spawnSync(
      tools.BRIT_BIN,
      [
        'build',
        'fingerprint',
        join(f.seed, 'app/build-manifest.json'),
        '--step',
        'gate-app',
        '--commit',
        commit,
      ],
      { cwd: f.seed, encoding: 'utf8' }
    );
    assert.equal(result.status, 0, result.stderr);
    return JSON.parse(result.stdout).fingerprints[0].fingerprint;
  };
  assert.equal(fingerprint(first), fingerprint(second));
  assert.equal(
    fingerprint(first),
    must(git(f.seed, ['rev-parse', `${first}:app`]), 'app oid').stdout.trim()
  );
});

void test('the stub brit-build-ref answers signer, put, get and find in the parsers shapes', async () => {
  const f = seeded({ seed: seed => seedGateClosure(seed, { reuse: ['app'] }) });
  const tools = stubTools(f.base);
  const evidence = await import(
    pathToFileURL(join(repository, 'genesis/orchestrator/gate-evidence.mjs')).href
  );
  const attest = await import(
    pathToFileURL(join(repository, 'genesis/orchestrator/scripts/brit-attest.mjs')).href
  );
  const signer = evidence.signerIdentity(f.seed, spawnSync, {
    env: process.env,
    buildRefBin: tools.BRIT_BUILD_REF_BIN,
  });
  assert.match(signer.agentId, /^[0-9a-f]{64}$/);
  const commit = must(git(f.seed, ['rev-parse', 'HEAD']), 'head').stdout.trim();
  const put = spawnSync(
    tools.BRIT_BUILD_REF_BIN,
    [
      '--repo',
      f.seed,
      'build',
      'put',
      '--step',
      'gate:app',
      '--inputs-hash',
      'fixture-key',
      '--success',
      'true',
      '--commit',
      commit,
    ],
    { encoding: 'utf8' }
  );
  assert.equal(put.status, 0, put.stderr);
  const project = { name: 'app' };
  const found = evidence.findEvidence(project, 'fixture-key', {
    buildRefBin: tools.BRIT_BUILD_REF_BIN,
    root: f.seed,
  });
  assert.equal(found.rows.length, 1);
  assert.equal(found.rows[0].attestation.success, true);
  assert.equal(
    attest.readBuildNote('gate:app', commit, {
      buildRefBin: tools.BRIT_BUILD_REF_BIN,
      root: f.seed,
    }).state,
    'verified'
  );
  assert.equal(
    attest.readBuildNote('gate:app', '0'.repeat(40), {
      buildRefBin: tools.BRIT_BUILD_REF_BIN,
      root: f.seed,
    }).state,
    'absent'
  );
});

void test('the stub rakia parses through rakiaAffected', async () => {
  const f = seeded();
  const tools = stubTools(f.base);
  const oracle = await import(
    pathToFileURL(join(repository, 'genesis/orchestrator/gate-oracle.mjs')).href
  );
  assert.ok(
    oracle.rakiaAffected(f.seed, ['app/x'], { rakiaBin: tools.RAKIA_BIN }).has('fixture:gate-app')
  );
  assert.equal(oracle.rakiaAffected(f.seed, ['docs/x'], { rakiaBin: tools.RAKIA_BIN }).size, 0);
});

void test("pathWithout('epr') hides epr and keeps node, git and just", () => {
  const path = pathWithout('epr');
  for (const tool of ['node', 'git', 'just']) {
    assert.doesNotThrow(() => {
      const executable = path.split(':').find(directory => {
        try {
          accessSync(join(directory, tool), constants.X_OK);
          return true;
        } catch {
          return false;
        }
      });
      assert.ok(executable);
    });
  }
  assert.equal(
    path.split(':').some(directory => {
      try {
        accessSync(join(directory, 'epr'), constants.X_OK);
        return true;
      } catch {
        return false;
      }
    }),
    false
  );
});

void test('the app gate logs ns, tree and cwd to the absolute common dir, and is red only with left and right', () => {
  const f = seeded({ seed: seed => seedGateClosure(seed, { reuse: ['app'] }) });
  const clone = join(f.base, 'gate-clone');
  must(git(f.base, ['clone', '-q', '-b', 'dev', f.origin, clone]), 'clone gate seed');
  const runGate = () =>
    runSync(
      [
        'just',
        '--justfile',
        join(clone, 'app/justfile'),
        '--working-directory',
        join(clone, 'app'),
        'gate',
      ],
      { cwd: join(clone, 'app') }
    );
  assert.equal(runGate().status, 0);
  writeFileSync(join(clone, 'app/left.txt'), 'left\n');
  assert.equal(runGate().status, 0);
  writeFileSync(join(clone, 'app/right.txt'), 'right\n');
  assert.notEqual(runGate().status, 0);
  const common = must(
    git(clone, ['rev-parse', '--path-format=absolute', '--git-common-dir']),
    'common dir'
  ).stdout.trim();
  const lines = must(
    runSync([
      'node',
      '-e',
      "process.stdout.write(require('fs').readFileSync(process.argv[1]))",
      join(common, 'gate-calls.log'),
    ]),
    'gate log'
  )
    .stdout.trim()
    .split('\n');
  assert.equal(lines.length, 3);
  for (const line of lines) {
    const [ns, tree, cwd] = line.split(' ');
    assert.match(ns, /^\d+$/);
    assert.match(tree, /^[0-9a-f]{40,64}$/);
    assert.equal(cwd, join(clone, 'app'));
  }
});

void test('MARKERS match one sample line each', async () => {
  const { MARKERS } = (await import(
    pathToFileURL(join(repository, 'genesis/orchestrator/gate-landing-markers.mjs')).href
  )) as { MARKERS: Record<string, Marker> };
  const samples: Record<string, string[]> = {
    eprRefused: ['/fixture/epr'],
    storeRefused: ['/fixture/store', 'epr flow intend'],
    intent: ['bafyintent', `I${'a'.repeat(40)}`, 'b'.repeat(12), '1', 'bafyprior'],
    noChangeId: ['c'.repeat(12)],
    degraded: ['not JSON'],
    skipped: ['bafyintent', 'dormant'],
    governance: ['d'.repeat(40), 'reuse'],
    moved: ['e'.repeat(40)],
    claimed: [],
    held: ['3'],
    overrideWait: ['2026-11-01T00:00:00Z', '4'],
    daemonDown: [],
    interrupted: ['bafyintent'],
    dismissed: ['bafyone,bafytwo', 'bafycapsule'],
    gateBanner: [],
  };
  for (const [key, marker] of Object.entries(MARKERS)) {
    const line = marker.format(...samples[key]);
    assert.match(line, marker.re, key);
  }
  assert.doesNotMatch('[pre-push] Phase V: skipped intent bafyintent —', MARKERS.skipped.re);
});
