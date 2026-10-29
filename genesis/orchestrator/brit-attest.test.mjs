// Device-local Jenkins verdicts and produced closure notes.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { cidOfDigest, planAttestations, rawCid } from './scripts/brit-attest.mjs';
import { attestationRuns, readBuildAttestations } from './delivery-series.mjs';

const CLI = join(import.meta.dirname, 'scripts/brit-attest.mjs');
const HELPER = join(import.meta.dirname, 'scripts/brit-helper.sh');
const MANIFEST = Buffer.from('{"pipeline":"elohim"}\n');
const graph = {
  commitSha: 'c'.repeat(40), buildNumber: 2179,
  executionOrder: ['elohim-edge', 'elohim', 'elohim-genesis', 'elohim-sophia'],
  results: {
    'elohim-edge': { result: 'SUCCESS', duration: 1_800_000, buildNumber: 1760, url: 'https://jenkins/edge/1760/' },
    elohim: { result: 'FAILURE', duration: 7_200_000.4, buildNumber: 1725, url: 'https://jenkins/app/1725/' },
    'elohim-genesis': { result: 'DISPATCHED', duration: 0 },
    'elohim-sophia': { result: 'UNSTABLE', duration: 60_000 },
  },
};
const ctx = { commit: graph.commitSha, tree: 'a'.repeat(40),
  manifestBytes: p => p === 'elohim-sophia' ? null : MANIFEST,
  machine: { node: 'device', os: 'linux', arch: 'x64' } };
const flag = (args, key) => args[args.indexOf(key) + 1];
const K1 = 'sha256:' + '1'.repeat(64), K2 = 'sha256:' + '2'.repeat(64), D1 = 'sha256:' + '3'.repeat(64);

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
  const dir = mkdtempSync(join(tmpdir(), 'brit-helper-'));
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


function invoke(w, bin, extra = {}, args = []) {
  return spawnSync('node', [CLI, '--json', w.graphPath, ...args], { encoding: 'utf8',
    env: { ...fixtureGitEnv(w.dir), REPO_ROOT: w.dir, BRIT_BUILD_REF_BIN: bin, ...extra } });
}
function stub(w) {
  const path = join(w.dir, 'brit-stub');
  writeFileSync(path, `#!/bin/sh
printf '%s\\n' "$*" >> '${w.dir}/calls'
if [ "$4" = get ]; then
  if [ -f '${w.dir}/present' ]; then cat '${w.dir}/present'; else echo 'no build ref found for step=stub commit=stub' >&2; fi
else
  if [ -f '${w.dir}/fail' ]; then echo boom >&2; exit 101; fi
  echo bafystub
fi
`);
  chmodSync(path, 0o755);
  return path;
}

test('one ci: verdict per attestable pipeline; dispatched and manifest-less skipped', () => {
  const { planned, skipped } = planAttestations(graph, ctx);
  assert.deepEqual(planned.map(p => p.step), ['ci:elohim-edge', 'ci:elohim']);
  for (const p of planned) {
    assert.equal(p.kind, 'tree');
    assert.equal(flag(p.args, '--inputs-hash'), ctx.tree);
    const r = graph.results[p.pipeline];
    assert.deepEqual(JSON.parse(flag(p.args, '--output-json')), { pipeline: p.pipeline, commit: ctx.commit,
      result: r.result, buildNumber: r.buildNumber, url: r.url, durationMs: Math.round(r.duration), orchestratorBuild: 2179 });
    const hw = JSON.parse(flag(p.args, '--hardware'));
    assert.equal(hw.runner, 'jenkins');
    assert.equal(hw.performer, 'service:jenkins');
    assert.equal(hw.provenance, r.url);
    assert.equal(hw.inputsHashKind, 'tree');
    assert.deepEqual(hw.writer, ctx.machine);
    assert.doesNotMatch(p.args.join(' '), /performer_key|agent:deploy-service-/);
  }
  assert.equal(flag(planned[1].args, '--success'), 'false');
  assert.deepEqual(skipped.map(s => s.pipeline), ['elohim-genesis', 'elohim-sophia']);
});

test('archived Jenkins buildNumber strings become numeric provenance', () => {
  const { planned } = planAttestations({ ...graph, buildNumber: '2179' }, ctx);
  assert.equal(JSON.parse(flag(planned[0].args, '--hardware')).orchestratorBuild, 2179);
  assert.equal(JSON.parse(flag(planned[0].args, '--output-json')).orchestratorBuild, 2179);
});

test('closure fields fan out per produced step; a hit writes no step note', () => {
  const g = structuredClone(graph);
  const r = g.results['elohim-edge'];
  Object.assign(r, { closure: { 'build-storage': K1, 'build-doorway': K2 },
    outputDigest: { 'build-storage': D1 }, closureHit: { 'build-storage': false, 'build-doorway': true } });
  const { planned, skipped } = planAttestations(g, ctx);
  const step = planned.find(p => p.step === 'ci:elohim-edge:build-storage');
  assert.equal(step.kind, 'closure-v3');
  assert.equal(flag(step.args, '--inputs-hash'), K1);
  assert.equal(flag(step.args, '--output'), cidOfDigest(D1));
  assert.equal(JSON.parse(flag(step.args, '--hardware')).artifact, `harbor.ethosengine.com/ethosengine/elohim-storage@${D1}`);
  assert.ok(!planned.some(p => p.step.endsWith(':build-doorway')));
  assert.ok(skipped.some(s => s.step === 'build-doorway' && s.why === 'closure hit: nothing produced'));
  const output = JSON.parse(flag(planned[0].args, '--output-json'));
  for (const key of ['closure', 'outputDigest', 'closureHit']) assert.deepEqual(output[key], r[key]);
  r.closureHit['build-doorway'] = false;
  assert.ok(planAttestations(g, ctx).skipped.some(s => s.why === 'no digest archived'));
});

test('cidOfDigest wraps sha256 without re-hashing and refuses malformed digests', () => {
  assert.equal(cidOfDigest('sha256:' + createHash('sha256').update('').digest('hex')), rawCid(Buffer.alloc(0)));
  assert.equal(rawCid(Buffer.alloc(0)), 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku');
  for (const digest of ['bad', 'sha256:' + 'A'.repeat(64), 'sha256:aa', null]) assert.throws(() => cidOfDigest(digest));
});

test('same inputsHash is present; different inputsHash is rewritten', () => {
  const w = workspace();
  const bin = stub(w);
  writeFileSync(join(w.dir, 'present'), JSON.stringify({ inputsHash: w.git('rev-parse', `${w.sha}^{tree}`) }));
  const same = invoke(w, bin);
  assert.equal(same.status, 0, same.stderr);
  assert.equal(JSON.parse(same.stdout).present.length, 2);
  assert.doesNotMatch(readFileSync(join(w.dir, 'calls'), 'utf8'), /build put/);
  writeFileSync(join(w.dir, 'present'), JSON.stringify({ inputsHash: 'changed' }));
  const changed = invoke(w, bin);
  assert.equal(JSON.parse(changed.stdout).written.filter(w => w.rewritten).length, 2);
  assert.match(changed.stderr, /rewritten/);
});

test('--json emits one report and exits 0 for absent binary, absent commit, bad graph, failed put', () => {
  const w = workspace();
  const bin = stub(w);
  const absent = invoke(w, join(w.dir, 'absent'));
  assert.equal(absent.status, 0);
  assert.deepEqual(JSON.parse(absent.stdout).deferred, [{ why: 'brit-build-ref absent' }]);
  const noBin = invoke(w, '');
  assert.deepEqual(JSON.parse(noBin.stdout).deferred, [{ why: 'brit-build-ref absent' }]);
  writeFileSync(join(w.dir, 'fail'), '');
  const failed = invoke(w, bin);
  assert.equal(failed.status, 0);
  assert.equal(JSON.parse(failed.stdout).written.length, 0);
  assert.equal(JSON.parse(failed.stdout).deferred.length, 2, 'failed puts must be retried');
  assert.match(failed.stderr, /WARN:.*boom/);
  writeFileSync(w.graphPath, JSON.stringify({ ...graph, commitSha: 'f'.repeat(40) }));
  const missing = invoke(w, bin);
  assert.equal(missing.status, 0);
  assert.equal(JSON.parse(missing.stdout).deferred[0].why, `commit ${'f'.repeat(40)} not in this clone`);
  writeFileSync(w.graphPath, '{');
  const invalid = invoke(w, bin);
  assert.equal(invalid.status, 0);
  assert.equal(JSON.parse(invalid.stdout).deferred.length, 1);
});

test('orchestrator archives its graph and calls no brit helper', () => {
  const jf = readFileSync(join(import.meta.dirname, 'Jenkinsfile'), 'utf8');
  const stage = jf.slice(jf.indexOf("stage('Post Actual Build Graph')"), jf.indexOf("stage('Reconcile Build Graph')"));
  assert.match(stage, /archiveArtifacts\(artifacts: env.ACTUAL_BUILD_GRAPH_FILE/);
  assert.match(stage, /env.ACTUAL_BUILD_GRAPH_POSTED = 'true'/);
  assert.doesNotMatch(stage, /brit-helper.sh/);
});

test('closure helpers sit above pipeline and enrich exactly both result sites without shell bodies', () => {
  const jf = readFileSync(join(import.meta.dirname, 'Jenkinsfile'), 'utf8');
  assert.equal(jf.split('results[name] = withClosureFields(name, result)').length - 1, 2);
  const start = jf.indexOf('def hydrateClosureFields('), end = jf.indexOf('def printDecisionMatrix(');
  assert.ok(start > 0 && end < jf.indexOf('\npipeline {'));
  assert.ok(jf.indexOf('def withClosureFields(') < end);
  assert.doesNotMatch(jf.slice(start, end), /\bsh |"""/);
  assert.match(jf.slice(start, end), /\.edge\.buildEnv/);
  assert.equal(JSON.parse(readFileSync(join(import.meta.dirname, 'build-artifacts.json'))).edge.buildEnv, 'build.env');
});

test('brit-helper has no attest arm', () => {
  const r = spawnSync('sh', [HELPER, 'attest', '/unused'], { encoding: 'utf8' });
  assert.equal(r.status, 64);
  assert.match(r.stderr, /\{verify\|plan\|build-ref\}/);
  assert.doesNotMatch(readFileSync(HELPER, 'utf8'), /attest\)|brit-helper.sh attest/);
});

test('real brit notes stay on the device and a second run preserves builtAt',
  { skip: !process.env.BRIT_BUILD_REF_TEST_BIN }, t => {
    const w = workspace();
    const first = invoke(w, process.env.BRIT_BUILD_REF_TEST_BIN);
    assert.equal(first.status, 0, first.stderr);
    assert.equal(JSON.parse(first.stdout).written.length, 2, first.stderr);
    const note = () => JSON.parse(w.git('notes', '--ref', 'refs/notes/brit/build/ci%3Aelohim', 'show', w.sha));
    const before = note();
    assert.equal(before.stepName, 'ci:elohim');
    assert.equal(before.hardwareProfile.performer, 'service:jenkins');
    assert.equal(before.hardwareProfile.inputsHashKind, 'tree');
    assert.match(before.agentId, /^[0-9a-f]{64}$/);
    const second = invoke(w, process.env.BRIT_BUILD_REF_TEST_BIN);
    assert.equal(JSON.parse(second.stdout).present.length, 2, second.stderr);
    assert.equal(note().builtAt, before.builtAt);
    const runs = attestationRuns(readBuildAttestations(w.dir));
    assert.equal(runs.length, 1);
    assert.equal(runs[0].number, 2179);
    assert.deepEqual(runs[0].outcome, { 'elohim-edge': 'SUCCESS', elohim: 'FAILURE' });
    const args = planAttestations(graph, { ...ctx, commit: w.sha }).planned[0].args;
    const started = performance.now();
    execFileSync(process.env.BRIT_BUILD_REF_TEST_BIN, ['--repo', w.dir, ...args], { env: fixtureGitEnv(w.dir) });
    t.diagnostic(`real build put: ${(performance.now() - started).toFixed(3)} ms (one fixture call, including launcher)`);
  });
