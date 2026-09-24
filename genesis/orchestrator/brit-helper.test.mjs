/**
 * brit-helper.sh attest — one brit build attestation per pipeline an orchestrator run awaited.
 * Never fails the build. Run: node --test genesis/orchestrator/brit-helper.test.mjs
 *
 * The end-to-end case runs the real brit-build-ref when BRIT_BUILD_REF_TEST_BIN names one
 * (cargo build -p brit-build-ref in elohim/brit), and is skipped otherwise.
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { performerName, planAttestations, rawCid } from './scripts/brit-attest.mjs';
import { attestationRuns, readBuildAttestations, summarize } from './delivery-series.mjs';

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
const ctx = {
  commit: 'c0ffee',
  tree: 'tree123',
  manifestBytes: p => (p === 'elohim-sophia' ? null : MANIFEST),
  machine: { runner: 'jenkins', os: 'linux' },
};
const flag = (args, name) => args[args.indexOf(name) + 1];

test('rawCid is the raw CIDv1 sha2-256 brit computes (the empty-bytes vector)', () => {
  assert.equal(rawCid(Buffer.alloc(0)), 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku');
});

test('one put per awaited pipeline: success, failure, fields; dispatched and manifest-less skipped', () => {
  const { planned, skipped } = planAttestations(graph, ctx);
  assert.deepEqual(planned.map(p => p.pipeline), ['elohim-edge', 'elohim']);
  const [edge, app] = planned.map(p => p.args);
  assert.deepEqual(edge.slice(0, 4), ['build', 'put', '--step', 'elohim-edge']);
  assert.equal(flag(edge, '--success'), 'true');
  assert.equal(flag(app, '--success'), 'false');
  assert.equal(flag(app, '--duration-ms'), '7200000');
  assert.equal(flag(app, '--manifest'), rawCid(MANIFEST));
  assert.equal(flag(app, '--inputs-hash'), 'tree123');
  assert.equal(flag(app, '--commit'), 'c0ffee');
  assert.match(flag(app, '--output'), /^bafkrei/);
  assert.notEqual(flag(app, '--output'), flag(edge, '--output'), 'each run record has its own CID');
  const hw = JSON.parse(flag(app, '--hardware'));
  assert.equal(hw.performer, 'agent:deploy-service-matthew-elohim');
  assert.equal(hw.performer_key, 'unprovisioned', 'the Z.D agent is a name today, not a key');
  assert.deepEqual(
    skipped.map(s => s.pipeline),
    ['elohim-genesis', 'elohim-sophia', 'elohim-steward']
  );
  assert.equal(performerName('elohim-edge', 'eve'), 'agent:deploy-service-eve-elohim-edge');
});

/** A git repo with one commit and one pipeline manifest, as a CI workspace would hold. */
function workspace() {
  const dir = mkdtempSync(join(tmpdir(), 'brit-helper-'));
  const git = (...args) => execFileSync('git', args, { cwd: dir }).toString().trim();
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

function helper(args, env) {
  return spawnSync('sh', [HELPER, ...args], {
    encoding: 'utf8',
    env: { PATH: process.env.PATH, HOME: process.env.HOME, ...env },
  });
}

test('attest without brit-build-ref warns and exits 0', () => {
  const { dir, graphPath } = workspace();
  const r = helper(['attest', graphPath], { REPO_ROOT: dir });
  assert.equal(r.status, 0);
  assert.match(r.stderr, /brit-build-ref not installed; build attestations skipped/);
});

test('attest calls brit-build-ref once per awaited pipeline, and a failing brit never fails the build', () => {
  const { dir, sha, graphPath } = workspace();
  const log = join(dir, 'argv.log');
  const stub = join(dir, 'brit-build-ref-stub');
  writeFileSync(stub, `#!/bin/sh\necho "$*" >> '${log}'\necho bafystub\n`);
  chmodSync(stub, 0o755);
  const ok = helper(['attest', graphPath], { REPO_ROOT: dir, BRIT_BUILD_REF_BIN: stub });
  assert.equal(ok.status, 0, ok.stderr);
  const calls = readFileSync(log, 'utf8').trim().split('\n');
  assert.equal(calls.length, 2);
  assert.match(calls[0], new RegExp(`^--repo ${dir} build put --step elohim-edge .*--success true .*--commit ${sha}$`));
  assert.match(calls[1], /--step elohim .*--success false/);
  assert.match(ok.stderr, /2\/2 pipelines attested/);

  writeFileSync(stub, '#!/bin/sh\necho boom >&2\nexit 101\n');
  const bad = helper(['attest', graphPath], { REPO_ROOT: dir, BRIT_BUILD_REF_BIN: stub });
  assert.equal(bad.status, 0);
  assert.match(bad.stderr, /WARN: build put elohim-edge failed: boom/);

  const noGraph = helper(['attest', join(dir, 'missing.json')], { REPO_ROOT: dir, BRIT_BUILD_REF_BIN: stub });
  assert.equal(noGraph.status, 0);
  assert.match(noGraph.stderr, /missing\.json unreadable .*nothing attested/);
});

const REAL = process.env.BRIT_BUILD_REF_TEST_BIN;

/** CIDv1 bytes → base32 multibase string (brit serializes a CID as its bytes). */
function cidString(bytes) {
  const B32 = 'abcdefghijklmnopqrstuvwxyz234567';
  let bits = 0;
  let value = 0;
  let out = 'b';
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += B32[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += B32[(value << (5 - bits)) & 31];
  return out;
}

test(
  'end to end: real brit-build-ref writes the notes delivery-series --from attestations reads, pushed to a remote',
  { skip: REAL ? false : 'set BRIT_BUILD_REF_TEST_BIN to a built brit-build-ref' },
  () => {
    const { dir, sha, graphPath, git } = workspace();
    const remote = mkdtempSync(join(tmpdir(), 'brit-remote-'));
    execFileSync('git', ['init', '-q', '--bare', remote]);
    git('remote', 'add', 'origin', remote);
    const r = helper(['attest', graphPath], { REPO_ROOT: dir, BRIT_BUILD_REF_BIN: REAL, BRIT_NOTES_REMOTE: 'origin' });
    assert.equal(r.status, 0, r.stderr);
    assert.match(r.stderr, /2\/2 pipelines attested/);
    assert.match(r.stderr, /pushed refs\/notes\/brit\/build\/\* to origin/);

    const note = JSON.parse(git('notes', '--ref', 'refs/notes/brit/build/elohim', 'show', sha));
    assert.equal(note.stepName, 'elohim');
    assert.equal(note.success, false);
    assert.equal(note.buildDurationMs, 7_200_000);
    assert.equal(note.inputsHash, git('rev-parse', `${sha}^{tree}`));
    // brit serializes a CID as its bytes: the same bytes rawCid rendered.
    assert.equal(cidString(note.manifestCid), rawCid(MANIFEST), 'brit parses the CID byte-identically');
    assert.equal(note.hardwareProfile.performer, 'agent:deploy-service-matthew-elohim');
    assert.match(note.agentId, /^[0-9a-f]{64}$/, 'agent_id stays the key that signed');
    assert.match(note.signature, /^[0-9a-f]{128}$/);

    const pushed = execFileSync('git', ['for-each-ref', '--format=%(refname)', 'refs/notes/brit/build/'], { cwd: remote })
      .toString()
      .trim()
      .split('\n');
    assert.deepEqual(pushed.sort(), ['refs/notes/brit/build/elohim', 'refs/notes/brit/build/elohim-edge']);

    const runs = attestationRuns(readBuildAttestations(dir));
    assert.equal(runs.length, 1);
    assert.deepEqual(runs[0].outcome, { elohim: 'FAILURE', 'elohim-edge': 'SUCCESS' });
    assert.equal(summarize(runs).delivered, 0);
  }
);

test('the orchestrator attests once its actual build graph is posted, and never fails on it', () => {
  const jf = readFileSync(join(import.meta.dirname, 'Jenkinsfile'), 'utf8');
  const stage = jf.slice(jf.indexOf("stage('Post Actual Build Graph')"), jf.indexOf("stage('Reconcile Build Graph')"));
  assert.match(stage, /env\.ACTUAL_BUILD_GRAPH_POSTED = 'true'\s*\n\s*sh 'REPO_ROOT="\$WORKSPACE" sh genesis\/orchestrator\/scripts\/brit-helper\.sh attest [^']*\|\| true'/);
});
